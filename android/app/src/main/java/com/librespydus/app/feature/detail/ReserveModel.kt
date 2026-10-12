package com.librespydus.app.feature.detail

import com.librespydus.app.core.BookRef
import com.librespydus.app.core.FormatRef
import com.librespydus.app.core.LibraryBackend
import com.librespydus.app.core.UserFacingError
import com.librespydus.app.feature.search.BrowseLoadable
import com.librespydus.app.feature.search.trimmedOrNull
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import uniffi.librespydus_core.ReserveOptions
import uniffi.librespydus_core.ReserveResult
import uniffi.librespydus_core.SpydusException

/** What the reserve sheet reserves. */
data class ReserveRequest(val format: FormatRef, val book: BookRef)

/**
 * The reserve sheet's state (docs/native-apps.md §6): `reserveOptions` → pickup location,
 * availability, expiry choice → Confirm → `placeReservation` → result. Workers exercise it in
 * demo mode only (§2): never against a real library.
 *
 * A failure that needs a root sheet (sign in again, verification) goes to `needsRootSheet`: the
 * screen closes the reserve sheet and reports it.
 */
class ReserveModel {
    data class State(
        val options: BrowseLoadable<ReserveOptions> = BrowseLoadable.Idle,
        /** `ExpiryChoice.days` sent as `xdays`; the first choice by default. */
        val selectedDays: String? = null,
        val isSubmitting: Boolean = false,
        /** The library's answer (`ok = false` is data to show, not an error). */
        val result: ReserveResult? = null,
        /** A thrown failure on Confirm (network, unexpected page); Confirm can be tried again. */
        val submitError: UserFacingError? = null,
    ) {
        /** Placed: Confirm is replaced by Done, so a reservation can't be sent twice. */
        val isComplete: Boolean get() = result?.ok == true
    }

    private val _state = MutableStateFlow(State())
    val state: StateFlow<State> = _state.asStateFlow()

    fun selectDays(days: String) {
        _state.update { it.copy(selectedDays = days) }
    }

    suspend fun loadOptions(
        url: String?,
        backend: LibraryBackend,
        session: String?,
        libraryName: String?,
        needsRootSheet: (Throwable) -> Unit,
    ) {
        val reserveUrl = url.trimmedOrNull() ?: run {
            val error = UserFacingError(UserFacingError.Kind.UNEXPECTED, "This format can't be reserved from the app.")
            _state.update { it.copy(options = BrowseLoadable.Failed(error)) }
            return
        }
        if (session == null) {
            needsRootSheet(SpydusException.NotLoggedIn("not logged in"))
            return
        }
        _state.update { it.copy(options = BrowseLoadable.Loading) }
        try {
            val options = backend.reserveOptions(reserveUrl, session)
            _state.update { it.copy(options = BrowseLoadable.Loaded(options), selectedDays = options.expiryChoices.firstOrNull()?.days) }
        } catch (e: CancellationException) {
            _state.update { it.copy(options = BrowseLoadable.Idle) }
            throw e
        } catch (e: Exception) {
            val shown = UserFacingError.from(e, libraryName)
            if (requiresRootSheet(shown)) {
                _state.update { it.copy(options = BrowseLoadable.Idle) }
                needsRootSheet(e)
            } else {
                _state.update { it.copy(options = BrowseLoadable.Failed(shown)) }
            }
        }
    }

    /** Places the reservation once; [onPlaced] runs after the library accepted it. */
    suspend fun confirm(
        url: String?,
        backend: LibraryBackend,
        session: String?,
        libraryName: String?,
        needsRootSheet: (Throwable) -> Unit,
        onPlaced: () -> Unit = {},
    ) {
        val current = _state.value
        val reserveUrl = url.trimmedOrNull()
        if (current.isSubmitting || current.isComplete || reserveUrl == null) return
        if (session == null) {
            needsRootSheet(SpydusException.NotLoggedIn("not logged in"))
            return
        }
        _state.update { it.copy(isSubmitting = true, submitError = null, result = null) }
        try {
            val result = backend.placeReservation(reserveUrl, session, current.selectedDays, null)
            _state.update { it.copy(result = result, isSubmitting = false) }
            if (result.ok) onPlaced()
        } catch (e: CancellationException) {
            _state.update { it.copy(isSubmitting = false) }
            throw e
        } catch (e: Exception) {
            val shown = UserFacingError.from(e, libraryName)
            _state.update { it.copy(isSubmitting = false, submitError = if (requiresRootSheet(shown)) null else shown) }
            if (requiresRootSheet(shown)) needsRootSheet(e)
        }
    }

    companion object {
        /** "RIVERTON LIBRARY" → "Riverton Library"; mixed-case names are left alone. */
        fun titleCased(text: String): String {
            if (text != text.uppercase()) return text
            return text.lowercase().split(' ').joinToString(" ") { word -> word.replaceFirstChar { it.titlecase() } }
        }

        fun message(result: ReserveResult): String =
            result.message.trimmedOrNull() ?: if (result.ok) "Reservation placed." else "The library couldn't place this reservation."

        private fun requiresRootSheet(error: UserFacingError): Boolean =
            error.kind == UserFacingError.Kind.SIGN_IN_AGAIN || error.kind == UserFacingError.Kind.VERIFICATION
    }
}
