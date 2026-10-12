package com.librespydus.app.feature.borrowed

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.librespydus.app.core.AppState
import com.librespydus.app.core.LibraryBackend
import com.librespydus.app.core.UserFacingError
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Job
import kotlinx.coroutines.async
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import uniffi.librespydus_core.ReserveResult
import uniffi.librespydus_core.SpydusException
import java.time.LocalDate
import java.util.concurrent.atomic.AtomicLong

/** A screen section's load state (Borrowed's loans and reservations). */
sealed interface BorrowedLoad<out T> {
    data object Idle : BorrowedLoad<Nothing>
    data object Loading : BorrowedLoad<Nothing>
    data class Loaded<T>(val value: T) : BorrowedLoad<T>
    data class Failed(val error: UserFacingError) : BorrowedLoad<Nothing>

    val valueOrNull: T? get() = (this as? Loaded<T>)?.value
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
 * State for "Your Books": loans and reservations from the core, cancel/edit results, and which
 * rows have a mutation in flight (one at a time per row, so nothing is submitted twice).
 * Plain suspend functions over StateFlows, so it is unit-tested without Android; the screen runs
 * it through [BorrowedViewModel] so loads and mutations outlive a tab switch.
 */
class BorrowedModel(
    private val appState: AppState,
    private val today: () -> LocalDate = LocalDate::now,
    private val now: () -> Long = System::currentTimeMillis,
) {
    enum class Segment(val title: String) { LOANS("Loans"), RESERVATIONS("Reservations") }

    /** A cancel/edit result shown as a banner above the list. */
    data class Notice(val message: String, val isError: Boolean, val id: Long = ids.incrementAndGet())

    /** What a load depends on: a sign-in/out, library switch or verification makes it stale. */
    data class ReloadKey(val backend: LibraryBackend?, val session: String?, val account: Int, val verification: Int)

    private val _segment = MutableStateFlow(Segment.LOANS)
    private val _loans = MutableStateFlow<BorrowedLoad<List<LoanRow>>>(BorrowedLoad.Idle)
    private val _reservations = MutableStateFlow<BorrowedLoad<ReservationSections>>(BorrowedLoad.Idle)
    private val _busyRows = MutableStateFlow<Set<String>>(emptySet())
    private val _notice = MutableStateFlow<Notice?>(null)
    private val _confirmingCancel = MutableStateFlow<ReservationRow?>(null)
    private val _editing = MutableStateFlow<ReservationRow?>(null)

    val segment: StateFlow<Segment> = _segment.asStateFlow()
    val loans: StateFlow<BorrowedLoad<List<LoanRow>>> = _loans.asStateFlow()
    val reservations: StateFlow<BorrowedLoad<ReservationSections>> = _reservations.asStateFlow()

    /** Rows with a cancel or edit in flight. */
    val busyRows: StateFlow<Set<String>> = _busyRows.asStateFlow()
    val notice: StateFlow<Notice?> = _notice.asStateFlow()

    /** The row whose "Cancel" was tapped, awaiting confirmation. */
    val confirmingCancel: StateFlow<ReservationRow?> = _confirmingCancel.asStateFlow()

    /** The row whose edit sheet is shown. */
    val editing: StateFlow<ReservationRow?> = _editing.asStateFlow()

    /**
     * A sign-in or verification error from the edit sheet, reported once the sheet is gone (those
     * are root sheets and shouldn't appear over it).
     */
    private var deferredError: Throwable? = null

    /**
     * Bumped when a cancel or edit starts and ends: a reservations fetch that began before is out
     * of date and is dropped (it could bring a cancelled reservation back).
     */
    private var reservationsVersion = 0

    /** The key of the last load started, and when the last load finished. */
    private var lastKey: ReloadKey? = null
    private var lastLoadAt: Long? = null

    fun reloadKey() = ReloadKey(
        appState.backend.value,
        appState.session.value,
        appState.accountGeneration.value,
        appState.verificationGeneration.value,
    )

    fun select(segment: Segment) {
        _segment.value = segment
    }

    fun dismissNotice(id: Long? = null) {
        if (id == null || _notice.value?.id == id) _notice.value = null
    }

    // region Loading

    /**
     * What the screen should load as it appears: null (nothing), false (a full load: first time,
     * or after a sign-in/out, library switch or verification), or true (a quiet refresh: back on
     * the tab more than 30 s after the last load, e.g. after placing a reservation elsewhere).
     */
    fun pendingLoad(): Boolean? {
        val key = reloadKey()
        if (key != lastKey) return false
        val last = lastLoadAt ?: return null
        return if (key.session != null && now() - last > QUIET_REFRESH_MILLIS) true else null
    }

    /**
     * Loads loans and reservations together. [keepContent] (pull to refresh, quiet refresh) leaves
     * the current lists on screen until the new ones arrive.
     */
    suspend fun load(keepContent: Boolean = false) {
        val key = reloadKey()
        lastKey = key
        val session = key.session
        val backend = key.backend
        if (session == null || backend == null) {
            _loans.value = BorrowedLoad.Idle
            _reservations.value = BorrowedLoad.Idle
            _busyRows.value = emptySet()
            lastLoadAt = now()
            return
        }
        val version = reservationsVersion
        if (!keepContent || _loans.value !is BorrowedLoad.Loaded) _loans.value = BorrowedLoad.Loading
        if (!keepContent || _reservations.value !is BorrowedLoad.Loaded) _reservations.value = BorrowedLoad.Loading

        val (loanResult, reservationResult) = coroutineScope {
            val loans = async { capture { backend.loans(session) } }
            val reservations = async { capture { backend.reservations(session) } }
            loans.await() to reservations.await()
        }
        // A sign-out, library switch or verification while loading: the next load takes over.
        if (reloadKey() != key) return
        lastLoadAt = now()

        val day = today()
        apply(loanResult.map { LoanRow.rows(it.items, day) }, _loans)
        // A lost session was just reported (sign-in sheet); don't report it twice.
        if (appState.session.value != session || reservationsVersion != version) return
        apply(reservationResult.map(ReservationSections::from), _reservations)
    }

    /** Reloads only the reservations, keeping the list on screen (after a cancel or edit). */
    suspend fun reloadReservations() {
        val key = reloadKey()
        val session = key.session ?: return
        val backend = key.backend ?: return
        val version = reservationsVersion
        val result = capture { backend.reservations(session) }
        if (reloadKey() != key || reservationsVersion != version) return
        apply(result.map(ReservationSections::from), _reservations)
    }

    // endregion
    // region Cancel

    fun isBusy(row: ReservationRow): Boolean = row.id in _busyRows.value

    fun askToCancel(row: ReservationRow?) {
        _confirmingCancel.value = row
    }

    /** Cancels [row] after the user confirmed. Ignored while that row is busy or once it's gone. */
    suspend fun cancel(row: ReservationRow) {
        _confirmingCancel.value = null
        val href = row.cancelHref ?: return
        val key = reloadKey()
        val session = key.session ?: return
        val backend = key.backend ?: return
        if (isBusy(row) || _reservations.value.valueOrNull?.row(row.id) == null) return
        _busyRows.value += row.id
        reservationsVersion++
        try {
            val result = backend.cancelReservation(href, session)
            reservationsVersion++
            if (reloadKey() != key) return
            _notice.value = Notice(
                result.message?.trim()?.takeIf { it.isNotEmpty() }
                    ?: if (result.ok) "Reservation cancelled." else "The library didn't cancel this reservation.",
                isError = !result.ok,
            )
            if (result.ok) {
                _reservations.value.valueOrNull?.let { _reservations.value = BorrowedLoad.Loaded(it.removing(row.id)) }
                reloadReservations()
            }
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            reservationsVersion++
            if (reloadKey() != key) return
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
        reservationsVersion++
        _editing.value = row
    }

    /** [row]'s edit sheet finished: show the result and refresh on success. */
    suspend fun finishEdit(outcome: EditOutcome, row: ReservationRow) {
        // A sheet that's already gone (closed while loading) can still finish late: keep only its
        // error, reported now or when the sheet on screen closes.
        if (_editing.value?.id != row.id) {
            if (outcome is EditOutcome.Failed) _notice.value = Notice(report(outcome.error).message, isError = true)
            return
        }
        _editing.value = null
        reservationsVersion++
        when (outcome) {
            EditOutcome.Closed -> Unit
            is EditOutcome.Saved -> {
                val result = outcome.result
                _notice.value = Notice(
                    result.message?.trim()?.takeIf { it.isNotEmpty() } ?: "Reservation updated.",
                    isError = !result.ok,
                )
                if (result.ok) reloadReservations()
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

    private fun <T> apply(result: Result<T>, target: MutableStateFlow<BorrowedLoad<T>>) {
        result.fold(
            onSuccess = { target.value = BorrowedLoad.Loaded(it) },
            onFailure = { target.value = BorrowedLoad.Failed(report(it)) },
        )
    }

    /**
     * `AppState.report`, except while the edit sheet is up: sign-in and verification are root
     * sheets, so those wait until it closes ([reportDeferredError]).
     */
    private fun report(error: Throwable): UserFacingError {
        if (_editing.value == null || !needsRootSheet(error)) return appState.report(error)
        deferredError = error
        return UserFacingError.from(error, appState.library.value?.name)
    }

    private suspend fun <T> capture(work: suspend () -> T): Result<T> = try {
        Result.success(work())
    } catch (e: CancellationException) {
        throw e
    } catch (e: Exception) {
        Result.failure(e)
    }

    companion object {
        const val QUIET_REFRESH_MILLIS = 30_000L
        private val ids = AtomicLong()

        /** Errors `AppState.report` answers with a root sheet (login, verification). */
        fun needsRootSheet(error: Throwable): Boolean =
            error is SpydusException.NotLoggedIn || error is SpydusException.HumanVerificationRequired
    }
}

/**
 * Keeps [BorrowedModel] across tab switches (the tab's composition is disposed when another tab
 * is shown) and runs its work in [viewModelScope], so a load or cancel isn't cut short by leaving
 * the tab. Activity-scoped (see `BorrowedScreen`).
 */
class BorrowedViewModel(appState: AppState) : ViewModel() {
    val model = BorrowedModel(appState)
    private val _refreshing = MutableStateFlow(false)
    private var loadJob: Job? = null

    /** Pull to refresh is running. */
    val refreshing: StateFlow<Boolean> = _refreshing.asStateFlow()

    /** The screen appeared or its inputs changed: load if needed. */
    fun appear() {
        val keepContent = model.pendingLoad() ?: return
        loadJob?.cancel()
        loadJob = viewModelScope.launch { model.load(keepContent) }
    }

    /** Full reload (Retry). */
    fun reload() {
        loadJob?.cancel()
        loadJob = viewModelScope.launch { model.load(keepContent = false) }
    }

    fun pullToRefresh() {
        if (_refreshing.value) return
        _refreshing.value = true
        loadJob?.cancel()
        loadJob = viewModelScope.launch {
            try {
                model.load(keepContent = true)
            } finally {
                _refreshing.value = false
            }
        }
    }

    fun launch(block: suspend BorrowedModel.() -> Unit): Job = viewModelScope.launch { model.block() }
}
