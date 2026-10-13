package com.librespydus.app.core

import android.content.SharedPreferences
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import uniffi.librespydus_core.RecordDetail
import uniffi.librespydus_core.SearchRecord
import uniffi.librespydus_core.SpydusException

/** What a pick is keyed on (§5.11): the IRN `savePick` / `removePick` take. */
object PickIrn {
    /** A results card (search, a Featured list, the Pick list itself): its `workIrn`. */
    fun of(record: SearchRecord): String? = irn(record.workIrn)

    /**
     * A book page's IRN: a ref keyed `work:<IRN>` / `bib:<IRN>` is that IRN; any other ref (a loan,
     * reservation or related item: `brn:` / `url:` keys) takes the loaded [record]'s `bibIrn`. Null
     * while that record is still loading.
     */
    fun of(ref: BookRef, record: RecordDetail? = null): String? = ofKey(ref.id) ?: irn(record?.bibIrn)

    /** `work:<IRN>` / `bib:<IRN>` → the IRN; null for other keys. */
    fun ofKey(id: String): String? = when {
        id.startsWith(WORK) -> irn(id.removePrefix(WORK))
        id.startsWith(BIB) -> irn(id.removePrefix(BIB))
        else -> null
    }

    /** Trimmed, and only when it is a number (the core rejects anything else). */
    private fun irn(text: String?): String? = text?.trim()?.takeIf { it.isNotEmpty() && it.all(Char::isDigit) }

    private const val WORK = "work:"
    private const val BIB = "bib:"
}

/** My Picks' wording (§5.11), the same on iOS. */
object PicksText {
    const val TITLE = "My Picks"
    const val SUBTITLE = "Saved on your library account"
    const val SAVED = "Saved to My Picks"
    const val REMOVED = "Removed from My Picks"
    const val FAILED = "Couldn't update My Picks."
    const val SAVE_LABEL = "Save to My Picks"
    const val REMOVE_LABEL = "Remove from My Picks"
    const val EMPTY_TITLE = "No picks yet"
    const val EMPTY_MESSAGE = "Tap the bookmark on a book to save it to your library account."
    const val OFFER_TITLE = "Add your saved books to My Picks?"
    const val OFFER_ADD = "Add"
    const val OFFER_NOT_NOW = "Not now"

    /** "3 books saved on this phone can be added to your library account." */
    fun offerMessage(count: Int): String = "$count ${books(count)} saved on this phone can be added to your library account."

    /** "Added 3 books to My Picks", plus " · 1 couldn't be added" when some failed or had no IRN. */
    fun copied(added: Int, failed: Int): String =
        "Added $added ${books(added)} to My Picks" + if (failed > 0) " · $failed couldn't be added" else ""

    private fun books(count: Int) = if (count == 1) "book" else "books"
}

/** A short message over the tabs: a bookmark's confirmation, a failure, the copy's summary. */
data class PicksNotice(val id: Long, val message: String, val isError: Boolean = false)

/** The once-per-library offer to copy this phone's favourites into My Picks (§5.11). */
data class PicksOffer(
    /** The library it is remembered under ([PicksOfferStore]). */
    val scope: String,
    /** The library's local favourites when the reader signed in, newest first. */
    val books: List<BookRef>,
) {
    val message: String get() = PicksText.offerMessage(books.size)
}

/** The Pick list as Favourites shows it while signed in. */
data class PicksList(
    val phase: Phase = Phase.Idle,
    /** Every page loaded so far, without duplicates. */
    val records: List<SearchRecord> = emptyList(),
    /** The page after the last one loaded (paging past the first [PicksStore.MAX_PAGES]). */
    val nextPageUrl: String? = null,
    /** A page after the first is loading (the list's footer spinner). */
    val loadingMore: Boolean = false,
    /** The last page after the first failed: the footer offers Retry. */
    val loadMoreError: UserFacingError? = null,
    /** A reload of a list already shown (pull to refresh). */
    val refreshing: Boolean = false,
) {
    sealed interface Phase {
        data object Idle : Phase
        data object Loading : Phase

        /** Shown; [records] empty is "No picks yet". */
        data object Loaded : Phase
        data class Failed(val error: UserFacingError) : Phase
    }
}

/**
 * Whether the copy offer was already made, per library (either answer counts). The app keeps it in
 * SharedPreferences; tests in memory.
 */
interface PicksOfferStore {
    fun wasAsked(scope: String): Boolean
    fun markAsked(scope: String)

    /** Forgets every library's answer (debug `ls_reset`). */
    fun clear()
}

class InMemoryPicksOfferStore : PicksOfferStore {
    val asked = mutableSetOf<String>()

    override fun wasAsked(scope: String) = scope in asked
    override fun markAsked(scope: String) {
        asked += scope
    }
    override fun clear() = asked.clear()
}

/** [PicksOfferStore] in the `picks` SharedPreferences: `copyOffered.<sha256(scope)>`. */
class SharedPreferencesPicksOfferStore(private val prefs: SharedPreferences) : PicksOfferStore {
    override fun wasAsked(scope: String) = prefs.getBoolean(key(scope), false)
    override fun markAsked(scope: String) = prefs.edit().putBoolean(key(scope), true).apply()
    override fun clear() = prefs.edit().clear().apply()

    private fun key(scope: String) = COPY_OFFERED + FileAccountSnapshotStorage.sha256(scope)

    companion object {
        const val PREFS_NAME = "picks"
        private const val COPY_OFFERED = "copyOffered."
    }
}

/**
 * My Picks (§5.11): while signed in, the reader's Pick list on the library account is what
 * Favourites shows and what the bookmark saves to. One instance in [AppState], bound to the
 * selected library's backend and session ([bind]); everything is dropped when either changes.
 *
 * - [irns]: the picked set, the `workIrn` of every record on the list's first [MAX_PAGES] pages,
 *   loaded by [reload] (after sign-in, at launch while signed in, when Favourites refreshes) and
 *   updated by every save and remove.
 * - [list]: those pages as Favourites shows them, paged further by [loadMore].
 * - [toggle] / [remove]: optimistic; a failure flips back with "Couldn't update My Picks.", and
 *   `NotLoggedIn` goes to [report] (AppState's session-expired path).
 * - [offer]: after an interactive sign-in, the once-per-library offer to copy local favourites.
 *
 * Main-thread confined like the other stores; calls run in [scope], so leaving a screen never
 * cancels a save halfway. A result for an older binding is dropped.
 */
class PicksStore(
    private val scope: CoroutineScope,
    private val offers: PicksOfferStore = InMemoryPicksOfferStore(),
    private val report: (Throwable) -> UserFacingError = { UserFacingError.from(it) },
    private val now: () -> Long = System::currentTimeMillis,
) {
    private var backend: LibraryBackend? = null
    private var session: String? = null

    /** Bumped by [bind]: work for an older binding drops its result. */
    private var generation = 0
    private var loadJob: Job? = null
    private var copyJob: Job? = null
    private var loadedAt: Long? = null

    /** A save since the last load: the list doesn't show it yet. */
    private var stale = false

    /** IRN → true while it is being saved, false while it is being removed. */
    private val inFlight = mutableMapOf<String, Boolean>()
    private var noticeId = 0L

    private val _irns = MutableStateFlow<Set<String>>(emptySet())
    private val _pending = MutableStateFlow<Set<String>>(emptySet())
    private val _list = MutableStateFlow(PicksList())
    private val _notice = MutableStateFlow<PicksNotice?>(null)
    private val _offer = MutableStateFlow<PicksOffer?>(null)

    /** The IRNs on the reader's Pick list (empty while signed out). */
    val irns: StateFlow<Set<String>> = _irns.asStateFlow()

    /** IRNs with a save or remove in flight (a second tap waits for it). */
    val pending: StateFlow<Set<String>> = _pending.asStateFlow()

    val list: StateFlow<PicksList> = _list.asStateFlow()
    val notice: StateFlow<PicksNotice?> = _notice.asStateFlow()
    val offer: StateFlow<PicksOffer?> = _offer.asStateFlow()

    fun isPicked(irn: String?): Boolean = irn != null && irn in _irns.value

    /** Starts over for [backend] and [session] (null: signed out); nothing of the old list is kept. */
    fun bind(backend: LibraryBackend?, session: String?) {
        generation++
        loadJob?.cancel()
        copyJob?.cancel()
        loadJob = null
        copyJob = null
        this.backend = backend
        this.session = session
        loadedAt = null
        stale = false
        inFlight.clear()
        _pending.value = emptySet()
        _irns.value = emptySet()
        _list.value = PicksList()
        _offer.value = null
    }

    /** Signed out, session expired or library changed. */
    fun clear() = bind(null, null)

    /** Forgets which libraries were offered the copy (debug `ls_reset`). */
    fun forgetOffers() = offers.clear()

    // region Loading

    /**
     * Loads the list from its first page, then on to at most [MAX_PAGES] pages, showing each as it
     * arrives. The set keeps what it had until the last page is in, then becomes exactly the list
     * (plus saves still in flight, minus removes in flight). Replaces a load already running.
     */
    fun reload(): Job? {
        val backend = backend ?: return null
        val session = session ?: return null
        loadJob?.cancel()
        val generation = generation
        _list.update {
            if (it.phase == PicksList.Phase.Loaded) it.copy(refreshing = true, loadMoreError = null)
            else PicksList(phase = PicksList.Phase.Loading)
        }
        val job = scope.launch {
            val records = mutableListOf<SearchRecord>()
            val requested = mutableSetOf<String>()
            val before = _irns.value
            var url: String? = null
            var pages = 0
            try {
                while (true) {
                    val page = backend.picks(session, url)
                    if (generation != this@PicksStore.generation) return@launch
                    if (pages == 0 && page.records.isEmpty() && !page.noResults) throw UnrecognisedPage()
                    url?.let(requested::add)
                    pages++
                    records.addDistinct(page.records)
                    val next = page.nextPageUrl?.trim()?.takeIf { it.isNotEmpty() && it !in requested }
                    val more = next != null && pages < MAX_PAGES
                    _list.value = PicksList(
                        phase = PicksList.Phase.Loaded,
                        records = records.withoutRemovesInFlight(),
                        nextPageUrl = next,
                        loadingMore = more,
                    )
                    if (!more) {
                        _irns.value = withInFlight(irnsOf(records))
                        break
                    }
                    _irns.value = withInFlight(before + irnsOf(records))
                    url = next
                }
                loadedAt = now()
                stale = false
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                if (generation != this@PicksStore.generation) return@launch
                val shown = report(e)
                // NotLoggedIn: AppState has cleared the session, and with it this store.
                if (generation != this@PicksStore.generation) return@launch
                when {
                    pages > 0 -> _list.update { it.copy(nextPageUrl = url, loadingMore = false, loadMoreError = shown, refreshing = false) }
                    _list.value.records.isNotEmpty() -> {
                        _list.update { it.copy(refreshing = false) }
                        post(shown.message, isError = true)
                    }
                    else -> _list.value = PicksList(phase = PicksList.Phase.Failed(shown))
                }
            }
        }
        loadJob = job
        return job
    }

    /**
     * Favourites came into view: reloads when the list hasn't loaded (or failed), something was
     * saved since, or it is older than [STALE_AFTER_MS].
     */
    fun refreshIfStale(): Job? {
        if (backend == null || session == null || loadJob?.isActive == true) return null
        val phase = _list.value.phase
        val old = loadedAt?.let { now() - it > STALE_AFTER_MS } ?: true
        return if (stale || old || phase == PicksList.Phase.Idle || phase is PicksList.Phase.Failed) reload() else null
    }

    /** The next page past those loaded (scrolling to the end of a long list). Once at a time. */
    fun loadMore(): Job? {
        val backend = backend ?: return null
        val session = session ?: return null
        val current = _list.value
        val url = current.nextPageUrl ?: return null
        if (current.phase != PicksList.Phase.Loaded || current.loadingMore || current.loadMoreError != null) return null
        if (loadJob?.isActive == true) return null
        val generation = generation
        _list.update { it.copy(loadingMore = true) }
        val job = scope.launch {
            try {
                val page = backend.picks(session, url)
                if (generation != this@PicksStore.generation) return@launch
                val records = _list.value.records.toMutableList().apply { addDistinct(page.records) }
                val next = page.nextPageUrl?.trim()?.takeIf { it.isNotEmpty() && it != url }
                _list.update { it.copy(records = records, nextPageUrl = next, loadingMore = false) }
                _irns.update { it + irnsOf(page.records).filterNot { irn -> inFlight[irn] == false } }
            } catch (e: CancellationException) {
                if (generation == this@PicksStore.generation) _list.update { it.copy(loadingMore = false) }
                throw e
            } catch (e: Exception) {
                if (generation != this@PicksStore.generation) return@launch
                val shown = report(e)
                if (generation != this@PicksStore.generation) return@launch
                _list.update { it.copy(loadingMore = false, loadMoreError = shown) }
            }
        }
        loadJob = job
        return job
    }

    /** The footer's Retry after a page failed. */
    fun retryLoadMore(): Job? {
        _list.update { it.copy(loadMoreError = null) }
        return loadMore()
    }

    // endregion
    // region Save / remove

    /**
     * The bookmark, signed in: saves [irn] when it isn't picked, else removes it. Flips at once;
     * ignored while the same IRN is still being saved or removed.
     */
    fun toggle(irn: String): Job? = change(irn, save = irn !in _irns.value)

    /** Favourites' remove (the row's bookmark or its menu): the row leaves at once. */
    fun remove(irn: String): Job? = change(irn, save = false)

    private fun change(irn: String, save: Boolean): Job? {
        val backend = backend ?: return null
        val session = session ?: return null
        if (irn in inFlight) return null
        val generation = generation
        val index = _list.value.records.indexOfFirst { PickIrn.of(it) == irn }
        val row = _list.value.records.getOrNull(index)?.takeUnless { save }
        inFlight[irn] = save
        _pending.value = inFlight.keys.toSet()
        _irns.update { if (save) it + irn else it - irn }
        if (row != null) _list.update { list -> list.copy(records = list.records.filterNot { PickIrn.of(it) == irn }) }

        fun revert() {
            _irns.update { if (save) it - irn else it + irn }
            if (row != null) {
                _list.update { list ->
                    if (list.records.any { PickIrn.of(it) == irn }) list
                    else list.copy(records = list.records.toMutableList().apply { add(index.coerceAtMost(size), row) })
                }
            }
        }

        return scope.launch {
            try {
                if (save) backend.savePick(irn, session) else backend.removePick(irn, session)
                if (generation != this@PicksStore.generation) return@launch
                if (save) stale = true
                post(if (save) PicksText.SAVED else PicksText.REMOVED)
            } catch (e: CancellationException) {
                if (generation == this@PicksStore.generation) revert()
                throw e
            } catch (e: Exception) {
                if (generation != this@PicksStore.generation) return@launch
                revert()
                if (e is SpydusException.NotLoggedIn) {
                    report(e)
                } else {
                    if (e is SpydusException.HumanVerificationRequired) report(e)
                    post(PicksText.FAILED, isError = true)
                }
            } finally {
                if (generation == this@PicksStore.generation) {
                    inFlight.remove(irn)
                    _pending.value = inFlight.keys.toSet()
                }
            }
        }
    }

    // endregion
    // region Copy local favourites, once

    /**
     * An interactive sign-in (never a restored session) just completed: offers to copy [favourites]
     * (the library's local list) into My Picks, unless it is empty or [scope] was asked before.
     */
    fun offerAfterSignIn(scope: String, favourites: List<BookRef>) {
        if (backend == null || session == null || favourites.isEmpty() || offers.wasAsked(scope)) return
        _offer.value = PicksOffer(scope, favourites)
    }

    /**
     * The reader answered the offer ([add]: Add, else Not now or dismissed). Either way the library
     * is never asked again. Add saves each favourite with an IRN, one at a time, oldest first; a
     * ref without one loads `record(url)` for its `bibIrn`, else is skipped. Then a summary, and
     * the set is reloaded. The local list is kept.
     */
    fun answerOffer(add: Boolean): Job? {
        val offer = _offer.value ?: return null
        _offer.value = null
        offers.markAsked(offer.scope)
        return if (add) copy(offer.books) else null
    }

    private fun copy(books: List<BookRef>): Job? {
        val backend = backend ?: return null
        val session = session ?: return null
        val generation = generation
        val oldestFirst = books.sortedBy { it.savedAt ?: 0L }
        val job = scope.launch {
            var added = 0
            var failed = 0
            for ((index, book) in oldestFirst.withIndex()) {
                val irn = try {
                    PickIrn.of(book) ?: book.url?.trim()?.takeIf { it.isNotEmpty() }?.let { PickIrn.of(book, backend.record(it)) }
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Exception) {
                    null
                }
                if (generation != this@PicksStore.generation) return@launch
                if (irn == null) {
                    failed++
                    continue
                }
                try {
                    backend.savePick(irn, session)
                    if (generation != this@PicksStore.generation) return@launch
                    added++
                    _irns.update { it + irn }
                } catch (e: CancellationException) {
                    throw e
                } catch (e: SpydusException.NotLoggedIn) {
                    if (generation == this@PicksStore.generation) report(e)
                    return@launch
                } catch (e: Exception) {
                    if (generation != this@PicksStore.generation) return@launch
                    if (e is SpydusException.HumanVerificationRequired) {
                        report(e)
                        failed += oldestFirst.size - index
                        break
                    }
                    failed++
                }
            }
            post(PicksText.copied(added, failed), isError = added == 0)
            stale = true
            reload()
        }
        copyJob = job
        return job
    }

    // endregion
    // region Notices

    fun dismissNotice(id: Long) {
        if (_notice.value?.id == id) _notice.value = null
    }

    private fun post(message: String, isError: Boolean = false) {
        _notice.value = PicksNotice(++noticeId, message, isError)
    }

    // endregion

    private fun irnsOf(records: List<SearchRecord>): Set<String> = records.mapNotNullTo(mutableSetOf(), PickIrn::of)

    /** [irns] with the saves in flight added and the removes in flight taken out. */
    private fun withInFlight(irns: Set<String>): Set<String> =
        irns + inFlight.filterValues { it }.keys - inFlight.filterValues { !it }.keys

    private fun List<SearchRecord>.withoutRemovesInFlight(): List<SearchRecord> =
        filterNot { record -> PickIrn.of(record)?.let { inFlight[it] == false } == true }

    /** Appends [incoming] records not already listed (same IRN, else same detail URL). */
    private fun MutableList<SearchRecord>.addDistinct(incoming: List<SearchRecord>) {
        val seen = mapNotNullTo(mutableSetOf(), ::identity)
        for (record in incoming) {
            val key = identity(record)
            if (key == null || seen.add(key)) add(record)
        }
    }

    private fun identity(record: SearchRecord): String? =
        PickIrn.of(record)?.let { "irn:$it" } ?: record.detailUrl?.trim()?.takeIf { it.isNotEmpty() }?.let { "url:$it" }

    /** A first page with no records that isn't the list's own "no items" page. */
    private class UnrecognisedPage : Exception("The Pick list page wasn't recognised (no records and no \"no items\" message).")

    companion object {
        /** Pages the picked set is read from (20 records each). */
        const val MAX_PAGES = 10

        /** Favourites reloads a list older than this when it comes into view. */
        const val STALE_AFTER_MS = 2 * 60 * 1000L
    }
}
