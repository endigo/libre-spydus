package com.librespydus.app.core

import android.content.Context
import android.content.SharedPreferences
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/** Small string secrets (session tokens, verification cookies) keyed by name. */
interface SecretStore {
    fun get(key: String): String?

    /** Stores [value], or removes the entry when it is null. */
    fun put(key: String, value: String?)

    fun clear()
}

/**
 * [SecretStore] over SharedPreferences, each value encrypted with AES-256-GCM under a
 * non-exportable Android Keystore key (androidx.security-crypto is deprecated). Stored as
 * Base64(iv ‖ ciphertext+tag); the entry name is bound in as associated data so a value can't be
 * moved to another entry. A value that no longer decrypts (key invalidated, data restored onto
 * another device) is dropped and reads as absent.
 *
 * Blocking (Keystore + prefs): call off the main thread.
 */
class KeystoreSecretStore(
    context: Context,
    prefsName: String = PREFS_NAME,
    private val keyAlias: String = KEY_ALIAS,
) : SecretStore {
    private val prefs: SharedPreferences = context.getSharedPreferences(prefsName, Context.MODE_PRIVATE)

    @Synchronized
    override fun get(key: String): String? {
        val stored = prefs.getString(key, null) ?: return null
        return runCatching { decrypt(key, stored) }.getOrElse {
            prefs.edit().remove(key).apply()
            null
        }
    }

    @Synchronized
    override fun put(key: String, value: String?) {
        if (value == null) prefs.edit().remove(key).apply()
        else prefs.edit().putString(key, encrypt(key, value)).apply()
    }

    @Synchronized
    override fun clear() {
        prefs.edit().clear().apply()
    }

    private fun encrypt(name: String, plain: String): String {
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.ENCRYPT_MODE, key())
        cipher.updateAAD(name.toByteArray())
        val sealed = cipher.doFinal(plain.toByteArray())
        return Base64.encodeToString(cipher.iv + sealed, Base64.NO_WRAP)
    }

    private fun decrypt(name: String, stored: String): String {
        val bytes = Base64.decode(stored, Base64.NO_WRAP)
        require(bytes.size > IV_BYTES) { "truncated value" }
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.DECRYPT_MODE, key(), GCMParameterSpec(TAG_BITS, bytes, 0, IV_BYTES))
        cipher.updateAAD(name.toByteArray())
        return String(cipher.doFinal(bytes, IV_BYTES, bytes.size - IV_BYTES))
    }

    private fun key(): SecretKey {
        val keyStore = KeyStore.getInstance(ANDROID_KEYSTORE).apply { load(null) }
        (keyStore.getKey(keyAlias, null) as? SecretKey)?.let { return it }
        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, ANDROID_KEYSTORE)
        generator.init(
            KeyGenParameterSpec.Builder(keyAlias, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256)
                .build(),
        )
        return generator.generateKey()
    }

    companion object {
        /** Excluded from backup in res/xml (the key never leaves this device). */
        const val PREFS_NAME = "librespydus_secure"
        const val KEY_ALIAS = "librespydus_secrets"
        private const val ANDROID_KEYSTORE = "AndroidKeyStore"
        private const val TRANSFORMATION = "AES/GCM/NoPadding"
        private const val IV_BYTES = 12
        private const val TAG_BITS = 128
    }
}

/** In-memory [SecretStore] for tests and previews. */
class InMemorySecretStore : SecretStore {
    private val values = mutableMapOf<String, String>()

    @Synchronized override fun get(key: String) = values[key]

    @Synchronized override fun put(key: String, value: String?) {
        if (value == null) values.remove(key) else values[key] = value
    }

    @Synchronized override fun clear() = values.clear()
}
