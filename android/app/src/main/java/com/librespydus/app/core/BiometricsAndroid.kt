package com.librespydus.app.core

import android.content.Context
import android.os.Build
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyPermanentlyInvalidatedException
import android.security.keystore.KeyProperties
import android.util.Base64
import android.util.Log
import androidx.biometric.BiometricManager
import androidx.biometric.BiometricManager.Authenticators.BIOMETRIC_STRONG
import androidx.biometric.BiometricPrompt
import androidx.core.content.ContextCompat
import androidx.fragment.app.FragmentActivity
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.LifecycleOwner
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeoutOrNull
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import java.io.File
import java.lang.ref.WeakReference
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec
import kotlin.coroutines.resume

/** How a biometric prompt ended. */
sealed interface PromptOutcome {
    /** The check passed; [crypto] is the unlocked cipher when one was passed in. */
    class Succeeded(val crypto: BiometricPrompt.CryptoObject?) : PromptOutcome

    /** "Enter PIN" (the negative button). */
    data object UsePin : PromptOutcome

    /** Closed by the reader or the system, too many tries, or no strong biometric to use. */
    data class Ended(val code: Int) : PromptOutcome

    /** No activity in the foreground to show it on. */
    data object NotShown : PromptOutcome
}

/**
 * Shows `BiometricPrompt` (`BIOMETRIC_STRONG`, androidx.biometric) on the app's activity (§5.14).
 * [MainActivity][com.librespydus.app.MainActivity] [attach]es itself on create and reports its
 * window's focus; [resumed] and [focused] say whether it is in the foreground with nothing over it
 * (no activity counts as neither), so a check can wait for the screen to be the app's.
 */
class BiometricPrompter(context: Context) {
    private val app = context.applicationContext
    private var activity: WeakReference<FragmentActivity>? = null
    private val _resumed = MutableStateFlow(false)
    private val _focused = MutableStateFlow(false)

    /** The activity is resumed: a prompt can show now. */
    val resumed: StateFlow<Boolean> = _resumed.asStateFlow()

    /**
     * The activity's own window has focus: no sheet or dialog is over it (the app's, or the system's
     * offer to save the password).
     */
    val focused: StateFlow<Boolean> = _focused.asStateFlow()

    fun onWindowFocusChanged(activity: FragmentActivity, hasFocus: Boolean) {
        if (current() === activity) _focused.value = hasFocus
    }

    fun attach(activity: FragmentActivity) {
        this.activity = WeakReference(activity)
        activity.lifecycle.addObserver(object : DefaultLifecycleObserver {
            override fun onResume(owner: LifecycleOwner) {
                if (current() === activity) _resumed.value = true
            }

            override fun onPause(owner: LifecycleOwner) {
                if (current() === activity) _resumed.value = false
            }

            override fun onDestroy(owner: LifecycleOwner) {
                if (current() === activity) {
                    this@BiometricPrompter.activity = null
                    _resumed.value = false
                    _focused.value = false
                }
            }
        })
    }

    fun isAvailable(): Boolean =
        BiometricManager.from(app).canAuthenticate(BIOMETRIC_STRONG) == BiometricManager.BIOMETRIC_SUCCESS

    /**
     * Shows the prompt: title [title], negative button [negative]; [crypto] is unlocked by it. A
     * check started with nothing over the app that the system then cancels because something came
     * over it (the offer to save the password appears a moment after a sign-in) shows again once the
     * app has the screen back, up to [MAX_SHOWS] times in all.
     */
    suspend fun authenticate(title: String, negative: String, crypto: BiometricPrompt.CryptoObject?): PromptOutcome {
        var shows = 0
        while (true) {
            val hadScreen = _resumed.value && _focused.value
            val outcome = show(title, negative, crypto)
            shows++
            val interrupted = outcome is PromptOutcome.Ended && outcome.code == BiometricPrompt.ERROR_CANCELED && hadScreen
            if (!interrupted || shows >= MAX_SHOWS) return outcome
            val back = withTimeoutOrNull(SCREEN_BACK_TIMEOUT_MS) { combine(_resumed, _focused) { r, f -> r && f }.first { it } }
            if (back == null) return outcome
            delay(SETTLE_MS)
        }
    }

    private suspend fun show(title: String, negative: String, crypto: BiometricPrompt.CryptoObject?): PromptOutcome =
        withContext(Dispatchers.Main.immediate) {
            val activity = current()?.takeIf { _resumed.value } ?: return@withContext PromptOutcome.NotShown
            suspendCancellableCoroutine<PromptOutcome> { cont ->
                val callback = object : BiometricPrompt.AuthenticationCallback() {
                    override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) {
                        if (cont.isActive) cont.resume(PromptOutcome.Succeeded(result.cryptoObject))
                    }

                    override fun onAuthenticationError(errorCode: Int, errString: CharSequence) {
                        val outcome = if (errorCode == BiometricPrompt.ERROR_NEGATIVE_BUTTON) PromptOutcome.UsePin else PromptOutcome.Ended(errorCode)
                        if (cont.isActive) cont.resume(outcome)
                    }
                    // onAuthenticationFailed: one try didn't match; the prompt stays up for another.
                }
                val prompt = BiometricPrompt(activity, ContextCompat.getMainExecutor(activity), callback)
                val info = BiometricPrompt.PromptInfo.Builder()
                    .setTitle(title)
                    .setNegativeButtonText(negative)
                    .setAllowedAuthenticators(BIOMETRIC_STRONG)
                    .setConfirmationRequired(false)
                    .build()
                if (crypto != null) prompt.authenticate(info, crypto) else prompt.authenticate(info)
                cont.invokeOnCancellation { activity.runOnUiThread { prompt.cancelAuthentication() } }
            }
        }

    private fun current(): FragmentActivity? = activity?.get()

    private companion object {
        const val MAX_SHOWS = 3
        const val SCREEN_BACK_TIMEOUT_MS = 60_000L
        const val SETTLE_MS = 300L
    }
}

/**
 * [BiometricVault] for real libraries (§5.14): each library's details as JSON, sealed with
 * AES-256-GCM under the Keystore key [keyAlias], which needs a strong biometric check for every use
 * and is invalidated when a face or finger is added. One small JSON file per library
 * (`sha256(origin)`) under `noBackupFilesDir`, so it is never backed up: the username in plain
 * beside Base64(IV ‖ ciphertext+tag), the origin bound in as associated data. Saving and reading
 * pass the cipher through [prompter] as a `CryptoObject`.
 */
class KeystoreBiometricVault(
    context: Context,
    private val prompter: BiometricPrompter,
    private val keyAlias: String = KEY_ALIAS,
) : BiometricVault {
    private val dir = File(context.applicationContext.noBackupFilesDir, DIR_NAME)

    override fun isAvailable() = prompter.isAvailable()

    override fun username(origin: String): String? = stored(origin)?.username

    override suspend fun save(origin: String, details: SignInDetails, libraryName: String): Boolean {
        val cipher = withContext(Dispatchers.IO) { encrypter() } ?: return false
        val outcome = prompter.authenticate(Copy.Biometrics.reason(libraryName), Copy.Biometrics.enterPin, BiometricPrompt.CryptoObject(cipher))
        val unlocked = (outcome as? PromptOutcome.Succeeded)?.crypto?.cipher ?: return false
        return withContext(Dispatchers.IO) {
            try {
                unlocked.updateAAD(origin.toByteArray())
                val sealed = unlocked.iv + unlocked.doFinal(Json.encodeToString(SignInDetails.serializer(), details).toByteArray())
                val stored = Stored(details.username, Base64.encodeToString(sealed, Base64.NO_WRAP))
                dir.mkdirs()
                val target = file(origin)
                val temp = File(dir, "${target.name}.tmp")
                temp.writeText(Json.encodeToString(Stored.serializer(), stored))
                temp.renameTo(target)
            } catch (e: Exception) {
                Log.w(TAG, "couldn't save the sign-in details", e)
                false
            }
        }
    }

    override suspend fun read(origin: String, libraryName: String): BiometricRead {
        val sealed = withContext(Dispatchers.IO) { stored(origin)?.let { runCatching { Base64.decode(it.sealed, Base64.NO_WRAP) }.getOrNull() } }
        if (sealed == null || sealed.size <= IV_BYTES) {
            delete(origin)
            return BiometricRead.Invalidated
        }
        val cipher = try {
            withContext(Dispatchers.IO) {
                Cipher.getInstance(TRANSFORMATION).apply { init(Cipher.DECRYPT_MODE, key(), GCMParameterSpec(TAG_BITS, sealed, 0, IV_BYTES)) }
            }
        } catch (e: KeyPermanentlyInvalidatedException) {
            // A face or finger was added: the key (and every library's details) can't be used again.
            deleteAll()
            return BiometricRead.Invalidated
        } catch (e: Exception) {
            Log.w(TAG, "couldn't open the sign-in details", e)
            delete(origin)
            return BiometricRead.Invalidated
        }
        val outcome = prompter.authenticate(Copy.Biometrics.reason(libraryName), Copy.Biometrics.enterPin, BiometricPrompt.CryptoObject(cipher))
        val unlocked = (outcome as? PromptOutcome.Succeeded)?.crypto?.cipher ?: return BiometricRead.Declined
        return try {
            unlocked.updateAAD(origin.toByteArray())
            val plain = unlocked.doFinal(sealed, IV_BYTES, sealed.size - IV_BYTES)
            BiometricRead.Unlocked(Json.decodeFromString(SignInDetails.serializer(), String(plain)))
        } catch (e: Exception) {
            Log.w(TAG, "couldn't read the sign-in details", e)
            delete(origin)
            BiometricRead.Invalidated
        }
    }

    override fun delete(origin: String) {
        file(origin).delete()
    }

    override fun deleteAll() {
        dir.deleteRecursively()
        runCatching { keyStore().deleteEntry(keyAlias) }
    }

    /** A cipher ready to encrypt; a key invalidated by a new face or finger is replaced first. */
    private fun encrypter(): Cipher? {
        repeat(2) {
            try {
                return Cipher.getInstance(TRANSFORMATION).apply { init(Cipher.ENCRYPT_MODE, key()) }
            } catch (e: KeyPermanentlyInvalidatedException) {
                deleteAll()
            } catch (e: Exception) {
                Log.w(TAG, "can't prepare the sign-in key", e)
                return null
            }
        }
        return null
    }

    @Synchronized
    private fun key(): SecretKey {
        (keyStore().getKey(keyAlias, null) as? SecretKey)?.let { return it }
        val spec = KeyGenParameterSpec.Builder(keyAlias, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
            .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
            .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
            .setKeySize(256)
            .setUserAuthenticationRequired(true)
            .setInvalidatedByBiometricEnrollment(true)
        // A strong biometric check for every use (before API 30 that's the default for this key).
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) spec.setUserAuthenticationParameters(0, KeyProperties.AUTH_BIOMETRIC_STRONG)
        return KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, ANDROID_KEYSTORE).run {
            init(spec.build())
            generateKey()
        }
    }

    private fun keyStore() = KeyStore.getInstance(ANDROID_KEYSTORE).apply { load(null) }

    private fun file(origin: String) = File(dir, FileAccountSnapshotStorage.sha256(origin))

    /** [origin]'s file, or null (none, or unreadable). Small: read on the caller's thread. */
    private fun stored(origin: String): Stored? =
        file(origin).takeIf { it.exists() }?.let { runCatching { Json.decodeFromString(Stored.serializer(), it.readText()) }.getOrNull() }

    /** One library's file: the username in plain (no check needed) and the sealed details. */
    @Serializable
    private class Stored(val username: String, val sealed: String)

    companion object {
        private const val TAG = "KeystoreBiometricVault"
        const val KEY_ALIAS = "librespydus_signin"
        const val DIR_NAME = "signin"
        private const val ANDROID_KEYSTORE = "AndroidKeyStore"
        private const val TRANSFORMATION = "AES/GCM/NoPadding"
        private const val IV_BYTES = 12
        private const val TAG_BITS = 128
    }
}

/**
 * Demo mode's [BiometricVault] (§5.14): the details stay in memory (gone on relaunch), but the real
 * prompt still runs (without a cipher) so the flow can be tried with the demo account.
 */
class PromptedMemoryVault(private val prompter: BiometricPrompter) : BiometricVault {
    private val details = mutableMapOf<String, SignInDetails>()

    override fun isAvailable() = prompter.isAvailable()

    @Synchronized override fun username(origin: String): String? = details[origin]?.username

    override suspend fun save(origin: String, details: SignInDetails, libraryName: String): Boolean {
        val outcome = prompter.authenticate(Copy.Biometrics.reason(libraryName), Copy.Biometrics.enterPin, null)
        if (outcome !is PromptOutcome.Succeeded) return false
        synchronized(this) { this.details[origin] = details }
        return true
    }

    override suspend fun read(origin: String, libraryName: String): BiometricRead {
        val kept = synchronized(this) { details[origin] } ?: return BiometricRead.Invalidated
        val outcome = prompter.authenticate(Copy.Biometrics.reason(libraryName), Copy.Biometrics.enterPin, null)
        return if (outcome is PromptOutcome.Succeeded) BiometricRead.Unlocked(kept) else BiometricRead.Declined
    }

    @Synchronized override fun delete(origin: String) {
        details.remove(origin)
    }

    @Synchronized override fun deleteAll() = details.clear()
}
