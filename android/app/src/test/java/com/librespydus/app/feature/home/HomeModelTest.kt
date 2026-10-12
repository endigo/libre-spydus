package com.librespydus.app.feature.home

import com.librespydus.app.core.CoreHelpers
import com.librespydus.app.core.DemoBackend
import com.librespydus.app.core.DemoData
import com.librespydus.app.core.ResultsSort
import com.librespydus.app.core.Route
import com.librespydus.app.core.UserFacingError
import com.librespydus.app.feature.search.BrowseLoadable
import com.librespydus.app.feature.search.BrowseReporter
import com.librespydus.app.feature.search.PagingFakeBackend
import com.librespydus.app.feature.search.SearchFixtures
import com.librespydus.app.feature.search.SearchResultsModel
import com.librespydus.app.feature.search.errorOrNull
import com.librespydus.app.feature.search.valueOrNull
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Test
import uniffi.librespydus_core.AccountSnapshot
import uniffi.librespydus_core.PatronItem
import uniffi.librespydus_core.PatronList
import uniffi.librespydus_core.Reservations
import uniffi.librespydus_core.SpydusException
import java.time.LocalDate
import java.time.format.DateTimeFormatter
import java.util.Locale

class HomeGreetingTest {
    @Test
    fun firstNames() {
        val cases = listOf(
            "Borrower, Test" to "Test",
            "Test Borrower" to "Test",
            "Smith, Jane Mary" to "Jane",
            "SMITH, JANE" to "Jane",
            "jane smith" to "Jane",
            "Watson, MARY-JANE" to "Mary-Jane",
            "O'Brien, Siobhán" to "Siobhán",
            "Mrs Jane Smith" to "Jane",
            "Smith, Dr. Jane" to "Jane",
            "Smith, J. Jane" to "Jane",
            "  Jane   Smith  " to "Jane",
            "Jane" to "Jane",
            "McKenzie, DeShawn" to "DeShawn",
        )
        for ((name, expected) in cases) assertEquals(name, expected, HomeGreeting.firstName(name))
    }

    @Test
    fun noUsableFirstName() {
        for (name in listOf(null, "", "   ", "Smith,", "J. Smith", "Mr", "12345678")) {
            assertNull(name, HomeGreeting.firstName(name))
            assertEquals("Hello!", HomeGreeting.title(name))
        }
    }

    @Test
    fun greetingTitle() {
        assertEquals("Hello Test!", HomeGreeting.title("Borrower, Test"))
        assertEquals("Hello Test!", HomeGreeting.title("Test Borrower"))
    }
}

class LoanShelfItemTest {
    private val today = LocalDate.of(2026, 10, 24)

    private fun loan(title: String, due: String?, bib: String? = null) = PatronItem(
        irn = null, bibIrn = bib ?: title, title = title, bibUrl = null, author = "Haig, Matt", year = null,
        coverUrls = emptyList(), isbn = null, fields = due?.let { mapOf("Due" to it) }.orEmpty(), actions = emptyList(),
    )

    private fun item(due: String?) = LoanShelfItem.from(loan("Book", due), today)

    @Test
    fun captionsAndTones() {
        assertEquals("2 days overdue", item("22 Oct 2026").caption)
        assertEquals(LoanShelfItem.Tone.OVERDUE, item("22 Oct 2026").tone)
        assertEquals("1 day overdue", item("23 Oct 2026").caption)
        assertEquals("Due today", item("24 Oct 2026").caption)
        assertEquals(LoanShelfItem.Tone.SOON, item("24 Oct 2026").tone)
        assertEquals("3 days left", item("27 Oct 2026").caption)
        assertEquals(LoanShelfItem.Tone.SOON, item("27 Oct 2026").tone)
        assertEquals(LoanShelfItem.Tone.NORMAL, item("28 Oct 2026").tone)
        assertEquals("14 days left", item("7 Nov 2026").caption)
        // Beyond two weeks the date reads better than a count.
        val far = item("20 Nov 2026")
        assertEquals("Due ${DateTimeFormatter.ofPattern("d MMM", Locale.getDefault()).format(far.dueDate)}", far.caption)
        assertEquals(LoanShelfItem.Tone.NORMAL, far.tone)
        // Other date styles the library uses.
        assertEquals("3 days left", item("27/10/2026").caption)
        // Unreadable dates are shown as the library wrote them; no date, no caption.
        assertEquals("Due Recalled", item("Recalled").caption)
        assertNull(item("Recalled").dueDate)
        assertNull(item(null).caption)
    }

    @Test
    fun loansAreSortedByDueDateWithUndatedLast() {
        val items = LoanShelfItem.make(
            listOf(
                loan("Later", "10 Nov 2026"),
                loan("Undated A", null),
                loan("Overdue", "20 Oct 2026"),
                loan("Soon", "26 Oct 2026"),
                loan("Undated B", "?"),
                loan("Soon", "26 Oct 2026"), // the same loan listed twice
            ),
            today,
        )
        assertEquals(listOf("Overdue", "Soon", "Later", "Undated A", "Undated B"), items.map { it.ref.title })
    }
}

@OptIn(ExperimentalCoroutinesApi::class)
class HomeModelTest {
    private val report: BrowseReporter = { UserFacingError.from(it, "Canning") }
    private val f = SearchFixtures

    private fun context(backend: PagingFakeBackend, verification: Int = 0) =
        HomeLoadContext(backend, HomeReloadKey(backend.origin, verification), report)

    @Test
    fun arrivalsLoadOncePerKey() = runTest {
        val records = (1..14).map { f.record(it.toString()) } + f.record("1")
        val backend = PagingFakeBackend(Result.success(f.page(records)))
        val home = HomeModel()

        home.loadIfNeeded(context(backend))
        assertEquals(HomeModel.SHELF_LIMIT, home.arrivals.value.valueOrNull?.size)
        // New arrivals is the first shelf, newest first.
        assertEquals(listOf("${CoreHelpers.homeShelves()[0].query}|NEWEST"), backend.searchCalls)

        // Same key again (back from a pushed screen, or a sign-in): nothing reloads.
        home.loadIfNeeded(context(backend))
        assertEquals(1, backend.searchCalls.size)

        // A new library or a completed verification reloads it.
        home.loadIfNeeded(context(backend, verification = 1))
        assertEquals(2, backend.searchCalls.size)
    }

    @Test
    fun theLoansShelfComesFromTheAccountSnapshot() {
        val due = DemoData.formatDate(LocalDate.now())
        val snapshot = AccountSnapshot(
            DemoBackend.SNAPSHOT_VERSION, "Test Borrower",
            PatronList(1, listOf(PatronItem("1", "1", "Whistler", null, null, null, emptyList(), null, mapOf("Due" to due), emptyList()))),
            Reservations(PatronList(0, emptyList()), PatronList(0, emptyList())),
            fetchedAtMs = 0,
        )
        val offline = UserFacingError(UserFacingError.Kind.NETWORK, "Can't reach Canning. Check your connection.")
        assertEquals(listOf("Due today"), HomeLoans.state(snapshot, refreshing = false, error = null).valueOrNull?.map { it.caption })
        // A failed refresh keeps the saved covers.
        assertEquals(listOf("Whistler"), HomeLoans.state(snapshot, refreshing = false, error = offline).valueOrNull?.map { it.ref.title })
        // Nothing saved yet: a skeleton while it loads, else the error card.
        assertEquals(BrowseLoadable.Loading, HomeLoans.state(null, refreshing = true, error = null))
        assertEquals(BrowseLoadable.Loading, HomeLoans.state(null, refreshing = true, error = offline))
        assertEquals(UserFacingError.Kind.NETWORK, HomeLoans.state(null, refreshing = false, error = offline).errorOrNull?.kind)
    }

    @Test
    fun aShelfLoadingForTheKeyIsNotStartedAgain() = runTest {
        // Home asks again when it comes back into view (tab switch, back from a pushed screen).
        val backend = PagingFakeBackend(Result.success(f.page(listOf(f.record("1")))))
        backend.searchDelayMs = 100
        val home = HomeModel()
        val first = launch { home.loadIfNeeded(context(backend)) }
        advanceTimeBy(20)
        home.loadIfNeeded(context(backend))
        first.join()
        assertEquals(1, backend.searchCalls.size)
        assertEquals(1, home.arrivals.value.valueOrNull?.size)
    }

    @Test
    fun refreshKeepsTheCoversWhenItFails() = runTest {
        val backend = PagingFakeBackend(Result.success(f.page(listOf(f.record("1")))))
        val home = HomeModel()
        home.loadIfNeeded(context(backend))
        backend.first = Result.failure(SpydusException.Network("offline"))
        home.refresh(context(backend))
        assertEquals(1, home.arrivals.value.valueOrNull?.size)
    }

    @Test
    fun anUnrecognisedArrivalsPageIsAnError() = runTest {
        val backend = PagingFakeBackend(Result.success(f.page(emptyList(), noResults = false)))
        val home = HomeModel()
        home.loadIfNeeded(context(backend))
        assertEquals(SearchResultsModel.UNRECOGNISED_PAGE, home.arrivals.value.errorOrNull)
    }

    @Test
    fun aGenuinelyEmptyShelfIsEmptyNotAnError() = runTest {
        val backend = PagingFakeBackend(Result.success(f.page(emptyList(), noResults = true)))
        val home = HomeModel()
        home.loadIfNeeded(context(backend))
        assertEquals(BrowseLoadable.Loaded(emptyList<Any>()), home.arrivals.value)
    }

    @Test
    fun everyGenreHasAnIconAndARoute() {
        val genres = CoreHelpers.homeGenres()
        assertEquals(8, genres.size)
        for (genre in genres) {
            assertNotEquals("no icon for ${genre.key}", HomeGenreIcon.BOOKS, HomeGenreIcon.forKey(genre.key))
            assertEquals(Route.Results(genre.query, ResultsSort.RELEVANCE, genre.label), HomeModel.route(genre))
        }
        assertEquals(8, genres.map { HomeGenreIcon.forKey(it.key) }.toSet().size)
        assertEquals(HomeGenreIcon.BOOKS, HomeGenreIcon.forKey("unknown"))
    }

    @Test
    fun arrivalsViewAllOpensTheShelfQueryNewestFirst() {
        val shelf = CoreHelpers.homeShelves()[0]
        assertEquals(Route.Results(shelf.query, ResultsSort.NEWEST, shelf.title), HomeModel().arrivalsRoute)
    }
}
