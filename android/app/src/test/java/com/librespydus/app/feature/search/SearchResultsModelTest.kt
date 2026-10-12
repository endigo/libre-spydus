package com.librespydus.app.feature.search

import com.librespydus.app.core.DemoBackend
import com.librespydus.app.core.LibraryBackend
import com.librespydus.app.core.ResultsSort
import com.librespydus.app.core.UserFacingError
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.librespydus_core.AccountLinks
import uniffi.librespydus_core.AccountSnapshot
import uniffi.librespydus_core.Availability
import uniffi.librespydus_core.EditOptions
import uniffi.librespydus_core.EditOverrides
import uniffi.librespydus_core.PatronList
import uniffi.librespydus_core.RecordDetail
import uniffi.librespydus_core.RecordFormat
import uniffi.librespydus_core.RelatedList
import uniffi.librespydus_core.Reservations
import uniffi.librespydus_core.ReserveOptions
import uniffi.librespydus_core.ReserveResult
import uniffi.librespydus_core.SearchRecord
import uniffi.librespydus_core.SearchResults
import uniffi.librespydus_core.SpydusException

/**
 * An offline backend with scripted search pages (first page from `searchSorted`, later pages by
 * URL). [pageDelay] keeps a page request in flight so tests can race it.
 */
class PagingFakeBackend(
    var first: Result<SearchResults>,
    private val pages: MutableMap<String, Result<SearchResults>> = mutableMapOf(),
) : LibraryBackend {
    override val origin = SearchFixtures.ORIGIN
    val searchCalls = mutableListOf<String>()
    val pageCalls = mutableListOf<String>()
    var pageDelayMs = 0L
    var searchDelayMs = 0L
    var loansResult: Result<PatronList> = Result.success(PatronList(0, emptyList()))

    fun setPage(url: String, result: Result<SearchResults>) {
        pages[url] = result
    }

    override fun setVerificationCookie(cookie: String?) = Unit
    override suspend fun search(query: String) = searchSorted(query, ResultsSort.RELEVANCE)
    override suspend fun searchSorted(query: String, sort: ResultsSort): SearchResults {
        searchCalls += "$query|$sort"
        if (searchDelayMs > 0) delay(searchDelayMs)
        return first.getOrThrow()
    }

    override suspend fun page(url: String): SearchResults {
        pageCalls += url
        if (pageDelayMs > 0) delay(pageDelayMs)
        return (pages[url] ?: throw SpydusException.Upstream("no page $url")).getOrThrow()
    }

    override suspend fun availability(url: String): Availability = throw SpydusException.Upstream("fake")
    override suspend fun record(url: String): RecordDetail = throw SpydusException.Upstream("fake")
    override suspend fun related(url: String) = RelatedList(emptyList())
    override suspend fun login(username: String, password: String): String = throw SpydusException.CredentialsRejected("")
    override suspend fun account(session: String) = AccountLinks("Borrower, Test", null, null, null)
    override suspend fun loans(session: String): PatronList = loansResult.getOrThrow()
    override suspend fun reservations(session: String) = Reservations(PatronList(0, emptyList()), PatronList(0, emptyList()))
    override suspend fun accountSnapshot(session: String) =
        AccountSnapshot(DemoBackend.SNAPSHOT_VERSION, account(session).name, loans(session), reservations(session), 0)
    override suspend fun reserveOptions(url: String, session: String): ReserveOptions = throw SpydusException.NotLoggedIn("")
    override suspend fun placeReservation(url: String, session: String, xdays: String?, pickup: String?): ReserveResult = error("never POST in tests")
    override suspend fun cancelReservation(url: String, session: String): ReserveResult = error("never POST in tests")
    override suspend fun editOptions(url: String, session: String): EditOptions = throw SpydusException.NotLoggedIn("")
    override suspend fun editReservation(url: String, session: String, overrides: EditOverrides): ReserveResult = error("never POST in tests")
}

object SearchFixtures {
    const val ORIGIN = "https://canning.spydus.com"

    fun record(irn: String?, url: String? = null, title: String? = null, formats: List<RecordFormat> = emptyList()) = SearchRecord(
        rank = null, workIrn = irn, title = title ?: "Title ${irn ?: "?"}", detailUrl = url, author = "Haig, Matt",
        authorRole = null, authorUrl = null, year = "2024", language = null, summary = null, isbn = null,
        coverUrls = emptyList(), formats = formats,
    )

    fun page(records: List<SearchRecord>, next: String? = null, total: Long? = 25, noResults: Boolean = false) =
        SearchResults("q", "1", total, null, next, null, noResults, records)

    fun pageUrl(n: Int) = "$ORIGIN/cgi-bin/spydus.exe/SET/OPAC/ALLWRKENQ/1/$n?NREC=10"

    fun request(query: String = "matt haig", sort: ResultsSort = ResultsSort.RELEVANCE, attempt: Int = 0) =
        SearchResultsModel.Request(query, sort, ORIGIN, verification = 0, attempt = attempt)
}

@OptIn(ExperimentalCoroutinesApi::class)
class SearchResultsModelTest {
    private val f = SearchFixtures

    /** Mirrors AppState.report for a non-routed error (no sheets). */
    private val report: BrowseReporter = { UserFacingError.from(it, "Canning") }

    private fun ok(results: SearchResults) = Result.success(results)

    // region merge / dedupe

    @Test
    fun mergeDropsWorksAlreadyListedByWorkIrnOrDetailUrl() {
        val first = SearchResultsModel.merge(emptyList(), listOf(f.record("1", url = "u1"), f.record(null, url = "u2"), f.record("3")))
        val merged = SearchResultsModel.merge(
            first,
            listOf(
                f.record("1", url = "u1-new-set-id"), // same work on the next page (new set id in its URL)
                f.record(null, url = "u2"), // same detail URL, no work IRN
                f.record("4"),
                f.record("4"), // repeated within one page
            ),
        )
        assertEquals(listOf("work:1", "url:u2", "work:3", "work:4"), merged.map { it.key })
        assertEquals(first.map { it.id }, merged.take(3).map { it.id })
    }

    @Test
    fun recordsWithoutAKeyAreKeptWithUniqueIds() {
        val merged = SearchResultsModel.merge(emptyList(), listOf(f.record(null), f.record(null), f.record("9")))
        assertEquals(3, merged.size)
        assertEquals(3, merged.map { it.id }.toSet().size)
        assertNull(merged[0].key)
        assertNull(merged[1].key)
        val more = SearchResultsModel.merge(merged, listOf(f.record(null)))
        assertEquals(4, more.map { it.id }.toSet().size)
    }

    @Test
    fun formatChipsAreOnePerLabelAndDigitalIsFlagged() {
        val item = SearchResultsModel.merge(
            emptyList(),
            listOf(
                f.record(
                    "1",
                    formats = listOf(
                        RecordFormat("BK", "Book", null, null),
                        RecordFormat("BK", "Book", null, null),
                        RecordFormat("EBK", "eBook", null, null),
                    ),
                ),
            ),
        )[0]
        assertEquals(listOf("Book", "eBook"), item.formats.map { it.label })
        assertEquals(listOf(false, true), item.formats.map { it.isDigital })
    }

    @Test
    fun resultCountReadsNaturally() {
        assertEquals("1 result", resultCount(1))
        assertEquals("43 results", resultCount(43))
        assertNull(resultCount(null))
    }

    // endregion
    // region paging

    @Test
    fun loadsTheFirstPageThenAppendsTheNextWithoutDuplicates() = runTest {
        val backend = PagingFakeBackend(
            first = ok(f.page(listOf(f.record("1"), f.record("2")), next = f.pageUrl(2), total = 1234)),
            pages = mutableMapOf(f.pageUrl(2) to ok(f.page(listOf(f.record("2"), f.record("3")), next = f.pageUrl(3)))),
        )
        val model = SearchResultsModel()
        model.load(f.request(sort = ResultsSort.NEWEST), backend, report)
        val loaded = model.state.value
        assertEquals(SearchResultsModel.Phase.Loaded, loaded.phase)
        assertEquals(1234L, loaded.total)
        assertEquals(listOf("work:1", "work:2"), loaded.items.map { it.key })
        assertEquals(listOf("matt haig|NEWEST"), backend.searchCalls)

        model.loadMore()
        assertEquals(listOf("work:1", "work:2", "work:3"), model.state.value.items.map { it.key })
        assertEquals(f.pageUrl(3), model.state.value.nextPageUrl)
        assertEquals(listOf(f.pageUrl(2)), backend.pageCalls)
    }

    @Test
    fun concurrentLoadMoreFetchesEachPageOnce() = runTest {
        val backend = PagingFakeBackend(
            first = ok(f.page(listOf(f.record("1")), next = f.pageUrl(2))),
            pages = mutableMapOf(f.pageUrl(2) to ok(f.page(listOf(f.record("2")), next = null))),
        )
        backend.pageDelayMs = 50
        val model = SearchResultsModel()
        model.load(f.request(), backend, report)
        repeat(3) { launch { model.loadMore() } }
        advanceUntilIdle()
        assertEquals(listOf(f.pageUrl(2)), backend.pageCalls)
        assertEquals(2, model.state.value.items.size)
        assertFalse(model.state.value.hasMore)
        // At the end, further calls do nothing.
        model.loadMore()
        assertEquals(1, backend.pageCalls.size)
    }

    @Test
    fun aNextLinkBackToAFetchedPageEndsPaging() = runTest {
        val backend = PagingFakeBackend(
            first = ok(f.page(listOf(f.record("1")), next = f.pageUrl(2))),
            pages = mutableMapOf(f.pageUrl(2) to ok(f.page(listOf(f.record("2")), next = f.pageUrl(2)))),
        )
        val model = SearchResultsModel()
        model.load(f.request(), backend, report)
        model.loadMore()
        assertNull(model.state.value.nextPageUrl)
        model.loadMore()
        assertEquals(listOf(f.pageUrl(2)), backend.pageCalls)
    }

    @Test
    fun noResultsAndUnrecognisedPagesAreDifferentStates() = runTest {
        val model = SearchResultsModel()
        model.load(f.request("zzqxjv"), PagingFakeBackend(ok(f.page(emptyList(), total = null, noResults = true))), report)
        assertEquals(SearchResultsModel.Phase.NoResults, model.state.value.phase)

        model.load(f.request("other"), PagingFakeBackend(ok(f.page(emptyList(), total = null, noResults = false))), report)
        assertEquals(SearchResultsModel.Phase.Failed(SearchResultsModel.UNRECOGNISED_PAGE), model.state.value.phase)
        assertTrue(model.state.value.items.isEmpty())
    }

    @Test
    fun aFailedNextPageStopsAutoPagingUntilRetried() = runTest {
        val backend = PagingFakeBackend(
            first = ok(f.page(listOf(f.record("1")), next = f.pageUrl(2))),
            pages = mutableMapOf(f.pageUrl(2) to ok(f.page(emptyList(), noResults = false))), // unrecognised page
        )
        val model = SearchResultsModel()
        model.load(f.request(), backend, report)
        model.loadMore()
        assertEquals(SearchResultsModel.UNRECOGNISED_PAGE, model.state.value.loadMoreError)
        assertEquals(1, model.state.value.items.size)
        assertTrue(model.state.value.hasMore)

        model.loadMore() // scrolling doesn't retry by itself
        assertEquals(1, backend.pageCalls.size)

        backend.setPage(f.pageUrl(2), Result.failure(SpydusException.Network("timed out")))
        model.retryLoadMore()
        assertEquals(UserFacingError.Kind.NETWORK, model.state.value.loadMoreError?.kind)

        backend.setPage(f.pageUrl(2), ok(f.page(listOf(f.record("2")))))
        model.retryLoadMore()
        assertNull(model.state.value.loadMoreError)
        assertEquals(listOf("work:1", "work:2"), model.state.value.items.map { it.key })
        assertEquals(3, backend.pageCalls.size)
    }

    @Test
    fun aPageForAnOlderSearchIsDropped() = runTest {
        val backend = PagingFakeBackend(
            first = ok(f.page(listOf(f.record("1")), next = f.pageUrl(2))),
            pages = mutableMapOf(f.pageUrl(2) to ok(f.page(listOf(f.record("old-2"))))),
        )
        backend.pageDelayMs = 100
        val model = SearchResultsModel()
        model.load(f.request("first"), backend, report)
        val stalePage = launch { model.loadMore() }
        advanceTimeBy(20)
        backend.first = ok(f.page(listOf(f.record("new-1")), next = null))
        model.load(f.request("second"), backend, report)
        stalePage.join()
        assertEquals(listOf("work:new-1"), model.state.value.items.map { it.key })
        assertFalse(model.state.value.isLoadingMore)
    }

    @Test
    fun theSameRequestKeepsLoadedPagesAndANewAttemptReloads() = runTest {
        val backend = PagingFakeBackend(
            first = ok(f.page(listOf(f.record("1")), next = f.pageUrl(2))),
            pages = mutableMapOf(f.pageUrl(2) to ok(f.page(listOf(f.record("2"))))),
        )
        val model = SearchResultsModel()
        model.load(f.request(), backend, report)
        model.loadMore()
        // Coming back from a detail screen runs the same request again.
        model.load(f.request(), backend, report)
        assertEquals(2, model.state.value.items.size)
        assertEquals(1, backend.searchCalls.size)
        // Search pressed again / Retry: a new attempt searches again from page 1.
        model.load(f.request(attempt = 1), backend, report)
        assertEquals(1, model.state.value.items.size)
        assertEquals(2, backend.searchCalls.size)
    }

    @Test
    fun theSameRequestIsNotStartedAgainWhileItLoads() = runTest {
        // The screen asks again when it comes back into view (tab switch, back from detail).
        val backend = PagingFakeBackend(ok(f.page(listOf(f.record("1")))))
        backend.searchDelayMs = 100
        val model = SearchResultsModel()
        val first = launch { model.load(f.request(), backend, report) }
        advanceTimeBy(20)
        model.load(f.request(), backend, report)
        first.join()
        assertEquals(1, backend.searchCalls.size)
        assertEquals(SearchResultsModel.Phase.Loaded, model.state.value.phase)
    }

    @Test
    fun errorsGoThroughReport() = runTest {
        val reported = mutableListOf<Throwable>()
        val backend = PagingFakeBackend(Result.failure(SpydusException.HumanVerificationRequired("human verification required")))
        val model = SearchResultsModel()
        model.load(f.request(), backend) { error ->
            reported += error
            UserFacingError.from(error, "Canning")
        }
        assertEquals(1, reported.size)
        val phase = model.state.value.phase as SearchResultsModel.Phase.Failed
        assertEquals(UserFacingError.Kind.VERIFICATION, phase.error.kind)
    }

    // endregion
}
