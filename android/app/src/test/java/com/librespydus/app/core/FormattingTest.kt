package com.librespydus.app.core

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import java.time.LocalDate

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
}
