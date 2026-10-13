package com.librespydus.app.feature.featured

import com.librespydus.app.core.Copy
import com.librespydus.app.core.ResultsSort
import com.librespydus.app.core.UserFacingError
import com.librespydus.app.feature.home.HomeReloadKey
import com.librespydus.app.feature.search.BrowseLoadable
import com.librespydus.app.feature.search.BrowseReporter
import com.librespydus.app.feature.search.SearchResultsModel
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.librespydus_core.FeaturedCollection
import uniffi.librespydus_core.FeaturedKind
import uniffi.librespydus_core.SpydusException

private val report: BrowseReporter = { UserFacingError.from(it, "Canning") }

class HomeFeaturedModelTest {
    private val f = FeaturedFixtures
    private val key = HomeReloadKey(FeaturedFixtures.ORIGIN, 0)

    @Test
    fun theSectionIsHiddenWhenTheLibraryHasNoFeaturedList() = runTest {
        val backend = FeaturedBackend()
        val model = HomeFeaturedModel()
        model.loadIfNeeded(backend, key)
        assertTrue(model.links.value.isEmpty())
        assertEquals(1, backend.featuredCalls.size)
    }

    @Test
    fun theLinksShowInOrderAndLoadOncePerKey() = runTest {
        val backend = FeaturedBackend().apply { featuredResult = Result.success(f.canningLinks) }
        val model = HomeFeaturedModel()
        model.loadIfNeeded(backend, key)
        assertEquals(listOf("What's New", "Coming Soon", "Library of Things"), model.links.value.map { it.title })

        model.loadIfNeeded(backend, key) // back on Home: no reload
        assertEquals(1, backend.featuredCalls.size)
        model.loadIfNeeded(backend, key.copy(verification = 1)) // verified: reload
        assertEquals(2, backend.featuredCalls.size)
    }

    @Test
    fun aFailureHidesTheSectionWithoutAnError() = runTest {
        val backend = FeaturedBackend().apply { featuredResult = Result.failure(SpydusException.Network("offline")) }
        val model = HomeFeaturedModel()
        model.loadIfNeeded(backend, key)
        assertTrue(model.links.value.isEmpty())
    }

    @Test
    fun aFailedRefreshKeepsTheTilesButAnotherLibraryNeverShowsThem() = runTest {
        val backend = FeaturedBackend().apply { featuredResult = Result.success(f.canningLinks) }
        val model = HomeFeaturedModel()
        model.loadIfNeeded(backend, key)
        backend.featuredResult = Result.failure(SpydusException.Network("offline"))
        model.refresh(backend, key)
        assertEquals(3, model.links.value.size)

        model.loadIfNeeded(backend, HomeReloadKey("https://hutt.spydus.co.nz", 0))
        assertTrue(model.links.value.isEmpty())
    }

    @Test
    fun linksWithoutATitleOrUrlAreDropped() = runTest {
        val backend = FeaturedBackend().apply {
            featuredResult = Result.success(f.canningLinks + f.link(" ", FeaturedKind.WEB, "https://x") + f.link("No URL", FeaturedKind.WEB, ""))
        }
        val model = HomeFeaturedModel()
        model.loadIfNeeded(backend, key)
        assertEquals(3, model.links.value.size)
    }
}

@OptIn(ExperimentalCoroutinesApi::class)
class CollectionModelTest {
    private val f = FeaturedFixtures
    private val request = CollectionModel.Request(FeaturedFixtures.ORIGIN, 0)

    private fun backendWith(sections: Int) = FeaturedBackend().apply {
        collectionResult = Result.success(FeaturedCollection("Library of Things", (1..sections).map { f.section(it) }))
        (1..sections).forEach { pages[f.sectionUrl(it)] = Result.success(f.results(listOf(f.editionRecord("$it")))) }
    }

    @Test
    fun thePageListsOneIdleShelfPerSectionAndLoadsNoShelfUntilItIsInView() = runTest {
        val backend = backendWith(sections = 4)
        val model = CollectionModel(f.LOT_URL)
        model.load(request, backend, report)

        val state = model.state.value
        assertEquals(listOf(f.LOT_URL), backend.collectionCalls)
        assertEquals((1..4).map { "Section $it" }, state.shelves.map { it.section.title })
        assertTrue(state.shelves.all { it.state == CollectionModel.ShelfState.Idle })
        assertFalse(state.hasNoSections)
        assertTrue(backend.pageCalls.isEmpty())

        model.loadShelf(f.sectionUrl(2), backend, report)
        assertEquals(listOf(f.sectionUrl(2)), backend.pageCalls)
        val loaded = model.state.value.shelves[1].state as CollectionModel.ShelfState.Loaded
        assertEquals(listOf("work:2"), loaded.books.map { it.id })

        model.loadShelf(f.sectionUrl(2), backend, report) // scrolled back into view: no reload
        assertEquals(1, backend.pageCalls.size)
    }

    @Test
    fun atMostThreeShelvesLoadAtOnce() = runTest {
        val backend = backendWith(sections = 6)
        (1..6).forEach { backend.gates[f.sectionUrl(it)] = CompletableDeferred() }
        val model = CollectionModel(f.LOT_URL)
        model.load(request, backend, report)

        // All six scroll into view at once.
        (1..6).forEach { n -> launch { model.loadShelf(f.sectionUrl(n), backend, report) } }
        advanceUntilIdle()
        assertEquals(3, backend.inFlightPages)
        assertEquals((1..3).map(f::sectionUrl), backend.pageCalls)
        assertTrue(model.state.value.shelves.all { it.state == CollectionModel.ShelfState.Loading })

        backend.gates.getValue(f.sectionUrl(1)).complete(Unit)
        advanceUntilIdle()
        assertEquals(3, backend.inFlightPages)
        assertEquals(4, backend.pageCalls.size)

        backend.gates.values.forEach { it.complete(Unit) }
        advanceUntilIdle()
        assertEquals(3, backend.maxInFlightPages)
        assertEquals(6, backend.pageCalls.size)
        assertTrue(model.state.value.shelves.all { it.state is CollectionModel.ShelfState.Loaded })
    }

    @Test
    fun anEmptySectionIsHidden() = runTest {
        val backend = backendWith(sections = 3)
        backend.pages[f.sectionUrl(2)] = Result.success(f.results(emptyList()))
        val model = CollectionModel(f.LOT_URL)
        model.load(request, backend, report)
        (1..3).forEach { model.loadShelf(f.sectionUrl(it), backend, report) }

        val state = model.state.value
        assertEquals(CollectionModel.ShelfState.Empty, state.shelves[1].state)
        assertEquals(listOf("Section 1", "Section 3"), state.visibleShelves.map { it.section.title })
        assertFalse(state.allSectionsEmpty)
    }

    @Test
    fun aPageWhoseSectionsAreAllEmptySendsYouToTheWebsite() = runTest {
        val backend = backendWith(sections = 2)
        (1..2).forEach { backend.pages[f.sectionUrl(it)] = Result.success(f.results(emptyList())) }
        val model = CollectionModel(f.LOT_URL)
        model.load(request, backend, report)
        assertFalse(model.state.value.allSectionsEmpty) // not known until each has loaded
        model.loadShelf(f.sectionUrl(1), backend, report)
        assertFalse(model.state.value.allSectionsEmpty)
        model.loadShelf(f.sectionUrl(2), backend, report)
        assertTrue(model.state.value.allSectionsEmpty)
        assertTrue(model.state.value.visibleShelves.isEmpty())
    }

    @Test
    fun aFailedSectionKeepsItsRowAndRetries() = runTest {
        val backend = backendWith(sections = 2)
        backend.pages[f.sectionUrl(1)] = Result.failure(SpydusException.Network("offline"))
        val model = CollectionModel(f.LOT_URL)
        model.load(request, backend, report)
        model.loadShelf(f.sectionUrl(1), backend, report)

        val failed = model.state.value.shelves[0]
        assertTrue(failed.state is CollectionModel.ShelfState.Failed)
        assertTrue((failed.state as CollectionModel.ShelfState.Failed).error.isRetryable)
        assertEquals(2, model.state.value.visibleShelves.size)

        model.loadShelf(f.sectionUrl(1), backend, report) // back in view: stays failed until Retry
        assertEquals(1, backend.pageCalls.size)

        backend.pages[f.sectionUrl(1)] = Result.success(f.results(listOf(f.editionRecord("7"))))
        model.loadShelf(f.sectionUrl(1), backend, report, retry = true)
        assertTrue(model.state.value.shelves[0].state is CollectionModel.ShelfState.Loaded)
    }

    @Test
    fun aPageWithoutSectionsSendsYouToTheWebsite() = runTest {
        val backend = FeaturedBackend().apply { collectionResult = Result.success(FeaturedCollection("eLibrary", emptyList())) }
        val model = CollectionModel("${f.ORIGIN}/cgi-bin/spydus.exe/MSGTRNGEN/OPAC/ELIBRARY?HOMEPRMS=UD_ELIBRARY")
        model.load(request, backend, report)
        assertTrue(model.state.value.hasNoSections)
        assertEquals("This collection is on the library’s website.", Copy.Featured.websiteOnly)
    }

    @Test
    fun aFailedPageShowsTheErrorAndRetriesWithANewAttempt() = runTest {
        val backend = FeaturedBackend().apply { collectionResult = Result.failure(SpydusException.Network("offline")) }
        val model = CollectionModel(f.LOT_URL)
        model.load(request, backend, report)
        assertTrue(model.state.value.page is BrowseLoadable.Failed)
        assertFalse(model.state.value.hasNoSections)

        model.load(request, backend, report) // same request: not again
        assertEquals(1, backend.collectionCalls.size)

        backend.collectionResult = Result.success(FeaturedCollection("Library of Things", listOf(f.section(1))))
        model.load(request.copy(attempt = 1), backend, report)
        assertEquals(1, model.state.value.shelves.size)
    }

    @Test
    fun aReloadedPageDropsShelvesStillWaitingForASlot() = runTest {
        val backend = backendWith(sections = 4)
        (1..4).forEach { backend.gates[f.sectionUrl(it)] = CompletableDeferred() }
        val model = CollectionModel(f.LOT_URL)
        model.load(request, backend, report)
        (1..4).forEach { n -> launch { model.loadShelf(f.sectionUrl(n), backend, report) } }
        advanceUntilIdle()

        model.load(request.copy(verification = 1), backend, report) // verified: a fresh page
        backend.gates.values.forEach { it.complete(Unit) }
        advanceUntilIdle()
        assertEquals(3, backend.pageCalls.size) // the fourth never started
        assertTrue(model.state.value.shelves.all { it.state == CollectionModel.ShelfState.Idle })
    }
}

class ListResultsTest {
    private val f = FeaturedFixtures
    private val first = "${FeaturedFixtures.ORIGIN}/cgi-bin/spydus.exe/RUNSQRY/OPAC/BIBENQ?SQRYIRN=4050924&QRYTEXT=Coming%20Soon&NRECS=20"
    private val second = "${FeaturedFixtures.ORIGIN}/cgi-bin/spydus.exe/SET/OPAC/BIBENQ/2?NREC=20"

    private fun request(attempt: Int = 0) =
        SearchResultsModel.Request("Coming Soon", ResultsSort.RELEVANCE, f.ORIGIN, verification = 0, attempt = attempt, url = first)

    @Test
    fun aListStartsFromItsUrlAndPagesThroughNextPageUrl() = runTest {
        val backend = FeaturedBackend().apply {
            pages[first] = Result.success(f.results(listOf(f.editionRecord("1"), f.editionRecord("2")), next = second))
            pages[second] = Result.success(f.results(listOf(f.editionRecord("2"), f.editionRecord("3"))))
        }
        val model = SearchResultsModel()
        model.load(request(), backend, report)

        assertTrue(backend.searchCalls.isEmpty())
        assertEquals(listOf(first), backend.pageCalls)
        assertEquals(SearchResultsModel.Phase.Loaded, model.state.value.phase)
        assertEquals(second, model.state.value.nextPageUrl)
        // Edition-level rows: no format chips.
        assertTrue(model.state.value.items.all { it.formats.isEmpty() })

        model.loadMore()
        assertEquals(listOf(first, second), backend.pageCalls)
        assertEquals(listOf("work:1", "work:2", "work:3"), model.state.value.items.map { it.key })
        assertNull(model.state.value.nextPageUrl)
        assertFalse(model.state.value.hasMore)
    }

    @Test
    fun aNextLinkBackToTheFirstPageEndsTheList() = runTest {
        val backend = FeaturedBackend().apply {
            pages[first] = Result.success(f.results(listOf(f.editionRecord("1")), next = second))
            pages[second] = Result.success(f.results(listOf(f.editionRecord("2")), next = first))
        }
        val model = SearchResultsModel()
        model.load(request(), backend, report)
        model.loadMore()
        assertNull(model.state.value.nextPageUrl)
        assertEquals(2, backend.pageCalls.size)
    }

    @Test
    fun anEmptyListIsNoResultsAndAFailureRetries() = runTest {
        val backend = FeaturedBackend().apply { pages[first] = Result.success(f.results(emptyList(), noResults = true)) }
        val model = SearchResultsModel()
        model.load(request(), backend, report)
        assertEquals(SearchResultsModel.Phase.NoResults, model.state.value.phase)

        backend.pages[first] = Result.failure(SpydusException.Network("offline"))
        model.load(request(attempt = 1), backend, report)
        assertTrue(model.state.value.phase is SearchResultsModel.Phase.Failed)

        backend.pages[first] = Result.success(f.results(listOf(f.editionRecord("1"))))
        model.load(request(attempt = 2), backend, report)
        assertEquals(SearchResultsModel.Phase.Loaded, model.state.value.phase)
    }
}
