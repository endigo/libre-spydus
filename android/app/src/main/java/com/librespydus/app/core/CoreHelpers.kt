package com.librespydus.app.core

import uniffi.librespydus_core.AccountHolding
import uniffi.librespydus_core.AccountSnapshot
import uniffi.librespydus_core.Availability
import uniffi.librespydus_core.LibraryInfo
import uniffi.librespydus_core.PatronItem
import uniffi.librespydus_core.RecordFormat
import uniffi.librespydus_core.accountHolding as coreAccountHolding
import uniffi.librespydus_core.availabilitySummary as coreAvailabilitySummary
import uniffi.librespydus_core.homeGenres as coreHomeGenres
import uniffi.librespydus_core.homeShelves as coreHomeShelves
import uniffi.librespydus_core.isDigitalFormat as coreIsDigitalFormat
import uniffi.librespydus_core.listLibraries as coreListLibraries
import uniffi.librespydus_core.patronItemRecordUrl as corePatronItemRecordUrl
import uniffi.librespydus_core.userAgent as coreUserAgent

/** A Home genre tile (§6): [key] picks the icon, [query] feeds the results route. */
data class Genre(val key: String, val label: String, val query: String)

/** A Home shelf (§6), e.g. "New arrivals". */
data class Shelf(val key: String, val title: String, val query: String, val sort: ResultsSort)

/** Availability for one format, ready to render (§4.1 `AvailabilitySummary`). */
data class AvailabilitySummary(val available: Long?, val copies: Long?, val reserved: Long?, val label: String)

/**
 * The core's shared app helpers (docs/native-apps.md §4.1), mapped to app types so screens and
 * routes stay independent of the generated bindings.
 */
object CoreHelpers {
    /** The User-Agent the core sends; the verification web view must use the same one (the WAF token may be tied to it). */
    val userAgent: String by lazy { coreUserAgent() }

    /**
     * Ebook/eaudio/streaming formats: show "Available online" when there are no counts. Whether a
     * format can be reserved depends on its `reserveUrl` alone — some libraries (e.g. Brazoria
     * County) take holds on ebooks.
     */
    fun isDigitalFormat(format: RecordFormat): Boolean = coreIsDigitalFormat(format)

    fun isDigitalFormat(format: FormatRef): Boolean = isDigitalFormat(format.toRecordFormat())

    /** "2 of 3 available", "3 copies · 1 reserved", … from `Availability.counts`. */
    fun availabilitySummary(availability: Availability): AvailabilitySummary =
        coreAvailabilitySummary(availability).let { AvailabilitySummary(it.available, it.copies, it.reserved, it.label) }

    /** Home genre tiles; `key` is a stable id for the icon. */
    fun homeGenres(): List<Genre> = coreHomeGenres().map { Genre(it.key, it.label, it.query) }

    /** Home shelves; the first is "New arrivals". */
    fun homeShelves(): List<Shelf> =
        coreHomeShelves().map { Shelf(it.key, it.title, it.query, ResultsSort.from(it.sort)) }

    /**
     * The URL that loads a loan's or reservation's bib record without a session
     * (`…/BIBENQ?SETLVL=&IRN={bibIrn}`). The row's own `bibUrl` names a set in the patron's OPAC
     * session and loads an empty record without it. Falls back to `bibUrl`.
     */
    fun patronItemRecordUrl(item: PatronItem): String? = corePatronItemRecordUrl(item)

    /** [patronItemRecordUrl] for a bib IRN and a link to it (a saved ref that has no row any more). */
    fun bibRecordUrl(bibIrn: String, bibUrl: String): String? =
        patronItemRecordUrl(PatronItem(null, bibIrn, null, bibUrl, null, null, emptyList(), null, emptyMap(), emptyList()))

    /**
     * The loan or reservation in [snapshot] of any of [bibIrns] (a loan wins over a ready
     * reservation, which wins over a pending one), for a book page to show instead of Reserve.
     */
    fun accountHolding(snapshot: AccountSnapshot, bibIrns: List<String>): AccountHolding? =
        coreAccountHolding(snapshot, bibIrns)
}

/** Civica's library directory (one public GET per call, not cached by the core). */
fun interface LibraryDirectory {
    suspend fun list(includeTest: Boolean): List<LibraryInfo>

    companion object {
        val Core = LibraryDirectory { includeTest -> coreListLibraries(includeTest) }
    }
}
