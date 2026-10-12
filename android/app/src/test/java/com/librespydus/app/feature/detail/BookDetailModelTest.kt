package com.librespydus.app.feature.detail

import com.librespydus.app.core.BookRef
import com.librespydus.app.core.DemoBackend
import com.librespydus.app.core.DemoData
import com.librespydus.app.core.FakeBackend
import com.librespydus.app.core.LibraryBackend
import com.librespydus.app.core.ResultsSort
import com.librespydus.app.core.UserFacingError
import com.librespydus.app.feature.search.BrowseReporter
import com.librespydus.app.feature.search.errorOrNull
import com.librespydus.app.feature.search.valueOrNull
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
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
import uniffi.librespydus_core.RecordRelated
import uniffi.librespydus_core.RelatedList
import uniffi.librespydus_core.Reservations
import uniffi.librespydus_core.ReserveOptions
import uniffi.librespydus_core.ReserveResult
import uniffi.librespydus_core.SearchRecord
import uniffi.librespydus_core.SearchResults
import uniffi.librespydus_core.SpydusException
import java.io.File
import java.time.LocalDate
import kotlin.time.Duration

/**
 * An offline backend for detail tests: canned records, searches, availability and related lists;
 * it records the URLs asked for. Account calls are unused.
 */
class DetailBackend(
    private val records: MutableMap<String, Result<RecordDetail>> = mutableMapOf(),
    private val searchRecords: List<SearchRecord> = emptyList(),
    private val availabilityByUrl: Map<String, Availability> = emptyMap(),
    private val relatedByUrl: Map<String, Result<RelatedList>> = emptyMap(),
) : LibraryBackend {
    override val origin = "https://canning.spydus.com"
    val recordUrls = mutableListOf<String>()
    val searches = mutableListOf<String>()
    val relatedUrls = mutableListOf<String>()

    fun setRecord(url: String, result: Result<RecordDetail>) {
        records[url] = result
    }

    override fun setVerificationCookie(cookie: String?) = Unit

    override suspend fun search(query: String): SearchResults {
        searches += query
        return SearchResults(query, "1", searchRecords.size.toLong(), null, null, null, searchRecords.isEmpty(), searchRecords)
    }

    override suspend fun searchSorted(query: String, sort: ResultsSort) = search(query)
    override suspend fun page(url: String) = search(url)
    override suspend fun availability(url: String): Availability = availabilityByUrl[url] ?: throw SpydusException.Upstream("no availability")

    override suspend fun record(url: String): RecordDetail {
        recordUrls += url
        return (records[url] ?: throw SpydusException.Upstream("no bib record link in format fragment")).getOrThrow()
    }

    override suspend fun related(url: String): RelatedList {
        relatedUrls += url
        return (relatedByUrl[url] ?: throw SpydusException.Network("timed out")).getOrThrow()
    }

    override suspend fun login(username: String, password: String): String = throw SpydusException.CredentialsRejected("")
    override suspend fun account(session: String): AccountLinks = throw SpydusException.NotLoggedIn("")
    override suspend fun loans(session: String): PatronList = throw SpydusException.NotLoggedIn("")
    override suspend fun reservations(session: String): Reservations = throw SpydusException.NotLoggedIn("")
    override suspend fun accountSnapshot(session: String): AccountSnapshot = throw SpydusException.NotLoggedIn("")
    override suspend fun reserveOptions(url: String, session: String): ReserveOptions = throw SpydusException.NotLoggedIn("")
    override suspend fun placeReservation(url: String, session: String, xdays: String?, pickup: String?): ReserveResult = error("never POST in tests")
    override suspend fun cancelReservation(url: String, session: String): ReserveResult = error("never POST in tests")
    override suspend fun editOptions(url: String, session: String): EditOptions = throw SpydusException.NotLoggedIn("")
    override suspend fun editReservation(url: String, session: String, overrides: EditOverrides): ReserveResult = error("never POST in tests")
}

/** [BookDetailModel] against an offline backend. */
class BookDetailModelTest {
    private val d = DetailFixtures
    private val report: BrowseReporter = { UserFacingError.from(it, "Canning") }

    @Test
    fun searchRefsLoadTheBibRecordThroughTheSelectedFormat() = runTest {
        val ref = BookRef.from(d.searchRecord)
        val bk = ref.formats[0].availabilityUrl!!
        val backend = DetailBackend(
            records = mutableMapOf(bk to Result.success(d.midnightLibrary)),
            availabilityByUrl = mapOf(bk to d.availability(mapOf("copies" to 2L))),
        )
        val model = BookDetailModel(ref)
        assertEquals("BK", model.state.value.selectedFormat?.code)
        assertEquals(HeaderAvailability.Loading(DetailState.CHECKING), model.state.value.headerAvailability)

        model.load(backend, report)
        model.loadAvailability(backend, report)

        assertEquals(listOf(bk), backend.recordUrls)
        assertTrue(backend.searches.isEmpty())
        val state = model.state.value
        assertEquals(HeaderAvailability.Line(AvailabilityLine("2 copies", AvailabilityLine.Tone.NORMAL)), state.headerAvailability)
        assertEquals("BK", state.reservableFormat?.code)
        assertEquals(6, state.subjects.size)

        model.select(1) // eBook: no reserve URL
        assertNull(model.state.value.reservableFormat)
        assertEquals(HeaderAvailability.Loading(DetailState.CHECKING), model.state.value.headerAvailability)
        model.loadAvailability(backend, report) // the eBook has no counts here: the fake fails it
        assertEquals(HeaderAvailability.AvailabilityFailed, model.state.value.headerAvailability)
    }

    @Test
    fun aFailedFormatUrlFallsBackToTheRefUrl() = runTest {
        val ref = BookRef.from(d.searchRecord)
        val backend = DetailBackend(records = mutableMapOf(ref.url!! to Result.success(d.midnightLibrary)))
        val model = BookDetailModel(ref)
        model.load(backend, report)
        assertEquals(listOf(ref.formats[0].availabilityUrl!!, ref.url), backend.recordUrls)
        assertEquals(d.midnightLibrary, model.state.value.record.valueOrNull)
    }

    @Test
    fun aNetworkFailureDoesNotTryTheOtherUrl() = runTest {
        val ref = BookRef.from(d.searchRecord)
        val bk = ref.formats[0].availabilityUrl!!
        val backend = DetailBackend(records = mutableMapOf(bk to Result.failure(SpydusException.Network("offline"))))
        val model = BookDetailModel(ref)
        model.load(backend, report)
        assertEquals(listOf(bk), backend.recordUrls)
        assertEquals(UserFacingError.Kind.NETWORK, model.state.value.record.errorOrNull?.kind)
    }

    @Test
    fun favouriteKeepsTheIdStoresTheBookmarkAndFormats() = runTest {
        val ref = BookRef.from(d.searchRecord)
        val backend = DetailBackend(records = mutableMapOf(ref.formats[0].availabilityUrl!! to Result.success(d.midnightLibrary)))
        val model = BookDetailModel(ref)
        model.load(backend, report)
        val saved = model.state.value.favouriteRef
        assertEquals(ref.id, saved.id)
        assertEquals(d.midnightLibrary.bookmarkUrl, saved.url)
        assertEquals(ref.formats, saved.formats)
    }

    @Test
    fun refsWithoutFormatsReFindThemBySearch() = runTest {
        val ref = BookRef.from(d.related("467086", "The long night", author = "White, Christian", isbn = "9781399197809"))
        val backend = DetailBackend(records = mutableMapOf(ref.url!! to Result.success(d.longNight)), searchRecords = d.longNightSearch)
        val model = BookDetailModel(ref)
        assertEquals(FormatLookup.Pending, model.state.value.formatLookup)
        assertEquals(HeaderAvailability.Loading(DetailState.FINDING), model.state.value.headerAvailability)
        assertNull(model.state.value.reservableFormat)

        model.load(backend, report)

        val state = model.state.value
        assertEquals(listOf("The long night white"), backend.searches)
        assertEquals(FormatLookup.Found, state.formatLookup)
        assertEquals(listOf("BK"), state.formats.map { it.code })
        assertEquals("BK", state.reservableFormat?.code)
        assertEquals(listOf("BK"), state.favouriteRef.formats.map { it.code })
        assertEquals(d.longNight.bookmarkUrl, state.favouriteRef.url)
    }

    @Test
    fun aWorkLevelRecordIsUpgradedToTheBibRecordOnceFormatsAreFound() = runTest {
        val workUrl = "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/WRKENQ?SETLVL=&IRN=15642035"
        val work = RecordDetail(
            bibIrn = "15642035", brn = null, title = "The Midnight Library", seriesTitle = null, isbn = "1786892707",
            coverUrls = emptyList(), summary = "Somewhere out beyond…", details = listOf(d.row("Creator", "Haig, Matt, 1975-, author")),
            bookmarkUrl = workUrl, related = RecordRelated(null, "https://canning.spydus.com/works-by", null, null),
        )
        val hit = d.search(work = "15642035", title = "The Midnight Library", author = "Haig, Matt, 1975-")
        val backend = DetailBackend(
            records = mutableMapOf(workUrl to Result.success(work), hit.formats[0].availabilityUrl!! to Result.success(d.midnightLibrary)),
            searchRecords = listOf(hit),
        )
        val model = BookDetailModel(BookRef.fromUrl(workUrl))
        model.load(backend, report)

        val state = model.state.value
        assertEquals(listOf(workUrl, hit.formats[0].availabilityUrl!!), backend.recordUrls)
        assertEquals(FormatLookup.Found, state.formatLookup)
        assertEquals(d.midnightLibrary, state.record.valueOrNull)
        assertEquals(d.midnightLibrary.bookmarkUrl, state.favouriteRef.url)
        assertEquals(listOf(RelatedPanel.SIMILAR, RelatedPanel.AUTHOR, RelatedPanel.VERSIONS), state.visiblePanels)
    }

    @Test
    fun noConfidentMatchHidesAvailabilityAndBorrow() = runTest {
        val ref = BookRef("bib:1", "Hollow bones", author = "Mayne, Andrew", url = "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&BRN=465783")
        val record = RecordDetail(
            bibIrn = null, brn = "465783", title = "Hollow bones", seriesTitle = null, isbn = null, coverUrls = emptyList(),
            summary = "A thriller.", details = emptyList(), bookmarkUrl = ref.url, related = RecordRelated(null, null, null, null),
        )
        val backend = DetailBackend(
            records = mutableMapOf(ref.url!! to Result.success(record)),
            searchRecords = listOf(d.search(work = "9", title = "Hollow Bones", author = "Smith, Jo")),
        )
        val model = BookDetailModel(ref)
        model.load(backend, report)
        assertEquals(FormatLookup.NotFound, model.state.value.formatLookup)
        assertEquals(HeaderAvailability.CheckWebsite, model.state.value.headerAvailability)
        assertNull(model.state.value.reservableFormat)
    }

    @Test
    fun aFailedRecordShowsAnErrorAndRetries() = runTest {
        val ref = BookRef.fromUrl("https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&BRN=275039")
        val backend = DetailBackend()
        val model = BookDetailModel(ref)
        model.load(backend, report)
        assertEquals(UserFacingError.Kind.UNEXPECTED, model.state.value.record.errorOrNull?.kind)
        assertEquals(HeaderAvailability.None, model.state.value.headerAvailability)

        backend.setRecord(ref.url!!, Result.success(d.midnightLibrary))
        model.load(backend, report)
        assertNotNull(model.state.value.record.valueOrNull)
        assertEquals("The Midnight Library", model.state.value.ref.title)
        assertEquals("Haig, Matt, 1975-, author", model.state.value.ref.author)
    }

    @Test
    fun relatedPanelsLoadConcurrentlyAndHideEmptyOrFailedOnes() = runTest {
        val ref = BookRef.from(d.searchRecord)
        val backend = DetailBackend(
            records = mutableMapOf(ref.formats[0].availabilityUrl!! to Result.success(d.midnightLibrary)),
            relatedByUrl = mapOf(
                "https://canning.spydus.com/similar" to Result.success(RelatedList(listOf(d.related("467086", "The long night")))),
                "https://canning.spydus.com/author" to Result.success(RelatedList(listOf(d.related("275039", "The Midnight Library")))),
            ),
        )
        val model = BookDetailModel(ref)
        model.load(backend, report)
        assertEquals(listOf(RelatedPanel.SIMILAR, RelatedPanel.AUTHOR, RelatedPanel.VERSIONS), model.state.value.visiblePanels)

        model.loadRelated(backend)

        // "More by this author" only had this book; "Other editions" failed.
        assertEquals(listOf(RelatedPanel.SIMILAR), model.state.value.visiblePanels)
        assertEquals(listOf("brn:467086"), model.state.value.related[RelatedPanel.SIMILAR]?.valueOrNull?.map { it.id })
        assertEquals(
            setOf("https://canning.spydus.com/similar", "https://canning.spydus.com/author", "https://canning.spydus.com/versions"),
            backend.relatedUrls.toSet(),
        )
        // Loaded panels aren't fetched again.
        model.loadRelated(backend)
        assertEquals(4, backend.relatedUrls.size)
    }
}

/** [ReserveModel] against the demo backend (fixture account data; nothing is ever posted). */
class ReserveModelTest {
    private fun demo() = DemoBackend(
        FakeBackend(),
        DemoData.load({ File("src/main/assets/demo/$it").readText() }, "https://canning.spydus.com", LocalDate.of(2026, 10, 12)),
        mutationDelay = Duration.ZERO,
    )

    @Test
    fun reserveDefaultsToTheFirstExpiryAndSubmitsOnce() = runTest {
        val demo = demo()
        val model = ReserveModel()
        val rootSheetErrors = mutableListOf<Throwable>()
        var placed = 0
        model.loadOptions(DetailFixtures.RESERVE_URL, demo, DemoBackend.SESSION, "Canning") { rootSheetErrors += it }
        val options = model.state.value.options.valueOrNull!!
        assertTrue(options.canReserve)
        assertEquals(options.expiryChoices.first().days, model.state.value.selectedDays)

        model.selectDays(options.expiryChoices.last().days)
        model.confirm(DetailFixtures.RESERVE_URL, demo, DemoBackend.SESSION, "Canning", { rootSheetErrors += it }, onPlaced = { placed++ })
        assertEquals(true, model.state.value.result?.ok)
        assertTrue(model.state.value.isComplete)
        assertFalse(model.state.value.isSubmitting)
        assertEquals(1, placed)

        // Confirm again does nothing: a reservation can't be sent twice.
        val pendingBefore = demo.reservations(DemoBackend.SESSION).pending.items.size
        model.confirm(DetailFixtures.RESERVE_URL, demo, DemoBackend.SESSION, "Canning", { rootSheetErrors += it }, onPlaced = { placed++ })
        assertEquals(pendingBefore, demo.reservations(DemoBackend.SESSION).pending.items.size)
        assertEquals(1, placed)
        assertTrue(rootSheetErrors.isEmpty())
    }

    @Test
    fun reserveWithoutASessionAsksForSignIn() = runTest {
        val model = ReserveModel()
        val rootSheetErrors = mutableListOf<Throwable>()
        model.loadOptions(DetailFixtures.RESERVE_URL, DetailBackend(), session = null, libraryName = "Canning") { rootSheetErrors += it }
        assertEquals(1, rootSheetErrors.size)
        assertTrue(rootSheetErrors.first() is SpydusException.NotLoggedIn)

        // A session the library no longer accepts goes the same way (DetailBackend throws NotLoggedIn).
        model.loadOptions(DetailFixtures.RESERVE_URL, DetailBackend(), session = "stale", libraryName = "Canning") { rootSheetErrors += it }
        assertEquals(2, rootSheetErrors.size)
        assertNull(model.state.value.options.errorOrNull)
    }

    @Test
    fun aFormatWithoutAReserveUrlCantBeReserved() = runTest {
        val model = ReserveModel()
        model.loadOptions(null, DetailBackend(), DemoBackend.SESSION, "Canning") { error("no root sheet expected") }
        assertEquals("This format can't be reserved from the app.", model.state.value.options.errorOrNull?.message)
    }

    @Test
    fun pickupNamesAndMessagesReadNaturally() {
        assertEquals("Riverton Library", ReserveModel.titleCased("RIVERTON LIBRARY"))
        assertEquals("McKenzie Library", ReserveModel.titleCased("McKenzie Library"))
        assertEquals("Reservation placed.", ReserveModel.message(ReserveResult(true, null)))
        assertEquals("Already reserved.", ReserveModel.message(ReserveResult(false, "Already reserved.")))
        assertEquals("The library couldn't place this reservation.", ReserveModel.message(ReserveResult(false, " ")))
    }
}
