package com.librespydus.app.feature.detail

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.librespydus.app.core.BookRef
import com.librespydus.app.core.Copy
import com.librespydus.app.core.FormatRef
import com.librespydus.app.core.LibraryBackend
import com.librespydus.app.core.ResultsSort
import com.librespydus.app.core.UserFacingError
import com.librespydus.app.feature.featured.CopyRow
import com.librespydus.app.feature.featured.EditionLevel
import com.librespydus.app.feature.search.BrowseLoadable
import com.librespydus.app.feature.search.BrowseReporter
import com.librespydus.app.feature.search.errorOrNull
import com.librespydus.app.feature.search.isLoading
import com.librespydus.app.feature.search.trimmedOrNull
import com.librespydus.app.feature.search.valueOrNull
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Semaphore
import kotlinx.coroutines.sync.withPermit
import uniffi.librespydus_core.RecordDetail
import uniffi.librespydus_core.RecordRelated
import uniffi.librespydus_core.RelatedList
import uniffi.librespydus_core.SavedBook
import uniffi.librespydus_core.SpydusException

/**
 * Whether the formats came with the ref or are being looked up (docs/native-apps.md §4: formats
 * live on search records only).
 */
sealed interface FormatLookup {
    data object Provided : FormatLookup

    /** Waiting for the record before searching. */
    data object Pending : FormatLookup
    data object Searching : FormatLookup
    data object Found : FormatLookup

    /** No confident match: availability and Reserve are hidden. */
    data object NotFound : FormatLookup
    data class Failed(val error: UserFacingError) : FormatLookup
}

/** The availability line under the byline. */
sealed interface HeaderAvailability {
    data class Loading(val text: String) : HeaderAvailability
    data class Line(val line: AvailabilityLine) : HeaderAvailability
    data object AvailabilityFailed : HeaderAvailability
    data object LookupFailed : HeaderAvailability

    /** No formats matched: "Check availability on the library’s website". */
    data object CheckWebsite : HeaderAvailability
    data object None : HeaderAvailability
}

/** Everything the book detail screen draws. */
data class DetailState(
    /** What the header draws: the opened ref, filled in from the record (bookmark URL, cover, title). */
    val ref: BookRef,
    val record: BrowseLoadable<RecordDetail> = BrowseLoadable.Idle,
    val formats: List<FormatRef> = ref.formats,
    val selectedFormatIndex: Int? = FormatSelection.defaultIndex(ref.formats),
    val formatLookup: FormatLookup = if (ref.formats.isEmpty()) FormatLookup.Pending else FormatLookup.Provided,
    /** Keyed by `availabilityUrl`, so switching back to a format doesn't fetch it again. */
    val availability: Map<String, BrowseLoadable<FormatAvailability>> = emptyMap(),
    val related: Map<RelatedPanel, BrowseLoadable<List<BookRef>>> = emptyMap(),
    /** [BookDetailModel.selectHeldFormat] is still picking the patron's edition. */
    val choosingHeldFormat: Boolean = false,
    /** An edition-level item's copies (§5.10), keyed by holdings URL. */
    val copies: Map<String, BrowseLoadable<List<CopyRow>>> = emptyMap(),
) {
    val selectedFormat: FormatRef? get() = selectedFormatIndex?.let { formats.getOrNull(it) }

    /**
     * One bib with no format tabs (every item of a Featured collection, §5.10): no format chips,
     * no format re-find, and a "Copies" list on the page.
     */
    val isEditionLevel: Boolean get() = EditionLevel.isEditionLevel(formats)

    /** Format chips to pick from: only when there is more than one real format. */
    val showsFormatChips: Boolean get() = formats.size > 1 && !isEditionLevel

    /** The selected format's holdings URL, once its availability has loaded. */
    val holdingsUrl: String?
        get() = selectedFormat?.availabilityUrl.trimmedOrNull()?.let { availability[it]?.valueOrNull?.holdingsUrl }

    /**
     * The "Copies" section of an edition-level item: loading until the availability (then the
     * holdings) arrive; null — no section — for other items, when the availability failed or has no
     * holdings link, and when there are no copies.
     */
    val copiesSection: BrowseLoadable<List<CopyRow>>?
        get() {
            if (!isEditionLevel) return null
            val url = selectedFormat?.availabilityUrl.trimmedOrNull() ?: return null
            val loaded = when (val availabilityState = availability[url] ?: BrowseLoadable.Idle) {
                BrowseLoadable.Idle, BrowseLoadable.Loading -> return BrowseLoadable.Loading
                is BrowseLoadable.Failed -> return null
                is BrowseLoadable.Loaded -> availabilityState.value
            }
            val holdings = loaded.holdingsUrl ?: return null
            return when (val state = copies[holdings] ?: BrowseLoadable.Idle) {
                BrowseLoadable.Idle -> BrowseLoadable.Loading
                is BrowseLoadable.Loaded -> state.takeIf { it.value.isNotEmpty() }
                else -> state
            }
        }

    /**
     * The format "Reserve" reserves: the selected one, when it has a reserve URL (physical, or
     * digital at libraries that take ebook holds).
     */
    val reservableFormat: FormatRef?
        get() {
            if (formatLookup != FormatLookup.Provided && formatLookup != FormatLookup.Found) return null
            return selectedFormat?.takeIf { it.reserveUrl.trimmedOrNull() != null }
        }

    val headerAvailability: HeaderAvailability
        get() {
            when (formatLookup) {
                FormatLookup.Pending -> return if (record.errorOrNull == null) HeaderAvailability.Loading(FINDING) else HeaderAvailability.None
                FormatLookup.Searching -> return HeaderAvailability.Loading(FINDING)
                FormatLookup.NotFound -> return HeaderAvailability.CheckWebsite
                is FormatLookup.Failed -> return HeaderAvailability.LookupFailed
                FormatLookup.Provided, FormatLookup.Found -> Unit
            }
            val format = selectedFormat ?: return HeaderAvailability.None
            val url = format.availabilityUrl.trimmedOrNull()
                ?: return HeaderAvailability.Line(AvailabilityLine.of(null, format))
            return when (val state = availability[url] ?: BrowseLoadable.Idle) {
                BrowseLoadable.Idle, BrowseLoadable.Loading -> HeaderAvailability.Loading(CHECKING)
                is BrowseLoadable.Loaded -> HeaderAvailability.Line(AvailabilityLine.of(state.value.summary, format))
                is BrowseLoadable.Failed -> HeaderAvailability.AvailabilityFailed
            }
        }

    /**
     * The bib IRNs of the format on screen, to look up in the saved account (§6 "Already yours"):
     * the selected format's own bib (its availability's `bibRecordIrn`) once known, else the
     * record's and the opened loan's or reservation's, minus any known to be another format's (so a
     * format whose availability is still loading isn't taken for its neighbour). While the patron's
     * edition is being picked the default format isn't the page's yet, so the opened row's IRN
     * stands and the pill doesn't flicker to Reserve. Editions are separate bibs, so a loan of the
     * book doesn't hide Reserve on the large print.
     */
    val heldIrns: List<String>
        get() {
            fun irnOf(format: FormatRef) =
                format.availabilityUrl.trimmedOrNull()?.let { availability[it]?.valueOrNull?.bibRecordIrn.trimmedOrNull() }
            if (!choosingHeldFormat) selectedFormat?.let(::irnOf)?.let { return listOf(it) }
            val others = formats.filterIndexed { index, _ -> index != selectedFormatIndex }.mapNotNull(::irnOf).toSet()
            return listOfNotNull(record.valueOrNull?.bibIrn.trimmedOrNull(), ref.bibIrn.trimmedOrNull())
                .distinct()
                .filter { it !in others }
        }

    /**
     * The page can't tell yet which edition is on screen: the patron's is being picked, or the
     * selected format's availability (without one, the record) hasn't come back. A patron with
     * loans or reservations sees no Reserve until then, rather than Reserve flashing before
     * "On loan" / "Reserved" replaces it.
     */
    val isIdentifyingEdition: Boolean
        get() {
            if (choosingHeldFormat) return true
            val url = selectedFormat?.availabilityUrl.trimmedOrNull()
            val pending = if (url != null) availability[url] ?: BrowseLoadable.Idle else record
            return pending is BrowseLoadable.Idle || pending is BrowseLoadable.Loading
        }

    /** What a favourite stores: the stable bookmark URL once the record is known, and the formats. */
    val favouriteRef: BookRef get() = ref.copy(formats = formats, savedAt = null)

    val subjects: List<SubjectChip> get() = record.valueOrNull?.let { DetailSubjects.extract(it.details) }.orEmpty()

    val facts: List<DetailFact> get() = record.valueOrNull?.let { DetailFacts.rows(it.details) }.orEmpty()

    val visiblePanels: List<RelatedPanel>
        get() = record.valueOrNull?.let { RelatedPanel.visible(it.related, related) }.orEmpty()

    val hasRelatedPanels: Boolean
        get() = record.valueOrNull?.related?.let { links -> RelatedPanel.entries.any { it.url(links) != null } } ?: false

    companion object {
        const val FINDING = Copy.Detail.findingFormats
        const val CHECKING = Copy.Detail.checkingAvailability
    }
}

/**
 * State for one book detail screen: the record, formats (given or re-found), availability per
 * format and the related panels. Every call goes through the [LibraryBackend] passed in, and
 * failures that need the app (verification, sign-in) go through the [BrowseReporter]. Calls run on
 * the main thread; the screen launches them in [viewModelScope].
 */
class BookDetailModel(
    /** The ref as opened (its URL picks which record to load). */
    val opened: BookRef,
) : ViewModel() {
    private val _state = MutableStateFlow(DetailState(ref = opened))
    val state: StateFlow<DetailState> = _state.asStateFlow()

    /** Renewing from the "On loan" pill (§5.12). */
    val renewal = PillRenewal()

    /** The related links whose panels were requested (a record upgrade brings new ones). */
    private var relatedRequestedFor: RecordRelated? = null

    /**
     * The page saved on the device is on screen and its fresh record hasn't come back (§5.13). Its
     * related links are from an older search set, so the carousels wait for the fresh ones.
     */
    private var showsSavedRecord = false

    /** The saved page was looked for (once, before the first request). */
    private var savedPageRead = false

    /** A fresh record is loading behind the saved one. */
    private var refreshingRecord = false

    /** The carousels came into view while the saved record was on screen. */
    private var relatedWanted = false

    /** Carousels still showing saved (or an earlier record's) items until their fresh list arrives. */
    private var staleRelated: Set<RelatedPanel> = emptySet()

    /** The newest list each carousel loaded (fresh, else saved), unfiltered: what a save stores. */
    private var relatedLists: Map<RelatedPanel, RelatedList> = emptyMap()

    /** The user picked a format: [selectHeldFormat] no longer changes the selection. */
    private var userSelected = false

    fun select(formatIndex: Int) {
        userSelected = true
        _state.update {
            if (formatIndex in it.formats.indices) it.copy(selectedFormatIndex = formatIndex, choosingHeldFormat = false) else it
        }
    }

    /**
     * Loads what's missing: the record, then (for a ref without formats) its formats. Safe to call
     * again after a failure or verification. The page saved on the device shows first (§5.13): its
     * record is then fetched again while the formats are found from it.
     */
    suspend fun load(backend: LibraryBackend, report: BrowseReporter) {
        if (_state.value.record.isLoading || refreshingRecord) return
        showSavedPage(backend)
        if (showsSavedRecord) {
            refreshingRecord = true
            try {
                coroutineScope {
                    launch { loadRecord(backend, report) }
                    loadFormats(backend, report)
                }
            } finally {
                refreshingRecord = false
            }
            return
        }
        if (_state.value.record.valueOrNull == null) loadRecord(backend, report)
        loadFormats(backend, report)
    }

    /** With the record on screen: re-finds a ref's missing formats, then picks the patron's edition. */
    private suspend fun loadFormats(backend: LibraryBackend, report: BrowseReporter) {
        when (_state.value.formatLookup) {
            FormatLookup.Pending, is FormatLookup.Failed -> refindFormats(backend, report)
            else -> Unit
        }
        selectHeldFormat(backend, report)
    }

    /**
     * Loads the record from the first candidate URL that gives one ([DetailRecordSource]). An empty
     * record (a loan's session-set link opened without the patron's OPAC session) counts as a
     * failure, so the next URL is tried.
     */
    suspend fun loadRecord(backend: LibraryBackend, report: BrowseReporter) {
        val urls = DetailRecordSource.candidateUrls(opened, _state.value.selectedFormat)
        if (urls.isEmpty()) {
            val error = UserFacingError(UserFacingError.Kind.UNEXPECTED, Copy.Detail.noRecordLink)
            if (!showsSavedRecord) _state.update { it.copy(record = BrowseLoadable.Failed(error)) }
            return
        }
        if (!showsSavedRecord) _state.update { it.copy(record = BrowseLoadable.Loading) }
        var failure: Exception? = null
        for (url in urls) {
            try {
                val detail = backend.record(url)
                if (DetailRecordSource.isEmpty(detail)) {
                    failure = EmptyRecordException(url)
                    continue
                }
                applyFresh(detail, backend)
                return
            } catch (e: CancellationException) {
                if (!showsSavedRecord) _state.update { it.copy(record = BrowseLoadable.Idle) }
                throw e
            } catch (e: Exception) {
                failure = e
                // A challenge or a lost connection fails the other URL the same way.
                if (e is SpydusException.HumanVerificationRequired || e is SpydusException.Network) break
            }
        }
        if (showsSavedRecord) {
            // The saved page stays on screen, without an error; verification still asks.
            failure?.let(report)
            return
        }
        val shown = failure?.let(report)
        _state.update { it.copy(record = if (shown != null) BrowseLoadable.Failed(shown) else BrowseLoadable.Idle) }
    }

    /** Searches for the record's formats (title + author) and keeps them only on a confident match. */
    suspend fun refindFormats(backend: LibraryBackend, report: BrowseReporter) {
        val current = _state.value
        val detail = current.record.valueOrNull
        if (current.formats.isNotEmpty() || detail == null) return
        val query = FormatRefinder.searchQuery(current.ref, detail)
        if (query == null) {
            _state.update { it.copy(formatLookup = FormatLookup.NotFound) }
            return
        }
        _state.update { it.copy(formatLookup = FormatLookup.Searching) }
        try {
            val results = backend.searchSorted(query, ResultsSort.RELEVANCE)
            val hit = FormatRefinder.match(_state.value.ref, detail, results.records)
            if (hit == null) {
                _state.update { it.copy(formatLookup = FormatLookup.NotFound) }
                return
            }
            val formats = hit.formats.map(FormatRef::from)
            _state.update {
                it.copy(
                    formats = formats,
                    selectedFormatIndex = FormatSelection.defaultIndex(formats),
                    ref = if (it.ref.coverUrl == null) it.ref.copy(coverUrl = hit.coverUrls.firstOrNull()) else it.ref,
                    formatLookup = FormatLookup.Found,
                )
            }
            upgradeWorkLevelRecord(backend)
        } catch (e: CancellationException) {
            _state.update { it.copy(formatLookup = FormatLookup.Pending) }
            throw e
        } catch (e: Exception) {
            val shown = report(e)
            _state.update { it.copy(formatLookup = if (shown != null) FormatLookup.Failed(shown) else FormatLookup.Pending) }
        }
    }

    /**
     * An edition-level item's copies, from its availability's holdings URL (once per URL; again
     * after a failure). Nothing for other items or before the availability is in.
     */
    suspend fun loadCopies(backend: LibraryBackend, report: BrowseReporter) {
        val state = _state.value
        if (!state.isEditionLevel) return
        val url = state.holdingsUrl ?: return
        when (state.copies[url]) {
            is BrowseLoadable.Loaded, BrowseLoadable.Loading -> return
            else -> Unit
        }
        setCopies(url, BrowseLoadable.Loading)
        try {
            setCopies(url, BrowseLoadable.Loaded(CopyRow.rows(backend.holdings(url))))
        } catch (e: CancellationException) {
            setCopies(url, BrowseLoadable.Idle)
            throw e
        } catch (e: Exception) {
            setCopies(url, report(e)?.let { BrowseLoadable.Failed(it) } ?: BrowseLoadable.Idle)
        }
    }

    private fun setCopies(url: String, value: BrowseLoadable<List<CopyRow>>) {
        _state.update { it.copy(copies = it.copies + (url to value)) }
    }

    /** The selected format's availability (once per URL; again after a failure). */
    suspend fun loadAvailability(backend: LibraryBackend, report: BrowseReporter) {
        val url = _state.value.selectedFormat?.availabilityUrl.trimmedOrNull() ?: return
        loadAvailability(url, backend, report)
    }

    private suspend fun loadAvailability(url: String, backend: LibraryBackend, report: BrowseReporter) {
        when (_state.value.availability[url]) {
            is BrowseLoadable.Loaded, BrowseLoadable.Loading -> return
            else -> Unit
        }
        setAvailability(url, BrowseLoadable.Loading)
        try {
            setAvailability(url, BrowseLoadable.Loaded(FormatAvailability.of(backend.availability(url))))
        } catch (e: CancellationException) {
            setAvailability(url, BrowseLoadable.Idle)
            throw e
        } catch (e: Exception) {
            setAvailability(url, report(e)?.let { BrowseLoadable.Failed(it) } ?: BrowseLoadable.Idle)
        }
    }

    private fun setAvailability(url: String, value: BrowseLoadable<FormatAvailability>) {
        _state.update { it.copy(availability = it.availability + (url to value)) }
    }

    /**
     * A page opened from a loan or reservation (the ref knows the row's bib IRN) shows the format
     * the patron has: once the formats are known, every format's availability is loaded (at most
     * [MAX_CONCURRENT] at once, each URL once) and the format whose `bibRecordIrn` is that IRN is
     * selected as soon as it arrives. Without a match the default selection stays; a format the
     * user picked meanwhile is never changed.
     */
    suspend fun selectHeldFormat(backend: LibraryBackend, report: BrowseReporter) {
        val irn = opened.bibIrn.trimmedOrNull() ?: return
        val state = _state.value
        if (state.formats.size < 2 || (state.formatLookup != FormatLookup.Provided && state.formatLookup != FormatLookup.Found)) return
        fun selectIfHeld() {
            if (userSelected) return
            _state.update { current ->
                val index = current.formats.indexOfFirst { format ->
                    format.availabilityUrl.trimmedOrNull()?.let { current.availability[it]?.valueOrNull?.bibRecordIrn } == irn
                }
                if (index >= 0 && index != current.selectedFormatIndex) current.copy(selectedFormatIndex = index) else current
            }
        }
        selectIfHeld()
        if (!userSelected) _state.update { it.copy(choosingHeldFormat = true) }
        try {
            val permits = Semaphore(MAX_CONCURRENT)
            coroutineScope {
                for (url in state.formats.mapNotNull { it.availabilityUrl.trimmedOrNull() }.distinct()) {
                    launch {
                        permits.withPermit { loadAvailability(url, backend, report) }
                        selectIfHeld()
                    }
                }
            }
        } finally {
            _state.update { it.copy(choosingHeldFormat = false) }
        }
    }

    /**
     * Starts loading the related panels once per record, when the carousels come into view. The
     * saved record's links are from an older search set: those wait for the fresh record.
     */
    fun startRelated(backend: LibraryBackend) {
        if (showsSavedRecord) {
            relatedWanted = true
            return
        }
        val links = _state.value.record.valueOrNull?.related ?: return
        if (relatedRequestedFor == links) return
        relatedRequestedFor = links
        viewModelScope.launch { loadRelated(backend) }
    }

    /** After verification or a retry: load panels that failed again (if they were requested). */
    fun retryFailedRelated(backend: LibraryBackend) {
        if (relatedRequestedFor == null || _state.value.related.values.none { it is BrowseLoadable.Failed }) return
        viewModelScope.launch { loadRelated(backend) }
    }

    /**
     * Every related panel the record links to, concurrently (at most four GETs). A failed or empty
     * panel is hidden, never reported; one showing saved items keeps them when it fails. The page is
     * saved again once fresh lists came back.
     */
    suspend fun loadRelated(backend: LibraryBackend) {
        val detail = _state.value.record.valueOrNull ?: return
        val pending = RelatedPanel.entries.mapNotNull { panel ->
            val url = panel.url(detail.related) ?: return@mapNotNull null
            when (_state.value.related[panel]) {
                BrowseLoadable.Loading -> null
                is BrowseLoadable.Loaded -> if (panel in staleRelated) panel to url else null
                else -> panel to url
            }
        }
        if (pending.isEmpty()) return
        val loading = pending.filter { it.first !in staleRelated }.map { it.first to BrowseLoadable.Loading }
        _state.update { state -> state.copy(related = state.related + loading) }
        var loadedAny = false
        coroutineScope {
            for ((panel, url) in pending) {
                launch {
                    fun sameRecord() = _state.value.record.valueOrNull?.related == detail.related
                    val list = try {
                        backend.related(url)
                    } catch (e: CancellationException) {
                        if (panel !in staleRelated) _state.update { it.copy(related = it.related + (panel to BrowseLoadable.Idle)) }
                        throw e
                    } catch (e: Exception) {
                        if (panel !in staleRelated && sameRecord()) {
                            _state.update { it.copy(related = it.related + (panel to BrowseLoadable.Failed(UserFacingError.from(e)))) }
                        }
                        return@launch
                    }
                    if (!sameRecord()) return@launch
                    staleRelated = staleRelated - panel
                    relatedLists = relatedLists + (panel to list)
                    loadedAny = true
                    val items = RelatedPanel.items(list, excludingBrn = detail.brn)
                    _state.update { it.copy(related = it.related + (panel to BrowseLoadable.Loaded(items))) }
                }
            }
        }
        if (loadedAny) savePage(backend)
    }

    /**
     * A work-level record (opened by a work URL) has no subjects, imprint or similar titles; once
     * formats are known, the selected one's `availabilityUrl` loads the full bib record. Kept as
     * is if that fails.
     */
    private suspend fun upgradeWorkLevelRecord(backend: LibraryBackend) {
        val current = _state.value.record.valueOrNull ?: return
        if (!DetailRecordSource.isWorkLevel(current.bookmarkUrl) && !DetailRecordSource.isWorkLevel(opened.url)) return
        val url = _state.value.selectedFormat?.availabilityUrl.trimmedOrNull() ?: return
        val detail = try {
            backend.record(url)
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            return
        }
        if (!DetailRecordSource.isEmpty(detail)) applyFresh(detail, backend)
    }

    /**
     * Shows the page saved on the device for this book (§5.13), once, before the first request: its
     * record, and its lists as the carousels' items until fresh ones arrive.
     */
    private suspend fun showSavedPage(backend: LibraryBackend) {
        if (savedPageRead) return
        savedPageRead = true
        if (_state.value.record.valueOrNull != null) return
        val key = opened.savedPageKey ?: return
        val saved = backend.savedBook(key) ?: return
        if (DetailRecordSource.isEmpty(saved.record) || _state.value.record.valueOrNull != null) return
        apply(saved.record)
        showsSavedRecord = true
        val lists = RelatedPanel.entries.mapNotNull { panel -> panel.list(saved)?.let { panel to it } }.toMap()
        relatedLists = lists
        staleRelated = lists.keys
        val items = lists.mapValues { (_, list) -> BrowseLoadable.Loaded(RelatedPanel.items(list, excludingBrn = saved.record.brn)) }
        _state.update { it.copy(related = it.related + items) }
    }

    /**
     * A record just fetched: shown and saved. A work-level record (no subjects or related titles)
     * never replaces a bib record on screen, saved or loaded by the upgrade path while the saved
     * page's record was refreshing.
     */
    private suspend fun applyFresh(detail: RecordDetail, backend: LibraryBackend) {
        showsSavedRecord = false
        val current = _state.value.record.valueOrNull
        val keepsBib = DetailRecordSource.isWorkLevel(detail.bookmarkUrl) && current != null &&
            !DetailRecordSource.isWorkLevel(current.bookmarkUrl)
        if (!keepsBib) apply(detail)
        if (relatedWanted) {
            relatedWanted = false
            startRelated(backend)
        }
        if (!keepsBib) savePage(backend)
    }

    /**
     * Saves the page on the device (§5.13) after something fresh came back: the record on screen
     * and, for each carousel it links to, the newest list that loaded.
     */
    private suspend fun savePage(backend: LibraryBackend) {
        val key = opened.savedPageKey ?: return
        val detail = _state.value.record.valueOrNull ?: return
        val lists = relatedLists.filterKeys { it.url(detail.related) != null }
        val book = SavedBook(
            record = detail,
            similar = lists[RelatedPanel.SIMILAR],
            author = lists[RelatedPanel.AUTHOR],
            series = lists[RelatedPanel.SERIES],
            versions = lists[RelatedPanel.VERSIONS],
        )
        backend.saveBook(key, book)
    }

    private fun apply(detail: RecordDetail) {
        val state = _state.value
        val current = state.record.valueOrNull
        var related = state.related
        if (current != null && current.related != detail.related) {
            // New links (a fresh record's carry a new search set): carousels with items keep them
            // until their new lists arrive; the others load again.
            staleRelated = related.filterValues { it is BrowseLoadable.Loaded }.keys
            related = related.filterKeys { it in staleRelated }
        }
        _state.update { it.copy(record = BrowseLoadable.Loaded(detail), ref = it.ref.updatedWith(detail), related = related) }
    }

    companion object {
        /** Availability requests in flight at once while looking for the held format (docs §2: ≤ 6). */
        const val MAX_CONCURRENT = 6
    }
}

/** Every candidate URL gave an empty record (shown as "The library’s website returned something unexpected."). */
class EmptyRecordException(url: String) : Exception("The library returned an empty record for $url")
