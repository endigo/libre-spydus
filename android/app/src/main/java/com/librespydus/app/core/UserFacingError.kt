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
        /** [libraryName] fills "Can’t reach {library}"; falls back to "your library". */
        fun from(error: Throwable, libraryName: String? = null): UserFacingError {
            val library = libraryName?.takeIf { it.isNotBlank() } ?: Copy.Common.yourLibrary
            return when (error) {
                is SpydusException.Network ->
                    UserFacingError(Kind.NETWORK, Copy.Errors.network(library), error.message)
                is SpydusException.HumanVerificationRequired ->
                    UserFacingError(Kind.VERIFICATION, Copy.Errors.verification(library).capitalised(), error.message)
                is SpydusException.NotLoggedIn ->
                    UserFacingError(Kind.SIGN_IN_AGAIN, Copy.Errors.signInAgain, error.message)
                is SpydusException.CredentialsRejected ->
                    UserFacingError(Kind.CREDENTIALS, Copy.Errors.credentialsRejected, error.message)
                is SpydusException ->
                    UserFacingError(Kind.UNEXPECTED, UNEXPECTED, error.message)
                else ->
                    UserFacingError(Kind.UNEXPECTED, UNEXPECTED, error.message ?: error::class.simpleName)
            }
        }

        const val UNEXPECTED = Copy.Errors.unexpected

        private fun String.capitalised() = replaceFirstChar { it.uppercaseChar() }
    }
}
