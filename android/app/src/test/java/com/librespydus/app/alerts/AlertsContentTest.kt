package com.librespydus.app.alerts

import com.librespydus.app.core.AlertKind
import com.librespydus.app.core.AlertText
import com.librespydus.app.core.BorrowedSegment
import com.librespydus.app.core.MorningCheckTime
import com.librespydus.app.core.toLocalDate
import com.librespydus.app.core.toSimpleDate
import org.junit.Assert.assertEquals
import org.junit.Test
import uniffi.librespydus_core.DueReminder
import java.time.Duration
import java.time.LocalDate
import java.time.LocalDateTime
import java.time.ZoneId
import java.time.ZonedDateTime

/** The alert wording (§5.8, identical on iOS) and when the morning check runs. */
class AlertsContentTest {
    private val today = LocalDate.of(2026, 10, 13) // a Tuesday

    @Test
    fun readyBodyNamesTheBranchYourBooksShows() {
        // "Pickup location", title-cased the way Your Books shows it.
        assertEquals(
            "Tomorrow, and tomorrow, and tomorrow · Riverton Library",
            AlertText.readyBody(item("1", "Tomorrow, and tomorrow, and tomorrow", mapOf("Status" to "Awaiting pickup", "Pickup location" to "RIVERTON LIBRARY"))),
        )
        // Canning writes the branch into Status.
        assertEquals(
            "The Thursday Murder Club · Cannington Library",
            AlertText.readyBody(item("2", "The Thursday Murder Club", mapOf("Status" to "Please pickup from: CANNINGTON LIBRARY by 20 Oct 2026"))),
        )
    }

    @Test
    fun readyBodyWithoutBranchOrTitle() {
        assertEquals("Project Hail Mary", AlertText.readyBody(item("1", "Project Hail Mary", mapOf("Status" to "Awaiting pickup"))))
        assertEquals("Project Hail Mary", AlertText.readyBody(item("1", "  Project Hail Mary ")))
        assertEquals("A reservation · Riverton Library", AlertText.readyBody(item("1", null, mapOf("Pickup location" to "Riverton Library"))))
        assertEquals("A reservation", AlertText.readyBody(item("1", " ")))
    }

    @Test
    fun readyNotification() {
        val n = AlertText.ready("abc", "19000001", item("19000001", "Dune", mapOf("Pickup location" to "Riverton Library")))
        assertEquals("ready:abc:19000001", n.id)
        assertEquals("Ready to collect", n.title)
        assertEquals("Dune · Riverton Library", n.body)
        assertEquals(AlertKind.READY, n.kind)
        assertEquals(BorrowedSegment.RESERVATIONS, n.kind.opens)
        assertEquals("ready", n.kind.channelId)
        assertEquals("Ready to collect", n.kind.channelName)
    }

    @Test
    fun dueTitleSaysTodayTomorrowOrTheWeekday() {
        assertEquals("Due today", AlertText.dueTitle(today, today))
        assertEquals("Due tomorrow", AlertText.dueTitle(today.plusDays(1), today))
        assertEquals("Due Thursday 15 Oct", AlertText.dueTitle(today.plusDays(2), today))
        assertEquals("Due Friday 16 Oct", AlertText.dueTitle(today.plusDays(3), today))
        assertEquals("Due Monday 2 Nov", AlertText.dueTitle(LocalDate.of(2026, 11, 2), today))
        assertEquals("Due Friday 1 Jan", AlertText.dueTitle(LocalDate.of(2027, 1, 1), today))
    }

    @Test
    fun dueBodyNamesOneTwoOrTheFirstAndHowManyMore() {
        val a = loan("1", "Project Hail Mary", "15 Oct 2026")
        val b = loan("2", "Lessons in chemistry", "15 Oct 2026")
        val c = loan("3", "Where the crawdads sing", "15 Oct 2026")
        val d = loan("4", "Dune", "15 Oct 2026")
        assertEquals("Project Hail Mary", AlertText.dueBody(listOf(a)))
        assertEquals("Project Hail Mary and Lessons in chemistry", AlertText.dueBody(listOf(a, b)))
        assertEquals("Project Hail Mary and 2 more", AlertText.dueBody(listOf(a, b, c)))
        assertEquals("Project Hail Mary and 3 more", AlertText.dueBody(listOf(a, b, c, d)))
        assertEquals("A loan and Dune", AlertText.dueBody(listOf(loan("5", null, "15 Oct 2026"), d)))
    }

    @Test
    fun dueNotification() {
        val reminder = DueReminder(
            key = "due:2026-10-15",
            due = LocalDate.of(2026, 10, 15).toSimpleDate(),
            remindOn = today.toSimpleDate(),
            items = listOf(loan("1", "Project Hail Mary", "15 Oct 2026"), loan("2", "Dune", "15 Oct 2026")),
        )
        val n = AlertText.due("abc", reminder, today)
        assertEquals("due:abc:2026-10-15", n.id)
        assertEquals("Due Thursday 15 Oct", n.title)
        assertEquals("Project Hail Mary and Dune", n.body)
        assertEquals(AlertKind.DUE, n.kind)
        assertEquals(BorrowedSegment.LOANS, n.kind.opens)
        assertEquals("due", n.kind.channelId)
        assertEquals("Due-date reminders", n.kind.channelName)
    }

    @Test
    fun simpleDateRoundTrips() {
        assertEquals(today, today.toSimpleDate().toLocalDate())
        assertEquals(LocalDate.of(2028, 2, 29), LocalDate.of(2028, 2, 29).toSimpleDate().toLocalDate())
    }

    // region next 07:45

    private val perth = ZoneId.of("Australia/Perth")

    private fun at(zone: ZoneId, y: Int, m: Int, d: Int, h: Int, min: Int) = ZonedDateTime.of(LocalDateTime.of(y, m, d, h, min), zone)

    @Test
    fun beforeQuarterToEightTheCheckIsThatMorning() {
        val now = at(perth, 2026, 10, 13, 6, 30)
        assertEquals(at(perth, 2026, 10, 13, 7, 45), MorningCheckTime.next(now))
        assertEquals(Duration.ofMinutes(75), MorningCheckTime.delayUntilNext(now))
        assertEquals(Duration.ofMinutes(1), MorningCheckTime.delayUntilNext(at(perth, 2026, 10, 13, 7, 44)))
    }

    @Test
    fun atOrAfterQuarterToEightTheCheckIsTomorrowMorning() {
        assertEquals(at(perth, 2026, 10, 14, 7, 45), MorningCheckTime.next(at(perth, 2026, 10, 13, 7, 45)))
        assertEquals(at(perth, 2026, 10, 14, 7, 45), MorningCheckTime.next(at(perth, 2026, 10, 13, 7, 46)))
        assertEquals(at(perth, 2026, 10, 14, 7, 45), MorningCheckTime.next(at(perth, 2026, 10, 13, 23, 59)))
        // Month and year ends.
        assertEquals(at(perth, 2026, 11, 1, 7, 45), MorningCheckTime.next(at(perth, 2026, 10, 31, 12, 0)))
        assertEquals(at(perth, 2027, 1, 1, 7, 45), MorningCheckTime.next(at(perth, 2026, 12, 31, 8, 0)))
        assertEquals(Duration.ofHours(23).plusMinutes(59), MorningCheckTime.delayUntilNext(at(perth, 2026, 10, 13, 7, 46)))
    }

    @Test
    fun aClockChangeOvernightStillLandsOnQuarterToEightLocalTime() {
        // Sydney springs forward at 02:00 on Sunday 4 Oct 2026: the night is an hour shorter.
        val sydney = ZoneId.of("Australia/Sydney")
        val beforeSpring = at(sydney, 2026, 10, 3, 20, 0)
        assertEquals(at(sydney, 2026, 10, 4, 7, 45), MorningCheckTime.next(beforeSpring))
        assertEquals(Duration.ofHours(10).plusMinutes(45), MorningCheckTime.delayUntilNext(beforeSpring))
        // New York falls back at 02:00 on Sunday 1 Nov 2026: the night is an hour longer.
        val newYork = ZoneId.of("America/New_York")
        val beforeFall = at(newYork, 2026, 10, 31, 20, 0)
        assertEquals(at(newYork, 2026, 11, 1, 7, 45), MorningCheckTime.next(beforeFall))
        assertEquals(Duration.ofHours(12).plusMinutes(45), MorningCheckTime.delayUntilNext(beforeFall))
        assertEquals(7, MorningCheckTime.next(beforeFall).hour)
        assertEquals(45, MorningCheckTime.next(beforeFall).minute)
    }

    // endregion
}
