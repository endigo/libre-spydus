package com.librespydus.app.feature.borrowed

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.librespydus_core.PatronItem

/** Canning puts the pickup branch in a reservation's Status field (real list pages, 2026-10-12). */
class StatusTextTest {
    @Test fun pendingStatusNamesTheBranch() {
        assertEquals(StatusText(pickup = "RIVERTON LIBRARY"), StatusText.parse("Requested pickup location: RIVERTON LIBRARY"))
    }

    @Test fun readyStatusNamesTheBranchAndDate() {
        assertEquals(
            StatusText(pickup = "RIVERTON LIBRARY", pickupBy = "17 Oct 2026"),
            StatusText.parse("Please pickup from: RIVERTON LIBRARY by 17 Oct 2026"),
        )
    }

    @Test fun otherStatusesAreKept() {
        assertEquals(StatusText(status = "In transit"), StatusText.parse("In transit"))
        assertEquals(StatusText(), StatusText.parse("  "))
        assertEquals(StatusText(), StatusText.parse(null))
    }

    @Test fun pendingRowShowsPendingAndTheBranch() {
        val row = ReservationRow.of(item("Requested pickup location: RIVERTON LIBRARY", mapOf("Rank" to "5")), ReservationRow.Kind.PENDING)
        assertEquals("Pending", row.statusLabel)
        assertTrue(row.meta!!.contains("Collect from Riverton Library"))
        assertTrue(row.meta!!.contains("#5 in queue"))
    }

    @Test fun readyRowShowsCollectByDate() {
        val row = ReservationRow.of(item("Please pickup from: RIVERTON LIBRARY by 17 Oct 2026"), ReservationRow.Kind.READY, java.time.LocalDate.of(2026, 10, 12))
        assertEquals("Ready to collect", row.statusLabel)
        assertEquals("Collect from Riverton Library by 17 Oct", row.meta)
    }

    private fun item(status: String, extra: Map<String, String> = emptyMap()) = PatronItem(
        irn = "1", bibIrn = "2", title = "A book", bibUrl = null, author = null, year = null, coverUrls = emptyList(),
        isbn = null, fields = extra + ("Status" to status), actions = emptyList(),
    )
}
