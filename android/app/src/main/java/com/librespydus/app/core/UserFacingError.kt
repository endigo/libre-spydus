package com.librespydus.app.core

import uniffi.librespydus_core.SpydusException

/**
 * What a screen shows for a failed call (§5.7). [detail] is the technical message for the
 * error state's disclosure; never shown by default.
 */
data class UserFacingError(
    val kind: Kind,
    val message: String,
    val detail: String? = null,
) {
    enum class Kind {
        /** `Network`: offline, timeout, unreachable library. */
        NETWORK,

        /** `HumanVerificationRequired`: AppState presents the verification sheet. */
        VERIFICATION,

        /** `NotLoggedIn`: AppState clears the session and presents login. */
        SIGN_IN_AGAIN,

        /** `CredentialsRejected` (login sheet). */
        CREDENTIALS,

        /** `OriginForbidden` / `Upstream` / `Parse` / anything unexpected. */
        UNEXPECTED,
    }

    /** True when retrying could help without the user doing anything else first. */
    val isRetryable: Boolean get() = kind == Kind.NETWORK || kind == Kind.UNEXPECTED || kind == Kind.VERIFICATION

    companion object {
        /** [libraryName] fills "Can't reach {library}"; falls back to "the library". */
        fun from(error: Throwable, libraryName: String? = null): UserFacingError {
            val library = libraryName?.takeIf { it.isNotBlank() } ?: "the library"
            return when (error) {
                is SpydusException.Network ->
                    UserFacingError(Kind.NETWORK, "Can't reach $library. Check your connection.", error.message)
                is SpydusException.HumanVerificationRequired ->
                    UserFacingError(Kind.VERIFICATION, "$library asks you to confirm you're human.".capitalised(), error.message)
                is SpydusException.NotLoggedIn ->
                    UserFacingError(Kind.SIGN_IN_AGAIN, "Please sign in again.", error.message)
                is SpydusException.CredentialsRejected ->
                    UserFacingError(Kind.CREDENTIALS, "Card number or PIN not recognised.", error.message)
                is SpydusException ->
                    UserFacingError(Kind.UNEXPECTED, UNEXPECTED, error.message)
                else ->
                    UserFacingError(Kind.UNEXPECTED, UNEXPECTED, error.message ?: error::class.simpleName)
            }
        }

        const val UNEXPECTED = "The library site returned something unexpected."

        private fun String.capitalised() = replaceFirstChar { it.uppercaseChar() }
    }
}
