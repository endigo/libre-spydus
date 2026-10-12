package com.librespydus.app.core

import uniffi.librespydus_core.Availability
import uniffi.librespydus_core.LibraryInfo
import uniffi.librespydus_core.RecordFormat
import uniffi.librespydus_core.listLibraries as coreListLibraries

/** A Home genre tile (§6): [key] picks the icon, [query] feeds the results route. */
data class Genre(val key: String, val label: String, val query: String)

/** A Home shelf (§6), e.g. "New arrivals". */
data class Shelf(val key: String, val title: String, val query: String, val sort: ResultsSort)

/** Availability for one format, ready to render (§4.1 `AvailabilitySummary`). */
data class AvailabilitySummary(val available: Long?, val copies: Long?, val reserved: Long?, val label: String)

/**
 * App-side seams for the helpers the P task is adding to the core (docs/native-apps.md §4.1).
 * Each has a local fallback so wave-1 code compiles and runs today; screens call these, and the
 * coordinator swaps the bodies for the core calls after P merges, without touching any screen.
 */
object CoreHelpers {
    /**
     * The User-Agent the core sends; the verification web view must use the same one (the WAF
     * token may be tied to it). TODO(P): return `uniffi.librespydus_core.userAgent()`.
     */
    val userAgent: String =
        "Mozilla/5.0 (Linux; Android 14; Pixel 10 Pro) AppleWebKit/537.36 (KHTML, like Gecko) " +
            "Chrome/129.0.0.0 Mobile Safari/537.36"

    private val DIGITAL_CODES = setOf("EBK", "EAUD", "EAB", "EBOOK", "EAUDIO", "EMAG", "EVID", "EVIDEO", "EMUS", "ECOMIC", "EJNL", "ONLINE", "WEB", "STREAM")
    private val DIGITAL_LABEL = Regex("""\be-?book|\be-?audio|\bonline\b|\bdigital\b|\bstreaming\b|\bemagazine|\be-?magazine""", RegexOption.IGNORE_CASE)

    /**
     * Digital formats (eBook, eAudiobook, …) have no reserve URL and no branch availability:
     * show "Available online" and no pickup UI. TODO(P): `isDigitalFormat(format)`.
     */
    fun isDigitalFormat(format: RecordFormat): Boolean =
        format.code.uppercase() in DIGITAL_CODES || DIGITAL_LABEL.containsMatchIn(format.label.orEmpty())

    fun isDigitalFormat(format: FormatRef): Boolean = isDigitalFormat(format.toRecordFormat())

    /**
     * "2 of 3 available", "3 copies · 5 reserved", … from `Availability.counts`.
     * TODO(P): `availabilitySummary(availability)`.
     */
    fun availabilitySummary(availability: Availability): AvailabilitySummary {
        val counts = availability.counts
        val copies = availability.copies ?: counts["copies"] ?: counts["total"]
        val reserved = availability.reserved ?: counts["reserved"] ?: counts["reservations"]
        val available = counts["available"] ?: counts["on shelf"] ?: counts["in"]
        val label = when {
            available != null && copies != null -> "$available of $copies available"
            available != null -> "$available available"
            copies != null && reserved != null && reserved > 0 -> "${plural(copies, "copy", "copies")} · $reserved reserved"
            copies != null -> plural(copies, "copy", "copies")
            else -> "Availability unknown"
        }
        return AvailabilitySummary(available, copies, reserved, label)
    }

    /** Home genre tiles. TODO(P): `homeGenres()`. */
    fun homeGenres(): List<Genre> = listOf(
        Genre("biography", "Biographies", "biography"),
        Genre("fiction", "Fiction", "fiction"),
        Genre("crime", "Crime", "crime fiction"),
        Genre("history", "History", "history"),
        Genre("science", "Science", "science"),
        Genre("children", "Children", "juvenile fiction"),
        Genre("cooking", "Cooking", "cooking"),
        Genre("travel", "Travel", "travel"),
    )

    /** Home shelves; the first is "New arrivals". TODO(P): `homeShelves()`. */
    fun homeShelves(): List<Shelf> = listOf(
        Shelf("new", "New arrivals", "fiction", ResultsSort.NEWEST),
    )

    private fun plural(n: Long, one: String, many: String) = if (n == 1L) "1 $one" else "$n $many"
}

/** Civica's library directory (one public GET per call, not cached by the core). */
fun interface LibraryDirectory {
    suspend fun list(includeTest: Boolean): List<LibraryInfo>

    companion object {
        val Core = LibraryDirectory { includeTest -> coreListLibraries(includeTest) }
    }
}
