package com.librespydus.app.core

import uniffi.librespydus_core.Availability
import uniffi.librespydus_core.RecordDetail
import uniffi.librespydus_core.RecordFormat
import uniffi.librespydus_core.SearchRecord
import uniffi.librespydus_core.SearchResults
import uniffi.librespydus_core.SpydusException

/**
 * Demo mode's Pick list (§5.11), in memory, for [DemoBackend]. Seeded with two of the demo
 * catalogue's records: The Midnight Library as a work with its three format chips, and The
 * Thursday Murder Club's Book as an edition-level card (one format with an empty code, as saved
 * from a Featured list). Answers like the real list: newest first, `noResults` when empty, the
 * list's size after each save or remove.
 *
 * A save needs a card for its IRN: a catalogue work or edition, a results card seen in a search
 * or page ([remember]), or a record loaded on a book page (an edition-level card from it). Any
 * other IRN fails like the library's "None found" page.
 */
internal class DemoPicks(private val catalogue: DemoCatalogue, origin: String) {
    private val base = "${origin.trimEnd('/')}/cgi-bin/spydus.exe"
    private val list = mutableListOf<SearchRecord>()
    private val seen = mutableMapOf<String, SearchRecord>()

    init {
        catalogue.works.firstOrNull { it.workIrn == SEED_WORK }?.let { list += workCard(it) }
        editionOf(SEED_EDITION)?.let { (work, edition) -> list += editionCard(work, edition) }
    }

    @Synchronized
    fun page(): SearchResults =
        SearchResults(null, null, list.size.toLong(), null, null, null, list.isEmpty(), list.mapIndexed { i, r -> r.copy(rank = i + 1L) })

    /** Adds [irn]'s card at the top (once); returns the list's size. */
    @Synchronized
    fun save(irn: String): Long {
        if (list.none { it.workIrn == irn }) list.add(0, cardFor(irn) ?: throw SpydusException.Upstream("None found"))
        return list.size.toLong()
    }

    /** Takes [irn] off the list; returns how many remain. */
    @Synchronized
    fun remove(irn: String): Long {
        list.removeAll { it.workIrn == irn }
        return list.size.toLong()
    }

    /** A results card the app was shown, so saving its `workIrn` lists that card. */
    @Synchronized
    fun remember(record: SearchRecord) {
        record.workIrn?.takeIf { it.isNotBlank() }?.let { seen[it] = record }
    }

    /** A record loaded on a book page, so saving its `bibIrn` (a loan, a related item) lists it. */
    @Synchronized
    fun remember(detail: RecordDetail) {
        val irn = detail.bibIrn?.takeIf { it.isNotBlank() } ?: return
        if (irn !in seen && editionOf(irn) == null) seen[irn] = recordCard(irn, detail)
    }

    /** An edition-level card's availability (`…/BIBENQ?SETLVL=&IRN={bib}&FMT=RECFMT`) for a catalogue edition. */
    fun availability(url: String): Availability? {
        if (!url.startsWith("$base/") || !url.contains(EDITION_FORMAT)) return null
        val irn = IRN_PARAM.find(url)?.groupValues?.get(1) ?: return null
        val (work, edition) = editionOf(irn) ?: return null
        return Availability(
            bibRecordIrn = edition.bibIrn,
            title = work.title,
            publisher = edition.imprint,
            counts = edition.counts,
            reserved = edition.counts["reserved"],
            copies = edition.counts["copies"],
            holdingsUrl = null,
            recordUrl = null,
        )
    }

    private fun cardFor(irn: String): SearchRecord? =
        catalogue.works.firstOrNull { it.workIrn == irn }?.let(::workCard)
            ?: editionOf(irn)?.let { (work, edition) -> editionCard(work, edition) }
            ?: seen[irn]

    private fun editionOf(bibIrn: String): Pair<DemoCatalogue.Work, DemoCatalogue.Edition>? =
        catalogue.works.firstNotNullOfOrNull { work -> work.editions.firstOrNull { it.bibIrn == bibIrn }?.let { work to it } }

    /** The work as the demo catalogue's own search lists it (every format, so chips show). */
    private fun workCard(work: DemoCatalogue.Work): SearchRecord =
        catalogue.search(work.title)?.records?.firstOrNull { it.workIrn == work.workIrn }
            ?: card(work.workIrn, work.title, work.author, work.year, work.editions.firstNotNullOfOrNull { it.isbn }, work.editions.first().cover, emptyList())

    /** One edition as a saved-search list shows it: `workIrn` = its bib IRN, one format with an empty code. */
    private fun editionCard(work: DemoCatalogue.Work, edition: DemoCatalogue.Edition): SearchRecord =
        card(
            irn = edition.bibIrn,
            title = work.title,
            author = work.author,
            year = work.year,
            isbn = edition.isbn,
            cover = edition.cover,
            formats = listOf(
                RecordFormat(
                    code = "",
                    label = null,
                    reserveUrl = if (edition.digital) null else "$base/PGM/OPAC/CCOPT/${DemoCatalogue.SEARCH_SET}/0/V/0/R/9/BIBENQ?SVL=${edition.bibIrn}",
                    availabilityUrl = editionUrl(edition.bibIrn),
                ),
            ),
        )

    /** A record from outside the catalogue (the real library's), listed as an edition-level card. */
    private fun recordCard(irn: String, detail: RecordDetail): SearchRecord = card(
        irn = irn,
        title = detail.title ?: BookRef.UNTITLED,
        author = detail.details.firstOrNull { it.label.trim().equals("author", ignoreCase = true) }?.values?.firstOrNull()?.text,
        year = null,
        isbn = detail.isbn,
        cover = detail.coverUrls.firstOrNull(),
        formats = listOf(RecordFormat(code = "", label = null, reserveUrl = null, availabilityUrl = editionUrl(irn))),
    )

    private fun card(irn: String, title: String, author: String?, year: String?, isbn: String?, cover: String?, formats: List<RecordFormat>) =
        SearchRecord(
            rank = null,
            workIrn = irn,
            title = title,
            detailUrl = "$base/ENQ/OPAC/BIBENQ?SETLVL=&IRN=$irn",
            author = author,
            authorRole = author?.let { "author" },
            authorUrl = null,
            year = year,
            language = "English",
            summary = null,
            isbn = isbn,
            coverUrls = listOfNotNull(cover),
            formats = formats,
        )

    private fun editionUrl(bibIrn: String) = "$base/ENQ/OPAC/BIBENQ?SETLVL=&IRN=$bibIrn$EDITION_FORMAT"

    companion object {
        /** The Midnight Library (Book, Large Print, eBook). */
        const val SEED_WORK = "15642035"

        /** The Thursday Murder Club, Book edition (the account has the Large Print reserved). */
        const val SEED_EDITION = "99000601"

        private const val EDITION_FORMAT = "&FMT=RECFMT"
        private val IRN_PARAM = Regex("""[?&]IRN=(\d+)""")
    }
}
