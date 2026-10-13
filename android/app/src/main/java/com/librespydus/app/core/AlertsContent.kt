package com.librespydus.app.core

import com.librespydus.app.feature.borrowed.ReservationRow
import uniffi.librespydus_core.DueReminder
import uniffi.librespydus_core.PatronItem
import uniffi.librespydus_core.SimpleDate
import uniffi.librespydus_core.patronItemKey
import java.time.Duration
import java.time.LocalDate
import java.time.LocalTime
import java.time.ZonedDateTime
import java.time.format.DateTimeFormatter
import java.time.temporal.ChronoUnit
import java.util.Locale

/** The two kinds of alert (docs/native-apps.md §5.8), one notification channel each. */
enum class AlertKind(val channelId: String, val channelName: String, val opens: BorrowedSegment) {
    READY("ready", Copy.Alerts.readyChannel, BorrowedSegment.RESERVATIONS),
    DUE("due", Copy.Alerts.dueChannel, BorrowedSegment.LOANS),
}

/**
 * One local notification. [id] is `ready:<originHash>:<key>` or `due:<originHash>:<YYYY-MM-DD>`
 * (the Android notification tag), the same identifiers iOS uses. A tap opens Your Books on
 * [kind]'s segment and the book page of the one item [keys] names (§5.8 "Tap", [AlertTap]):
 * [origin] is the library, [keys] the `patronItemKey`s of the reservation or the loans.
 */
data class AlertNotification(
    val id: String,
    val kind: AlertKind,
    val title: String,
    val body: String,
    val origin: String,
    val keys: List<String>,
)

/**
 * What the alerts say (§5.8), word for word the same on iOS. Pure, so the wording is unit-tested
 * (AlertsContentTest). Dates are java.time (Gregorian) in English, like the rest of Your Books.
 */
object AlertText {
    private val WEEKDAY_DATE: DateTimeFormatter = DateTimeFormatter.ofPattern("EEEE d MMM", Locale.ENGLISH)
    private val WEEKDAY_DATE_YEAR: DateTimeFormatter = DateTimeFormatter.ofPattern("EEEE d MMM yyyy", Locale.ENGLISH)

    /**
     * "Tomorrow, and tomorrow, and tomorrow · Riverton Library": the branch is the one Your Books
     * shows for the row ("Pickup location", else the branch Canning writes into Status); just the
     * title when there is none.
     */
    fun readyBody(item: PatronItem): String {
        val title = item.title.nonBlank() ?: Copy.Alerts.untitledReservation
        val branch = ReservationRow.of(item, ReservationRow.Kind.READY).pickup
        return if (branch != null) Copy.Alerts.readyBody(title, branch) else title
    }

    /** "Due today", "Due tomorrow", else "Due Thursday 16 Oct" (the year only when it isn't this one). */
    fun dueTitle(due: LocalDate, today: LocalDate): String = when (ChronoUnit.DAYS.between(today, due)) {
        0L -> Copy.Loans.dueToday
        1L -> Copy.Loans.dueTomorrow
        else -> Copy.Loans.dueOn((if (due.year == today.year) WEEKDAY_DATE else WEEKDAY_DATE_YEAR).format(due))
    }

    /** "Project Hail Mary", "Project Hail Mary and Lessons in chemistry", "Project Hail Mary and 2 more". */
    fun dueBody(items: List<PatronItem>): String {
        val titles = items.map { it.title.nonBlank() ?: Copy.Alerts.untitledLoan }
        return when (titles.size) {
            0 -> ""
            1 -> titles[0]
            2 -> Copy.Alerts.dueTwo(titles[0], titles[1])
            else -> Copy.Alerts.dueMore(titles.size - 1, titles[0])
        }
    }

    fun ready(originHash: String, origin: String, key: String, item: PatronItem) =
        AlertNotification("ready:$originHash:$key", AlertKind.READY, Copy.Alerts.readyTitle, readyBody(item), origin, listOf(key))

    fun due(originHash: String, origin: String, reminder: DueReminder, today: LocalDate): AlertNotification {
        val due = reminder.due.toLocalDate()
        return AlertNotification(
            "due:$originHash:$due", AlertKind.DUE, dueTitle(due, today), dueBody(reminder.items),
            origin, reminder.items.map(::patronItemKey),
        )
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
