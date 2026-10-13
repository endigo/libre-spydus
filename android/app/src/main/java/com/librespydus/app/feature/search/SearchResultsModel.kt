package com.librespydus.app.feature.search

import androidx.lifecycle.ViewModel
import com.librespydus.app.core.BookRef
import com.librespydus.app.core.CoreHelpers
import com.librespydus.app.core.FormatRef
import com.librespydus.app.core.LibraryBackend
import com.librespydus.app.core.ResultsSort
import com.librespydus.app.core.UserFacingError
import com.librespydus.app.feature.featured.EditionLevel
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import uniffi.librespydus_core.SearchRecord

/**
 * One search, paged: the first page from `searchSorted` (or, for a [Request.url], `page(url)`),
 * later pages from `nextPageUrl` → `page` (infinite scroll). Shared by the Search tab,
 * [SearchResultsScreen] and [ListResultsScreen].
 *
 * Records are merged without duplicates (`workIrn`, else `detailUrl`); each page URL is fetched at
 * most once, and a result that arrives after a newer search started is dropped. Calls run on the
 * main thread (the screens launch them in [androidx.lifecycle.viewModelScope], so pushing a
 * detail screen mid-load doesn't cancel them).
 */
class SearchResultsModel : ViewModel() {
    /**
     * What a load is for. Any change (query, sort, library, verification, a retry's [attempt])
     * starts a fresh search, while the same request again (back from a detail screen) keeps the
     * pages already loaded.
     */
    data class Request(
        val query: String,
        val sort: ResultsSort,
        val origin: String,
        val verification: Int,
        val attempt: Int = 0,
        /**
         * A results list to start from instead of [query] (§5.10: a collection section, a Featured
         * saved search); [sort] doesn't apply.
         */
        val url: String? = null,
    )

    sealed interface Phase {
        data object Idle : Phase
        data object Loading : Phase
        data object Loaded : Phase

        /** The OPAC's own "did not find any records" page. */
        data object NoResults : Phase
        data class Failed(val error: UserFacingError) : Phase
    }

    data class State(
        val phase: Phase = Phase.Idle,
        val items: List<SearchResultItem> = emptyList(),
        /** `totalResults` from the first page. */
        val total: Long? = null,
        val nextPageUrl: String? = null,
        val isLoadingMore: Boolean = false,
        /** The last next-page failure: the footer offers Retry and automatic paging stops. */
        val loadMoreError: UserFacingError? = null,
        val request: Request? = null,
    ) {
        val hasMore: Boolean get() = nextPageUrl != null
    }

    private val _state = MutableStateFlow(State())
    val state: StateFlow<State> = _state.asStateFlow()

    private var backend: LibraryBackend? = null
    private var report: BrowseReporter = { null }

    /** Bumped by every new search: late answers for an older one are dropped. */
    private var generation = 0
    private val requestedPages = mutableSetOf<String>()

    /** The request whose load finished (success, no results or failure). */
    private var settledRequest: Request? = null

    /** The request whose first page is being fetched. */
    private var loadingRequest: Request? = null

    /**
     * Runs [request]'s first page, unless it's the request already shown or already loading (the
     * screen asks again when it comes back into view, e.g. after a tab switch mid-load).
     */
    suspend fun load(request: Request, backend: LibraryBackend, report: BrowseReporter) {
        if (request == loadingRequest) return
        if (request == settledRequest && _state.value.phase != Phase.Idle) return
        val generation = ++generation
        loadingRequest = request
        try {
            loadFirstPage(request, generation, backend, report)
        } finally {
            if (loadingRequest == request) loadingRequest = null
        }
    }

    private suspend fun loadFirstPage(request: Request, generation: Int, backend: LibraryBackend, report: BrowseReporter) {
        this.backend = backend
        this.report = report
        settledRequest = null
        requestedPages.clear()
        _state.value = State(phase = Phase.Loading, request = request)

        try {
            val page = request.url?.let { url ->
                requestedPages += url
                backend.page(url)
            } ?: backend.searchSorted(request.query, request.sort)
            if (generation != this.generation) return
            settledRequest = request
            if (page.records.isEmpty()) {
                val phase = if (page.noResults) Phase.NoResults else Phase.Failed(UNRECOGNISED_PAGE)
                _state.update { it.copy(phase = phase) }
                return
            }
            _state.update {
                it.copy(
                    phase = Phase.Loaded,
                    items = merge(emptyList(), page.records),
                    total = page.totalResults,
                    nextPageUrl = page.nextPageUrl.trimmedOrNull(),
                )
            }
        } catch (e: CancellationException) {
            // A cancelled load stays Loading and runs again next time.
            throw e
        } catch (e: Exception) {
            if (generation != this.generation) return
            val shown = report(e) ?: return
            settledRequest = request
            _state.update { it.copy(phase = Phase.Failed(shown)) }
        }
    }

    /**
     * Fetches the next page, once. Ignored while a page is loading, after a page failed (use
     * [retryLoadMore]), and at the end of the results.
     */
    suspend fun loadMore() {
        if (_state.value.loadMoreError != null) return
        fetchNextPage()
    }

    suspend fun retryLoadMore() {
        _state.update { it.copy(loadMoreError = null) }
        fetchNextPage()
    }

    private suspend fun fetchNextPage() {
        val current = _state.value
        val backend = backend ?: return
        val url = current.nextPageUrl ?: return
        if (current.phase != Phase.Loaded || current.isLoadingMore || url in requestedPages) return
        val generation = generation
        requestedPages += url
        _state.update { it.copy(isLoadingMore = true) }
        try {
            val page = backend.page(url)
            if (generation != this.generation) return
            if (page.records.isEmpty()) {
                if (page.noResults) {
                    _state.update { it.copy(nextPageUrl = null, isLoadingMore = false) }
                } else {
                    requestedPages -= url
                    _state.update { it.copy(loadMoreError = UNRECOGNISED_PAGE, isLoadingMore = false) }
                }
                return
            }
            // A next link that points back at a page already fetched would loop forever.
            val next = page.nextPageUrl.trimmedOrNull()?.takeUnless { it in requestedPages }
            _state.update { it.copy(items = merge(it.items, page.records), nextPageUrl = next, isLoadingMore = false) }
        } catch (e: CancellationException) {
            if (generation == this.generation) {
                requestedPages -= url
                _state.update { it.copy(isLoadingMore = false) }
            }
            throw e
        } catch (e: Exception) {
            if (generation != this.generation) return
            requestedPages -= url
            val shown = report(e)
            _state.update { it.copy(isLoadingMore = false, loadMoreError = shown ?: it.loadMoreError) }
        }
    }

    companion object {
        /** A page with no records that isn't the OPAC's "no records" page. */
        val UNRECOGNISED_PAGE = UserFacingError(
            kind = UserFacingError.Kind.UNEXPECTED,
            message = UserFacingError.UNEXPECTED,
            detail = "The results page wasn't recognised (no records and no \"did not find any records\" message).", // copy-exempt: technical detail under Details
        )

        /** [existing] followed by the records of [incoming] not already listed, in order. */
        fun merge(existing: List<SearchResultItem>, incoming: List<SearchRecord>): List<SearchResultItem> {
            val merged = existing.toMutableList()
            val seen = existing.mapNotNullTo(mutableSetOf()) { it.key }
            for (record in incoming) {
                val key = dedupeKey(record)
                if (key != null && !seen.add(key)) continue
                merged += SearchResultItem.from(record, key, merged.size)
            }
            return merged
        }

        /** The same work on two pages: `workIrn`, else `detailUrl` (null: never a duplicate). */
        fun dedupeKey(record: SearchRecord): String? {
            record.workIrn.trimmedOrNull()?.let { return "work:$it" }
            record.detailUrl.trimmedOrNull()?.let { return "url:$it" }
            return null
        }
    }
}

/** A format chip on a result row. */
data class ResultFormat(val format: FormatRef, val label: String, val isDigital: Boolean)

/** One result row. */
data class SearchResultItem(
    /** Unique within a result list: the dedupe key, else the row's position. */
    val id: String,
    /** [SearchResultsModel.dedupeKey]. */
    val key: String?,
    val ref: BookRef,
    /** Format chips, one per label. */
    val formats: List<ResultFormat>,
) {
    companion object {
        fun from(record: SearchRecord, key: String?, position: Int): SearchResultItem {
            val ref = BookRef.from(record)
            val labels = mutableSetOf<String>()
            // An edition-level item's one format has no code or label: no chip (§5.10).
            val formats = ref.formats
                .filterNot { EditionLevel.isEditionLevel(it) }
                .filter { labels.add(it.displayLabel.lowercase()) }
                .map { ResultFormat(it, it.displayLabel, CoreHelpers.isDigitalFormat(it)) }
            return SearchResultItem(id = key ?: "row:$position", key = key, ref = ref, formats = formats)
        }
    }
}
