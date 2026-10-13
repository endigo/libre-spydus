package com.librespydus.app.feature.featured

import com.librespydus.app.core.FakeBackend
import com.librespydus.app.core.LibraryBackend
import com.librespydus.app.core.ResultsSort
import kotlinx.coroutines.CompletableDeferred
import uniffi.librespydus_core.Availability
import uniffi.librespydus_core.FeaturedCollection
import uniffi.librespydus_core.FeaturedKind
import uniffi.librespydus_core.FeaturedLink
import uniffi.librespydus_core.FeaturedSection
import uniffi.librespydus_core.Holding
import uniffi.librespydus_core.RecordDetail
import uniffi.librespydus_core.RecordFormat
import uniffi.librespydus_core.RecordRelated
import uniffi.librespydus_core.SearchRecord
import uniffi.librespydus_core.SearchResults
import uniffi.librespydus_core.SpydusException

/**
 * An offline backend for the Featured tests: scripted `featured`, `collection`, `page` (optionally
 * held open by a gate, to count concurrent requests), `availability`, `record` and `holdings`.
 * Everything else is [FakeBackend]'s.
 */
class FeaturedBackend(private val base: FakeBackend = FakeBackend()) : LibraryBackend by base {
    var featuredResult: Result<List<FeaturedLink>> = Result.success(emptyList())
    var collectionResult: Result<FeaturedCollection> = Result.success(FeaturedCollection(null, emptyList()))
    val pages = mutableMapOf<String, Result<SearchResults>>()
    val availabilityByUrl = mutableMapOf<String, Availability>()
    val records = mutableMapOf<String, RecordDetail>()
    val holdingsByUrl = mutableMapOf<String, Result<List<Holding>>>()

    /** When set, `page(url)` waits for `gates[url]` before answering. */
    val gates = mutableMapOf<String, CompletableDeferred<Unit>>()

    val featuredCalls = mutableListOf<Unit>()
    val collectionCalls = mutableListOf<String>()
    val pageCalls = mutableListOf<String>()
    val recordUrls = mutableListOf<String>()
    val searchCalls = mutableListOf<String>()
    val holdingsCalls = mutableListOf<String>()
    var inFlightPages = 0
    var maxInFlightPages = 0

    override suspend fun featured(): List<FeaturedLink> {
        featuredCalls += Unit
        return featuredResult.getOrThrow()
    }

    override suspend fun collection(url: String): FeaturedCollection {
        collectionCalls += url
        return collectionResult.getOrThrow()
    }

    override suspend fun page(url: String): SearchResults {
        pageCalls += url
        inFlightPages += 1
        maxInFlightPages = maxOf(maxInFlightPages, inFlightPages)
        try {
            gates[url]?.await()
            return (pages[url] ?: throw SpydusException.Upstream("no page $url")).getOrThrow()
        } finally {
            inFlightPages -= 1
        }
    }

    override suspend fun search(query: String): SearchResults {
        searchCalls += query
        return base.search(query)
    }

    override suspend fun searchSorted(query: String, sort: ResultsSort) = search(query)

    override suspend fun availability(url: String): Availability =
        availabilityByUrl[url] ?: throw SpydusException.Upstream("no availability $url")

    override suspend fun record(url: String): RecordDetail {
        recordUrls += url
        return records[url] ?: throw SpydusException.Upstream("no record $url")
    }

    override suspend fun holdings(url: String): List<Holding> {
        holdingsCalls += url
        return (holdingsByUrl[url] ?: Result.success(emptyList())).getOrThrow()
    }
}

object FeaturedFixtures {
    const val ORIGIN = "https://canning.spydus.com"
    const val LOT_URL = "$ORIGIN/cgi-bin/spydus.exe/MSGTRNGEN/OPAC/LIBRARYOFTHINGS?HOMEPRMS=UD_LIBRARYOFTHINGS"

    fun link(title: String, kind: FeaturedKind, url: String, icon: String? = null, description: String? = null) =
        FeaturedLink(title, description, icon, kind, url)

    val canningLinks = listOf(
        link("What's New", FeaturedKind.COLLECTION, "$ORIGIN/cgi-bin/spydus.exe/MSGTRNGEN/OPAC/NEWITEMS?HOMEPRMS=UD_NEWITEMS", "book"),
        link("Coming Soon", FeaturedKind.SEARCH, "$ORIGIN/cgi-bin/spydus.exe/RUNSQRY/OPAC/BIBENQ?SQRYIRN=4050924&QRYTEXT=Coming%20Soon&NRECS=20", "balloons"),
        link("Library of Things", FeaturedKind.COLLECTION, LOT_URL, "game-board"),
    )

    fun sectionUrl(n: Int) = "$ORIGIN/cgi-bin/spydus.exe/RUNSQRY/OPAC/BIBENQ?SQRYIRN=$n&SETLVL=SET&ISGLB=0&NRECS=20"

    fun section(n: Int, title: String = "Section $n") = FeaturedSection(title, sectionUrl(n))

    /** A saved-search card: one bib, one format with an empty code and no label (§5.10). */
    fun editionRecord(irn: String, title: String = "Mini Keyboard : Musical Instruments") = SearchRecord(
        rank = 1, workIrn = irn, title = title,
        detailUrl = "$ORIGIN/cgi-bin/spydus.exe/FULL/OPAC/BIBENQ/78607055/$irn,1",
        author = null, authorRole = null, authorUrl = null, year = null, language = null,
        summary = "Mini keys! Big fun!", isbn = null,
        coverUrls = listOf("https://img.example/$irn.jpg"),
        formats = listOf(editionFormat(irn)),
    )

    fun editionFormat(irn: String) = RecordFormat(
        code = "",
        label = null,
        reserveUrl = "$ORIGIN/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/78607055/0/V/0/R/9/BIBENQ?SVL=$irn&RECFMT=",
        availabilityUrl = "$ORIGIN/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&IRN=$irn&FMT=RECFMT",
    )

    fun holdingsUrl(irn: String) = "$ORIGIN/cgi-bin/spydus.exe/XHLD/OPAC/BIBENQ/78607060/$irn"

    fun results(records: List<SearchRecord>, next: String? = null, noResults: Boolean = records.isEmpty()) =
        SearchResults(null, null, records.size.toLong(), null, next, null, noResults, records)

    fun record(irn: String, title: String = "Mini Keyboard : Musical Instruments") = RecordDetail(
        bibIrn = irn, brn = "307745", title = title, seriesTitle = null, isbn = null, coverUrls = emptyList(),
        summary = "Mini keys! Big fun!", details = emptyList(),
        bookmarkUrl = "$ORIGIN/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?BRN=307745",
        related = RecordRelated(null, null, null, null),
    )

    /** Edition-level availability: counts from the holdings, and the holdings URL. */
    fun availability(irn: String, copies: Long, available: Long) = Availability(
        bibRecordIrn = irn, title = null, publisher = null,
        counts = mapOf("copies" to copies, "available" to available), reserved = null, copies = copies,
        holdingsUrl = holdingsUrl(irn), recordUrl = null,
    )

    fun holding(location: String?, status: String?, available: Boolean, callNumber: String? = null, description: String? = null) =
        Holding(location, "Library of Things", callNumber, status, description, available)
}
