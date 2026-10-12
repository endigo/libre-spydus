package com.librespydus.app.core

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import java.time.LocalDate
import java.time.ZoneId
import java.time.ZonedDateTime
import java.util.Locale

class FormattingTest {
    @Test
    fun displayAuthor() {
        assertEquals("Matt Haig", Formatting.displayAuthor("Haig, Matt, author"))
        assertEquals("Matt Haig", Formatting.displayAuthor("Haig, Matt, 1975-"))
        assertEquals("Gale Galligan", Formatting.displayAuthor("Galligan, Gale, 1964- illustrator"))
        assertEquals("J. K. Rowling", Formatting.displayAuthor("Rowling, J. K."))
        assertEquals("Canning Library Service", Formatting.displayAuthor("Canning Library Service"))
        assertNull(Formatting.displayAuthor(null))
        assertNull(Formatting.displayAuthor(" , "))
        assertEquals("By Andy Weir", Formatting.byLine("Weir, Andy"))
    }

    @Test
    fun firstName() {
        assertEquals("Test", Formatting.firstName("Test Borrower"))
        assertEquals("Test", Formatting.firstName("Borrower, Test"))
        assertEquals("Test", Formatting.firstName("TEST BORROWER"))
        assertEquals("McKenzie", Formatting.firstName("McKenzie Smith"))
        assertNull(Formatting.firstName("  "))
    }

    @Test
    fun dueDates() {
        val today = LocalDate.of(2026, 10, 12)
        assertEquals(LocalDate.of(2026, 10, 27), Formatting.parseDueDate("27 Oct 2026"))
        assertEquals(LocalDate.of(2026, 10, 27), Formatting.parseDueDate("27/10/2026"))
        assertNull(Formatting.parseDueDate("soon"))
        assertEquals("Due 27 Oct · 15 days left", Formatting.dueLine("27 Oct 2026", today))
        assertEquals("Due 13 Oct · 1 day left", Formatting.dueLine("13 Oct 2026", today))
        assertEquals("Due today", Formatting.dueLine("12 Oct 2026", today))
        assertEquals("Due 10 Oct · Overdue by 2 days", Formatting.dueLine("10 Oct 2026", today))
        assertEquals("Due whenever", Formatting.dueLine("whenever", today))
    }

    @Test
    fun updatedTextIsRelativeToNow() {
        val zone = ZoneId.of("Australia/Perth")
        fun at(day: Int, hour: Int, minute: Int, month: Int = 10) = ZonedDateTime.of(2026, month, day, hour, minute, 0, 0, zone).toInstant().toEpochMilli()
        val now = at(12, 16, 30) // Monday 12 Oct 2026, 4:30 pm
        // en-GB writes times as 16:20 and dates as 1 Oct 2026.
        fun text(fetchedAt: Long, locale: Locale = Locale.UK) = Formatting.updatedText(fetchedAt, now, zone, locale)

        assertEquals("Updated just now", text(now))
        assertEquals("Updated just now", text(now - 59_999))
        assertEquals("Updated just now", text(now + 5_000)) // the clock moved back
        assertEquals("Updated 1 min ago", text(now - 60_000))
        assertEquals("Updated 12 min ago", text(now - 12 * 60_000))
        assertEquals("Updated 59 min ago", text(now - 3_599_999))
        assertEquals("Updated today at 15:30", text(now - 3_600_000))
        assertEquals("Updated today at 09:05", text(at(12, 9, 5)))
        assertEquals("Updated yesterday at 16:20", text(at(11, 16, 20)))
        assertEquals("Updated yesterday at 23:50", text(at(11, 23, 50)))
        assertEquals("Updated Friday at 16:20", text(at(9, 16, 20)))
        assertEquals("Updated Tuesday at 08:00", text(at(6, 8, 0)))
        assertEquals("Updated 5 Oct 2026 at 16:20", text(at(5, 16, 20)))
        assertEquals("Updated 1 Sept 2026 at 16:20".replace("Sept", shortSeptember()), text(at(1, 16, 20, month = 9)))
    }

    @Test
    fun updatedTextAcrossMidnightAndInTheDevicesTimeFormat() {
        val zone = ZoneId.of("Australia/Perth")
        fun at(day: Int, hour: Int, minute: Int) = ZonedDateTime.of(2026, 10, day, hour, minute, 0, 0, zone).toInstant().toEpochMilli()
        // 20 minutes ago, though the date changed in between.
        assertEquals("Updated 20 min ago", Formatting.updatedText(at(11, 23, 50), at(12, 0, 10), zone, Locale.UK))
        // en-US: a 12-hour clock (newer JDKs put a narrow no-break space before PM).
        assertEquals(
            "Updated yesterday at 4:20 PM",
            Formatting.updatedText(at(11, 16, 20), at(12, 16, 30), zone, Locale.US).replace('\u202F', ' '),
        )
        // The screen passes the device's own 12/24-hour format.
        assertEquals(
            "Updated yesterday at 4:20 pm",
            Formatting.updatedText(at(11, 16, 20), at(12, 16, 30), zone, Locale.UK, time = { "${it.hour - 12}:${it.minute} pm" }),
        )
    }

    /** en-GB's abbreviated September differs between JDK versions ("Sep" / "Sept"). */
    private fun shortSeptember(): String =
        java.time.format.DateTimeFormatter.ofPattern("MMM", Locale.UK).format(LocalDate.of(2026, 9, 1))
}
