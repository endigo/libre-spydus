import Foundation
import LibreSpydusCore
import Testing
@testable import LibreSpydus

/// Book detail rules (Features/Detail/DetailLogic.swift), on shapes captured from live public
/// Canning / Hutt / Brazoria County pages.
struct DetailLogicTests {
    // MARK: Subjects

    @Test func subjectChipsComeFromSubjectRowsCappedAtSix() {
        let chips = DetailSubjects.extract(from: DetailFixtures.midnightLibrary.details)
        #expect(chips.count == 6)
        #expect(chips.first == SubjectChip(label: "Libraries · Fiction", query: "Libraries Fiction"))
        #expect(chips[1] == SubjectChip(label: "Choice (Psychology) · Fiction", query: "Choice Psychology Fiction"))
        #expect(chips.map(\.label).last == "Science fiction")
        #expect(!chips.map(\.label).contains("Fantasy fiction"))
    }

    @Test func genresComeFirstAndAudienceDropsItsPrefix() {
        let details = [
            DetailFixtures.row("Subject", "Dragons -- Juvenile fiction.", "Magic -- Juvenile fiction."),
            DetailFixtures.row("Audience", "Interest age level: For ages 6-11 years."),
            DetailFixtures.row("Genre/Form", "Fantasy fiction.", "Graphic novels."),
            DetailFixtures.row("Language", "English"),
        ]
        let chips = DetailSubjects.extract(from: details)
        #expect(chips.map(\.label) == [
            "Fantasy fiction", "Graphic novels", "Dragons · Juvenile fiction", "Magic · Juvenile fiction",
            "For ages 6-11 years",
        ])
        #expect(chips[4].query == "For ages 6-11 years")
    }

    @Test func subjectChipsAreDeduplicatedAndIndexTermsCount() {
        let details = [
            DetailFixtures.row("Index terms", "Graphic novels", "Libraries"),
            DetailFixtures.row("Subjects", "graphic novels.", "  ", "Time travel"),
        ]
        #expect(DetailSubjects.extract(from: details).map(\.label) == ["Graphic novels", "Libraries", "Time travel"])
        #expect(DetailSubjects.extract(from: [DetailFixtures.row("Imprint", "London : Bloomsbury, 2023.")]).isEmpty)
    }

    @Test func bisacSubjectsAreSplitAndCased() {
        // Brazoria County (US) writes subjects BISAC-style.
        let chips = DetailSubjects.extract(from: [
            DetailFixtures.row("Subject", "FICTION|Science Fiction|Time Travel|", "Electronic books", "DVD"),
        ])
        #expect(chips.map(\.label) == ["Fiction · Science Fiction · Time Travel", "Electronic books", "DVD"])
        #expect(chips[0].query == "Fiction Science Fiction Time Travel")
    }

    // MARK: Details list

    @Test func factsSkipWhatTheScreenAlreadyShows() {
        let facts = DetailFacts.rows(from: DetailFixtures.midnightLibrary.details)
        #expect(facts.map(\.label) == ["Imprint", "Collation", "Notes", "ISBN", "Dewey class", "Language"])
        #expect(facts[0].value == "Edinburgh : Canongate, 2020.\n©2020")
        #expect(facts[3].value == "9781786892720 (paperback)\n1786892723\n1786892707 (hardcover) …")
    }

    @Test func factsListSeveralAuthorsButNotTheBylineAlone() {
        let details = [
            DetailFixtures.row("Author", "Rowling, J. K., author", "Goes, Peter, illustrator"),
            DetailFixtures.row("Creator", "Haig, Matt, 1975-, author"),
            DetailFixtures.row("Go to", "Instances of this work (6)"),
        ]
        #expect(DetailFacts.rows(from: details) == [
            DetailFact(label: "Author", value: "Rowling, J. K., author\nGoes, Peter, illustrator"),
        ])
    }

    // MARK: Formats

    @Test func defaultFormatIsTheFirstPhysicalOne() {
        let ebook = FormatRef(code: "EBK", label: "eBook")
        let eaudio = FormatRef(code: "EAUD", label: "eAudioBook", reserveUrl: "https://mybcls.spydus.com/r")
        let book = FormatRef(code: "BK", label: "Book")
        let largePrint = FormatRef(code: "LP", label: "Large Print")
        #expect(FormatSelection.defaultIndex([ebook, book, largePrint]) == 1)
        #expect(FormatSelection.defaultIndex([book, ebook]) == 0)
        #expect(FormatSelection.defaultIndex([ebook, eaudio]) == 0)
        #expect(FormatSelection.defaultIndex([]) == nil)
    }

    @Test func digitalFormatsWithoutCountsReadAvailableOnline() {
        let ebook = FormatRef(code: "EBK", label: "eBook")
        let noCounts = availabilitySummary(availability: DetailFixtures.availability([:]))
        #expect(AvailabilityLine(summary: noCounts, format: ebook) == AvailabilityLine(text: "Available online", tone: .digital))
        #expect(AvailabilityLine(summary: nil, format: ebook).text == "Available online")

        // Brazoria County gives e-copies counts (and takes holds on them).
        let eaudio = FormatRef(code: "EAUD", label: "eAudioBook", reserveUrl: "https://mybcls.spydus.com/r")
        let counted = availabilitySummary(availability: DetailFixtures.availability(["available": 0, "copies": 3]))
        #expect(AvailabilityLine(summary: counted, format: eaudio) == AvailabilityLine(text: "0 of 3 available", tone: .digital))
    }

    @Test func physicalAvailabilityUsesTheCoreLabel() {
        let book = FormatRef(code: "BK", label: "Book")
        let canning = availabilitySummary(availability: DetailFixtures.availability(["copies": 2]))
        #expect(AvailabilityLine(summary: canning, format: book) == AvailabilityLine(text: "2 copies", tone: .normal))
        let onShelf = availabilitySummary(availability: DetailFixtures.availability(["available": 2, "copies": 3]))
        #expect(AvailabilityLine(summary: onShelf, format: book) == AvailabilityLine(text: "2 of 3 available", tone: .available))
    }

    // MARK: Record source

    @Test func workLevelRefsLoadTheBibRecordThroughTheFormat() {
        let format = TestData.searchRecord.formats.map(FormatRef.init)[0]
        let work = TestData.searchRecord.detailUrl
        #expect(DetailRecordSource.candidateURLs(refURL: work, format: format) == [format.availabilityUrl!, work!])

        let bookmark = "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&BRN=275039"
        #expect(DetailRecordSource.candidateURLs(refURL: bookmark, format: format) == [bookmark, format.availabilityUrl!])
        #expect(DetailRecordSource.candidateURLs(refURL: bookmark, format: nil) == [bookmark])
        #expect(DetailRecordSource.candidateURLs(refURL: nil, format: nil).isEmpty)
        #expect(DetailRecordSource.isWorkLevel("https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/WRKENQ?SETLVL=&IRN=15642035"))
    }

    // MARK: Related panels

    @Test func relatedPanelsHideEmptyFailedAndUnlinkedOnes() {
        let related = RecordRelated(similar: "https://x/similar", author: "https://x/author", series: nil,
                                    versions: "https://x/versions")
        let book = BookRef(id: "brn:1", title: "A")
        let failure = UserFacingError(kind: .network, message: "offline")

        #expect(RelatedPanel.visible(in: related, states: [:]) == [.similar, .author, .versions])
        #expect(RelatedPanel.visible(in: related, states: [.similar: .loading, .author: .loaded([book]),
                                                           .versions: .loaded([])]) == [.similar, .author])
        #expect(RelatedPanel.visible(in: related, states: [.similar: .failed(failure), .author: .loaded([book]),
                                                           .series: .loaded([book]), .versions: .loaded([book])])
            == [.author, .versions])
        #expect(RelatedPanel.visible(in: RecordRelated(similar: " ", author: nil, series: nil, versions: nil), states: [:]).isEmpty)
    }

    @Test func relatedItemsLeaveOutTheBookOnScreen() {
        let list = RelatedList(items: [
            DetailFixtures.related(brn: "275039", title: "The Midnight Library"),
            DetailFixtures.related(brn: "304411", title: "The comfort book"),
            DetailFixtures.related(brn: "304411", title: "The comfort book"),
            DetailFixtures.related(brn: "174629", title: "Evie and the animals"),
        ])
        let refs = RelatedPanel.items(list, excludingBrn: "275039")
        #expect(refs.map(\.id) == ["brn:304411", "brn:174629"])
        #expect(RelatedPanel.items(list, excludingBrn: nil).count == 3)
    }

    // MARK: Re-finding formats

    @Test func refindMatchesTheWorkIrnFromTheRecordsWorkRow() {
        // A "Similar books" item: its ISBN differs from the search result's, the work IRN doesn't.
        let ref = BookRef(DetailFixtures.related(brn: "467086", title: "The long night", author: "White, Christian",
                                                 isbn: "9781399197809"))
        let candidates = DetailFixtures.longNightSearch
        #expect(FormatRefinder.searchQuery(ref: ref, record: DetailFixtures.longNight) == "The long night white")
        #expect(FormatRefinder.match(ref: ref, record: DetailFixtures.longNight, in: candidates)?.workIrn == "15975964")
    }

    @Test func refindMatchesAnIsbnInEitherForm() {
        #expect(FormatRefinder.normalizedISBNs(["1786892707 (hardcover)", "978-1-78689-270-6", "n/a"]) == ["9781786892706"])
        #expect(FormatRefinder.normalizedISBNs(["080442957X"]) == ["9780804429573"])

        let loan = BookRef(id: "bib:8753080", title: "The midnight library", author: "Haig, Matt, author",
                           isbn: "1786892707")
        let candidate = DetailFixtures.search(work: "1", title: "Midnight library (different cataloguing)",
                                              author: nil, isbn: "9781786892706")
        #expect(FormatRefinder.match(ref: loan, record: nil, in: [candidate])?.workIrn == "1")
    }

    @Test func refindFallsBackToAUniqueTitleAndAuthor() {
        let ref = BookRef(id: "bib:1", title: "The Midnight Library [electronic resource] : a novel / Matt Haig.",
                          author: "Haig, Matt", year: "2020")
        let haig = DetailFixtures.search(work: "10", title: "The midnight library", author: "Haig, Matt, 1975-")
        let other = DetailFixtures.search(work: "11", title: "Midnight at the library", author: "Dubosarsky, Ursula")
        let namesake = DetailFixtures.search(work: "12", title: "The Midnight Library", author: "Kohara, Kazuno")
        #expect(FormatRefinder.match(ref: ref, record: nil, in: [other, namesake, haig])?.workIrn == "10")

        // Two same-title, same-author works: only the year can tell them apart.
        let reissue = DetailFixtures.search(work: "13", title: "The Midnight Library", author: "Haig, Matt", year: "2023")
        let original = DetailFixtures.search(work: "10", title: "The midnight library", author: "Haig, Matt", year: "2020")
        #expect(FormatRefinder.match(ref: ref, record: nil, in: [reissue, original])?.workIrn == "10")
        var undated = ref
        undated.year = nil
        #expect(FormatRefinder.match(ref: undated, record: nil, in: [reissue, original]) == nil)
    }

    @Test func refindRejectsUnconfidentMatches() {
        let ref = BookRef(id: "bib:1", title: "Hollow bones", author: "Mayne, Andrew")
        let near = DetailFixtures.search(work: "20", title: "Hollow bones and other stories", author: "Mayne, Andrew")
        let sameTitleOtherAuthor = DetailFixtures.search(work: "21", title: "Hollow Bones", author: "Smith, Jo")
        #expect(FormatRefinder.match(ref: ref, record: nil, in: [near, sameTitleOtherAuthor]) == nil)

        // A matching result without formats can't be used either.
        var formatless = DetailFixtures.search(work: "22", title: "Hollow bones", author: "Mayne, Andrew")
        formatless.formats = []
        #expect(FormatRefinder.match(ref: ref, record: nil, in: [formatless]) == nil)
    }

    @Test func refindQueryFallsBackToRecordTitleThenIsbn() {
        let untitled = BookRef(id: "brn:1", title: "Untitled")
        #expect(FormatRefinder.searchQuery(ref: untitled, record: DetailFixtures.midnightLibrary) == "The Midnight Library haig")
        let isbnOnly = BookRef(id: "url:x", title: "", isbn: "1786892707")
        #expect(FormatRefinder.searchQuery(ref: isbnOnly, record: nil) == "9781786892706")
        #expect(FormatRefinder.normalizedTitle("The Midnight Library / Matt Haig.") == "midnight library")
        #expect(FormatRefinder.surname("Haig, Matt, 1975-") == "haig")
        #expect(FormatRefinder.surname("Matt Haig") == "haig")
    }
}

/// Record and search shapes from live public pages (Canning, 2026-10-12).
enum DetailFixtures {
    static func row(_ label: String, _ values: String...) -> DetailRow {
        DetailRow(label: label, values: values.map { DetailValue(text: $0, url: nil) })
    }

    static func availability(_ counts: [String: Int64]) -> Availability {
        Availability(bibRecordIrn: "1", title: nil, publisher: nil, counts: counts, reserved: counts["reserved"],
                     copies: counts["copies"], holdingsUrl: nil, recordUrl: nil)
    }

    static func related(brn: String, title: String, author: String? = nil, isbn: String? = nil) -> RelatedItem {
        RelatedItem(title: title, bibUrl: "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/78563101?BRN=\(brn)&SETLVL=",
                    brn: brn, author: author, year: "2026", isbn: isbn, coverUrls: [])
    }

    static func search(work: String, title: String, author: String?, isbn: String? = nil, year: String? = nil) -> SearchRecord {
        let base = "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/78563304?SETLVL=SET&FMT=RECFMT&QRY=WRK01%5C\(work)"
        return SearchRecord(
            rank: 1, workIrn: work, title: title,
            detailUrl: "https://canning.spydus.com/cgi-bin/spydus.exe/FULL/OPAC/ALLWRKENQ/78563304/\(work),1",
            author: author, authorRole: nil, authorUrl: nil, year: year, language: nil, summary: nil, isbn: isbn,
            coverUrls: [],
            formats: [RecordFormat(code: "BK", label: "Book",
                                   reserveUrl: "https://canning.spydus.com/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/78563304/0/V/0/R/9/ALLWRKENQ?SVL=\(work)&RECFMT=BK",
                                   availabilityUrl: "\(base)\\BK")]
        )
    }

    /// `record(<BK availabilityUrl>)` for The Midnight Library (BRN 275039).
    static let midnightLibrary = RecordDetail(
        bibIrn: "8753080", brn: "275039", title: "The Midnight Library", seriesTitle: nil, isbn: "9781786892720",
        coverUrls: [], summary: "Somewhere out beyond the edge of the universe there is a library…",
        details: [
            row("Main title", "The Midnight Library / Matt Haig."),
            row("Author", "Haig, Matt, 1975-, author"),
            DetailRow(label: "Work", values: [DetailValue(
                text: "The Midnight Library",
                url: "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/WRKENQ/78563101?SETLVL=&IRN=15642035")]),
            row("Imprint", "Edinburgh : Canongate, 2020.", "©2020"),
            row("Collation", "288 pages ; 22 cm"),
            row("Notes", "\"One library. Infinite lives.\" --Page [1] of cover."),
            row("ISBN", "9781786892720 (paperback)", "1786892723", "1786892707 (hardcover)", "9781786892706 (hardcover)"),
            row("Dewey class", "823.92", "F"),
            row("Language", "English"),
            row("Subject", "Libraries -- Fiction", "Choice (Psychology) -- Fiction", "Decision making -- Fiction",
                "Life change events -- Fiction", "Speculative fiction", "Science fiction", "Fantasy fiction"),
            row("BRN", "275039"),
            row("Bookmark link", "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&BRN=275039"),
        ],
        bookmarkUrl: "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&BRN=275039",
        related: RecordRelated(similar: "https://canning.spydus.com/similar", author: "https://canning.spydus.com/author",
                               series: nil, versions: "https://canning.spydus.com/versions")
    )

    /// `record(<RelatedItem.bibUrl>)` for The long night (BRN 467086), opened from "Similar books".
    static let longNight = RecordDetail(
        bibIrn: nil, brn: "467086", title: "The long night", seriesTitle: nil, isbn: "9781399197809", coverUrls: [],
        summary: nil,
        details: [
            row("Main title", "The long night / Christian White."),
            row("Author", "White, Christian, author"),
            DetailRow(label: "Work", values: [DetailValue(
                text: "The Long Night",
                url: "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/WRKENQ/78563304?SETLVL=&IRN=15975964")]),
            row("ISBN", "9781399197809"),
        ],
        bookmarkUrl: "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&BRN=467086",
        related: RecordRelated(similar: nil, author: nil, series: nil, versions: nil)
    )

    /// `search("The long night White")` page 1, top results.
    static let longNightSearch = [
        search(work: "15975964", title: "The Long Night", author: "White, Christian", isbn: "9781004225712"),
        search(work: "15829282", title: "Betty White in Person", author: "White, Betty, 1922-", isbn: "9781620710432"),
        search(work: "15653031", title: "White Nights", author: "Cleeves, Ann, 1954-", isbn: "9781035012701"),
    ]
}
