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
    fun byLine(rawAuthor: String?): String? = displayAuthor(rawAuthor)?.let(Copy.Detail::byline)

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

    /** Danger colour threshold for loans (§6): ≤ 3 days left or overdue. */
    fun isDueSoon(dueText: String?, today: LocalDate = LocalDate.now()): Boolean =
        parseDueDate(dueText)?.let { daysUntil(it, today) <= 3 } ?: false

    /**
     * Your Books' "Updated …" line for an account fetched at [fetchedAtMs] (§5.2a): "Updated just
     * now", "Updated 12 min ago", then "Updated today at 4:20 pm", "Updated yesterday at 4:20 pm",
     * "Updated Monday at 4:20 pm" (within a week) and "Updated 3 Oct at 4:20 pm" (the year only
     * when it isn't this year), in [locale]'s day and month names and the [time] style (the screen
     * passes the device's 12/24-hour one).
     */
    fun updatedText(
        fetchedAtMs: Long,
        nowMs: Long,
        zone: ZoneId = ZoneId.systemDefault(),
        locale: Locale = Locale.getDefault(),
        time: (ZonedDateTime) -> String = { DateTimeFormatter.ofLocalizedTime(FormatStyle.SHORT).withLocale(locale).format(it) },
    ): String {
        val age = nowMs - fetchedAtMs
        if (age < 60_000) return Copy.YourBooks.updatedJustNow // also a fetch "in the future" (clock moved back)
        if (age < 3_600_000) return Copy.YourBooks.updatedMinutesAgo((age / 60_000).toInt())
        val fetched = Instant.ofEpochMilli(fetchedAtMs).atZone(zone)
        val today = Instant.ofEpochMilli(nowMs).atZone(zone).toLocalDate()
        val days = ChronoUnit.DAYS.between(fetched.toLocalDate(), today)
        return when {
            days == 0L -> Copy.YourBooks.updatedToday(time(fetched))
            days == 1L -> Copy.YourBooks.updatedYesterday(time(fetched))
            days < 7 -> Copy.YourBooks.updatedOn(fetched.dayOfWeek.getDisplayName(TextStyle.FULL, locale), time(fetched))
            else -> {
                val pattern = if (fetched.year == today.year) "d MMM" else "d MMM yyyy"
                Copy.YourBooks.updatedOn(DateTimeFormatter.ofPattern(pattern, locale).format(fetched), time(fetched))
            }
        }
    }

    /** Library picker sections (§6), in display order. */
    val REGION_ORDER = listOf("AU", "NZ", "US", "UK", "SG", "OTHER")

    fun regionLabel(region: String): String = when (region.uppercase()) {
        "AU" -> Copy.Onboarding.regionAU
        "NZ" -> Copy.Onboarding.regionNZ
        "US" -> Copy.Onboarding.regionUS
        "UK" -> Copy.Onboarding.regionUK
        "SG" -> Copy.Onboarding.regionSG
        else -> Copy.Onboarding.regionOther
    }
}
