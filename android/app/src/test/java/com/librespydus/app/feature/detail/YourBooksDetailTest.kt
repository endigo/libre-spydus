package com.librespydus.app.feature.detail

import com.librespydus.app.core.BookRef
import com.librespydus.app.core.FormatRef
import com.librespydus.app.core.UserFacingError
import com.librespydus.app.feature.search.BrowseReporter
import com.librespydus.app.feature.search.errorOrNull
import com.librespydus.app.feature.search.valueOrNull
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.librespydus_core.Availability
import uniffi.librespydus_core.PatronItem
import uniffi.librespydus_core.RecordDetail
import uniffi.librespydus_core.RecordFormat
import uniffi.librespydus_core.RecordRelated
import uniffi.librespydus_core.SearchRecord

/**
 * A book opened from Your Books or Home's loans shelf: its record loads without the patron's OPAC
 * session, and the page shows the format the patron has (docs/native-apps.md §4, §6).
 */
class YourBooksDetailTest {
    private val d = DetailFixtures
    private val report: BrowseReporter = { UserFacingError.from(it, "Canning") }
    private val base = "https://canning.spydus.com/cgi-bin/spydus.exe"

    private fun sessionSet(irn: String) = "$base/ENQ/OPAC/BIBENQ/78577835?SETLVL=&QRY=IRN($irn)&NAVLVL=SET"
    private fun sessionFree(irn: String) = "$base/ENQ/OPAC/BIBENQ?SETLVL=&IRN=$irn"

    private val empty = RecordDetail(null, null, null, null, null, emptyList(), null, emptyList(), null, RecordRelated(null, null, null, null))

    private fun loan(bibIrn: String) = PatronItem(
        irn = "29000004", bibIrn = bibIrn, title = "The midnight library", bibUrl = sessionSet(bibIrn),
        author = "Haig, Matt, author", year = "2020", coverUrls = emptyList(), isbn = null,
        fields = mapOf("Due" to "11 Oct 2026"), actions = emptyList(),
    )

    private fun availability(irn: String) = Availability(irn, null, null, mapOf("copies" to 2L), null, 2L, null, null)

    /** The Midnight Library's work in a search: a Book (8753080) and a Large Print (99000102) edition. */
    private val work = SearchRecord(
        rank = 1, workIrn = "15642035", title = "The Midnight Library",
        detailUrl = "$base/FULL/OPAC/ALLWRKENQ/78563304/15642035,1",
        author = "Haig, Matt, 1975-", authorRole = "author", authorUrl = null, year = "2020", language = "English",
        summary = null, isbn = null, coverUrls = emptyList(),
        formats = listOf(
            RecordFormat("BK", "Book", "$base/reserve/BK", "$base/avail/BK"),
            RecordFormat("LP", "Large Print", "$base/reserve/LP", "$base/avail/LP"),
        ),
    )

    private fun backend(records: Map<String, RecordDetail>, availability: Map<String, Availability> = emptyMap()) =
        DetailBackend(
            records = records.mapValuesTo(mutableMapOf()) { Result.success(it.value) },
            searchRecords = listOf(work),
            availabilityByUrl = availability,
        )

    @Test
    fun aLoanLoadsItsRecordWithTheSessionFreeUrl() = runTest {
        val ref = BookRef.from(loan("8753080"))
        assertEquals("bib:8753080", ref.id)
        val backend = backend(mapOf(sessionFree("8753080") to d.midnightLibrary, sessionSet("8753080") to empty))
        val model = BookDetailModel(ref)
        model.load(backend, report)

        assertEquals(listOf(sessionFree("8753080")), backend.recordUrls)
        val state = model.state.value
        assertEquals(d.midnightLibrary, state.record.valueOrNull)
        assertEquals(6, state.subjects.size)
        assertEquals("bib:8753080", state.ref.id)
        assertEquals("8753080", state.favouriteRef.bibIrn)
    }

    @Test
    fun anEmptyRecordFallsThroughToTheNextUrl() = runTest {
        // Saved before refs used the session-free URL: the old link is tried after it.
        val saved = BookRef("bib:8753080", "The midnight library", url = sessionSet("8753080"))
        val backend = backend(mapOf(sessionFree("8753080") to empty, sessionSet("8753080") to d.midnightLibrary))
        val model = BookDetailModel(saved)
        model.load(backend, report)
        assertEquals(listOf(sessionFree("8753080"), sessionSet("8753080")), backend.recordUrls.take(2))
        assertEquals(d.midnightLibrary, model.state.value.record.valueOrNull)
    }

    @Test
    fun onlyEmptyRecordsEndInTheErrorStateAndRetryWorks() = runTest {
        val ref = BookRef.from(loan("8753080"))
        val backend = backend(mapOf(sessionFree("8753080") to empty))
        val model = BookDetailModel(ref)
        model.load(backend, report)

        val error = model.state.value.record.errorOrNull
        assertEquals(UserFacingError.Kind.UNEXPECTED, error?.kind)
        assertEquals(UserFacingError.UNEXPECTED, error?.message)
        assertEquals(HeaderAvailability.None, model.state.value.headerAvailability)
        assertTrue(backend.searches.isEmpty())

        backend.setRecord(sessionFree("8753080"), Result.success(d.midnightLibrary))
        model.load(backend, report)
        assertEquals(d.midnightLibrary, model.state.value.record.valueOrNull)
    }

    @Test
    fun aPageFromYourBooksSelectsTheFormatOnLoan() = runTest {
        // The loan is the large print: the work's default format (Book) isn't it.
        val largePrint = d.midnightLibrary.copy(bibIrn = "99000102", brn = "990102")
        val backend = backend(
            records = mapOf(sessionFree("99000102") to largePrint),
            availability = mapOf("$base/avail/BK" to availability("8753080"), "$base/avail/LP" to availability("99000102")),
        )
        val model = BookDetailModel(BookRef.from(loan("99000102")))
        model.load(backend, report)

        val state = model.state.value
        assertEquals(FormatLookup.Found, state.formatLookup)
        assertEquals(listOf("BK", "LP"), state.formats.map { it.code })
        assertEquals("LP", state.selectedFormat?.code)
        assertEquals(listOf("99000102"), state.heldIrns)
        assertEquals(HeaderAvailability.Line(AvailabilityLine("2 copies", AvailabilityLine.Tone.NORMAL)), state.headerAvailability)
        // Both formats were checked once; the record wasn't reloaded.
        assertEquals(setOf("$base/avail/BK", "$base/avail/LP"), state.availability.keys)
        assertEquals(listOf(sessionFree("99000102")), backend.recordUrls)
    }

    @Test
    fun withoutAMatchTheDefaultFormatStays() = runTest {
        val backend = backend(
            records = mapOf(sessionFree("8753080") to d.midnightLibrary),
            availability = mapOf("$base/avail/BK" to availability("1"), "$base/avail/LP" to availability("2")),
        )
        val model = BookDetailModel(BookRef.from(loan("8753080")))
        model.load(backend, report)
        assertEquals("BK", model.state.value.selectedFormat?.code)
        // The format's own bib decides what the page is holding, not the loan's.
        assertEquals(listOf("1"), model.state.value.heldIrns)
        model.select(1)
        assertEquals(listOf("2"), model.state.value.heldIrns)
    }

    @Test
    fun aFormatThePatronPickedIsKept() = runTest {
        // A favourite saved from a loan's page keeps the formats and the row's bib.
        val formats = work.formats.map(FormatRef::from)
        val ref = BookRef.from(loan("99000102")).copy(formats = formats)
        val backend = backend(
            records = mapOf(sessionFree("99000102") to d.midnightLibrary.copy(bibIrn = "99000102")),
            availability = mapOf("$base/avail/BK" to availability("8753080"), "$base/avail/LP" to availability("99000102")),
        )
        val held = BookDetailModel(ref)
        held.load(backend, report)
        assertEquals("LP", held.state.value.selectedFormat?.code)

        val picked = BookDetailModel(ref)
        picked.select(0)
        picked.load(backend, report)
        assertEquals("BK", picked.state.value.selectedFormat?.code)
        assertTrue(backend.searches.isEmpty())
    }

    @Test
    fun aPageFromSearchDoesNotLookForAHeldFormat() = runTest {
        val ref = BookRef.from(work)
        val backend = backend(
            records = mapOf("$base/avail/BK" to d.midnightLibrary),
            availability = mapOf("$base/avail/BK" to availability("8753080"), "$base/avail/LP" to availability("99000102")),
        )
        val model = BookDetailModel(ref)
        model.load(backend, report)
        assertEquals("BK", model.state.value.selectedFormat?.code)
        assertTrue(model.state.value.availability.isEmpty())
        model.loadAvailability(backend, report)
        assertEquals(listOf("8753080"), model.state.value.heldIrns)
    }

    @Test
    fun theEditionIsUnknownUntilTheSelectedFormatsAvailabilityIsBack() = runTest {
        val backend = backend(
            records = mapOf("$base/avail/BK" to d.midnightLibrary),
            availability = mapOf("$base/avail/BK" to availability("8753080"), "$base/avail/LP" to availability("99000102")),
        )
        val model = BookDetailModel(BookRef.from(work))
        assertTrue(model.state.value.isIdentifyingEdition)
        model.load(backend, report)
        // The record is in, but it doesn't say which format the page shows.
        assertTrue(model.state.value.isIdentifyingEdition)
        model.loadAvailability(backend, report)
        assertFalse(model.state.value.isIdentifyingEdition)

        model.select(1)
        assertTrue(model.state.value.isIdentifyingEdition)
        // The large print's availability is loading: the book's IRN is known to be the other format's.
        assertEquals(emptyList<String>(), model.state.value.heldIrns)
        model.loadAvailability(backend, report)
        assertFalse(model.state.value.isIdentifyingEdition)
        assertEquals(listOf("99000102"), model.state.value.heldIrns)
    }

    @Test
    fun aFailedLookupStopsWaitingForTheEdition() = runTest {
        val backend = backend(records = emptyMap())
        val noRecord = BookDetailModel(BookRef.from(loan("8753080")))
        noRecord.load(backend, report)
        assertTrue(noRecord.state.value.record.errorOrNull != null)
        assertFalse(noRecord.state.value.isIdentifyingEdition)

        val noAvailability = BookDetailModel(BookRef.from(work))
        noAvailability.loadAvailability(backend, report)
        assertFalse(noAvailability.state.value.isIdentifyingEdition)
    }
}
