package com.librespydus.app.core

import android.content.SharedPreferences
import kotlinx.serialization.Serializable

/**
 * Signing back in with the phone's fingerprint check (docs/native-apps.md §5.14): the
 * pieces AppState uses, free of Android so the JVM tests can fake them. The prompt and the
 * Keystore are in BiometricsAndroid.kt.
 */
object BiometricText {
    /** `{method}` in the copy: Android always says "fingerprint" (iOS names Face ID / Touch ID). */
    const val METHOD = Copy.Biometrics.fingerprint
}

/** A typed sign-in that succeeded: what [BiometricVault] keeps behind the check. Never logged. */
@Serializable
data class SignInDetails(val username: String, val pin: String) {
    override fun toString() = "SignInDetails(…)" // copy-exempt: debug text that keeps the PIN out of logs

    companion object {
        /** The same sign-in name, case and surrounding spaces aside ("CL114D " = "cl114d"). */
        fun sameUsername(a: String, b: String) = a.trim().equals(b.trim(), ignoreCase = true)
    }
}

/** What reading a library's details gave. */
sealed interface BiometricRead {
    data class Unlocked(val details: SignInDetails) : BiometricRead

    /** Cancelled, "Enter PIN", too many failed tries, or the check can't run now: the details are kept. */
    data object Declined : BiometricRead

    /** The details can't be read any more (a face or finger was added, or they're gone): deleted. */
    data object Invalidated : BiometricRead
}

/**
 * Each library's sign-in details, readable only after a strong biometric check (§5.14). Keyed by
 * library origin. The username alone reads without the check ([username]), so a typed sign-in as
 * someone else can delete the details. [isAvailable] and [username] never show a prompt; [save] and
 * [read] do, on the activity in the foreground, and are cancelled with the coroutine.
 */
interface BiometricVault {
    /** The phone can run a strong biometric check now (one is enrolled and nothing blocks it). */
    fun isAvailable(): Boolean

    /** The username kept with [origin]'s details; null when it has none. */
    fun username(origin: String): String?

    /** Runs the check (Android: encrypting) and keeps [details]; false when the check didn't pass. */
    suspend fun save(origin: String, details: SignInDetails, libraryName: String): Boolean

    /** Runs the check and returns the details. Only called when there are some ([username]). */
    suspend fun read(origin: String, libraryName: String): BiometricRead

    fun delete(origin: String)

    fun deleteAll()

    /** No biometrics (previews, tests that don't care): none of §5.14 shows. */
    object Unavailable : BiometricVault {
        override fun isAvailable() = false
        override fun username(origin: String): String? = null
        override suspend fun save(origin: String, details: SignInDetails, libraryName: String) = false
        override suspend fun read(origin: String, libraryName: String): BiometricRead = BiometricRead.Invalidated
        override fun delete(origin: String) = Unit
        override fun deleteAll() = Unit
    }
}

/** The libraries whose offer was answered Not now (§5.14), until sign-out. Demo mode uses its own scopes. */
interface BiometricOfferStore {
    fun wasDeclined(scope: String): Boolean

    fun markDeclined(scope: String)

    fun forget(scope: String)

    fun clear()
}

class InMemoryBiometricOfferStore : BiometricOfferStore {
    val declined = mutableSetOf<String>()

    override fun wasDeclined(scope: String) = scope in declined
    override fun markDeclined(scope: String) {
        declined += scope
    }
    override fun forget(scope: String) {
        declined -= scope
    }
    override fun clear() = declined.clear()
}

/** [BiometricOfferStore] in the `biometrics` SharedPreferences: `offerDeclined.<sha256(scope)>`. */
class SharedPreferencesBiometricOfferStore(private val prefs: SharedPreferences) : BiometricOfferStore {
    override fun wasDeclined(scope: String) = prefs.getBoolean(key(scope), false)
    override fun markDeclined(scope: String) = prefs.edit().putBoolean(key(scope), true).apply()
    override fun forget(scope: String) = prefs.edit().remove(key(scope)).apply()
    override fun clear() = prefs.edit().clear().apply()

    private fun key(scope: String) = DECLINED + FileAccountSnapshotStorage.sha256(scope)

    companion object {
        const val PREFS_NAME = "biometrics"
        private const val DECLINED = "offerDeclined."
    }
}

/**
 * After a typed sign-in: "Sign in with fingerprint next time?" for [origin]. Holds the PIN
 * in memory only until the reader answers ([AppState.answerBiometricOffer]). [asks] false: the
 * sheet came from Profile's switch, so there's no question; the check just runs, at the same moment
 * the question would have shown (once the sheet has gone and the app is in the foreground).
 */
class BiometricOffer internal constructor(
    val origin: String,
    internal val scope: String,
    val libraryName: String,
    internal val details: SignInDetails,
    val asks: Boolean = true,
) {
    val title: String get() = Copy.Biometrics.offerTitle(BiometricText.METHOD)
    val message: String get() = Copy.Biometrics.offerMessage(libraryName, BiometricText.METHOD)

    override fun toString() = "BiometricOffer($origin)" // copy-exempt: debug text that keeps the PIN out of logs
}

/** [origin] has details kept (no prompt). */
fun BiometricVault.hasDetails(origin: String) = username(origin) != null
