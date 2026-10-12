package com.librespydus.app.feature.detail

import com.librespydus.app.core.AvailabilitySummary
import com.librespydus.app.core.BookRef
import com.librespydus.app.core.CoreHelpers
import com.librespydus.app.core.FormatRef
import com.librespydus.app.feature.search.BrowseLoadable
import com.librespydus.app.feature.search.trimmedOrNull
import uniffi.librespydus_core.Availability
import uniffi.librespydus_core.DetailRow
import uniffi.librespydus_core.RecordDetail
import uniffi.librespydus_core.RecordRelated
import uniffi.librespydus_core.RelatedList
import uniffi.librespydus_core.SearchRecord
import java.text.Normalizer
import java.util.Locale

// Pure rules behind the book detail screen, kept apart from the UI so they're unit-tested
// (DetailLogicTest). Mirrors ios/LibreSpydus/Features/Detail/DetailLogic.swift.

/** Which URL loads the record. */
object DetailRecordSource {
    /**
     * URLs to try with `record(url)`, best first.
     *
     * A search record's `detailUrl` (`…/FULL/OPAC/ALLWRKENQ/…`) loads the OPAC's work-level page:
     * a summary but no subjects, imprint, ISBNs, similar titles or other editions. A format's
     * `availabilityUrl` loads the full bib page, so it comes first for work-level refs. Bib URLs
     * (loans, related items, saved bookmarks) are tried first as they are.
     */
    fun candidateUrls(refUrl: String?, format: FormatRef?): List<String> {
        val formatUrl = format?.availabilityUrl.trimmedOrNull()
        val ref = refUrl.trimmedOrNull()
        val ordered = if (isWorkLevel(ref)) listOf(formatUrl, ref) else listOf(ref, formatUrl)
        return ordered.filterNotNull().distinct()
    }

    /**
     * [candidateUrls] for [ref], led by its session-free record URL when it has one
     * ([sessionFreeUrl]): a saved loan's `…/BIBENQ/{set}…` link loads an empty record without the
     * patron's OPAC session.
     */
    fun candidateUrls(ref: BookRef, format: FormatRef?): List<String> =
        (listOfNotNull(sessionFreeUrl(ref)) + candidateUrls(ref.url, format)).distinct()

    /**
     * `…/BIBENQ?SETLVL=&IRN={bib IRN}` for a ref keyed `bib:<IRN>` whose URL is a `…/BIBENQ/{set}…`
     * link (a loan or reservation saved before the app used the session-free URL); else null.
     */
    fun sessionFreeUrl(ref: BookRef): String? {
        val url = ref.url.trimmedOrNull() ?: return null
        if (!url.contains("/BIBENQ/", ignoreCase = true)) return null
        val irn = ref.bibIrn.trimmedOrNull() ?: ref.id.removePrefix("bib:").takeIf { ref.id.startsWith("bib:") }.trimmedOrNull()
            ?: return null
        return CoreHelpers.bibRecordUrl(irn, url)?.takeIf { it != url }
    }

    /**
     * A record without a title, summary or detail rows: what the OPAC sends for a session set the
     * app isn't part of ("Session must be logged in"). The screen tries the next URL.
     */
    fun isEmpty(record: RecordDetail): Boolean =
        record.title.trimmedOrNull() == null && record.summary.trimmedOrNull() == null && record.details.isEmpty()

    /** `…/ALLWRKENQ/…`, `…/WRKENQ?…`: a work (grouped editions), not one bib record. */
    fun isWorkLevel(url: String?): Boolean = url?.contains("WRKENQ", ignoreCase = true) == true
}

/**
 * A format's availability as the header shows it, and the bib record it resolves to
 * (`Availability.bibRecordIrn`, the edition's own IRN: what loans and reservations name).
 */
data class FormatAvailability(val summary: AvailabilitySummary, val bibRecordIrn: String?) {
    companion object {
        fun of(availability: Availability) =
            FormatAvailability(CoreHelpers.availabilitySummary(availability), availability.bibRecordIrn.trimmedOrNull())
    }
}

object DetailText {
    private val BRACKETS = Regex("""\s*\[[^\]]*]""")

    /**
     * A title for display: catalogue notes in brackets dropped
     * ("The Midnight Library [electronic resource] : A Novel" → "The Midnight Library : A Novel").
     */
    fun displayTitle(title: String): String = title.replace(BRACKETS, "").trim().ifEmpty { title }
}

/** What the detail screen keeps of a ref once its record has loaded (the opened `id` is kept). */
fun BookRef.updatedWith(detail: RecordDetail): BookRef {
    var updated = copy(
        url = detail.bookmarkUrl.trimmedOrNull() ?: url,
        coverUrl = coverUrl ?: detail.coverUrls.firstOrNull(),
        isbn = isbn ?: detail.isbn.trimmedOrNull(),
    )
    if (updated.title.isBlank() || updated.title == BookRef.UNTITLED) {
        detail.title.trimmedOrNull()?.let { updated = updated.copy(title = it) }
    }
    if (updated.author == null) {
        val row = detail.details.firstOrNull { it.label.trim().lowercase() in AUTHOR_LABELS }
        updated = updated.copy(author = row?.values?.firstOrNull()?.text.trimmedOrNull())
    }
    return updated
}

private val AUTHOR_LABELS = setOf("author", "creator")

object FormatSelection {
    /** The format shown first: the first physical one, else the first. */
    fun defaultIndex(formats: List<FormatRef>): Int? {
        if (formats.isEmpty()) return null
        return formats.indexOfFirst { !CoreHelpers.isDigitalFormat(it) }.takeIf { it >= 0 } ?: 0
    }
}

/** The availability line under the byline for the selected format. */
data class AvailabilityLine(val text: String, val tone: Tone) {
    enum class Tone { AVAILABLE, NORMAL, DIGITAL, MUTED }

    companion object {
        /** `availabilitySummary(...).label`; a digital format without counts reads "Available online". */
        fun of(summary: AvailabilitySummary?, format: FormatRef): AvailabilityLine {
            val digital = CoreHelpers.isDigitalFormat(format)
            val hasCounts = summary != null && (summary.available != null || summary.copies != null || summary.reserved != null)
            return when {
                digital && !hasCounts -> AvailabilityLine("Available online", Tone.DIGITAL)
                summary != null -> AvailabilityLine(
                    summary.label,
                    when {
                        (summary.available ?: 0) > 0 -> Tone.AVAILABLE
                        digital -> Tone.DIGITAL
                        else -> Tone.NORMAL
                    },
                )
                else -> AvailabilityLine("Ask the library about availability", Tone.MUTED)
            }
        }
    }
}

/** A tappable subject chip: [label] on the chip, [query] for the results route. */
data class SubjectChip(val label: String, val query: String)

object DetailSubjects {
    const val LIMIT = 6

    /**
     * Chips from rows labelled like Subject(s), Genre/Form, Index terms and Audience, in that
     * priority order (genres first), deduplicated, at most [limit].
     */
    fun extract(details: List<DetailRow>, limit: Int = LIMIT): List<SubjectChip> {
        val ranked = details.mapNotNull { row -> rank(row.label)?.let { it to row } }
            .withIndex()
            .sortedWith(compareBy({ it.value.first }, { it.index }))
            .map { it.value }
        val seen = mutableSetOf<String>()
        val chips = mutableListOf<SubjectChip>()
        for ((rank, row) in ranked) {
            for (value in row.values) {
                val chip = chip(value.text, isAudience = rank == 2) ?: continue
                if (!seen.add(chip.label.lowercase())) continue
                chips += chip
                if (chips.size == limit) return chips
            }
        }
        return chips
    }

    /** Whether a details row becomes chips (and so is left out of the details list). */
    fun isSubjectRow(label: String): Boolean = rank(label) != null

    /** 0 genre/form, 1 subject/topic/index terms, 2 audience; null for any other row. */
    private fun rank(label: String): Int? {
        val key = label.trim().lowercase()
        return when {
            "genre" in key || key == "form" -> 0
            key.startsWith("subject") || key.startsWith("topic") || key.startsWith("index term") -> 1
            key.startsWith("audience") || key.endsWith("audience") -> 2
            else -> null
        }
    }

    private val PART_TRIM = setOf('.', ';', ',')

    /**
     * "Libraries -- Fiction." → "Libraries · Fiction" / "Libraries Fiction";
     * "FICTION|Literary|" (BISAC, Brazoria County) → "Fiction · Literary" (short acronyms such as
     * "DVD" stay as they are); "Interest age level: For ages 6-11 years." → "For ages 6-11 years".
     */
    private fun chip(raw: String, isAudience: Boolean): SubjectChip? {
        var text = raw.trim()
        if (isAudience && ':' in text) text = text.substringAfterLast(':')
        val parts = text.replace("|", "--").split("--")
            .map { part -> part.trim { it.isWhitespace() || it in PART_TRIM } }
            .filter { it.isNotEmpty() }
            .map { part -> if (part.length > 3 && part == part.uppercase() && part != part.lowercase()) capitalizeWords(part) else part }
        if (parts.isEmpty()) return null
        val label = parts.joinToString(" · ")
        val query = parts.joinToString(" ")
            .map { if (it.isLetterOrDigit() || it == '-' || it == '\'') it else ' ' }
            .joinToString("")
            .split(' ')
            .filter { it.isNotEmpty() }
            .joinToString(" ")
        if (query.isEmpty()) return null
        return SubjectChip(label, query)
    }

    /** "SCIENCE FICTION" → "Science Fiction". */
    private fun capitalizeWords(text: String): String =
        text.lowercase().split(' ').joinToString(" ") { word ->
            word.split('-').joinToString("-") { it.replaceFirstChar { c -> c.titlecase(Locale.ROOT) } }
        }
}

/** One line of the compact details list. */
data class DetailFact(val label: String, val value: String)

object DetailFacts {
    const val MAX_ROWS = 8
    const val MAX_VALUES = 3

    /**
     * The library's details rows minus what the screen already shows (title, byline, subject
     * chips) and navigation-only rows (work link, bookmark, BRN, "Go to"). A single-author row is
     * the byline; several authors (illustrator, translator…) are listed.
     */
    fun rows(details: List<DetailRow>, maxRows: Int = MAX_ROWS): List<DetailFact> {
        val facts = mutableListOf<DetailFact>()
        for (row in details) {
            val key = row.label.trim().lowercase()
            if (key in HIDDEN || DetailSubjects.isSubjectRow(row.label)) continue
            val values = row.values.map { it.text.trim() }.filter { it.isNotEmpty() }
            if (values.isEmpty()) continue
            if (key in BYLINE_LABELS && values.size < 2) continue
            val shown = values.take(MAX_VALUES).toMutableList()
            if (values.size > MAX_VALUES) shown[MAX_VALUES - 1] = shown[MAX_VALUES - 1] + " …"
            facts += DetailFact(row.label.trim(), shown.joinToString("\n"))
            if (facts.size == maxRows) break
        }
        return facts
    }

    private val HIDDEN = setOf("main title", "title", "work", "bookmark link", "bookmark", "brn", "go to", "permalink")
    private val BYLINE_LABELS = setOf("author", "creator", "authors", "creators")
}

/** The related carousels, in display order. */
enum class RelatedPanel(val title: String) {
    SIMILAR("Similar books"),
    AUTHOR("More by this author"),
    SERIES("In this series"),
    VERSIONS("Other editions"),
    ;

    fun url(related: RecordRelated): String? = when (this) {
        SIMILAR -> related.similar
        AUTHOR -> related.author
        SERIES -> related.series
        VERSIONS -> related.versions
    }.trimmedOrNull()

    companion object {
        /**
         * Panels to draw, in order: the record links to them and they're loading or have items.
         * Empty and failed panels are hidden.
         */
        fun visible(related: RecordRelated, states: Map<RelatedPanel, BrowseLoadable<List<BookRef>>>): List<RelatedPanel> =
            entries.filter { panel ->
                if (panel.url(related) == null) return@filter false
                when (val state = states[panel] ?: BrowseLoadable.Idle) {
                    BrowseLoadable.Idle, BrowseLoadable.Loading -> true
                    is BrowseLoadable.Loaded -> state.value.isNotEmpty()
                    is BrowseLoadable.Failed -> false
                }
            }

        /** A panel's books as refs, without the book on screen (matched by BRN). */
        fun items(list: RelatedList, excludingBrn: String?): List<BookRef> {
            val brn = excludingBrn.trimmedOrNull()
            return list.items
                .filter { brn == null || it.brn.trimmedOrNull() != brn }
                .map(BookRef::from)
                .distinctBy { it.id }
        }
    }
}

/**
 * A ref opened without formats (loan, favourite, related item) re-finds them with a title +
 * author search after its record loads, and accepts a result only on a confident match.
 */
object FormatRefinder {
    /**
     * What to search for. ISBNs aren't in the OPAC keyword index (an ISBN search came back "no
     * records" on Canning, Hutt, Cumberland and Brazoria County), so it's the main title plus the
     * author's surname, falling back to the ISBN only when there is no title.
     */
    fun searchQuery(ref: BookRef, record: RecordDetail?): String? {
        val title = mainTitle(displayTitle(ref, record))
        val surname = surname(ref.author ?: author(record))
        if (title.isNotEmpty()) return listOfNotNull(title, surname).joinToString(" ")
        return isbns(ref, record).firstOrNull()
    }

    /**
     * The search result that is this book, or null when no result is a confident match: same work
     * IRN, else a shared ISBN, else the only result with the same title and author.
     */
    fun match(ref: BookRef, record: RecordDetail?, candidates: List<SearchRecord>): SearchRecord? {
        val usable = candidates.filter { it.formats.isNotEmpty() }
        val works = workIrns(ref, record)
        if (works.isNotEmpty()) {
            usable.firstOrNull { it.workIrn.trimmedOrNull() in works }?.let { return it }
        }
        val isbns = isbns(ref, record).toSet()
        if (isbns.isNotEmpty()) {
            usable.firstOrNull { candidate -> normalizedIsbns(listOf(candidate.isbn.orEmpty())).any { it in isbns } }?.let { return it }
        }
        val title = normalizedTitle(displayTitle(ref, record))
        if (title.isEmpty()) return null
        val surname = surname(ref.author ?: author(record))
        val sameTitle = usable.filter { candidate ->
            if (normalizedTitle(candidate.title.orEmpty()) != title) return@filter false
            val other = surname(candidate.author)
            surname == null || other == null || surname == other
        }
        if (sameTitle.size == 1) return sameTitle[0]
        ref.year.trimmedOrNull()?.let { year ->
            val sameYear = sameTitle.filter { it.year.trimmedOrNull() == year }
            if (sameYear.size == 1) return sameYear[0]
        }
        return null
    }

    /**
     * Work IRNs this book is known by: a `work:` ref id, the record's "Work" row link
     * (`…/WRKENQ/…?IRN=<work>`), or a work-level record's own IRN.
     */
    fun workIrns(ref: BookRef, record: RecordDetail?): Set<String> {
        val irns = mutableSetOf<String>()
        if (ref.id.startsWith("work:")) irns += ref.id.removePrefix("work:")
        record ?: return irns
        for (row in record.details) {
            if (row.label.trim().lowercase() != "work") continue
            for (value in row.values) queryItem("IRN", value.url)?.let { irns += it }
        }
        if (DetailRecordSource.isWorkLevel(record.bookmarkUrl)) {
            queryItem("IRN", record.bookmarkUrl)?.let { irns += it }
            record.bibIrn.trimmedOrNull()?.let { irns += it }
        }
        return irns
    }

    /** Every ISBN the ref and record mention, as ISBN-13. */
    fun isbns(ref: BookRef, record: RecordDetail?): List<String> {
        val texts = mutableListOf(ref.isbn.orEmpty(), record?.isbn.orEmpty())
        record?.details?.filter { it.label.lowercase().startsWith("isbn") }?.forEach { row -> texts += row.values.map { it.text } }
        return normalizedIsbns(texts)
    }

    private val TOKEN_SEPARATORS = Regex("""[\s();,]+""")

    /**
     * ISBN-10s and ISBN-13s found in [texts] ("9781786892720 (paperback)", "1-78689-272-3"), as
     * ISBN-13s, in order, without duplicates.
     */
    fun normalizedIsbns(texts: List<String>): List<String> {
        val result = mutableListOf<String>()
        for (text in texts) {
            for (token in text.split(TOKEN_SEPARATORS)) {
                if (token.isEmpty()) continue
                val isbn = isbn13(token.replace("-", "").uppercase()) ?: continue
                if (isbn !in result) result += isbn
            }
        }
        return result
    }

    private fun isbn13(text: String): String? {
        if (text.length == 13 && text.all { it.isDigit() }) return text
        if (text.length != 10 || !text.take(9).all { it.isDigit() } || !(text[9].isDigit() || text[9] == 'X')) return null
        val body = "978" + text.take(9)
        val sum = body.withIndex().sumOf { (index, char) -> char.digitToInt() * if (index % 2 == 0) 1 else 3 }
        return body + ((10 - sum % 10) % 10)
    }

    /** "The Midnight Library [electronic resource] : a novel / Matt Haig." → "midnight library". */
    fun normalizedTitle(title: String): String {
        val words = fold(mainTitle(title))
            .map { if (it.isLetterOrDigit()) it else ' ' }
            .joinToString("")
            .split(' ')
            .filter { it.isNotEmpty() }
        val trimmed = if (words.size > 1 && words.first() in ARTICLES) words.drop(1) else words
        return trimmed.joinToString(" ")
    }

    private val ARTICLES = setOf("the", "a", "an")
    private val BRACKET_NOTE = Regex("""\[[^\]]*]""")
    private val TITLE_TRIM = setOf('.', ':', '/', ';', ',')

    /** The title without bracketed notes, subtitle or statement of responsibility. */
    fun mainTitle(title: String): String {
        var text = title.replace(BRACKET_NOTE, "")
        for (separator in listOf(" / ", " : ", ": ")) {
            val index = text.indexOf(separator)
            if (index >= 0) text = text.substring(0, index)
        }
        return text.trim { it.isWhitespace() || it in TITLE_TRIM }
    }

    /** "Haig, Matt, 1975-" → "haig"; "Matt Haig" → "haig". */
    fun surname(author: String?): String? {
        val name = author.trimmedOrNull() ?: return null
        val part = if (',' in name) name.split(',').firstOrNull { it.isNotEmpty() }
        else name.split(' ').lastOrNull { it.isNotEmpty() }
        val folded = fold(part.orEmpty()).trim { !it.isLetter() }
        return folded.ifEmpty { null }
    }

    private fun displayTitle(ref: BookRef, record: RecordDetail?): String {
        if (ref.title != BookRef.UNTITLED && ref.title.isNotBlank()) return ref.title
        return record?.title.orEmpty()
    }

    private fun author(record: RecordDetail?): String? =
        record?.details?.firstOrNull { it.label.lowercase() in AUTHOR_LABELS }?.values?.firstOrNull()?.text

    /** Case- and diacritic-insensitive form: "Siobhán" → "siobhan". */
    private fun fold(text: String): String =
        Normalizer.normalize(text, Normalizer.Form.NFD).replace(Regex("""\p{Mn}+"""), "").lowercase(Locale.ROOT)

    /** A query parameter's value, matched in the raw text (OPAC URLs hold characters URI parsers reject). */
    private fun queryItem(name: String, url: String?): String? {
        url ?: return null
        return Regex("""[?&]${Regex.escape(name)}=([^&#]+)""", RegexOption.IGNORE_CASE).find(url)?.groupValues?.get(1)?.takeIf { it.isNotEmpty() }
    }
}
