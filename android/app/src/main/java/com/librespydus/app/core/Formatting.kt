package com.librespydus.app.core

import java.time.Instant
import java.time.LocalDate
import java.time.ZoneId
import java.time.ZonedDateTime
import java.time.format.DateTimeFormatter
import java.time.format.FormatStyle
import java.time.format.TextStyle
import java.time.temporal.ChronoUnit
import java.util.Locale
import uniffi.librespydus_core.parseDueDate as coreParseDueDate

/** Display helpers shared by the screens. Pure functions; unit-testable on the JVM. */
object Formatting {
    private val ROLE_WORDS = setOf(
        "author", "illustrator", "editor", "translator", "narrator", "compiler", "contributor",
        "adapter", "photographer", "director", "performer", "creator", "writer", "artist", "reader",
    )

    /**
     * The OPAC's catalogue-style author ("Haig, Matt, author", "Galligan, Gale, 1964- illustrator",
     * "Rowling, J. K.") as a display name ("Matt Haig"). Life dates and role words are dropped;
     * a single-part name (corporate authors) is returned trimmed.
     */
    fun displayAuthor(raw: String?): String? {
        val parts = raw?.split(',')?.map { it.trim() }?.filter { it.isNotEmpty() } ?: return null
        if (parts.isEmpty()) return null
        val surname = parts[0].trimEnd('.')
        val given = parts.drop(1).firstOrNull { part ->
            part.none { it.isDigit() } && part.lowercase().trimEnd('.') !in ROLE_WORDS
        }
        return if (given == null) surname else "$given $surname"
    }

    /** "By Matt Haig", or null without an author. */
    fun byLine(rawAuthor: String?): String? = displayAuthor(rawAuthor)?.let { "By $it" }

    /** First name for the Home greeting: "Test Borrower" → "Test", "Borrower, Test" → "Test". */
    fun firstName(accountName: String?): String? {
        val name = accountName?.trim()?.takeIf { it.isNotEmpty() } ?: return null
        val ordered = if (',' in name) name.substringAfter(',').trim() else name
        val first = ordered.split(' ').firstOrNull { it.isNotBlank() } ?: return null
        // OPACs often store names in capitals ("TEST BORROWER"); keep mixed case as given.
        return if (first.any { it.isLowerCase() }) first
        else first.lowercase().replaceFirstChar { it.titlecase(Locale.getDefault()) }
    }

    /** A loan's `Due` field ("27 Oct 2026", "27/10/2026", …) as a date, via the core's `parseDueDate`. */
    fun parseDueDate(text: String?): LocalDate? {
        val trimmed = text?.trim()?.takeIf { it.isNotEmpty() } ?: return null
        val day = coreParseDueDate(trimmed) ?: return null
        return LocalDate.of(day.year, day.month.toInt(), day.day.toInt())
    }

    /** Whole days from [today] to [due]; negative when overdue. */
    fun daysUntil(due: LocalDate, today: LocalDate = LocalDate.now()): Long = ChronoUnit.DAYS.between(today, due)

    /** "3 days left", "1 day left", "Due today", "Overdue by 2 days". */
    fun daysLeftLabel(days: Long): String = when {
        days > 1 -> "$days days left"
        days == 1L -> "1 day left"
        days == 0L -> "Due today"
        days == -1L -> "Overdue by 1 day"
        else -> "Overdue by ${-days} days"
    }

    /** Loans meta line: "Due 27 Oct · 3 days left" (§6). Falls back to the raw text. */
    fun dueLine(dueText: String?, today: LocalDate = LocalDate.now()): String? {
        val due = parseDueDate(dueText) ?: return dueText?.let { "Due $it" }
        val days = daysUntil(due, today)
        if (days == 0L) return "Due today"
        return "Due ${SHORT_DATE.format(due)} · ${daysLeftLabel(days)}"
    }

    /** Danger colour threshold for loans (§6): ≤ 3 days left or overdue. */
    fun isDueSoon(dueText: String?, today: LocalDate = LocalDate.now()): Boolean =
        parseDueDate(dueText)?.let { daysUntil(it, today) <= 3 } ?: false

    private val SHORT_DATE: DateTimeFormatter = DateTimeFormatter.ofPattern("d MMM", Locale.ENGLISH)

    /**
     * Your Books' "Updated …" line for an account fetched at [fetchedAtMs] (§5.2a): "Updated just
     * now", "Updated 12 min ago", then "Updated today at 4:20 pm", "Updated yesterday at 4:20 pm",
     * "Updated Monday at 4:20 pm" (within a week) and "Updated 3 Oct 2026 at 4:20 pm", in
     * [locale]'s date style and the [time] style (the screen passes the device's 12/24-hour one).
     */
    fun updatedText(
        fetchedAtMs: Long,
        nowMs: Long,
        zone: ZoneId = ZoneId.systemDefault(),
        locale: Locale = Locale.getDefault(),
        time: (ZonedDateTime) -> String = { DateTimeFormatter.ofLocalizedTime(FormatStyle.SHORT).withLocale(locale).format(it) },
    ): String {
        val age = nowMs - fetchedAtMs
        if (age < 60_000) return "Updated just now" // also a fetch "in the future" (clock moved back)
        if (age < 3_600_000) return "Updated ${age / 60_000} min ago"
        val fetched = Instant.ofEpochMilli(fetchedAtMs).atZone(zone)
        val days = ChronoUnit.DAYS.between(fetched.toLocalDate(), Instant.ofEpochMilli(nowMs).atZone(zone).toLocalDate())
        val day = when {
            days == 0L -> "today"
            days == 1L -> "yesterday"
            days < 7 -> fetched.dayOfWeek.getDisplayName(TextStyle.FULL, locale)
            else -> DateTimeFormatter.ofLocalizedDate(FormatStyle.MEDIUM).withLocale(locale).format(fetched)
        }
        return "Updated $day at ${time(fetched)}"
    }

    /** Library picker sections (§6), in display order. */
    val REGION_ORDER = listOf("AU", "NZ", "US", "UK", "SG", "OTHER")

    fun regionLabel(region: String): String = when (region.uppercase()) {
        "AU" -> "Australia"
        "NZ" -> "New Zealand"
        "US" -> "United States"
        "UK" -> "United Kingdom"
        "SG" -> "Singapore"
        else -> "Other"
    }
}
