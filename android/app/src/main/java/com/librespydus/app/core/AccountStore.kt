package com.librespydus.app.core

import android.util.Log
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.job
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import uniffi.librespydus_core.AccountSnapshot
import uniffi.librespydus_core.PatronList
import uniffi.librespydus_core.Reservations

/**
 * The signed-in borrower's account (name, loans, both reservation lists) as one `AccountSnapshot`,
 * shown at once and refreshed in the background (stale-while-revalidate, docs/native-apps.md
 * §5.2a). [AppState] reads the selected library's saved copy ([loadSaved]) and binds the store to
 * that library and session with it ([bind]); [refresh] then fetches a new one with
 * `accountSnapshot(session)`.
 *
 * - One refresh at a time: asking while one runs returns the running one. A result that arrives
 *   after the binding changed (library switch, sign-out, sign-in) is dropped.
 * - A failed refresh keeps the snapshot and sets [lastRefreshError]. The error goes through
 *   [report] first, so verification and sign-in prompts still appear (and `NotLoggedIn` ends with
 *   AppState calling [discard]).
 * - A binding with a cache key saves each fresh snapshot to [storage] under that key; demo mode
 *   binds without one, so its data stays in memory.
 * - Every fresh snapshot then goes to [onRefreshed] (the alerts, §5.8), demo or not.
 *
 * Confined to the main thread like AppState: call it from the UI or the app scope.
 */
class AccountStore(
    private val scope: CoroutineScope,
    private val storage: AccountSnapshotStorage,
    private val report: (Throwable) -> UserFacingError,
    private val now: () -> Long = System::currentTimeMillis,
    private val io: CoroutineDispatcher = Dispatchers.IO,
    private val onRefreshed: suspend (backend: LibraryBackend, snapshot: AccountSnapshot) -> Unit = { _, _ -> },
) {
    private class Binding(val backend: LibraryBackend, val session: String, val cacheKey: String?)

    private val _snapshot = MutableStateFlow<AccountSnapshot?>(null)
    private val _isRefreshing = MutableStateFlow(false)
    private val _lastRefreshError = MutableStateFlow<UserFacingError?>(null)

    /** The account to show, saved or fresh. Null while signed out or until the first one arrives. */
    val snapshot: StateFlow<AccountSnapshot?> = _snapshot.asStateFlow()

    /** A refresh is running. */
    val isRefreshing: StateFlow<Boolean> = _isRefreshing.asStateFlow()

    /** Why the last refresh failed. Cleared by a successful refresh and by [bind]. */
    val lastRefreshError: StateFlow<UserFacingError?> = _lastRefreshError.asStateFlow()

    private var binding: Binding? = null

    private var running: Job? = null

    /** The refresh running now, if any (join it to wait for the account to settle). */
    val runningRefresh: Job? get() = running

    /** When the last refresh for this binding started; spaces out automatic retries after a failure. */
    private var lastAttemptAt: Long? = null

    /** Orders saves and deletes, so a save can't land after the sign-out that deleted the file. */
    private val disk = Mutex()

    /**
     * The saved snapshot for [cacheKey], or null (none, or unreadable). Changes nothing: pass it to
     * [bind], which the caller runs together with switching the screens to that library, so the
     * store and the screens never point at different libraries.
     */
    suspend fun loadSaved(cacheKey: String): AccountSnapshot? = withContext(io) { storage.load(cacheKey) }

    /** Deletes the snapshot saved under [cacheKey], after any save of it already under way. */
    suspend fun deleteSaved(cacheKey: String) = disk.withLock { withContext(io) { storage.clear(cacheKey) } }

    /** Deletes every library's saved snapshot (debug `ls_reset`), after any save already under way. */
    suspend fun deleteAllSaved() = disk.withLock { withContext(io) { storage.clearAll() } }

    /**
     * Switches to [session] at [backend]'s library (null: signed out), showing [saved] (from
     * [loadSaved]) until a refresh arrives. With a [cacheKey] fresh snapshots are saved under it;
     * without one (demo mode) nothing touches the disk. A refresh for the previous binding is
     * cancelled; none is started. Not suspending, so AppState publishes the same library and
     * session in the same step.
     */
    fun bind(backend: LibraryBackend?, session: String?, cacheKey: String?, saved: AccountSnapshot? = null) {
        cancelRunning()
        binding = if (backend != null && session != null) Binding(backend, session, cacheKey) else null
        _snapshot.value = saved?.takeIf { binding != null }
        _lastRefreshError.value = null
        lastAttemptAt = null
    }

    /**
     * Signed out, or the session expired: drops the snapshot and deletes this library's saved copy
     * (other libraries keep theirs). The returned job finishes once the file is gone.
     */
    fun discard(): Job {
        val key = binding?.cacheKey
        cancelRunning()
        binding = null
        _snapshot.value = null
        _lastRefreshError.value = null
        lastAttemptAt = null
        return scope.launch {
            if (key != null) deleteSaved(key)
        }
    }

    /**
     * Starts a background refresh, or returns the one already running. Without [force] it starts
     * only when [isDue]. Null when nothing runs (signed out, or not due). Join the job to wait.
     */
    fun refresh(force: Boolean = false): Job? {
        running?.let { return it }
        val bound = binding ?: return null
        if (!force && !isDue()) return null
        return start(bound)
    }

    /**
     * A reservation was placed, cancelled or edited, or a loan renewed: a refresh that was already
     * running may have read the account before the change, so it is replaced by a new one.
     */
    fun refreshAfterChange(): Job? {
        cancelRunning()
        return refresh(force = true)
    }

    /**
     * Takes the reservation with the action link [href] (its cancel link) out of the snapshot
     * straight away, so a cancelled reservation disappears before the refresh that confirms it.
     */
    fun removeReservation(href: String) {
        val bound = binding ?: return
        val updated = _snapshot.value?.withoutReservation(href) ?: return
        _snapshot.value = updated
        bound.cacheKey?.let { key -> scope.launch { save(bound, key, updated) } }
    }

    /**
     * The loan with the action link [href] (its renew link) is now due [due], as the library
     * writes it: shows the new date straight away, before the refresh that confirms it (§5.12).
     */
    fun renewLoan(href: String, due: String) {
        val bound = binding ?: return
        val updated = _snapshot.value?.withLoanDue(href, due) ?: return
        _snapshot.value = updated
        bound.cacheKey?.let { key -> scope.launch { save(bound, key, updated) } }
    }

    /** No snapshot yet, or it is [STALE_AFTER_MS] old (or from the future: the clock moved back). */
    fun isStale(at: Long = now()): Boolean {
        val fetchedAt = _snapshot.value?.fetchedAtMs ?: return true
        return at - fetchedAt !in 0 until STALE_AFTER_MS
    }

    /**
     * An automatic refresh (foreground, opening Your Books or Home) should run: the snapshot is
     * stale and the last attempt, if it failed, wasn't within [STALE_AFTER_MS]. Pull to refresh and
     * Retry force one instead.
     */
    fun isDue(at: Long = now()): Boolean {
        if (!isStale(at)) return false
        val last = lastAttemptAt ?: return true
        return at - last !in 0 until STALE_AFTER_MS
    }

    private fun start(bound: Binding): Job {
        lastAttemptAt = now()
        _isRefreshing.value = true
        val job = scope.launch(start = CoroutineStart.LAZY) {
            val self = coroutineContext.job
            try {
                val fresh = bound.backend.accountSnapshot(bound.session)
                if (binding !== bound || running !== self) return@launch
                _snapshot.value = fresh
                _lastRefreshError.value = null
                bound.cacheKey?.let { save(bound, it, fresh) }
                if (binding === bound) notifyRefreshed(bound, fresh)
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                if (binding !== bound || running !== self) return@launch
                val shown = report(e)
                // NotLoggedIn: report() signed out, which discarded this binding.
                if (binding === bound) _lastRefreshError.value = shown
            } finally {
                if (running === self) {
                    running = null
                    _isRefreshing.value = false
                }
            }
        }
        running = job
        job.start()
        return job
    }

    private suspend fun notifyRefreshed(bound: Binding, snapshot: AccountSnapshot) {
        try {
            withContext(NonCancellable) { onRefreshed(bound.backend, snapshot) }
        } catch (e: Exception) {
            Log.w(TAG, "couldn't process the fresh account snapshot", e)
        }
    }

    private fun cancelRunning() {
        running?.cancel()
        running = null
        _isRefreshing.value = false
    }

    private suspend fun save(bound: Binding, key: String, snapshot: AccountSnapshot) = disk.withLock {
        // Signed out or switched while waiting: the file was deleted, or is another binding's.
        if (binding !== bound) return@withLock
        try {
            withContext(io + NonCancellable) { storage.save(key, snapshot) }
        } catch (e: Exception) {
            Log.w(TAG, "couldn't save the account snapshot", e)
        }
    }

    companion object {
        private const val TAG = "AccountStore"

        /** Automatic refreshes wait until the snapshot is this old (2 minutes). */
        const val STALE_AFTER_MS = 2 * 60 * 1000L
    }
}

/** This snapshot without the reservation whose actions include [href]; null when none has it. */
internal fun AccountSnapshot.withoutReservation(href: String): AccountSnapshot? {
    fun PatronList.without(): PatronList {
        val kept = items.filterNot { item -> item.actions.any { it.href == href } }
        if (kept.size == items.size) return this
        return PatronList(total?.let { maxOf(0L, it - (items.size - kept.size)) }, kept)
    }
    val ready = reservations.ready.without()
    val pending = reservations.pending.without()
    if (ready === reservations.ready && pending === reservations.pending) return null
    return copy(reservations = Reservations(ready, pending))
}

/** This snapshot with the loan whose actions include [href] due [due]; null when none has it (or it already is). */
internal fun AccountSnapshot.withLoanDue(href: String, due: String): AccountSnapshot? {
    var changed = false
    val items = loans.items.map { item ->
        if (item.actions.none { it.href == href } || item.fields["Due"] == due) return@map item
        changed = true
        item.copy(fields = LinkedHashMap(item.fields).apply { this["Due"] = due })
    }
    return if (changed) copy(loans = PatronList(loans.total, items)) else null
}
