package com.librespydus.app.feature.detail

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.librespydus.app.core.BookRef
import com.librespydus.app.core.FormatRef
import com.librespydus.app.core.LibraryBackend
import com.librespydus.app.core.ResultsSort
import com.librespydus.app.core.UserFacingError
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

    /** No formats matched: "Check availability on the library website". */
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
) {
    val selectedFormat: FormatRef? get() = selectedFormatIndex?.let { formats.getOrNull(it) }

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
     * record's and the opened loan's or reservation's. Editions are separate bibs, so a loan of the
     * book doesn't hide Reserve on the large print.
     */
    val heldIrns: List<String>
        get() {
            val url = selectedFormat?.availabilityUrl.trimmedOrNull()
            url?.let { availability[it]?.valueOrNull?.bibRecordIrn }?.let { return listOf(it) }
            return listOfNotNull(record.valueOrNull?.bibIrn.trimmedOrNull(), ref.bibIrn.trimmedOrNull()).distinct()
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
        const val FINDING = "Finding formats…"
        const val CHECKING = "Checking availability…"
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

    /** The related links whose panels were requested (a record upgrade brings new ones). */
    private var relatedRequestedFor: RecordRelated? = null

    /** The user picked a format: [selectHeldFormat] no longer changes the selection. */
    private var userSelected = false

    fun select(formatIndex: Int) {
        userSelected = true
        _state.update { if (formatIndex in it.formats.indices) it.copy(selectedFormatIndex = formatIndex) else it }
    }

    /**
     * Loads what's missing: the record, then (for a ref without formats) its formats. Safe to call
     * again after a failure or verification.
     */
    suspend fun load(backend: LibraryBackend, report: BrowseReporter) {
        if (_state.value.record.isLoading) return
        if (_state.value.record.valueOrNull == null) loadRecord(backend, report)
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
            val error = UserFacingError(UserFacingError.Kind.UNEXPECTED, "This book has no link to the library's record.")
            _state.update { it.copy(record = BrowseLoadable.Failed(error)) }
            return
        }
        _state.update { it.copy(record = BrowseLoadable.Loading) }
        var failure: Exception? = null
        for (url in urls) {
            try {
                val detail = backend.record(url)
                if (DetailRecordSource.isEmpty(detail)) {
                    failure = EmptyRecordException(url)
                    continue
                }
                apply(detail)
                return
            } catch (e: CancellationException) {
                _state.update { it.copy(record = BrowseLoadable.Idle) }
                throw e
            } catch (e: Exception) {
                failure = e
                // A challenge or a lost connection fails the other URL the same way.
                if (e is SpydusException.HumanVerificationRequired || e is SpydusException.Network) break
            }
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
        val permits = Semaphore(MAX_CONCURRENT)
        coroutineScope {
            for (url in state.formats.mapNotNull { it.availabilityUrl.trimmedOrNull() }.distinct()) {
                launch {
                    permits.withPermit { loadAvailability(url, backend, report) }
                    selectIfHeld()
                }
            }
        }
    }

    /** Starts loading the related panels once per record, when the carousels come into view. */
    fun startRelated(backend: LibraryBackend) {
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
     * panel is hidden, never reported.
     */
    suspend fun loadRelated(backend: LibraryBackend) {
        val detail = _state.value.record.valueOrNull ?: return
        val pending = RelatedPanel.entries.mapNotNull { panel ->
            val url = panel.url(detail.related) ?: return@mapNotNull null
            when (_state.value.related[panel]) {
                is BrowseLoadable.Loaded, BrowseLoadable.Loading -> null
                else -> panel to url
            }
        }
        if (pending.isEmpty()) return
        _state.update { state -> state.copy(related = state.related + pending.map { it.first to BrowseLoadable.Loading }) }
        coroutineScope {
            for ((panel, url) in pending) {
                launch {
                    val result: BrowseLoadable<List<BookRef>> = try {
                        BrowseLoadable.Loaded(RelatedPanel.items(backend.related(url), excludingBrn = detail.brn))
                    } catch (e: CancellationException) {
                        _state.update { it.copy(related = it.related + (panel to BrowseLoadable.Idle)) }
                        throw e
                    } catch (e: Exception) {
                        BrowseLoadable.Failed(UserFacingError.from(e))
                    }
                    _state.update { state ->
                        if (state.record.valueOrNull?.related == detail.related) state.copy(related = state.related + (panel to result))
                        else state
                    }
                }
            }
        }
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
        if (!DetailRecordSource.isEmpty(detail)) apply(detail)
    }

    private fun apply(detail: RecordDetail) {
        _state.update { state ->
            val current = state.record.valueOrNull
            state.copy(
                record = BrowseLoadable.Loaded(detail),
                ref = state.ref.updatedWith(detail),
                related = if (current != null && current.related != detail.related) emptyMap() else state.related,
            )
        }
    }

    companion object {
        /** Availability requests in flight at once while looking for the held format (docs §2: ≤ 6). */
        const val MAX_CONCURRENT = 6
    }
}

/** Every candidate URL gave an empty record (shown as "The library site returned something unexpected."). */
class EmptyRecordException(url: String) : Exception("The library returned an empty record for $url")
