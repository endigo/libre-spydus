package com.librespydus.app.feature

import com.librespydus.app.core.BookRef
import com.librespydus.app.core.CoreHelpers
import com.librespydus.app.core.LibraryBackend
import com.librespydus.app.core.UserFacingError
import com.librespydus.app.feature.detail.BookDetailModel
import com.librespydus.app.feature.detail.DetailBackend
import com.librespydus.app.feature.detail.DetailFixtures
import com.librespydus.app.feature.detail.RelatedPanel
import com.librespydus.app.feature.featured.FeaturedBackend
import com.librespydus.app.feature.featured.FeaturedFixtures
import com.librespydus.app.feature.featured.HomeFeaturedModel
import com.librespydus.app.feature.home.HomeLoadContext
import com.librespydus.app.feature.home.HomeModel
import com.librespydus.app.feature.home.HomeReloadKey
import com.librespydus.app.feature.search.BrowseReporter
import com.librespydus.app.feature.search.PagingFakeBackend
import com.librespydus.app.feature.search.SearchFixtures
import com.librespydus.app.feature.search.errorOrNull
import com.librespydus.app.feature.search.valueOrNull
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import uniffi.librespydus_core.FeaturedLink
import uniffi.librespydus_core.PatronItem
import uniffi.librespydus_core.RecordDetail
import uniffi.librespydus_core.RecordRelated
import uniffi.librespydus_core.RelatedList
import uniffi.librespydus_core.SavedBook
import uniffi.librespydus_core.SearchResults
import uniffi.librespydus_core.SpydusException

/**
 * In-memory saved pages (docs/native-apps.md §5.13) over any test [base] backend: what each key
 * holds, and every save made.
 */
class SavedPagesBackend(private val base: LibraryBackend) : LibraryBackend by base {
    val books = mutableMapOf<String, SavedBook>()
    val shelves = mutableMapOf<String, SearchResults>()
    var featured: List<FeaturedLink>? = null
    val bookSaves = mutableListOf<String>()
    val shelfSaves = mutableListOf<String>()
    var featuredSaves = 0

    /** Runs as each `record` request goes out (before its answer). */
    var beforeRecord: (() -> Unit)? = null

    override suspend fun record(url: String): RecordDetail {
        beforeRecord?.invoke()
        return base.record(url)
    }

    override suspend fun savedBook(key: String) = books[key]

    override suspend fun saveBook(key: String, book: SavedBook) {
        books[key] = book
        bookSaves += key
    }

    override suspend fun savedShelf(key: String) = shelves[key]

    override suspend fun saveShelf(key: String, results: SearchResults) {
        shelves[key] = results
        shelfSaves += key
    }

    override suspend fun savedFeatured() = featured

    override suspend fun saveFeatured(links: List<FeaturedLink>) {
        featured = links
        featuredSaves += 1
    }
}

/** The book page shows its saved copy first, then the fresh one (§5.13). */
@OptIn(ExperimentalCoroutinesApi::class)
class SavedBookPageTest {
    private val d = DetailFixtures
    private val key = "work:15974354"
    private val report: BrowseReporter = { UserFacingError.from(it, "Canning") }
    private val freshSimilar = RelatedList(listOf(d.related("2", "Fresh similar")))
    private val freshAuthor = RelatedList(listOf(d.related("3", "Fresh by author")))
    private val savedSimilar = RelatedList(listOf(d.related("1", "Saved similar")))

    /** The Midnight Library as saved by an earlier visit: an older summary, related links from an older search set. */
    private val savedRecord = d.midnightLibrary.copy(
        summary = "Saved summary",
        related = RecordRelated("https://canning.spydus.com/old-similar", "https://canning.spydus.com/old-author", null, null),
    )
    private val savedPage = SavedBook(savedRecord, savedSimilar, null, null, null)

    // The carousels start in the model's viewModelScope.
    @Before
    fun setMain() = Dispatchers.setMain(UnconfinedTestDispatcher())

    @After
    fun resetMain() = Dispatchers.resetMain()

    private fun BookDetailModel.titles(panel: RelatedPanel) = state.value.related[panel]?.valueOrNull?.map { it.title }

    @Test
    fun theSavedPageShowsBeforeAnyRequestAndTheFreshOneReplacesIt() = runTest {
        val ref = BookRef.from(d.searchRecord)
        val bk = ref.formats[0].availabilityUrl!!
        val detail = DetailBackend(
            records = mutableMapOf(bk to Result.success(d.midnightLibrary)),
            relatedByUrl = mapOf(
                "https://canning.spydus.com/similar" to Result.success(freshSimilar),
                "https://canning.spydus.com/author" to Result.success(freshAuthor),
            ),
        )
        val backend = SavedPagesBackend(detail).apply { books[key] = savedPage }
        val model = BookDetailModel(ref)
        var seen: Pair<String?, List<String>?>? = null
        backend.beforeRecord = { if (seen == null) seen = model.state.value.record.valueOrNull?.summary to model.titles(RelatedPanel.SIMILAR) }

        model.load(backend, report)

        assertEquals("Saved summary" to listOf("Saved similar"), seen)
        assertEquals(d.midnightLibrary, model.state.value.record.valueOrNull)
        // The fresh record's links are new: its carousel keeps the saved items until they load.
        assertEquals(listOf("Saved similar"), model.titles(RelatedPanel.SIMILAR))
        assertEquals(listOf(key), backend.bookSaves)
        assertEquals(SavedBook(d.midnightLibrary, savedSimilar, null, null, null), backend.books[key])

        model.loadRelated(backend)

        assertEquals(
            setOf("https://canning.spydus.com/similar", "https://canning.spydus.com/author", "https://canning.spydus.com/versions"),
            detail.relatedUrls.toSet(),
        )
        assertEquals(listOf("Fresh similar"), model.titles(RelatedPanel.SIMILAR))
        assertEquals(listOf("Fresh by author"), model.titles(RelatedPanel.AUTHOR))
        assertEquals(listOf(RelatedPanel.SIMILAR, RelatedPanel.AUTHOR), model.state.value.visiblePanels) // "versions" failed
        assertEquals(listOf(key, key), backend.bookSaves)
        assertEquals(SavedBook(d.midnightLibrary, freshSimilar, freshAuthor, null, null), backend.books[key])
    }

    @Test
    fun aFailedRefreshKeepsTheSavedPageWithoutAnError() = runTest {
        val ref = BookRef.from(d.searchRecord)
        val bk = ref.formats[0].availabilityUrl!!
        val detail = DetailBackend(
            records = mutableMapOf(bk to Result.failure(SpydusException.Network("offline"))),
            relatedByUrl = mapOf("https://canning.spydus.com/similar" to Result.success(freshSimilar)),
        )
        val backend = SavedPagesBackend(detail).apply { books[key] = savedPage }
        val model = BookDetailModel(ref)
        var reported = 0

        model.load(backend) {
            reported += 1
            UserFacingError.from(it, "Canning")
        }

        assertEquals(savedRecord, model.state.value.record.valueOrNull)
        assertNull(model.state.value.record.errorOrNull)
        assertEquals(1, reported) // still reported: verification would still ask
        assertTrue(backend.bookSaves.isEmpty())
        // The carousels came into view: the saved record's links are from an old search set.
        model.startRelated(backend)
        assertTrue(detail.relatedUrls.isEmpty())
        assertEquals(listOf("Saved similar"), model.titles(RelatedPanel.SIMILAR))

        // The library answers again: the page refreshes, and so do the carousels that came into view.
        detail.setRecord(bk, Result.success(d.midnightLibrary))
        model.load(backend, report)
        assertEquals(d.midnightLibrary, model.state.value.record.valueOrNull)
        assertEquals(listOf("Fresh similar"), model.titles(RelatedPanel.SIMILAR))
        assertFalse("https://canning.spydus.com/old-similar" in detail.relatedUrls)
    }

    @Test
    fun aWorkLevelRecordDoesNotReplaceASavedBibRecord() = runTest {
        val ref = BookRef.from(d.searchRecord.copy(formats = emptyList())) // work-level URL, no formats
        val workLevel = d.midnightLibrary.copy(
            bookmarkUrl = "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/WRKENQ?SETLVL=&IRN=15642035",
            details = emptyList(),
            related = RecordRelated(null, null, null, null),
        )
        val detail = DetailBackend(records = mutableMapOf(ref.url!! to Result.success(workLevel)))
        val backend = SavedPagesBackend(detail).apply { books[key] = SavedBook(d.midnightLibrary, null, null, null, null) }
        val model = BookDetailModel(ref)

        model.load(backend, report)

        assertEquals(listOf(ref.url), detail.recordUrls)
        assertEquals(d.midnightLibrary, model.state.value.record.valueOrNull)
        assertTrue(backend.bookSaves.isEmpty())
    }

    @Test
    fun aFirstVisitSavesTheRecordThenItsLists() = runTest {
        val ref = BookRef.from(d.searchRecord)
        val bk = ref.formats[0].availabilityUrl!!
        val detail = DetailBackend(
            records = mutableMapOf(bk to Result.success(d.midnightLibrary)),
            relatedByUrl = mapOf("https://canning.spydus.com/similar" to Result.success(freshSimilar)),
        )
        val backend = SavedPagesBackend(detail)
        val model = BookDetailModel(ref)

        model.load(backend, report)
        assertEquals(SavedBook(d.midnightLibrary, null, null, null, null), backend.books[key])
        model.loadRelated(backend)
        assertEquals(listOf(key, key), backend.bookSaves)
        assertEquals(freshSimilar, backend.books[key]?.similar)
        assertNull(backend.books[key]?.author) // failed: not saved
    }

    @Test
    fun refsWithoutAStableIdAreNeitherReadNorSaved() = runTest {
        val url = "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/78554992?SETLVL=&BRN=275039"
        val ref = BookRef.fromUrl(url)
        assertNull(ref.savedPageKey)
        val backend = SavedPagesBackend(DetailBackend(records = mutableMapOf(url to Result.success(d.midnightLibrary))))
        backend.books[ref.id] = savedPage
        val model = BookDetailModel(ref)
        var seen: RecordDetail? = savedRecord
        backend.beforeRecord = { seen = model.state.value.record.valueOrNull }

        model.load(backend, report)

        assertNull(seen)
        assertEquals(d.midnightLibrary, model.state.value.record.valueOrNull)
        assertTrue(backend.bookSaves.isEmpty())
    }

    @Test
    fun savedPageKeys() {
        assertEquals("work:15974354", BookRef.from(d.searchRecord).savedPageKey)
        assertEquals("brn:467086", BookRef.from(d.related("467086", "The long night")).savedPageKey)
        val loan = PatronItem("1", "8753080", "The Midnight Library", null, null, null, emptyList(), null, emptyMap(), emptyList())
        assertEquals("bib:8753080", BookRef.from(loan).savedPageKey)
        assertNull(BookRef.fromUrl("https://canning.spydus.com/x").savedPageKey)
        assertNull(BookRef(id = "title:untitled", title = "Untitled").savedPageKey)
    }
}

/** Home's New arrivals and Featured tiles show their saved copies first (§5.13). */
class SavedHomeTest {
    private val s = SearchFixtures
    private val f = FeaturedFixtures
    private val shelfKey = CoreHelpers.homeShelves()[0].key
    private val report: BrowseReporter = { UserFacingError.from(it, "Canning") }

    private fun context(backend: LibraryBackend) = HomeLoadContext(backend, HomeReloadKey(backend.origin, 0), report)

    @Test
    fun freshArrivalsReplaceTheSavedOnesAndAreSaved() = runTest {
        val fresh = s.page(listOf(s.record("1"), s.record("2")))
        val backend = SavedPagesBackend(PagingFakeBackend(Result.success(fresh))).apply { shelves[shelfKey] = s.page(listOf(s.record("9"))) }
        val home = HomeModel()

        home.loadIfNeeded(context(backend))

        assertEquals(listOf("Title 1", "Title 2"), home.arrivals.value.valueOrNull?.map { it.title })
        assertEquals(listOf(shelfKey), backend.shelfSaves)
        assertEquals(fresh, backend.shelves[shelfKey])
    }

    @Test
    fun savedArrivalsStayWhenTheFreshOnesFail() = runTest {
        val paging = PagingFakeBackend(Result.failure(SpydusException.Network("offline")))
        val backend = SavedPagesBackend(paging).apply { shelves[shelfKey] = s.page(listOf(s.record("9"))) }
        val home = HomeModel()

        home.loadIfNeeded(context(backend))

        assertEquals(1, paging.searchCalls.size)
        assertEquals(listOf("Title 9"), home.arrivals.value.valueOrNull?.map { it.title })
        assertTrue(backend.shelfSaves.isEmpty())
    }

    @Test
    fun anUnrecognisedPageDoesNotReplaceSavedArrivals() = runTest {
        val backend = SavedPagesBackend(PagingFakeBackend(Result.success(s.page(emptyList()))))
            .apply { shelves[shelfKey] = s.page(listOf(s.record("9"))) }
        val home = HomeModel()

        home.loadIfNeeded(context(backend))

        assertEquals(listOf("Title 9"), home.arrivals.value.valueOrNull?.map { it.title })
        assertTrue(backend.shelfSaves.isEmpty())
    }

    @Test
    fun withoutSavedArrivalsAFailureShowsAsBefore() = runTest {
        val backend = SavedPagesBackend(PagingFakeBackend(Result.failure(SpydusException.Network("offline"))))
        val home = HomeModel()

        home.loadIfNeeded(context(backend))

        assertEquals(UserFacingError.Kind.NETWORK, home.arrivals.value.errorOrNull?.kind)
    }

    @Test
    fun savedTilesStayWhenTheLoadFails() = runTest {
        val featured = FeaturedBackend().apply { featuredResult = Result.failure(SpydusException.Network("offline")) }
        val backend = SavedPagesBackend(featured).apply { this.featured = listOf(f.canningLinks.last()) }
        val row = HomeFeaturedModel()

        row.loadIfNeeded(backend, HomeReloadKey(backend.origin, 0))

        assertEquals(listOf(f.canningLinks.last().title), row.links.value.map { it.title })
        assertEquals(0, backend.featuredSaves)
    }

    @Test
    fun freshTilesReplaceTheSavedOnesAndAreSaved() = runTest {
        val featured = FeaturedBackend().apply { featuredResult = Result.success(f.canningLinks) }
        val backend = SavedPagesBackend(featured).apply { this.featured = listOf(f.canningLinks.last()) }
        val row = HomeFeaturedModel()

        row.loadIfNeeded(backend, HomeReloadKey(backend.origin, 0))

        assertEquals(f.canningLinks.map { it.title }, row.links.value.map { it.title })
        assertEquals(f.canningLinks, backend.featured)
    }

    @Test
    fun aLibraryWithoutAListSavesItsEmptyList() = runTest {
        val backend = SavedPagesBackend(FeaturedBackend()).apply { featured = listOf(f.canningLinks.last()) }
        val row = HomeFeaturedModel()

        row.loadIfNeeded(backend, HomeReloadKey(backend.origin, 0))

        assertTrue(row.links.value.isEmpty())
        assertEquals(emptyList<FeaturedLink>(), backend.featured)
        assertEquals(1, backend.featuredSaves)
    }
}
