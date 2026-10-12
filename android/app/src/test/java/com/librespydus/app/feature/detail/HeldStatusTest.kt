package com.librespydus.app.feature.detail

import com.librespydus.app.core.AvailabilitySummary
import com.librespydus.app.core.BookRef
import com.librespydus.app.core.BorrowedSegment
import com.librespydus.app.core.DemoBackend
import com.librespydus.app.core.FormatRef
import com.librespydus.app.feature.search.BrowseLoadable
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.librespydus_core.AccountHolding
import uniffi.librespydus_core.AccountSnapshot
import uniffi.librespydus_core.HoldingKind
import uniffi.librespydus_core.PatronItem
import uniffi.librespydus_core.PatronList
import uniffi.librespydus_core.RecordDetail
import uniffi.librespydus_core.RecordRelated
import uniffi.librespydus_core.Reservations
import java.time.LocalDate

/** The book page's "Already yours" rules (docs/native-apps.md §6): IRNs, the holding, the pill. */
class HeldStatusTest {
    private val today = LocalDate.of(2026, 10, 12)
    private val origin = "https://canning.spydus.com/cgi-bin/spydus.exe"

    private fun item(bibIrn: String, fields: Map<String, String> = emptyMap()) = PatronItem(
        irn = "1$bibIrn", bibIrn = bibIrn, title = "The midnight library",
        bibUrl = "$origin/ENQ/OPAC/BIBENQ/78577835?SETLVL=&QRY=IRN($bibIrn)&NAVLVL=SET",
        author = "Haig, Matt", year = "2020", coverUrls = emptyList(), isbn = null, fields = fields, actions = emptyList(),
    )

    private fun status(kind: HoldingKind, fields: Map<String, String>) = HeldStatus.of(AccountHolding(kind, item("8753080", fields)), today)

    private fun snapshot(loans: List<PatronItem> = emptyList(), ready: List<PatronItem> = emptyList(), pending: List<PatronItem> = emptyList()) =
        AccountSnapshot(
            DemoBackend.SNAPSHOT_VERSION, "Test Borrower", PatronList(loans.size.toLong(), loans),
            Reservations(PatronList(ready.size.toLong(), ready), PatronList(pending.size.toLong(), pending)), 0L,
        )

    // region Pill

    @Test
    fun aLoanShowsItsDueDate() {
        val loan = status(HoldingKind.LOAN, mapOf("Due" to "14 Oct 2026"))
        assertEquals("On loan · Due 14 Oct", loan.text)
        assertEquals("On loan, due 14 October", loan.accessibilityLabel)
        assertEquals(HeldStatus.Tone.LOAN, loan.tone)
        assertEquals(BorrowedSegment.LOANS, loan.segment)
        // Due today is not overdue yet; other date styles read the same way.
        assertEquals("On loan · Due 12 Oct", status(HoldingKind.LOAN, mapOf("Due" to "12/10/2026")).text)
    }

    @Test
    fun anOverdueLoanSaysSoInTheDangerTone() {
        val overdue = status(HoldingKind.LOAN, mapOf("Due" to "11 Oct 2026"))
        assertEquals("On loan · Overdue", overdue.text)
        assertEquals("On loan, overdue, was due 11 October", overdue.accessibilityLabel)
        assertEquals(HeldStatus.Tone.OVERDUE, overdue.tone)
        assertEquals(BorrowedSegment.LOANS, overdue.segment)
    }

    @Test
    fun aLoanWithoutAReadableDate() {
        assertEquals("On loan", status(HoldingKind.LOAN, emptyMap()).text)
        val raw = status(HoldingKind.LOAN, mapOf("Due" to "end of term"))
        assertEquals("On loan · Due end of term", raw.text)
        assertEquals("On loan, due end of term", raw.accessibilityLabel)
        assertEquals(HeldStatus.Tone.LOAN, raw.tone)
    }

    @Test
    fun aReadyReservationNamesThePickupBranch() {
        val ready = status(HoldingKind.RESERVATION_READY, mapOf("Status" to "Awaiting pickup", "Pickup location" to "RIVERTON LIBRARY"))
        assertEquals("Ready to collect · Riverton Library", ready.text)
        assertEquals("Ready to collect at Riverton Library", ready.accessibilityLabel)
        assertEquals(HeldStatus.Tone.READY, ready.tone)
        assertEquals(BorrowedSegment.RESERVATIONS, ready.segment)
        // Canning writes the branch into Status.
        assertEquals(
            "Ready to collect · Riverton Library",
            status(HoldingKind.RESERVATION_READY, mapOf("Status" to "Please pickup from: RIVERTON LIBRARY by 17 Oct 2026")).text,
        )
        assertEquals("Ready to collect", status(HoldingKind.RESERVATION_READY, mapOf("Status" to "Awaiting pickup")).text)
    }

    @Test
    fun aPendingReservationNamesTheBranchElseTheQueue() {
        val withBranch = status(HoldingKind.RESERVATION_PENDING, mapOf("Status" to "Waiting", "Rank" to "2", "Pickup location" to "Riverton Library"))
        assertEquals("Reserved · Riverton Library", withBranch.text)
        assertEquals("Reserved, pick up at Riverton Library", withBranch.accessibilityLabel)
        assertEquals(HeldStatus.Tone.RESERVED, withBranch.tone)
        assertEquals(BorrowedSegment.RESERVATIONS, withBranch.segment)
        assertEquals(
            "Reserved · Riverton Library",
            status(HoldingKind.RESERVATION_PENDING, mapOf("Status" to "Requested pickup location: RIVERTON LIBRARY")).text,
        )

        val queued = status(HoldingKind.RESERVATION_PENDING, mapOf("Status" to "Waiting", "Rank" to "2"))
        assertEquals("Reserved · #2 in queue", queued.text)
        assertEquals("Reserved, number 2 in the queue", queued.accessibilityLabel)
        val ranked = status(HoldingKind.RESERVATION_PENDING, mapOf("Rank" to "2 of 5"))
        assertEquals("Reserved · Rank 2 of 5", ranked.text)
        assertEquals("Reserved, rank 2 of 5", ranked.accessibilityLabel)

        val plain = status(HoldingKind.RESERVATION_PENDING, mapOf("Status" to "In transit"))
        assertEquals("Reserved", plain.text)
        assertEquals("Reserved", plain.accessibilityLabel)
    }

    // endregion
    // region Holding

    @Test
    fun noSnapshotOrNoIrnsMeansNoPill() {
        assertNull(HeldStatus.holding(null, listOf("8753080")))
        assertNull(HeldStatus.holding(snapshot(loans = listOf(item("8753080"))), emptyList()))
        assertNull(HeldStatus.holding(snapshot(), listOf("8753080")))
    }

    @Test
    fun aLoanWinsOverAReservationOfTheSameBib() {
        val account = snapshot(
            loans = listOf(item("8753080", mapOf("Due" to "14 Oct 2026"))),
            pending = listOf(item("8753080"), item("99000102")),
        )
        assertEquals(HoldingKind.LOAN, HeldStatus.holding(account, listOf("8753080"))?.kind)
        assertEquals(HoldingKind.RESERVATION_PENDING, HeldStatus.holding(account, listOf("99000102"))?.kind)
        assertNull(HeldStatus.holding(account, listOf("99000103")))
    }

    // endregion
    // region IRNs of the format on screen

    private val bk = FormatRef("BK", "Book", "$origin/reserve/BK", "$origin/avail/BK")
    private val lp = FormatRef("LP", "Large Print", "$origin/reserve/LP", "$origin/avail/LP")
    private val record = RecordDetail("8753080", "275039", "The midnight library", null, null, emptyList(), "A library.", emptyList(), null, RecordRelated(null, null, null, null))

    private fun loaded(irn: String?) =
        BrowseLoadable.Loaded(FormatAvailability(AvailabilitySummary(null, 2, null, "2 copies"), irn))

    @Test
    fun theSelectedFormatsOwnBibIsPreferred() {
        val opened = BookRef("bib:8753080", "The midnight library", bibIrn = "8753080")
        val state = DetailState(
            ref = opened,
            record = BrowseLoadable.Loaded(record),
            formats = listOf(bk, lp),
            selectedFormatIndex = 1,
            formatLookup = FormatLookup.Found,
            availability = mapOf(bk.availabilityUrl!! to loaded("8753080"), lp.availabilityUrl!! to loaded("99000102")),
        )
        assertEquals(listOf("99000102"), state.heldIrns)
        assertEquals(listOf("8753080"), state.copy(selectedFormatIndex = 0).heldIrns)
    }

    @Test
    fun withoutTheFormatsBibTheRecordAndOpenedRowAreUsed() {
        val opened = BookRef("bib:8753080", "The midnight library", bibIrn = "8753080")
        // Nothing loaded yet: the opened loan's bib.
        assertEquals(listOf("8753080"), DetailState(ref = opened).heldIrns)
        // Record loaded, availability still loading (or failed, or not naming a bib).
        val other = record.copy(bibIrn = "8753081")
        val state = DetailState(
            ref = opened, record = BrowseLoadable.Loaded(other), formats = listOf(bk, lp), selectedFormatIndex = 1,
            formatLookup = FormatLookup.Found, availability = mapOf(lp.availabilityUrl!! to BrowseLoadable.Loading),
        )
        assertEquals(listOf("8753081", "8753080"), state.heldIrns)
        assertEquals(listOf("8753081", "8753080"), state.copy(availability = mapOf(lp.availabilityUrl!! to loaded(null))).heldIrns)
        // A page from search: no row, only the record.
        assertEquals(listOf("8753081"), state.copy(ref = BookRef("work:1", "The midnight library")).heldIrns)
        assertTrue(DetailState(ref = BookRef("work:1", "x")).heldIrns.isEmpty())
    }

    @Test
    fun reserveStaysForAFormatThatIsNotHeld() {
        val account = snapshot(loans = listOf(item("8753080", mapOf("Due" to "14 Oct 2026"))))
        val state = DetailState(
            ref = BookRef("work:15642035", "The midnight library", formats = listOf(bk, lp)),
            record = BrowseLoadable.Loaded(record),
            availability = mapOf(bk.availabilityUrl!! to loaded("8753080"), lp.availabilityUrl!! to loaded("99000102")),
        )
        // The book (default) is on loan: the pill.
        assertEquals("BK", state.selectedFormat?.code)
        assertEquals("On loan · Due 14 Oct", HeldStatus.holding(account, state.heldIrns)?.let { HeldStatus.of(it, today) }?.text)
        // The large print is not: Reserve.
        val largePrint = state.copy(selectedFormatIndex = 1)
        assertNull(HeldStatus.holding(account, largePrint.heldIrns))
        assertEquals("LP", largePrint.reservableFormat?.code)
    }

    @Test
    fun aHeldDigitalFormatShowsThePillToo() {
        val ebook = FormatRef("EBK", "eBook", null, "$origin/avail/EBK")
        val account = snapshot(loans = listOf(item("99000103", mapOf("Due" to "20 Oct 2026"))))
        val state = DetailState(
            ref = BookRef("work:15642035", "The midnight library", formats = listOf(ebook)),
            record = BrowseLoadable.Loaded(record),
            availability = mapOf(ebook.availabilityUrl!! to loaded("99000103")),
        )
        assertNull(state.reservableFormat)
        assertEquals(HoldingKind.LOAN, HeldStatus.holding(account, state.heldIrns)?.kind)
    }

    // endregion
    // region Record URLs

    @Test
    fun aSavedLoanLinkAlsoTriesTheSessionFreeUrlFirst() {
        val sessionSet = "$origin/ENQ/OPAC/BIBENQ/78500000?QRY=IRN(8753080)&SETLVL=SET"
        val sessionFree = "$origin/ENQ/OPAC/BIBENQ?SETLVL=&IRN=8753080"
        // A favourite saved from a loan before refs carried bibIrn: only the id knows the IRN.
        val saved = BookRef("bib:8753080", "The midnight library", url = sessionSet)
        assertEquals(sessionFree, DetailRecordSource.sessionFreeUrl(saved))
        assertEquals(listOf(sessionFree, sessionSet), DetailRecordSource.candidateUrls(saved, null))
        assertEquals(listOf(sessionFree, sessionSet, bk.availabilityUrl), DetailRecordSource.candidateUrls(saved, bk))
        // Already session-free, a bookmark, or not a bib ref: unchanged.
        assertNull(DetailRecordSource.sessionFreeUrl(saved.copy(url = sessionFree)))
        val bookmark = "$origin/ENQ/OPAC/BIBENQ?SETLVL=&BRN=275039"
        assertEquals(listOf(bookmark), DetailRecordSource.candidateUrls(saved.copy(url = bookmark), null))
        assertNull(DetailRecordSource.sessionFreeUrl(BookRef("brn:467086", "x", url = "$origin/ENQ/OPAC/BIBENQ/78563101?BRN=467086&SETLVL=")))
    }

    @Test
    fun anEmptyRecordIsNotARecord() {
        val empty = RecordDetail(null, null, null, null, null, emptyList(), null, emptyList(), null, RecordRelated(null, null, null, null))
        assertTrue(DetailRecordSource.isEmpty(empty))
        assertTrue(DetailRecordSource.isEmpty(empty.copy(title = " ", bibIrn = "8753080", bookmarkUrl = "$origin/x")))
        assertFalse(DetailRecordSource.isEmpty(empty.copy(title = "The midnight library")))
        assertFalse(DetailRecordSource.isEmpty(empty.copy(summary = "A library.")))
        assertFalse(DetailRecordSource.isEmpty(record.copy(title = null, summary = null, details = listOf(DetailFixtures.row("ISBN", "1")))))
    }

    // endregion
}
