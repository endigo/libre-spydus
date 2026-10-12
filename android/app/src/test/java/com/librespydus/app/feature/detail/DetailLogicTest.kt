package com.librespydus.app.feature.detail

import com.librespydus.app.core.AvailabilitySummary
import com.librespydus.app.core.BookRef
import com.librespydus.app.core.CoreHelpers
import com.librespydus.app.core.FormatRef
import com.librespydus.app.core.UserFacingError
import com.librespydus.app.feature.search.BrowseLoadable
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.librespydus_core.Availability
import uniffi.librespydus_core.DetailRow
import uniffi.librespydus_core.DetailValue
import uniffi.librespydus_core.RecordDetail
import uniffi.librespydus_core.RecordFormat
import uniffi.librespydus_core.RecordRelated
import uniffi.librespydus_core.RelatedItem
import uniffi.librespydus_core.RelatedList
import uniffi.librespydus_core.SearchRecord

/** Book detail rules (DetailLogic.kt), on shapes captured from live public Canning / Hutt / Brazoria County pages. */
class DetailLogicTest {
    private val d = DetailFixtures

    // region Subjects

    @Test
    fun subjectChipsComeFromSubjectRowsCappedAtSix() {
        val chips = DetailSubjects.extract(d.midnightLibrary.details)
        assertEquals(6, chips.size)
        assertEquals(SubjectChip("Libraries · Fiction", "Libraries Fiction"), chips.first())
        assertEquals(SubjectChip("Choice (Psychology) · Fiction", "Choice Psychology Fiction"), chips[1])
        assertEquals("Science fiction", chips.last().label)
        assertFalse(chips.map { it.label }.contains("Fantasy fiction"))
    }

    @Test
    fun genresComeFirstAndAudienceDropsItsPrefix() {
        val details = listOf(
            d.row("Subject", "Dragons -- Juvenile fiction.", "Magic -- Juvenile fiction."),
            d.row("Audience", "Interest age level: For ages 6-11 years."),
            d.row("Genre/Form", "Fantasy fiction.", "Graphic novels."),
            d.row("Language", "English"),
        )
        val chips = DetailSubjects.extract(details)
        assertEquals(
            listOf("Fantasy fiction", "Graphic novels", "Dragons · Juvenile fiction", "Magic · Juvenile fiction", "For ages 6-11 years"),
            chips.map { it.label },
        )
        assertEquals("For ages 6-11 years", chips[4].query)
    }

    @Test
    fun subjectChipsAreDeduplicatedAndIndexTermsCount() {
        val details = listOf(
            d.row("Index terms", "Graphic novels", "Libraries"),
            d.row("Subjects", "graphic novels.", "  ", "Time travel"),
        )
        assertEquals(listOf("Graphic novels", "Libraries", "Time travel"), DetailSubjects.extract(details).map { it.label })
        assertTrue(DetailSubjects.extract(listOf(d.row("Imprint", "London : Bloomsbury, 2023."))).isEmpty())
    }

    @Test
    fun bisacSubjectsAreSplitAndCased() {
        // Brazoria County (US) writes subjects BISAC-style.
        val chips = DetailSubjects.extract(listOf(d.row("Subject", "FICTION|Science Fiction|Time Travel|", "Electronic books", "DVD")))
        assertEquals(listOf("Fiction · Science Fiction · Time Travel", "Electronic books", "DVD"), chips.map { it.label })
        assertEquals("Fiction Science Fiction Time Travel", chips[0].query)
    }

    // endregion
    // region Details list

    @Test
    fun factsSkipWhatTheScreenAlreadyShows() {
        val facts = DetailFacts.rows(d.midnightLibrary.details)
        assertEquals(listOf("Imprint", "Collation", "Notes", "ISBN", "Dewey class", "Language"), facts.map { it.label })
        assertEquals("Edinburgh : Canongate, 2020.\n©2020", facts[0].value)
        assertEquals("9781786892720 (paperback)\n1786892723\n1786892707 (hardcover) …", facts[3].value)
    }

    @Test
    fun factsListSeveralAuthorsButNotTheBylineAlone() {
        val details = listOf(
            d.row("Author", "Rowling, J. K., author", "Goes, Peter, illustrator"),
            d.row("Creator", "Haig, Matt, 1975-, author"),
            d.row("Go to", "Instances of this work (6)"),
        )
        assertEquals(listOf(DetailFact("Author", "Rowling, J. K., author\nGoes, Peter, illustrator")), DetailFacts.rows(details))
    }

    // endregion
    // region Formats

    @Test
    fun defaultFormatIsTheFirstPhysicalOne() {
        val ebook = FormatRef("EBK", "eBook")
        val eaudio = FormatRef("EAUD", "eAudioBook", reserveUrl = "https://mybcls.spydus.com/r")
        val book = FormatRef("BK", "Book")
        val largePrint = FormatRef("LP", "Large Print")
        assertEquals(1, FormatSelection.defaultIndex(listOf(ebook, book, largePrint)))
        assertEquals(0, FormatSelection.defaultIndex(listOf(book, ebook)))
        assertEquals(0, FormatSelection.defaultIndex(listOf(ebook, eaudio)))
        assertNull(FormatSelection.defaultIndex(emptyList()))
    }

    @Test
    fun digitalFormatsWithoutCountsReadAvailableOnline() {
        val ebook = FormatRef("EBK", "eBook")
        val noCounts = summary(emptyMap())
        assertEquals(AvailabilityLine("Available online", AvailabilityLine.Tone.DIGITAL), AvailabilityLine.of(noCounts, ebook))
        assertEquals("Available online", AvailabilityLine.of(null, ebook).text)

        // Brazoria County gives e-copies counts (and takes holds on them).
        val eaudio = FormatRef("EAUD", "eAudioBook", reserveUrl = "https://mybcls.spydus.com/r")
        val counted = summary(mapOf("available" to 0L, "copies" to 3L))
        assertEquals(AvailabilityLine("0 of 3 available", AvailabilityLine.Tone.DIGITAL), AvailabilityLine.of(counted, eaudio))
    }

    @Test
    fun physicalAvailabilityUsesTheCoreLabel() {
        val book = FormatRef("BK", "Book")
        assertEquals(AvailabilityLine("2 copies", AvailabilityLine.Tone.NORMAL), AvailabilityLine.of(summary(mapOf("copies" to 2L)), book))
        assertEquals(
            AvailabilityLine("2 of 3 available", AvailabilityLine.Tone.AVAILABLE),
            AvailabilityLine.of(summary(mapOf("available" to 2L, "copies" to 3L)), book),
        )
        assertEquals(AvailabilityLine.Tone.MUTED, AvailabilityLine.of(null, book).tone)
    }

    // endregion
    // region Record source

    @Test
    fun workLevelRefsLoadTheBibRecordThroughTheFormat() {
        val format = FormatRef.from(d.searchRecord.formats[0])
        val work = d.searchRecord.detailUrl
        assertEquals(listOf(format.availabilityUrl!!, work!!), DetailRecordSource.candidateUrls(work, format))

        val bookmark = "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&BRN=275039"
        assertEquals(listOf(bookmark, format.availabilityUrl), DetailRecordSource.candidateUrls(bookmark, format))
        assertEquals(listOf(bookmark), DetailRecordSource.candidateUrls(bookmark, null))
        assertTrue(DetailRecordSource.candidateUrls(null, null).isEmpty())
        assertTrue(DetailRecordSource.isWorkLevel("https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/WRKENQ?SETLVL=&IRN=15642035"))
    }

    @Test
    fun displayTitleDropsBracketedNotes() {
        assertEquals("The Midnight Library : A Novel", DetailText.displayTitle("The Midnight Library [electronic resource] : A Novel"))
        assertEquals("[sound recording]", DetailText.displayTitle("[sound recording]"))
    }

    @Test
    fun aLoadedRecordKeepsTheIdAndStoresTheBookmark() {
        val ref = BookRef.from(d.searchRecord)
        val updated = ref.updatedWith(d.midnightLibrary)
        assertEquals(ref.id, updated.id)
        assertEquals(d.midnightLibrary.bookmarkUrl, updated.url)
        assertEquals(ref.title, updated.title)

        val bare = BookRef.fromUrl(d.midnightLibrary.bookmarkUrl!!).updatedWith(d.midnightLibrary)
        assertEquals("The Midnight Library", bare.title)
        assertEquals("Haig, Matt, 1975-, author", bare.author)
        assertEquals("9781786892720", bare.isbn)
    }

    // endregion
    // region Related panels

    @Test
    fun relatedPanelsHideEmptyFailedAndUnlinkedOnes() {
        val related = RecordRelated("https://x/similar", "https://x/author", null, "https://x/versions")
        val book = BookRef("brn:1", "A")
        val failure = UserFacingError(UserFacingError.Kind.NETWORK, "offline")

        assertEquals(listOf(RelatedPanel.SIMILAR, RelatedPanel.AUTHOR, RelatedPanel.VERSIONS), RelatedPanel.visible(related, emptyMap()))
        assertEquals(
            listOf(RelatedPanel.SIMILAR, RelatedPanel.AUTHOR),
            RelatedPanel.visible(
                related,
                mapOf(RelatedPanel.SIMILAR to BrowseLoadable.Loading, RelatedPanel.AUTHOR to BrowseLoadable.Loaded(listOf(book)), RelatedPanel.VERSIONS to BrowseLoadable.Loaded(emptyList())),
            ),
        )
        assertEquals(
            listOf(RelatedPanel.AUTHOR, RelatedPanel.VERSIONS),
            RelatedPanel.visible(
                related,
                mapOf(
                    RelatedPanel.SIMILAR to BrowseLoadable.Failed(failure),
                    RelatedPanel.AUTHOR to BrowseLoadable.Loaded(listOf(book)),
                    RelatedPanel.SERIES to BrowseLoadable.Loaded(listOf(book)),
                    RelatedPanel.VERSIONS to BrowseLoadable.Loaded(listOf(book)),
                ),
            ),
        )
        assertTrue(RelatedPanel.visible(RecordRelated(" ", null, null, null), emptyMap()).isEmpty())
    }

    @Test
    fun relatedItemsLeaveOutTheBookOnScreen() {
        val list = RelatedList(
            listOf(
                d.related("275039", "The Midnight Library"),
                d.related("304411", "The comfort book"),
                d.related("304411", "The comfort book"),
                d.related("174629", "Evie and the animals"),
            ),
        )
        assertEquals(listOf("brn:304411", "brn:174629"), RelatedPanel.items(list, excludingBrn = "275039").map { it.id })
        assertEquals(3, RelatedPanel.items(list, excludingBrn = null).size)
    }

    // endregion
    // region Re-finding formats

    @Test
    fun refindMatchesTheWorkIrnFromTheRecordsWorkRow() {
        // A "Similar books" item: its ISBN differs from the search result's, the work IRN doesn't.
        val ref = BookRef.from(d.related("467086", "The long night", author = "White, Christian", isbn = "9781399197809"))
        assertEquals("The long night white", FormatRefinder.searchQuery(ref, d.longNight))
        assertEquals("15975964", FormatRefinder.match(ref, d.longNight, d.longNightSearch)?.workIrn)
    }

    @Test
    fun refindMatchesAnIsbnInEitherForm() {
        assertEquals(listOf("9781786892706"), FormatRefinder.normalizedIsbns(listOf("1786892707 (hardcover)", "978-1-78689-270-6", "n/a")))
        assertEquals(listOf("9780804429573"), FormatRefinder.normalizedIsbns(listOf("080442957X")))

        val loan = BookRef("bib:8753080", "The midnight library", author = "Haig, Matt, author", isbn = "1786892707")
        val candidate = d.search(work = "1", title = "Midnight library (different cataloguing)", author = null, isbn = "9781786892706")
        assertEquals("1", FormatRefinder.match(loan, null, listOf(candidate))?.workIrn)
    }

    @Test
    fun refindFallsBackToAUniqueTitleAndAuthor() {
        val ref = BookRef("bib:1", "The Midnight Library [electronic resource] : a novel / Matt Haig.", author = "Haig, Matt", year = "2020")
        val haig = d.search(work = "10", title = "The midnight library", author = "Haig, Matt, 1975-")
        val other = d.search(work = "11", title = "Midnight at the library", author = "Dubosarsky, Ursula")
        val namesake = d.search(work = "12", title = "The Midnight Library", author = "Kohara, Kazuno")
        assertEquals("10", FormatRefinder.match(ref, null, listOf(other, namesake, haig))?.workIrn)

        // Two same-title, same-author works: only the year can tell them apart.
        val reissue = d.search(work = "13", title = "The Midnight Library", author = "Haig, Matt", year = "2023")
        val original = d.search(work = "10", title = "The midnight library", author = "Haig, Matt", year = "2020")
        assertEquals("10", FormatRefinder.match(ref, null, listOf(reissue, original))?.workIrn)
        assertNull(FormatRefinder.match(ref.copy(year = null), null, listOf(reissue, original)))
    }

    @Test
    fun refindRejectsUnconfidentMatches() {
        val ref = BookRef("bib:1", "Hollow bones", author = "Mayne, Andrew")
        val near = d.search(work = "20", title = "Hollow bones and other stories", author = "Mayne, Andrew")
        val sameTitleOtherAuthor = d.search(work = "21", title = "Hollow Bones", author = "Smith, Jo")
        assertNull(FormatRefinder.match(ref, null, listOf(near, sameTitleOtherAuthor)))

        // A matching result without formats can't be used either.
        val formatless = d.search(work = "22", title = "Hollow bones", author = "Mayne, Andrew").copy(formats = emptyList())
        assertNull(FormatRefinder.match(ref, null, listOf(formatless)))
    }

    @Test
    fun refindQueryFallsBackToRecordTitleThenIsbn() {
        val untitled = BookRef("brn:1", BookRef.UNTITLED)
        assertEquals("The Midnight Library haig", FormatRefinder.searchQuery(untitled, d.midnightLibrary))
        val isbnOnly = BookRef("url:x", "", isbn = "1786892707")
        assertEquals("9781786892706", FormatRefinder.searchQuery(isbnOnly, null))
        assertEquals("midnight library", FormatRefinder.normalizedTitle("The Midnight Library / Matt Haig."))
        assertEquals("haig", FormatRefinder.surname("Haig, Matt, 1975-"))
        assertEquals("haig", FormatRefinder.surname("Matt Haig"))
        assertEquals("siobhan", FormatRefinder.surname("O'Brien Siobhán"))
    }

    // endregion

    private fun summary(counts: Map<String, Long>): AvailabilitySummary = CoreHelpers.availabilitySummary(d.availability(counts))
}

/** Record and search shapes from live public pages (Canning, 2026-10-12). */
object DetailFixtures {
    const val RESERVE_URL = "https://canning.spydus.com/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/78529863/0/V/0/R/9/ALLWRKENQ?SVL=15974354&RECFMT=BK"

    val searchRecord = SearchRecord(
        rank = 1, workIrn = "15974354", title = "Baby-Sitter's Club graphic novels. 18, Jessi Ramsey, pet-sitter",
        detailUrl = "https://canning.spydus.com/cgi-bin/spydus.exe/FULL/OPAC/ALLWRKENQ/78529863/15974354,1",
        author = "Crenshaw, Ellen T.", authorRole = "author", authorUrl = null, year = "2025", language = "English",
        summary = null, isbn = "9781546144717",
        coverUrls = listOf("https://secure.syndetics.com/index.aspx?isbn=9781546144717/lc.gif&client=citycanning"),
        formats = listOf(
            RecordFormat("BK", "Book", RESERVE_URL, "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/78529863?FMT=RECFMT&QRY=BK"),
            RecordFormat("EBK", "eBook", null, "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/78529863?FMT=RECFMT&QRY=EBK"),
        ),
    )

    fun row(label: String, vararg values: String) = DetailRow(label, values.map { DetailValue(it, null) })

    fun availability(counts: Map<String, Long>) =
        Availability("1", null, null, counts, counts["reserved"], counts["copies"], null, null)

    fun related(brn: String, title: String, author: String? = null, isbn: String? = null) = RelatedItem(
        title = title,
        bibUrl = "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/78563101?BRN=$brn&SETLVL=",
        brn = brn, author = author, year = "2026", isbn = isbn, coverUrls = emptyList(),
    )

    fun search(work: String, title: String, author: String?, isbn: String? = null, year: String? = null): SearchRecord {
        val base = "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/78563304?SETLVL=SET&FMT=RECFMT&QRY=WRK01%5C$work"
        return SearchRecord(
            rank = 1, workIrn = work, title = title,
            detailUrl = "https://canning.spydus.com/cgi-bin/spydus.exe/FULL/OPAC/ALLWRKENQ/78563304/$work,1",
            author = author, authorRole = null, authorUrl = null, year = year, language = null, summary = null, isbn = isbn,
            coverUrls = emptyList(),
            formats = listOf(
                RecordFormat(
                    "BK", "Book",
                    "https://canning.spydus.com/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/78563304/0/V/0/R/9/ALLWRKENQ?SVL=$work&RECFMT=BK",
                    "$base\\BK",
                ),
            ),
        )
    }

    /** `record(<BK availabilityUrl>)` for The Midnight Library (BRN 275039). */
    val midnightLibrary = RecordDetail(
        bibIrn = "8753080", brn = "275039", title = "The Midnight Library", seriesTitle = null, isbn = "9781786892720",
        coverUrls = emptyList(), summary = "Somewhere out beyond the edge of the universe there is a library…",
        details = listOf(
            row("Main title", "The Midnight Library / Matt Haig."),
            row("Author", "Haig, Matt, 1975-, author"),
            DetailRow(
                "Work",
                listOf(DetailValue("The Midnight Library", "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/WRKENQ/78563101?SETLVL=&IRN=15642035")),
            ),
            row("Imprint", "Edinburgh : Canongate, 2020.", "©2020"),
            row("Collation", "288 pages ; 22 cm"),
            row("Notes", "\"One library. Infinite lives.\" --Page [1] of cover."),
            row("ISBN", "9781786892720 (paperback)", "1786892723", "1786892707 (hardcover)", "9781786892706 (hardcover)"),
            row("Dewey class", "823.92", "F"),
            row("Language", "English"),
            row(
                "Subject", "Libraries -- Fiction", "Choice (Psychology) -- Fiction", "Decision making -- Fiction",
                "Life change events -- Fiction", "Speculative fiction", "Science fiction", "Fantasy fiction",
            ),
            row("BRN", "275039"),
            row("Bookmark link", "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&BRN=275039"),
        ),
        bookmarkUrl = "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&BRN=275039",
        related = RecordRelated("https://canning.spydus.com/similar", "https://canning.spydus.com/author", null, "https://canning.spydus.com/versions"),
    )

    /** `record(<RelatedItem.bibUrl>)` for The long night (BRN 467086), opened from "Similar books". */
    val longNight = RecordDetail(
        bibIrn = null, brn = "467086", title = "The long night", seriesTitle = null, isbn = "9781399197809", coverUrls = emptyList(),
        summary = null,
        details = listOf(
            row("Main title", "The long night / Christian White."),
            row("Author", "White, Christian, author"),
            DetailRow(
                "Work",
                listOf(DetailValue("The Long Night", "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/WRKENQ/78563304?SETLVL=&IRN=15975964")),
            ),
            row("ISBN", "9781399197809"),
        ),
        bookmarkUrl = "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&BRN=467086",
        related = RecordRelated(null, null, null, null),
    )

    /** `search("The long night White")` page 1, top results. */
    val longNightSearch = listOf(
        search(work = "15975964", title = "The Long Night", author = "White, Christian", isbn = "9781004225712"),
        search(work = "15829282", title = "Betty White in Person", author = "White, Betty, 1922-", isbn = "9781620710432"),
        search(work = "15653031", title = "White Nights", author = "Cleeves, Ann, 1954-", isbn = "9781035012701"),
    )
}
