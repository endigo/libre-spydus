import Foundation
import LibreSpydusCore

/// The public catalogue data of every demo loan and reservation (`Resources/DemoData/demo-catalogue.json`),
/// so their book pages work offline in demo mode. Captured from Canning's public pages (cookieless
/// GETs, 2026-10-12) and trimmed: one or two editions per work, record links other than "Work"
/// dropped, no related panels.
///
/// `DemoBackend` answers from it the way the OPAC does:
/// - `search` for a query that names a work (every word of its main title) → that work and its formats;
/// - `availability(<a format's availabilityUrl>)` → the counts and the edition's `bibRecordIrn`;
/// - `record` of the session-free `…/BIBENQ?SETLVL=&IRN=<irn>` (`patronItemRecordUrl`), the
///   edition's bookmark link or a format's availability URL → the edition's full record;
/// - `record` of a loan's or reservation's own `…/BIBENQ/<set>?…QRY=IRN(<irn>)` link → an empty
///   record: that set lives in the patron's OPAC session, and without it the OPAC answers "Session
///   must be logged in".
struct DemoCatalogue: Sendable {
    /// The search set every generated URL uses (the demo fixtures' own).
    static let setId = "78500000"

    struct Edition: Sendable {
        var format: RecordFormat
        var bibIrn: String
        var availability: Availability
        var record: RecordDetail
    }

    struct Work: Sendable {
        /// As a search result, with one format per edition.
        var record: SearchRecord
        var editions: [Edition]
        /// `FormatRefinder.normalizedTitle` words: a query holding all of them finds this work.
        var titleWords: Set<String>
    }

    private(set) var works: [Work]
    private var recordsByURL: [String: RecordDetail] = [:]
    private var editionsByIrn: [String: Edition] = [:]
    private var editionsByAvailabilityURL: [String: Edition] = [:]

    init(works: [Work]) {
        self.works = works
        for edition in works.flatMap(\.editions) {
            editionsByIrn[edition.bibIrn] = edition
            if let url = edition.format.availabilityUrl {
                editionsByAvailabilityURL[url] = edition
                recordsByURL[url] = edition.record
            }
            if let bookmark = edition.record.bookmarkUrl { recordsByURL[bookmark] = edition.record }
        }
    }

    /// From the bundled JSON, with every generated URL on `origin`.
    init(_ dto: DemoCatalogueDTO, origin: String) {
        self.init(works: dto.works.map { work in
            let editions = work.formats.map { format in
                Edition(
                    format: RecordFormat(code: format.code, label: format.label,
                                         reserveUrl: Self.reserveURL(origin: origin, work: work.workIrn, code: format.code),
                                         availabilityUrl: Self.availabilityURL(origin: origin, work: work.workIrn, code: format.code)),
                    bibIrn: format.bibIrn,
                    availability: Availability(bibRecordIrn: format.bibIrn, title: format.record.title,
                                               publisher: format.publisher, counts: format.counts,
                                               reserved: format.counts["reserved"], copies: format.counts["copies"],
                                               holdingsUrl: nil, recordUrl: nil),
                    record: format.record.model
                )
            }
            let record = SearchRecord(
                rank: 1, workIrn: work.workIrn, title: work.title,
                detailUrl: "\(origin)/cgi-bin/spydus.exe/FULL/OPAC/ALLWRKENQ/\(Self.setId)/\(work.workIrn),1",
                author: work.author, authorRole: work.authorRole, authorUrl: nil, year: work.year,
                language: work.language, summary: nil, isbn: work.isbn, coverUrls: work.coverUrls,
                formats: editions.map(\.format)
            )
            return Work(record: record, editions: editions, titleWords: Self.words(FormatRefinder.normalizedTitle(work.title)))
        })
    }

    // MARK: Answers

    /// The works `query` names, as a results page; nil when it names none (ask the library).
    func search(query: String) -> SearchResults? {
        let words = Self.words(FormatRefinder.normalizedTitle(query))
        let hits = works.filter { !$0.titleWords.isEmpty && $0.titleWords.isSubset(of: words) }.map(\.record)
        guard !hits.isEmpty else { return nil }
        let ranked = hits.enumerated().map { index, record in
            var record = record
            record.rank = Int64(index + 1)
            return record
        }
        return SearchResults(query: query, setId: Self.setId, totalResults: Int64(ranked.count), showing: nil,
                             nextPageUrl: nil, prevPageUrl: nil, noResults: false, records: ranked)
    }

    /// A demo format's availability (an edition-level card's `…/BIBENQ?SETLVL=&IRN=<irn>&FMT=RECFMT`
    /// too, as My Picks lists one); nil for any other URL.
    func availability(url: String) -> Availability? {
        if let edition = editionsByAvailabilityURL[url] { return edition.availability }
        guard url.range(of: "FMT=RECFMT", options: .caseInsensitive) != nil, let irn = Self.sessionFreeIrn(url) else { return nil }
        return editionsByIrn[irn]?.availability
    }

    /// A demo edition's record, an empty record for a patron row's session-set link, nil otherwise.
    func record(url: String) -> RecordDetail? {
        if let record = recordsByURL[url] { return record }
        if Self.isSessionSetLink(url) { return Self.emptyRecord }
        if let irn = Self.sessionFreeIrn(url), let edition = editionsByIrn[irn] { return edition.record }
        return nil
    }

    /// The bib IRN a demo format's availability URL resolves to.
    func bibIrn(availabilityUrl: String) -> String? {
        editionsByAvailabilityURL[availabilityUrl]?.bibIrn
    }

    // MARK: URLs

    /// A loan's or reservation's own link, as the OPAC writes it: a set in the patron's session.
    static func sessionSetURL(origin: String, irn: String) -> String {
        "\(origin)/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/\(setId)?SETLVL=&QRY=IRN(\(irn))&NAVLVL=SET"
    }

    static func availabilityURL(origin: String, work: String, code: String) -> String {
        "\(origin)/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/\(setId)?SETLVL=SET&FMT=RECFMT&XSLT=XRecFmtDisplay.xsl&SETID=\(setId)&QRY=WRK01%5C\(work)\\\(code)"
    }

    static func reserveURL(origin: String, work: String, code: String) -> String {
        "\(origin)/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/\(setId)/0/V/0/R/9/ALLWRKENQ?SVL=\(work)&RECFMT=\(code)"
    }

    /// `…/BIBENQ/<set>?…QRY=IRN(<irn>)…`: a bib in a search set (the patron's, for account rows).
    static func isSessionSetLink(_ url: String) -> Bool {
        url.firstMatch(of: sessionSetPattern) != nil
    }

    /// The IRN of a set-less `…/BIBENQ?…IRN=<irn>` link (`patronItemRecordUrl`).
    static func sessionFreeIrn(_ url: String) -> String? {
        guard let match = url.firstMatch(of: sessionFreePattern), let irn = match.output[1].substring else { return nil }
        return String(irn)
    }

    /// What the core returns for a page without a record ("Session must be logged in").
    static let emptyRecord = RecordDetail(
        bibIrn: nil, brn: nil, title: nil, seriesTitle: nil, isbn: nil, coverUrls: [], summary: nil, details: [],
        bookmarkUrl: nil, related: RecordRelated(similar: nil, author: nil, series: nil, versions: nil)
    )

    nonisolated(unsafe) private static let sessionSetPattern = try! Regex(#"/BIBENQ/[^/?]+\?.*QRY=IRN\("#).ignoresCase()
    nonisolated(unsafe) private static let sessionFreePattern = try! Regex(#"/BIBENQ\?(?:.*&)?IRN=(\d+)"#).ignoresCase()

    private static func words(_ text: String) -> Set<String> {
        Set(text.split(separator: " ").map(String.init))
    }
}

// MARK: - JSON shapes

struct DemoCatalogueDTO: Decodable {
    struct Work: Decodable {
        var workIrn: String
        var title: String
        var author: String?
        var authorRole: String?
        var year: String?
        var language: String?
        var isbn: String?
        var coverUrls: [String]
        var formats: [Format]
    }

    struct Format: Decodable {
        var code: String
        var label: String?
        var bibIrn: String
        var publisher: String?
        var counts: [String: Int64]
        var record: RecordDetailDTO
    }

    var works: [Work]
}

struct RecordDetailDTO: Decodable {
    struct Row: Decodable {
        struct Value: Decodable {
            var text: String
            var url: String?
        }
        var label: String
        var values: [Value]
    }

    struct Related: Decodable {
        var similar: String?
        var author: String?
        var series: String?
        var versions: String?
    }

    var bibIrn: String?
    var brn: String?
    var title: String?
    var seriesTitle: String?
    var isbn: String?
    var coverUrls: [String]
    var summary: String?
    var details: [Row]
    var bookmarkUrl: String?
    var related: Related

    var model: RecordDetail {
        RecordDetail(
            bibIrn: bibIrn, brn: brn, title: title, seriesTitle: seriesTitle, isbn: isbn, coverUrls: coverUrls,
            summary: summary,
            details: details.map { row in
                DetailRow(label: row.label, values: row.values.map { DetailValue(text: $0.text, url: $0.url) })
            },
            bookmarkUrl: bookmarkUrl,
            related: RecordRelated(similar: related.similar, author: related.author, series: related.series,
                                   versions: related.versions)
        )
    }
}
