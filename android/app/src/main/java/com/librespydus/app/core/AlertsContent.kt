package com.librespydus.app.core

import com.librespydus.app.feature.borrowed.ReservationRow
import uniffi.librespydus_core.DueReminder
import uniffi.librespydus_core.PatronItem
import uniffi.librespydus_core.SimpleDate
import java.time.Duration
import java.time.LocalDate
import java.time.LocalTime
import java.time.ZonedDateTime
import java.time.format.DateTimeFormatter
import java.time.temporal.ChronoUnit
import java.util.Locale

/** The two kinds of alert (docs/native-apps.md §5.8), one notification channel each. */
enum class AlertKind(val channelId: String, val channelName: String, val opens: BorrowedSegment) {
    READY("ready", "Ready to collect", BorrowedSegment.RESERVATIONS),
    DUE("due", "Due-date reminders", BorrowedSegment.LOANS),
}

/**
 * One local notification. [id] is `ready:<originHash>:<key>` or `due:<originHash>:<YYYY-MM-DD>`
 * (the Android notification tag), the same identifiers iOS uses. A tap opens Your Books on
 * [kind]'s segment.
 */
data class AlertNotification(val id: String, val kind: AlertKind, val title: String, val body: String)

/**
 * What the alerts say (§5.8), word for word the same on iOS. Pure, so the wording is unit-tested
 * (AlertsContentTest). Dates are java.time (Gregorian) in English, like the rest of Your Books.
 */
object AlertText {
    const val READY_TITLE = "Ready to collect"

    /** A ready reservation without a title. */
    const val UNTITLED_RESERVATION = "A reservation"

    /** A loan without a title (§5.8 doesn't name one; the same words as iOS). */
    const val UNTITLED_LOAN = "A loan"

    private val WEEKDAY_DATE: DateTimeFormatter = DateTimeFormatter.ofPattern("EEEE d MMM", Locale.ENGLISH)

    /**
     * "Tomorrow, and tomorrow, and tomorrow · Riverton Library": the branch is the one Your Books
     * shows for the row ("Pickup location", else the branch Canning writes into Status); just the
     * title when there is none.
     */
    fun readyBody(item: PatronItem): String {
        val title = item.title.nonBlank() ?: UNTITLED_RESERVATION
        val branch = ReservationRow.of(item, ReservationRow.Kind.READY).pickup
        return if (branch != null) "$title · $branch" else title
    }

    /** "Due today", "Due tomorrow", else "Due Thursday 16 Oct". */
    fun dueTitle(due: LocalDate, today: LocalDate): String = when (ChronoUnit.DAYS.between(today, due)) {
        0L -> "Due today"
        1L -> "Due tomorrow"
        else -> "Due ${WEEKDAY_DATE.format(due)}"
    }

    /** "Project Hail Mary", "Project Hail Mary and Lessons in chemistry", "Project Hail Mary and 2 more". */
    fun dueBody(items: List<PatronItem>): String {
        val titles = items.map { it.title.nonBlank() ?: UNTITLED_LOAN }
        return when (titles.size) {
            0 -> ""
            1 -> titles[0]
            2 -> "${titles[0]} and ${titles[1]}"
            else -> "${titles[0]} and ${titles.size - 1} more"
        }
    }

    fun ready(originHash: String, key: String, item: PatronItem) =
        AlertNotification("ready:$originHash:$key", AlertKind.READY, READY_TITLE, readyBody(item))

    fun due(originHash: String, reminder: DueReminder, today: LocalDate): AlertNotification {
        val due = reminder.due.toLocalDate()
        return AlertNotification("due:$originHash:$due", AlertKind.DUE, dueTitle(due, today), dueBody(reminder.items))
    }

    private fun String?.nonBlank(): String? = this?.trim()?.takeIf { it.isNotEmpty() }
}

/** When the morning check runs (§5.8): 07:45 local time, the hour the library's own notices go out. */
object MorningCheckTime {
    val TIME: LocalTime = LocalTime.of(7, 45)

    /**
     * The first 07:45 after [now], in [now]'s zone. Computed on the local date, so a DST change
     * overnight still lands on 07:45 wall-clock time (a 07:45 inside a DST gap moves forward).
     */
    fun next(now: ZonedDateTime): ZonedDateTime {
        val today = now.toLocalDate().atTime(TIME).atZone(now.zone)
        return if (today.isAfter(now)) today else now.toLocalDate().plusDays(1).atTime(TIME).atZone(now.zone)
    }

    /** How long from [now] until [next]. */
    fun delayUntilNext(now: ZonedDateTime): Duration = Duration.between(now, next(now))
}

fun SimpleDate.toLocalDate(): LocalDate = LocalDate.of(year, month.toInt(), day.toInt())

fun LocalDate.toSimpleDate(): SimpleDate = SimpleDate(year, monthValue.toUByte(), dayOfMonth.toUByte())
