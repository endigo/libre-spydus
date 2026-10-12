package com.librespydus.app.core

import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.longOrNull
import uniffi.librespydus_core.Availability
import uniffi.librespydus_core.DetailRow
import uniffi.librespydus_core.DetailValue
import uniffi.librespydus_core.RecordDetail
import uniffi.librespydus_core.RecordFormat
import uniffi.librespydus_core.RecordRelated
import uniffi.librespydus_core.SearchRecord
import uniffi.librespydus_core.SearchResults
import java.text.Normalizer
import java.util.Locale

/**
 * Demo mode's offline catalogue (§5.4): the demo account's books as search results, format
 * availability and full bib records (`assets/demo/catalogue.json`), so a loan's or reservation's
 * page, the format it is of and "On loan" / "Reserved" all work without a network. Each method
 * answers the URLs this catalogue makes and the shapes the real OPAC uses for these books, and
 * returns null for anything else (DemoBackend then asks the real library):
 * - `…/BIBENQ?SETLVL=&IRN={bib}` (the session-free record URL) and the bookmark
 *   `…/BIBENQ?SETLVL=&BRN={brn}` → the full record;
 * - a demo row's own link, `…/BIBENQ/{ACCOUNT_SET}?SETLVL=&QRY=IRN({bib})&NAVLVL=SET`, → an empty
 *   record, as the real OPAC answers a set from the patron's session without it;
 * - a search that names a demo title → that work with its formats. Two works have more than one
 *   edition, and the account holds only one of them.
 */
class DemoCatalogue(val works: List<Work>, origin: String) {
    data class Edition(
        val code: String,
        val label: String,
        val bibIrn: String,
        val brn: String?,
        val isbn: String?,
        val imprint: String?,
        val collation: String?,
        val cover: String?,
        val digital: Boolean,
        val counts: Map<String, Long>,
    )

    data class Work(
        val workIrn: String,
        val title: String,
        val author: String,
        val year: String?,
        val summary: String,
        val genres: List<String>,
        val subjects: List<String>,
        val audience: String?,
        val editions: List<Edition>,
    )

    private val base = "${origin.trimEnd('/')}/cgi-bin/spydus.exe"
    private val byIrn = works.flatMap { work -> work.editions.map { it.bibIrn to (work to it) } }.toMap()
    private val byBrn = works.flatMap { work -> work.editions.mapNotNull { e -> e.brn?.let { it to (work to e) } } }.toMap()
    private val byAvailabilityUrl = works.flatMap { work -> work.editions.map { availabilityUrl(work, it) to (work to it) } }.toMap()
    private val byDetailUrl = works.associateBy(::detailUrl)

    /** The demo works whose title [query] names (a typed title, or the book page's title + author search). */
    fun search(query: String): SearchResults? {
        val words = " ${normalized(query)} "
        val hits = works.filter { work -> normalized(work.title, dropArticle = true).let { it.isNotEmpty() && " $it " in words } }
        if (hits.isEmpty()) return null
        return SearchResults(query, SEARCH_SET, hits.size.toLong(), null, null, null, false, hits.mapIndexed { i, work -> searchRecord(work, i + 1L) })
    }

    /** A demo format's availability, with the edition's own bib IRN as `bibRecordIrn`. */
    fun availability(url: String): Availability? {
        val (work, edition) = byAvailabilityUrl[url] ?: return null
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

    fun record(url: String): RecordDetail? {
        if (!url.startsWith("$base/")) return null
        if (ACCOUNT_SET_LINK.containsMatchIn(url)) return EMPTY_RECORD
        byAvailabilityUrl[url]?.let { (work, edition) -> return record(work, edition) }
        byDetailUrl[url]?.let { work -> return record(work, work.editions.first()) }
        if (!SESSION_FREE_LINK.containsMatchIn(url)) return null
        val hit = param("IRN", url)?.let(byIrn::get) ?: param("BRN", url)?.let(byBrn::get) ?: return null
        return record(hit.first, hit.second)
    }

    /** A loan's or reservation's `bibUrl` as the OPAC writes it, in the demo account's session set. */
    fun accountRowUrl(bibIrn: String): String = "$base/ENQ/OPAC/BIBENQ/$ACCOUNT_SET?SETLVL=&QRY=IRN($bibIrn)&NAVLVL=SET"

    private fun searchRecord(work: Work, rank: Long): SearchRecord {
        val first = work.editions.first()
        return SearchRecord(
            rank = rank,
            workIrn = work.workIrn,
            title = work.title,
            detailUrl = detailUrl(work),
            author = work.author,
            authorRole = "author",
            authorUrl = null,
            year = work.year,
            language = "English",
            summary = work.summary,
            isbn = work.editions.firstNotNullOfOrNull { it.isbn },
            coverUrls = listOfNotNull(first.cover),
            formats = work.editions.map { edition ->
                RecordFormat(
                    code = edition.code,
                    label = edition.label,
                    reserveUrl = if (edition.digital) null else "$base/PGM/OPAC/CCOPT/$SEARCH_SET/0/V/0/R/9/ALLWRKENQ?SVL=${work.workIrn}&RECFMT=${edition.code}",
                    availabilityUrl = availabilityUrl(work, edition),
                )
            },
        )
    }

    private fun record(work: Work, edition: Edition): RecordDetail {
        fun row(label: String, vararg values: String?, url: String? = null) =
            values.filterNotNull().takeIf { it.isNotEmpty() }?.let { texts -> DetailRow(label, texts.map { DetailValue(it, url) }) }
        val bookmark = edition.brn?.let { "$base/ENQ/OPAC/BIBENQ?SETLVL=&BRN=$it" } ?: "$base/ENQ/OPAC/BIBENQ?SETLVL=&IRN=${edition.bibIrn}"
        val byline = Formatting.displayAuthor(work.author)
        return RecordDetail(
            bibIrn = edition.bibIrn,
            brn = edition.brn,
            title = work.title,
            seriesTitle = null,
            isbn = edition.isbn,
            coverUrls = listOfNotNull(edition.cover),
            summary = work.summary,
            details = listOfNotNull(
                row("Main title", if (byline != null) "${work.title} / $byline." else work.title),
                row("Author", "${work.author}, author"),
                row("Work", work.title, url = "$base/ENQ/OPAC/WRKENQ?SETLVL=&IRN=${work.workIrn}"),
                row("Imprint", edition.imprint),
                row("Collation", edition.collation),
                row("ISBN", edition.isbn),
                row("Genre", *work.genres.toTypedArray()),
                row("Subject", *work.subjects.toTypedArray()),
                row("Audience", work.audience),
                row("BRN", edition.brn),
                row("Bookmark link", bookmark),
            ),
            bookmarkUrl = bookmark,
            related = RecordRelated(null, null, null, null),
        )
    }

    private fun detailUrl(work: Work) = "$base/FULL/OPAC/ALLWRKENQ/$SEARCH_SET/${work.workIrn},1"

    private fun availabilityUrl(work: Work, edition: Edition) =
        "$base/ENQ/OPAC/BIBENQ/$SEARCH_SET?SETLVL=SET&FMT=RECFMT&XSLT=XRecFmtDisplay.xsl&SETID=$SEARCH_SET&QRY=WRK01%5C${work.workIrn}+%2B+03902\\${edition.code}"

    companion object {
        /** The demo account's OPAC session set (its rows' links and actions use it). */
        const val ACCOUNT_SET = "78500000"

        /** The set id of demo search results. */
        const val SEARCH_SET = "78599000"

        private val ACCOUNT_SET_LINK = Regex("""/BIBENQ/$ACCOUNT_SET(?!\d)""", RegexOption.IGNORE_CASE)
        private val SESSION_FREE_LINK = Regex("""/BIBENQ\?""", RegexOption.IGNORE_CASE)
        private val ARTICLES = setOf("the", "a", "an")

        /** What the OPAC sends for a set from another session ("Session must be logged in"). */
        val EMPTY_RECORD = RecordDetail(null, null, null, null, null, emptyList(), null, emptyList(), null, RecordRelated(null, null, null, null))

        fun parse(json: JsonObject, origin: String): DemoCatalogue = DemoCatalogue(
            works = json.array("works").map { it.jsonObject }.map { work ->
                Work(
                    workIrn = work.str("workIrn")!!,
                    title = work.str("title")!!,
                    author = work.str("author")!!,
                    year = work.str("year"),
                    summary = work.str("summary")!!,
                    genres = work.strings("genres"),
                    subjects = work.strings("subjects"),
                    audience = work.str("audience"),
                    editions = work.array("editions").map { it.jsonObject }.map { e ->
                        Edition(
                            code = e.str("code")!!,
                            label = e.str("label")!!,
                            bibIrn = e.str("bibIrn")!!,
                            brn = e.str("brn"),
                            isbn = e.str("isbn"),
                            imprint = e.str("imprint"),
                            collation = e.str("collation"),
                            cover = e.str("cover"),
                            digital = e.str("digital") == "true",
                            counts = (e["counts"] as? JsonObject).orEmpty()
                                .mapNotNull { (key, value) -> value.jsonPrimitive.longOrNull?.let { key to it } }.toMap(),
                        )
                    },
                )
            },
            origin = origin,
        )

        /** Lower case, accents and punctuation dropped: "Baby-sitters club. 18" → "baby sitters club 18". */
        private fun normalized(text: String, dropArticle: Boolean = false): String {
            val words = Normalizer.normalize(text, Normalizer.Form.NFD).replace(Regex("""\p{Mn}+"""), "").lowercase(Locale.ROOT)
                .map { if (it.isLetterOrDigit()) it else ' ' }.joinToString("")
                .split(' ').filter { it.isNotEmpty() }
            val kept = if (dropArticle && words.size > 1 && words.first() in ARTICLES) words.drop(1) else words
            return kept.joinToString(" ")
        }

        /** A query parameter's value, matched in the raw text (OPAC URLs hold characters URI parsers reject). */
        private fun param(name: String, url: String): String? =
            Regex("""[?&]$name=([^&#]+)""", RegexOption.IGNORE_CASE).find(url)?.groupValues?.get(1)

        private fun JsonObject.str(key: String): String? = this[key]?.takeUnless { it is JsonNull }?.jsonPrimitive?.contentOrNull
        private fun JsonObject.array(key: String): JsonArray = (this[key] as? JsonArray) ?: JsonArray(emptyList())
        private fun JsonObject.strings(key: String): List<String> = array(key).mapNotNull { it.jsonPrimitive.contentOrNull }
    }
}
