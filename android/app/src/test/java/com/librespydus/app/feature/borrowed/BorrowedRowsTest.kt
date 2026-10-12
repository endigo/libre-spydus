package com.librespydus.app.feature.borrowed

import com.librespydus.app.feature.onboarding.demoData
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.librespydus_core.PatronAction
import uniffi.librespydus_core.PatronItem
import uniffi.librespydus_core.PatronList
import uniffi.librespydus_core.Reservations
import java.time.LocalDate

internal fun patronItem(
    title: String,
    irn: String? = null,
    fields: Map<String, String> = emptyMap(),
    actions: List<Pair<String, String>> = emptyList(),
) = PatronItem(
    irn = irn,
    bibIrn = null,
    title = title,
    bibUrl = null,
    author = "Haig, Matt, author",
    year = null,
    coverUrls = emptyList(),
    isbn = null,
    fields = fields,
    actions = actions.map { PatronAction(it.first, it.second) },
)

private const val BASE = "https://canning.spydus.com/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/78500000/0"

class BorrowedLoanTest {
    private val today = LocalDate.of(2026, 10, 12)

    @Test
    fun loansAreSortedByDueDateWithUnreadableDatesLast() {
        val items = listOf(
            patronItem("Later", "1", mapOf("Due" to "20 Oct 2026")),
            patronItem("No date", "2"),
            patronItem("Today", "3", mapOf("Due" to "12 Oct 2026")),
            patronItem("Overdue", "4", mapOf("Due" to "10/10/2026")),
            patronItem("Unreadable", "5", mapOf("Due" to "next week")),
            patronItem("Tomorrow", "6", mapOf("Due" to "2026-10-13")),
        )
        val rows = LoanRow.rows(items, today)
        assertEquals(listOf("Overdue", "Today", "Tomorrow", "Later", "No date", "Unreadable"), rows.map { it.ref.title })
        assertEquals(listOf(-2L, 0L, 1L, 8L, null, null), rows.map { it.due?.daysLeft })
    }

    @Test
    fun daysLeftWording() {
        fun meta(due: String) = LoanRow.of(patronItem("Book", fields = mapOf("Due" to due)), today).meta
        assertEquals("Due today · 12 Oct", meta("12 Oct 2026"))
        assertEquals("Due tomorrow · 13 Oct", meta("13 Oct 2026"))
        assertEquals("Due 15 Oct · 3 days left", meta("15 Oct 2026"))
        assertEquals("Due 27 Oct · 15 days left", meta("27 Oct 2026"))
        assertEquals("Due 11 Oct · 1 day overdue", meta("11 Oct 2026"))
        assertEquals("Due 2 Oct · 10 days overdue", meta("2 Oct 2026"))
        assertEquals("Due 1 Jan · 81 days left", meta("01/01/2027"))
    }

    @Test
    fun dangerColourWithinThreeDaysOrOverdue() {
        fun urgent(due: String) = LoanRow.of(patronItem("Book", fields = mapOf("Due" to due)), today).isUrgent
        assertTrue(urgent("10 Oct 2026"))
        assertTrue(urgent("12 Oct 2026"))
        assertTrue(urgent("15 Oct 2026"))
        assertFalse(urgent("16 Oct 2026"))
        assertFalse(urgent("whenever"))
        assertTrue(LoanRow.of(patronItem("Book", fields = mapOf("Due" to "11 Oct 2026")), today).due!!.isOverdue)
    }

    @Test
    fun unreadableDueDatesAreShownAsWritten() {
        fun meta(fields: Map<String, String>) = LoanRow.of(patronItem("Book", fields = fields), today).meta
        assertEquals("Due next week", meta(mapOf("Due" to "next week")))
        assertEquals("Overdue - see library", meta(mapOf("Due" to "Overdue - see library")))
        assertNull(meta(mapOf("Due" to "  ")))
        assertNull(meta(emptyMap()))
    }

    @Test
    fun rowIdsStayUniqueWhenTheLibraryRepeatsAnIrn() {
        val rows = LoanRow.rows(listOf(patronItem("A", "1"), patronItem("B", "1"), patronItem("C")), today)
        assertEquals(3, rows.map { it.id }.toSet().size)
        assertEquals(listOf("1", "1#2"), rows.take(2).map { it.id })
    }

    @Test
    fun demoLoansAreDatedRelativeToToday() {
        val rows = LoanRow.rows(demoData(today = today).loans, today)
        assertEquals(listOf(-1L, 2L, 9L, 16L), rows.map { it.due?.daysLeft })
        assertEquals("The midnight library", rows.first().ref.title)
    }
}

class BorrowedReservationTest {
    @Test
    fun actionHrefsMatchCancelAndEditLabels() {
        val reservation = patronItem(
            "Book",
            actions = listOf("Cancel reservation" to "$BASE/C/5/RSVCENQ?SVL=1", "EDIT RESERVATION" to "$BASE/ER/1/RSVCENQ?SVL=1"),
        )
        assertEquals("$BASE/C/5/RSVCENQ?SVL=1", reservation.cancelHref)
        assertEquals("$BASE/ER/1/RSVCENQ?SVL=1", reservation.editHref)

        val short = patronItem("Book", actions = listOf("Edit" to "$BASE/ER/1/RSVCENQ?SVL=2", "Cancel" to "$BASE/C/5/RSVCENQ?SVL=2"))
        assertEquals("$BASE/ER/1/RSVCENQ?SVL=2", short.editHref)
        assertEquals("$BASE/C/5/RSVCENQ?SVL=2", short.cancelHref)

        val loan = patronItem("Book", actions = listOf("Renew loan" to "$BASE/RL/1/LOANRENQ?SVL=3"))
        assertNull(loan.cancelHref)
        assertNull(loan.editHref)
        assertNull(patronItem("Book", actions = listOf("Cancel reservation" to " ")).cancelHref)
    }

    @Test
    fun readyForPickupComesFirstAndAppearsOnce() {
        val ready = patronItem("Ready", "1", mapOf("Status" to "Awaiting pickup", "Pickup location" to "RIVERTON LIBRARY"))
        val reservations = Reservations(
            ready = PatronList(1, listOf(ready)),
            pending = PatronList(
                3,
                listOf(
                    patronItem("In transit", "2", mapOf("Status" to "In transit")),
                    ready,
                    patronItem("Queued", "3", mapOf("Status" to "Pending", "Rank" to "4")),
                ),
            ),
        )
        val sections = ReservationSections.from(reservations)
        assertEquals(listOf("Ready"), sections.ready.map { it.ref.title })
        assertTrue(sections.ready.all { it.isReady })
        assertEquals(listOf("In transit", "Queued"), sections.pending.map { it.ref.title })
        assertTrue(sections.pending.none { it.isReady })
        assertEquals(3, sections.count)
        assertEquals(listOf("Queued"), sections.removing("2").pending.map { it.ref.title })
        assertNotNull(sections.row("3"))
        assertNull(sections.removing("3").row("3"))
    }

    @Test
    fun cardWording() {
        val ready = ReservationRow.of(
            patronItem("Ready", fields = mapOf("Status" to "Awaiting pickup", "Pickup location" to "WILLETTON & DISTRICT")),
            ReservationRow.Kind.READY,
        )
        assertEquals("Ready for pickup", ready.statusLabel)
        assertEquals("Pick up at Willetton & District", ready.meta)

        val pending = ReservationRow.of(
            patronItem(
                "Pending",
                fields = mapOf("Status" to "Pending", "Rank" to "4", "Placed" to "02 Oct 2026", "Pickup location" to "RIVERTON LIBRARY"),
            ),
            ReservationRow.Kind.PENDING,
        )
        assertEquals("Pending", pending.statusLabel)
        assertEquals("#4 in queue · Placed 2 Oct\nPick up at Riverton Library", pending.meta)

        val bare = ReservationRow.of(patronItem("Bare", fields = mapOf("Status" to "In transit")), ReservationRow.Kind.PENDING)
        assertEquals("In transit", bare.statusLabel)
        assertTrue(bare.isInTransit)
        assertNull(bare.meta)
        assertEquals("Pending", ReservationRow.of(patronItem("No status"), ReservationRow.Kind.PENDING).statusLabel)
        val oddRank = ReservationRow.of(patronItem("Odd", fields = mapOf("Rank" to "n/a", "Placed" to "soon")), ReservationRow.Kind.PENDING)
        assertEquals("Rank n/a · Placed soon", oddRank.meta)
    }

    @Test
    fun branchNamesAreTitleCasedOnlyWhenShouted() {
        assertEquals("Riverton Library", ReservationRow.displayName("RIVERTON LIBRARY"))
        assertEquals("Willetton & District", ReservationRow.displayName("WILLETTON & DISTRICT"))
        assertEquals("St. Mary-Le-Bow", ReservationRow.displayName("ST. MARY-LE-BOW"))
        assertEquals("Central Library", ReservationRow.displayName("Central Library"))
        assertEquals("iPad lending", ReservationRow.displayName("iPad lending"))
        assertEquals("123", ReservationRow.displayName("123"))
        assertNull(ReservationRow.displayName("  "))
        assertNull(ReservationRow.displayName(null))
    }

    @Test
    fun cancelConfirmationNamesTheBook() {
        val ready = ReservationRow.of(
            patronItem("The midnight library", fields = mapOf("Pickup location" to "RIVERTON LIBRARY")),
            ReservationRow.Kind.READY,
        )
        assertTrue(
            CancelConfirm.message(ready).contains("“The midnight library” is waiting for you at Riverton Library"),
        )
        val pending = ReservationRow.of(patronItem("Hollow bones"), ReservationRow.Kind.PENDING)
        assertTrue(CancelConfirm.message(pending).contains("lose your place in the queue"))
    }

    @Test
    fun demoReservationsSplitAsExpected() {
        val data = demoData()
        val sections = ReservationSections.from(
            Reservations(PatronList(null, data.ready), PatronList(null, data.pending)),
        )
        assertEquals(listOf("The midnight library"), sections.ready.map { it.ref.title })
        assertNotNull(sections.ready[0].cancelHref)
        assertNotNull(sections.ready[0].editHref)
        assertEquals(2, sections.pending.size)
        // The in-transit row has no actions: no Cancel / Edit buttons.
        val inTransit = sections.pending.first { it.status == "In transit" }
        assertNull(inTransit.cancelHref)
        assertNull(inTransit.editHref)
    }
}
