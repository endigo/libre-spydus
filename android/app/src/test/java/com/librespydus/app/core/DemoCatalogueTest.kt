package com.librespydus.app.core

import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.librespydus_core.HoldingKind
import java.io.File
import java.time.LocalDate
import kotlin.time.Duration

/**
 * Demo mode's offline books (docs/native-apps.md §5.4): the demo account behaves like a real one —
 * its rows' own links load empty records, the session-free URL loads the full one, and one book
 * has two editions of which the account holds one.
 */
class DemoCatalogueTest {
    private val today = LocalDate.of(2026, 10, 12)
    private val origin = "https://hutt.spydus.co.nz"

    private fun backend(public: FakeBackend = FakeBackend(origin)) = DemoBackend(
        public,
        DemoData.load(read = { File("src/main/assets/demo/$it").readText() }, origin = origin, today = today),
        today,
        mutationDelay = Duration.ZERO,
        snapshotDelay = Duration.ZERO,
    )

    @Test
    fun everyDemoRowLoadsOnlyWithTheSessionFreeUrl() = runTest {
        val public = FakeBackend(origin)
        val b = backend(public)
        val session = b.login("demo", "demo")
        val snapshot = b.accountSnapshot(session)
        val rows = snapshot.loans.items + snapshot.reservations.ready.items + snapshot.reservations.pending.items
        assertEquals(7, rows.size)
        for (row in rows) {
            val irn = row.bibIrn!!
            assertEquals("$origin/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/78500000?SETLVL=&QRY=IRN($irn)&NAVLVL=SET", row.bibUrl)
            // Like the real OPAC without the patron's session: an empty record.
            val viaRow = b.record(row.bibUrl!!)
            assertNull(viaRow.title)
            assertNull(viaRow.summary)
            assertTrue(viaRow.details.isEmpty())

            val ref = BookRef.from(row)
            assertEquals("$origin/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&IRN=$irn", ref.url)
            val record = b.record(ref.url!!)
            assertEquals(row.title, irn, record.bibIrn)
            assertFalse(row.title, record.title.isNullOrBlank())
            assertFalse(row.title, record.summary.isNullOrBlank())
            assertTrue(row.title, record.details.any { it.label == "Subject" && it.values.isNotEmpty() })
            // The bookmark a favourite stores loads it too.
            assertEquals(record, b.record(record.bookmarkUrl!!))
        }
        assertTrue("never asked the real library", public.searches.isEmpty())
    }

    @Test
    fun aSearchForADemoTitleGivesItsEditions() = runTest {
        val public = FakeBackend(origin)
        val b = backend(public)
        // The book page's own search (title + surname) and a typed title both find it.
        for (query in listOf("The midnight library haig", "midnight library")) {
            val hit = b.searchSorted(query, ResultsSort.RELEVANCE).records.single()
            assertEquals("15642035", hit.workIrn)
            assertEquals(listOf("BK", "LP", "EBK"), hit.formats.map { it.code })
        }
        val hit = b.search("midnight library").records.single()
        val irns = hit.formats.map { b.availability(it.availabilityUrl!!).bibRecordIrn }
        assertEquals(listOf("8753080", "99000102", "99000103"), irns)
        assertNull(hit.formats[2].reserveUrl) // the eBook can't be reserved
        assertEquals("4 copies · 2 reserved", CoreHelpers.availabilitySummary(b.availability(hit.formats[0].availabilityUrl!!)).label)
        // The format's own URL loads that edition's record.
        assertEquals("99000102", b.record(hit.formats[1].availabilityUrl!!).bibIrn)
        assertEquals("8753080", b.record(hit.detailUrl!!).bibIrn)

        // Only the book is on loan; the large print isn't held.
        val snapshot = b.accountSnapshot(b.login("demo", "demo"))
        assertEquals(HoldingKind.LOAN, CoreHelpers.accountHolding(snapshot, listOf("8753080"))?.kind)
        assertNull(CoreHelpers.accountHolding(snapshot, listOf("99000102")))
        // The Thursday Murder Club reservation is the large print, not the default book.
        val murder = b.search("thursday murder club").records.single()
        assertEquals(listOf("BK", "LP"), murder.formats.map { it.code })
        assertEquals("8753740", b.availability(murder.formats[1].availabilityUrl!!).bibRecordIrn)
        assertEquals(HoldingKind.RESERVATION_PENDING, CoreHelpers.accountHolding(snapshot, listOf("8753740"))?.kind)

        // Partial titles and anything else go to the real library.
        assertEquals("Project Hail Mary", b.search("midnight").records.single().title)
        assertEquals(listOf("midnight"), public.searches)
        assertNull(b.record("$origin/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&IRN=1").bibIrn)
    }

    @Test
    fun reservingADemoFormatAddsARowForThatEdition() = runTest {
        val b = backend()
        val session = b.login("demo", "demo")
        val largePrint = b.search("midnight library").records.single().formats.first { it.code == "LP" }

        assertTrue(b.placeReservation(largePrint.reserveUrl!!, session, xdays = "0", pickup = null).ok)

        val row = b.accountSnapshot(session).reservations.pending.items.first()
        assertEquals("The midnight library", row.title)
        assertEquals("99000102", row.bibIrn)
        assertEquals("$origin/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/78500000?SETLVL=&QRY=IRN(99000102)&NAVLVL=SET", row.bibUrl)
        // The book page now finds it for the large print.
        val holding = CoreHelpers.accountHolding(b.accountSnapshot(session), listOf("99000102"))
        assertEquals(HoldingKind.RESERVATION_PENDING, holding?.kind)
        assertNotNull(b.record(BookRef.from(row).url!!).title)
    }
}
