import Foundation
import LibreSpydusCore

// Pure rules behind the book detail screen, kept apart from the views so they're unit-tested
// (DetailLogicTests).

// MARK: - Which URL loads the record

enum DetailRecordSource {
    /// URLs to try with `record(url:)`, best first.
    ///
    /// A search record's `detailUrl` (`…/FULL/OPAC/ALLWRKENQ/…`) loads the OPAC's work-level page:
    /// a summary but no subjects, imprint, ISBNs, similar titles or other editions. A format's
    /// `availabilityUrl` loads the full bib page (the core supports it), so it comes first for
    /// work-level refs. Bib URLs (loans, related items, saved bookmarks) are tried first as they are,
    /// except a loan's or reservation's own set link saved before (`bibIrn` given): its
    /// session-free form goes first.
    static func candidateURLs(refURL: String?, bibIrn: String? = nil, format: FormatRef?) -> [String] {
        let formatURL = format?.availabilityUrl.nonEmpty
        let refURL = refURL.nonEmpty
        let ordered = isWorkLevel(refURL)
            ? [formatURL, refURL]
            : [sessionFreeURL(refURL, bibIrn: bibIrn), refURL, formatURL]
        var seen = Set<String>()
        return ordered.compactMap { $0 }.filter { seen.insert($0).inserted }
    }

    /// For a loan's or reservation's own link (`…/BIBENQ/<set>?…QRY=IRN(…)`, which needs the
    /// patron's OPAC session), the link that loads the same bib without one (`patronItemRecordUrl`).
    static func sessionFreeURL(_ url: String?, bibIrn: String?) -> String? {
        guard let url = url.nonEmpty, let bibIrn = bibIrn.nonEmpty,
              url.range(of: #"/BIBENQ/[^/?]+"#, options: [.regularExpression, .caseInsensitive]) != nil else { return nil }
        let item = PatronItem(irn: nil, bibIrn: bibIrn, title: nil, bibUrl: url, author: nil, year: nil, coverUrls: [],
                              isbn: nil, fields: [:], actions: [])
        return patronItemRecordUrl(item: item).flatMap { $0 == url ? nil : $0 }
    }

    /// `…/ALLWRKENQ/…`, `…/WRKENQ?…`: a work (grouped editions), not one bib record.
    static func isWorkLevel(_ url: String?) -> Bool {
        url?.range(of: "WRKENQ", options: .caseInsensitive) != nil
    }
}

extension RecordDetail {
    /// No title, summary or details: what the OPAC sends for a page it won't show (a session-set
    /// link without the session). Loading it counts as a failure.
    var isEmptyRecord: Bool {
        title.nonEmpty == nil && summary.nonEmpty == nil && details.isEmpty
    }
}

// MARK: - Display

enum DetailText {
    /// A title for display: catalogue notes in brackets dropped
    /// ("The Midnight Library [electronic resource] : A Novel" → "The Midnight Library : A Novel").
    static func displayTitle(_ title: String) -> String {
        let cleaned = title.replacingOccurrences(of: #"\s*\[[^\]]*\]"#, with: "", options: .regularExpression)
            .trimmingCharacters(in: .whitespacesAndNewlines)
        return cleaned.isEmpty ? title : cleaned
    }
}

// MARK: - Formats

enum FormatSelection {
    /// The format shown first: the first physical one, else the first.
    static func defaultIndex(_ formats: [FormatRef]) -> Int? {
        guard !formats.isEmpty else { return nil }
        return formats.firstIndex { !$0.isDigital } ?? 0
    }
}

/// A format's availability as the detail page keeps it: the counts' summary, and which edition
/// (bib record) the format is.
struct FormatAvailability: Equatable, Sendable {
    var summary: AvailabilitySummary
    /// `Availability.bibRecordIrn`: the edition the patron may have on loan or reserved.
    var bibRecordIrn: String?

    init(summary: AvailabilitySummary, bibRecordIrn: String?) {
        self.summary = summary
        self.bibRecordIrn = bibRecordIrn
    }

    init(_ availability: Availability) {
        self.init(summary: availabilitySummary(availability: availability), bibRecordIrn: availability.bibRecordIrn.nonEmpty)
    }
}

// MARK: - Already yours

enum HoldingIrns {
    /// The bib IRNs to look up in the account (`accountHolding`) for the edition on screen: the
    /// selected format's own (`Availability.bibRecordIrn`) once known; else the record's and the
    /// opened loan's or reservation's — minus any known to be another format's, so a format whose
    /// availability is still loading isn't taken for its neighbour.
    static func forPage(selectedFormatIrn: String?, recordIrn: String?, openedIrn: String?,
                        otherFormatIrns: Set<String> = []) -> [String] {
        if let irn = selectedFormatIrn.nonEmpty { return [irn] }
        var seen = Set<String>()
        return [recordIrn.nonEmpty, openedIrn.nonEmpty].compactMap { $0 }
            .filter { !otherFormatIrns.contains($0) && seen.insert($0).inserted }
    }
}

/// The availability line under the byline for the selected format.
struct AvailabilityLine: Equatable, Sendable {
    enum Tone: Equatable, Sendable { case available, normal, digital, muted }

    var text: String
    var tone: Tone

    /// `availabilitySummary(...).label`; a digital format without counts reads "Available online".
    init(summary: AvailabilitySummary?, format: FormatRef) {
        let hasCounts = summary.map { $0.available != nil || $0.copies != nil || $0.reserved != nil } ?? false
        if format.isDigital, !hasCounts {
            self.init(text: "Available online", tone: .digital)
        } else if let summary {
            let tone: Tone = if let available = summary.available, available > 0 {
                .available
            } else if format.isDigital {
                .digital
            } else {
                .normal
            }
            self.init(text: summary.label, tone: tone)
        } else {
            self.init(text: "Ask the library about availability", tone: .muted)
        }
    }

    init(text: String, tone: Tone) {
        self.text = text
        self.tone = tone
    }
}

// MARK: - Subjects

/// A tappable subject chip: `label` on the chip, `query` for `Route.results`.
struct SubjectChip: Hashable, Sendable {
    var label: String
    var query: String
}

enum DetailSubjects {
    static let limit = 6

    /// Chips from rows labelled like Subject(s), Genre/Form, Index terms and Audience, in that
    /// priority order (genres first), deduplicated, at most `limit`.
    static func extract(from details: [DetailRow], limit: Int = limit) -> [SubjectChip] {
        let ranked = details.compactMap { row in rank(row.label).map { (rank: $0, row: row) } }
            .enumerated()
            .sorted { ($0.element.rank, $0.offset) < ($1.element.rank, $1.offset) }
            .map(\.element)
        var seen = Set<String>()
        var chips: [SubjectChip] = []
        for (rank, row) in ranked {
            for value in row.values {
                guard let chip = chip(value.text, isAudience: rank == 2) else { continue }
                guard seen.insert(chip.label.lowercased()).inserted else { continue }
                chips.append(chip)
                if chips.count == limit { return chips }
            }
        }
        return chips
    }

    /// Whether a details row becomes chips (and so is left out of the details list).
    static func isSubjectRow(_ label: String) -> Bool {
        rank(label) != nil
    }

    /// 0 genre/form, 1 subject/topic/index terms, 2 audience; nil for any other row.
    private static func rank(_ label: String) -> Int? {
        let label = label.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        if label.contains("genre") || label == "form" { return 0 }
        if label.hasPrefix("subject") || label.hasPrefix("topic") || label.hasPrefix("index term") { return 1 }
        if label.hasPrefix("audience") || label.hasSuffix("audience") { return 2 }
        return nil
    }

    /// "Libraries -- Fiction." → "Libraries · Fiction" / "Libraries Fiction";
    /// "FICTION|Literary|" (BISAC, Brazoria County) → "Fiction · Literary" (short acronyms such as
    /// "DVD" stay as they are);
    /// "Interest age level: For ages 6-11 years." → "For ages 6-11 years".
    private static func chip(_ raw: String, isAudience: Bool) -> SubjectChip? {
        var text = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        if isAudience, let colon = text.lastIndex(of: ":") {
            text = String(text[text.index(after: colon)...])
        }
        let parts = text.replacingOccurrences(of: "|", with: "--").components(separatedBy: "--")
            .map { $0.trimmingCharacters(in: CharacterSet.whitespacesAndNewlines.union(CharacterSet(charactersIn: ".;,"))) }
            .filter { !$0.isEmpty }
            .map { part in part.count > 3 && part == part.uppercased() && part != part.lowercased() ? part.capitalized : part }
        guard !parts.isEmpty else { return nil }
        let label = parts.joined(separator: " · ")
        let query = parts.joined(separator: " ")
            .unicodeScalars.map { CharacterSet.alphanumerics.contains($0) || $0 == "-" || $0 == "'" ? Character($0) : " " }
            .reduce(into: "") { $0.append($1) }
            .split(separator: " ").joined(separator: " ")
        guard !query.isEmpty else { return nil }
        return SubjectChip(label: label, query: query)
    }
}

// MARK: - Details list

/// One line of the compact details list.
struct DetailFact: Hashable, Sendable {
    var label: String
    var value: String
}

enum DetailFacts {
    static let maxRows = 8
    static let maxValues = 3

    /// The library's details rows minus what the screen already shows (title, byline, subject
    /// chips) and navigation-only rows (work link, bookmark, BRN, "Go to"). A single-author row is
    /// the byline; several authors (illustrator, translator…) are listed.
    static func rows(from details: [DetailRow], maxRows: Int = maxRows) -> [DetailFact] {
        var facts: [DetailFact] = []
        for row in details {
            let key = row.label.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
            if hidden.contains(key) || DetailSubjects.isSubjectRow(row.label) { continue }
            let values = row.values.map { $0.text.trimmingCharacters(in: .whitespacesAndNewlines) }.filter { !$0.isEmpty }
            if values.isEmpty { continue }
            if bylineLabels.contains(key), values.count < 2 { continue }
            var shown = Array(values.prefix(maxValues))
            if values.count > maxValues { shown[maxValues - 1] += " …" }
            facts.append(DetailFact(label: row.label.trimmingCharacters(in: .whitespacesAndNewlines),
                                    value: shown.joined(separator: "\n")))
            if facts.count == maxRows { break }
        }
        return facts
    }

    private static let hidden: Set<String> = [
        "main title", "title", "work", "bookmark link", "bookmark", "brn", "go to", "permalink",
    ]
    private static let bylineLabels: Set<String> = ["author", "creator", "authors", "creators"]
}

// MARK: - Related panels

enum RelatedPanel: String, CaseIterable, Hashable, Sendable {
    case similar
    case author
    case series
    case versions

    var title: String {
        switch self {
        case .similar: "Similar books"
        case .author: "More by this author"
        case .series: "In this series"
        case .versions: "Other editions"
        }
    }

    func url(in related: RecordRelated) -> String? {
        switch self {
        case .similar: related.similar.nonEmpty
        case .author: related.author.nonEmpty
        case .series: related.series.nonEmpty
        case .versions: related.versions.nonEmpty
        }
    }

    /// Panels to draw, in order: the record links to them and they're loading or have items.
    /// Empty and failed panels are hidden.
    static func visible(in related: RecordRelated, states: [RelatedPanel: Loadable<[BookRef]>]) -> [RelatedPanel] {
        allCases.filter { panel in
            guard panel.url(in: related) != nil else { return false }
            switch states[panel] ?? .idle {
            case .idle, .loading: return true
            case .loaded(let items): return !items.isEmpty
            case .failed: return false
            }
        }
    }

    /// A panel's books as refs, without the book on screen (matched by BRN, then URL).
    static func items(_ list: RelatedList, excludingBrn brn: String?) -> [BookRef] {
        let brn = brn.nonEmpty
        var seen = Set<String>()
        return list.items
            .filter { item in brn == nil || item.brn.nonEmpty != brn }
            .map(BookRef.init)
            .filter { seen.insert($0.id).inserted }
    }
}

// MARK: - Re-finding formats

/// A ref opened without formats (loan, favourite, related item) re-finds them with a title +
/// author search after its record loads, and accepts a result only on a confident match.
enum FormatRefinder {
    /// What to search for. ISBNs aren't in the OPAC keyword index (an ISBN search came back
    /// "no records" on Canning, Hutt, Cumberland and Brazoria County), so it's the main title plus
    /// the author's surname, falling back to the ISBN only when there is no title.
    static func searchQuery(ref: BookRef, record: RecordDetail?) -> String? {
        let title = mainTitle(displayTitle(ref: ref, record: record))
        let surname = self.surname(ref.author ?? author(in: record))
        if let title = Optional(title).nonEmpty {
            return [title, surname].compactMap { $0 }.joined(separator: " ")
        }
        return isbns(ref: ref, record: record).first
    }

    /// The search result that is this book, or nil when no result is a confident match: same
    /// work IRN, else a shared ISBN, else the only result with the same title and author.
    static func match(ref: BookRef, record: RecordDetail?, in candidates: [SearchRecord]) -> SearchRecord? {
        let usable = candidates.filter { !$0.formats.isEmpty }
        let works = workIrns(ref: ref, record: record)
        if !works.isEmpty, let hit = usable.first(where: { $0.workIrn.nonEmpty.map(works.contains) ?? false }) {
            return hit
        }
        let isbns = Set(isbns(ref: ref, record: record))
        if !isbns.isEmpty, let hit = usable.first(where: { !Set(normalizedISBNs([$0.isbn ?? ""])).isDisjoint(with: isbns) }) {
            return hit
        }
        let title = normalizedTitle(displayTitle(ref: ref, record: record))
        guard !title.isEmpty else { return nil }
        let surname = self.surname(ref.author ?? author(in: record))
        let sameTitle = usable.filter { candidate in
            guard normalizedTitle(candidate.title ?? "") == title else { return false }
            guard let surname, let other = self.surname(candidate.author) else { return true }
            return surname == other
        }
        if sameTitle.count == 1 { return sameTitle[0] }
        if let year = ref.year.nonEmpty {
            let sameYear = sameTitle.filter { $0.year.nonEmpty == year }
            if sameYear.count == 1 { return sameYear[0] }
        }
        return nil
    }

    // MARK: Identity

    /// Work IRNs this book is known by: a `work:` ref id, the record's "Work" row link
    /// (`…/WRKENQ/…?IRN=<work>`), or a work-level record's own IRN.
    static func workIrns(ref: BookRef, record: RecordDetail?) -> Set<String> {
        var irns = Set<String>()
        if ref.id.hasPrefix("work:") { irns.insert(String(ref.id.dropFirst(5))) }
        guard let record else { return irns }
        for row in record.details where row.label.trimmingCharacters(in: .whitespaces).lowercased() == "work" {
            for value in row.values {
                if let irn = queryItem("IRN", in: value.url) { irns.insert(irn) }
            }
        }
        if DetailRecordSource.isWorkLevel(record.bookmarkUrl) {
            if let irn = queryItem("IRN", in: record.bookmarkUrl) { irns.insert(irn) }
            if let irn = record.bibIrn.nonEmpty { irns.insert(irn) }
        }
        return irns
    }

    /// Every ISBN the ref and record mention, as ISBN-13.
    static func isbns(ref: BookRef, record: RecordDetail?) -> [String] {
        var texts = [ref.isbn ?? "", record?.isbn ?? ""]
        for row in record?.details ?? [] where row.label.lowercased().hasPrefix("isbn") {
            texts += row.values.map(\.text)
        }
        return normalizedISBNs(texts)
    }

    /// ISBN-10s and ISBN-13s found in `texts` ("9781786892720 (paperback)", "1-78689-272-3"),
    /// as ISBN-13s, in order, without duplicates.
    static func normalizedISBNs(_ texts: [String]) -> [String] {
        var result: [String] = []
        for text in texts {
            for token in text.split(whereSeparator: { $0.isWhitespace || $0 == "(" || $0 == ")" || $0 == ";" || $0 == "," }) {
                let compact = token.filter { $0 != "-" }.uppercased()
                guard let isbn13 = isbn13(compact), !result.contains(isbn13) else { continue }
                result.append(isbn13)
            }
        }
        return result
    }

    private static func isbn13(_ text: String) -> String? {
        let chars = Array(text)
        if chars.count == 13, chars.allSatisfy(\.isNumber) { return text }
        guard chars.count == 10, chars.prefix(9).allSatisfy(\.isNumber),
              chars[9].isNumber || chars[9] == "X" else { return nil }
        let body = "978" + String(chars.prefix(9))
        let sum = body.enumerated().reduce(0) { total, pair in
            total + (pair.element.wholeNumberValue ?? 0) * (pair.offset.isMultiple(of: 2) ? 1 : 3)
        }
        return body + String((10 - sum % 10) % 10)
    }

    // MARK: Titles and authors

    /// "The Midnight Library [electronic resource] : a novel / Matt Haig." → "midnight library".
    static func normalizedTitle(_ title: String) -> String {
        let folded = mainTitle(title).folding(options: [.caseInsensitive, .diacriticInsensitive], locale: nil)
        let words = folded.unicodeScalars
            .map { CharacterSet.alphanumerics.contains($0) ? Character($0) : " " }
            .reduce(into: "") { $0.append($1) }
            .split(separator: " ")
            .map(String.init)
        let trimmed = ["the", "a", "an"].contains(words.first ?? "") && words.count > 1 ? Array(words.dropFirst()) : words
        return trimmed.joined(separator: " ")
    }

    /// The title without bracketed notes, subtitle or statement of responsibility.
    static func mainTitle(_ title: String) -> String {
        var text = title.replacingOccurrences(of: #"\[[^\]]*\]"#, with: "", options: .regularExpression)
        for separator in [" / ", " : ", ": "] {
            if let range = text.range(of: separator) { text = String(text[..<range.lowerBound]) }
        }
        return text.trimmingCharacters(in: CharacterSet.whitespacesAndNewlines.union(CharacterSet(charactersIn: ".:/;,")))
    }

    /// "Haig, Matt, 1975-" → "haig"; "Matt Haig" → "haig".
    static func surname(_ author: String?) -> String? {
        guard let author = author.nonEmpty else { return nil }
        let name = author.contains(",") ? String(author.split(separator: ",")[0]) : String(author.split(separator: " ").last ?? "")
        let folded = name.folding(options: [.caseInsensitive, .diacriticInsensitive], locale: nil)
            .trimmingCharacters(in: CharacterSet.letters.inverted)
        return folded.isEmpty ? nil : folded
    }

    private static func displayTitle(ref: BookRef, record: RecordDetail?) -> String {
        if ref.title != "Untitled", let title = Optional(ref.title).nonEmpty { return title }
        return record?.title ?? ""
    }

    private static func author(in record: RecordDetail?) -> String? {
        let row = record?.details.first { ["author", "creator"].contains($0.label.lowercased()) }
        return row?.values.first?.text
    }

    /// A query parameter's value. OPAC URLs often hold characters `URLComponents` rejects
    /// (`\`, `<`), so this matches the text directly.
    private static func queryItem(_ name: String, in url: String?) -> String? {
        guard let url, let match = url.firstMatch(of: try! Regex("[?&]\(name)=([^&#]+)").ignoresCase()),
              let value = match.output[1].substring, !value.isEmpty else { return nil }
        return String(value)
    }
}
