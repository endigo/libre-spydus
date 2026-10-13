package com.librespydus.app.feature.borrowed

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.librespydus.app.core.AppState
import com.librespydus.app.core.BorrowedSegment
import com.librespydus.app.core.Copy
import com.librespydus.app.core.Formatting
import com.librespydus.app.core.LibraryBackend
import com.librespydus.app.core.UserFacingError
import com.librespydus.app.ui.components.ToastTone
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import uniffi.librespydus_core.AccountSnapshot
import uniffi.librespydus_core.ReserveResult
import uniffi.librespydus_core.SpydusException
import java.time.LocalDate
import java.util.concurrent.atomic.AtomicLong

/** A screen section's load state (Your Books' loans and reservations). */
sealed interface BorrowedLoad<out T> {
    data object Idle : BorrowedLoad<Nothing>
    data object Loading : BorrowedLoad<Nothing>
    data class Loaded<T>(val value: T) : BorrowedLoad<T>
    data class Failed(val error: UserFacingError) : BorrowedLoad<Nothing>

    val valueOrNull: T? get() = (this as? Loaded<T>)?.value
}

/**
 * What "Your Books" shows for the account store's state (§5.2a): the lists come from the
 * snapshot, saved or fresh. Only without one is there a loading or error state; a failed refresh
 * with a snapshot keeps the lists and says so in [status].
 */
data class BorrowedContent(
    val loans: BorrowedLoad<List<LoanRow>>,
    val reservations: BorrowedLoad<ReservationSections>,
    /** The line under the segments; null without a snapshot. */
    val status: AccountStatus?,
) {
    companion object {
        fun of(snapshot: AccountSnapshot?, refreshing: Boolean, error: UserFacingError?, today: LocalDate): BorrowedContent {
            if (snapshot == null) {
                val state = if (error != null && !refreshing) BorrowedLoad.Failed(error) else BorrowedLoad.Loading
                return BorrowedContent(state, state, status = null)
            }
            return BorrowedContent(
                loans = BorrowedLoad.Loaded(LoanRow.rows(snapshot.loans.items, today)),
                reservations = BorrowedLoad.Loaded(ReservationSections.from(snapshot.reservations)),
                status = AccountStatus.of(snapshot, refreshing, error),
            )
        }
    }
}

/** The muted line under Your Books' segments, for a snapshot on screen. */
sealed interface AccountStatus {
    /** "Updated 12 min ago" ([Formatting.updatedText]), with a spinner while [refreshing]. */
    data class Updated(val fetchedAtMs: Long, val refreshing: Boolean) : AccountStatus

    /** The last refresh failed: "Couldn’t refresh · Showing the saved list" + Retry. */
    data class RefreshFailed(val error: UserFacingError) : AccountStatus

    companion object {
        const val REFRESH_FAILED = Copy.YourBooks.refreshFailed

        fun of(snapshot: AccountSnapshot, refreshing: Boolean, error: UserFacingError?): AccountStatus =
            if (error != null && !refreshing) RefreshFailed(error) else Updated(snapshot.fetchedAtMs, refreshing)
    }
}

/** How the edit sheet ended. */
sealed interface EditOutcome {
    /** Closed without saving. */
    data object Closed : EditOutcome

    /** The library accepted the change. */
    data class Saved(val result: ReserveResult) : EditOutcome

    /** Signed out or verification needed: reported once the sheet is gone. */
    data class Failed(val error: Throwable) : EditOutcome
}

/**
 * State for "Your Books" around the account store ([AppState.account], which holds the loans and
 * reservations): the segment, cancel/edit/renew results, and which rows have a mutation in flight
 * (one at a time per row, so nothing is submitted twice). A successful cancel takes the row out at
 * once, a renewed loan shows its new due date at once, and every change refreshes the account.
 * Plain suspend functions over StateFlows, so it is unit-tested without Android; the screen runs
 * it through [BorrowedViewModel] so mutations outlive a tab switch.
 */
class BorrowedModel(private val appState: AppState, private val today: () -> LocalDate = LocalDate::now) {
    enum class Segment(val title: String) {
        LOANS(Copy.YourBooks.loans),
        RESERVATIONS(Copy.YourBooks.reservations),
        ;

        companion object {
            fun of(segment: BorrowedSegment): Segment = when (segment) {
                BorrowedSegment.LOANS -> LOANS
                BorrowedSegment.RESERVATIONS -> RESERVATIONS
            }
        }
    }

    /** A cancel/edit result shown as a banner above the list. */
    data class Notice(val message: String, val isError: Boolean, val id: Long = ids.incrementAndGet())

    /** A renewal's progress and result, shown as a toast above the tab bar (§5.12). */
    data class Toast(val message: String, val tone: ToastTone, val id: Long = ids.incrementAndGet())

    /** What a mutation was started for: a sign-in/out or library switch meanwhile makes its result stale. */
    data class MutationKey(val backend: LibraryBackend?, val session: String?)

    private val _segment = MutableStateFlow(Segment.LOANS)
    private val _busyRows = MutableStateFlow<Set<String>>(emptySet())
    private val _notice = MutableStateFlow<Notice?>(null)
    private val _confirmingCancel = MutableStateFlow<ReservationRow?>(null)
    private val _editing = MutableStateFlow<ReservationRow?>(null)
    private val _renewing = MutableStateFlow<Set<String>>(emptySet())
    private val _confirmingRenew = MutableStateFlow<LoanRow?>(null)
    private val _toast = MutableStateFlow<Toast?>(null)

    val segment: StateFlow<Segment> = _segment.asStateFlow()

    /** Rows with a cancel or edit in flight. */
    val busyRows: StateFlow<Set<String>> = _busyRows.asStateFlow()
    val notice: StateFlow<Notice?> = _notice.asStateFlow()

    /** The row whose "Cancel" was tapped, awaiting confirmation. */
    val confirmingCancel: StateFlow<ReservationRow?> = _confirmingCancel.asStateFlow()

    /** The row whose edit sheet is shown. */
    val editing: StateFlow<ReservationRow?> = _editing.asStateFlow()

    /** Loans with a renewal in flight. */
    val renewing: StateFlow<Set<String>> = _renewing.asStateFlow()

    /** The loan whose "Renew" was chosen, awaiting confirmation. */
    val confirmingRenew: StateFlow<LoanRow?> = _confirmingRenew.asStateFlow()

    /** "Renewing…", then the renewal's result. */
    val toast: StateFlow<Toast?> = _toast.asStateFlow()

    /**
     * A sign-in or verification error from the edit sheet, reported once the sheet is gone (those
     * are root sheets and shouldn't appear over it).
     */
    private var deferredError: Throwable? = null

    fun mutationKey() = MutationKey(appState.backend.value, appState.session.value)

    /** The reservations on screen (from the account snapshot), or null before one has arrived. */
    fun reservations(): ReservationSections? =
        appState.account.snapshot.value?.let { ReservationSections.from(it.reservations) }

    /** The loans on screen (from the account snapshot), or null before one has arrived. */
    fun loans(): List<LoanRow>? = appState.account.snapshot.value?.let { LoanRow.rows(it.loans.items, today()) }

    fun select(segment: Segment) {
        _segment.value = segment
    }

    /** Shows the list another screen asked for with [AppState.openBorrowed] (the book page's status pill). */
    fun showPendingSegment() {
        appState.consumePendingBorrowedSegment()?.let { select(Segment.of(it)) }
    }

    fun dismissNotice(id: Long? = null) {
        if (id == null || _notice.value?.id == id) _notice.value = null
    }

    // region Cancel

    fun isBusy(row: ReservationRow): Boolean = row.id in _busyRows.value

    fun askToCancel(row: ReservationRow?) {
        _confirmingCancel.value = row
    }

    /** Cancels [row] after the user confirmed. Ignored while that row is busy or once it's gone. */
    suspend fun cancel(row: ReservationRow) {
        _confirmingCancel.value = null
        val href = row.cancelHref ?: return
        val key = mutationKey()
        val session = key.session ?: return
        val backend = key.backend ?: return
        if (isBusy(row) || reservations()?.row(row.id) == null) return
        _busyRows.value += row.id
        try {
            val result = backend.cancelReservation(href, session)
            if (mutationKey() != key) return
            _notice.value = Notice(
                result.message?.trim()?.takeIf { it.isNotEmpty() }
                    ?: if (result.ok) Copy.YourBooks.cancelled else Copy.YourBooks.cancelFailed,
                isError = !result.ok,
            )
            if (result.ok) {
                appState.account.removeReservation(href)
                appState.accountDidChange()
            }
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            if (mutationKey() != key) return
            _notice.value = Notice(report(e).message, isError = true)
        } finally {
            _busyRows.value -= row.id
        }
    }

    // endregion
    // region Edit

    /** Opens the edit sheet for [row] (not while it's busy). */
    fun edit(row: ReservationRow) {
        if (row.editHref == null || isBusy(row)) return
        _editing.value = row
    }

    /** [row]'s edit sheet finished: show the result and refresh on success. */
    fun finishEdit(outcome: EditOutcome, row: ReservationRow) {
        // A sheet that's already gone (closed while loading) can still finish late: keep only its
        // error, reported now or when the sheet on screen closes.
        if (_editing.value?.id != row.id) {
            if (outcome is EditOutcome.Failed) _notice.value = Notice(report(outcome.error).message, isError = true)
            return
        }
        _editing.value = null
        when (outcome) {
            EditOutcome.Closed -> Unit
            is EditOutcome.Saved -> {
                val result = outcome.result
                _notice.value = Notice(
                    result.message?.trim()?.takeIf { it.isNotEmpty() } ?: Copy.YourBooks.updated,
                    isError = !result.ok,
                )
                if (result.ok) appState.accountDidChange()
            }
            is EditOutcome.Failed -> deferredError = outcome.error
        }
    }

    /** After the edit sheet is gone: route a deferred sign-in/verification error. */
    fun reportDeferredError() {
        val error = deferredError ?: return
        deferredError = null
        _notice.value = Notice(appState.report(error).message, isError = true)
    }

    // endregion
    // region Renew (§5.12)

    fun isRenewing(row: LoanRow): Boolean = row.id in _renewing.value

    /** Asks to confirm renewing [row] (null closes the dialog). Not while it's renewing. */
    fun askToRenew(row: LoanRow?) {
        if (row != null && (row.renewHref == null || isRenewing(row))) return
        _confirmingRenew.value = row
    }

    /**
     * Renews [row] after the reader confirmed. Ignored while that loan is renewing or once it's
     * gone. "Renewing…" until the library answers; then "Renewed · Due 4 Nov" (the new date shows
     * on the row at once), the library's reason when it didn't renew, or "Couldn’t renew this
     * loan" when the call failed (`NotLoggedIn` signs out instead). Either answer refreshes the
     * account, so the row, Home's loans and the due-date reminders follow the library.
     */
    suspend fun renew(row: LoanRow) {
        _confirmingRenew.value = null
        val href = row.renewHref ?: return
        val key = mutationKey()
        val session = key.session ?: return
        val backend = key.backend ?: return
        if (isRenewing(row) || loans()?.any { it.id == row.id } != true) return
        _renewing.value += row.id
        val progress = showToast(Copy.YourBooks.renewing, ToastTone.PROGRESS)
        try {
            val result = backend.renewLoan(href, session)
            if (mutationKey() != key) return
            if (result.ok) {
                showToast(RenewText.renewed(result.due, today()), ToastTone.SUCCESS)
                result.due?.trim()?.takeIf { it.isNotEmpty() }?.let { appState.account.renewLoan(href, it) }
            } else {
                showToast(result.message?.trim()?.takeIf { it.isNotEmpty() } ?: Copy.YourBooks.renewFailed, ToastTone.ERROR)
            }
            appState.accountDidChange()
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            if (mutationKey() != key) return
            appState.report(e)
            // A lost session is the sign-in sheet (§5.2); anything else, the renewal failed.
            if (e !is SpydusException.NotLoggedIn) showToast(Copy.YourBooks.renewFailed, ToastTone.ERROR)
        } finally {
            _renewing.value -= row.id
            dismissToast(progress)
        }
    }

    /** Hides the toast [id] (or whichever is shown when null); a newer one stays. */
    fun dismissToast(id: Long? = null) {
        if (id == null || _toast.value?.id == id) _toast.value = null
    }

    private fun showToast(message: String, tone: ToastTone): Long = Toast(message, tone).also { _toast.value = it }.id

    // endregion

    /**
     * `AppState.report`, except while the edit sheet is up: sign-in and verification are root
     * sheets, so those wait until it closes ([reportDeferredError]).
     */
    private fun report(error: Throwable): UserFacingError {
        if (_editing.value == null || !needsRootSheet(error)) return appState.report(error)
        deferredError = error
        return UserFacingError.from(error, appState.library.value?.name)
    }

    companion object {
        private val ids = AtomicLong()

        /** Errors `AppState.report` answers with a root sheet (login, verification). */
        fun needsRootSheet(error: Throwable): Boolean =
            error is SpydusException.NotLoggedIn || error is SpydusException.HumanVerificationRequired
    }
}

/**
 * Keeps [BorrowedModel] across tab switches (the tab's composition is disposed when another tab
 * is shown) and runs its work in [viewModelScope], so a cancel isn't cut short by leaving the
 * tab. Activity-scoped (see `BorrowedScreen`).
 */
class BorrowedViewModel(private val appState: AppState) : ViewModel() {
    val model = BorrowedModel(appState)
    private val _pulling = MutableStateFlow(false)

    /** Pull to refresh is running (the pull indicator; background refreshes only show the status line). */
    val refreshing: StateFlow<Boolean> = _pulling.asStateFlow()

    /** The screen appeared: refresh the account if it is stale. */
    fun appear() {
        appState.account.refresh()
    }

    /** Retry after a failed refresh. */
    fun retry() {
        appState.account.refresh(force = true)
    }

    fun pullToRefresh() {
        if (_pulling.value) return
        val refresh = appState.account.refresh(force = true) ?: return
        _pulling.value = true
        viewModelScope.launch {
            try {
                refresh.join()
            } finally {
                _pulling.value = false
            }
        }
    }

    fun launch(block: suspend BorrowedModel.() -> Unit): Job = viewModelScope.launch { model.block() }
}
