import Foundation
import LibreSpydusCore

/// A book the app can open, list or save as a favourite (docs/native-apps.md §5.3/§5.5):
/// enough to draw a cover card and to load the record again.
struct BookRef: Codable, Hashable, Identifiable, Sendable {
    /// Stable key: `bib:<IRN>`, `brn:<BRN>`, `work:<IRN>`, else `url:<url>` / `title:<title>`.
    var id: String
    var title: String
    /// As the library gives it ("Haig, Matt, author"); display with `Formatting.authorName`.
    var author: String?
    var coverUrl: String?
    /// Loads with `LibraryBackend.record(url:)`. Prefer `RecordDetail.bookmarkUrl` (stable)
    /// over search URLs, whose set id can expire.
    var url: String?
    var isbn: String?
    var year: String?
    /// Search records only; a ref opened from a loan/favourite may re-find them by ISBN/title.
    var formats: [FormatRef]
    /// Set by `FavouritesStore` when saved.
    var savedAt: Date?

    init(id: String, title: String, author: String? = nil, coverUrl: String? = nil, url: String? = nil,
         isbn: String? = nil, year: String? = nil, formats: [FormatRef] = [], savedAt: Date? = nil) {
        self.id = id
        self.title = title
        self.author = author
        self.coverUrl = coverUrl
        self.url = url
        self.isbn = isbn
        self.year = year
        self.formats = formats
        self.savedAt = savedAt
    }

    /// A search result row (`SearchRecord.detailUrl` loads with `record`).
    init(_ record: SearchRecord) {
        self.init(
            id: Self.key(("work", record.workIrn), url: record.detailUrl, title: record.title),
            title: Self.title(record.title),
            author: record.author.nonEmpty,
            coverUrl: record.coverUrls.first,
            url: record.detailUrl,
            isbn: record.isbn.nonEmpty,
            year: record.year.nonEmpty,
            formats: record.formats.map(FormatRef.init)
        )
    }

    /// A loan or reservation (`PatronItem.bibUrl` loads with `record`).
    init(_ item: PatronItem) {
        self.init(
            id: Self.key(("bib", item.bibIrn), url: item.bibUrl, title: item.title),
            title: Self.title(item.title),
            author: item.author.nonEmpty,
            coverUrl: item.coverUrls.first,
            url: item.bibUrl,
            isbn: item.isbn.nonEmpty,
            year: item.year.nonEmpty
        )
    }

    /// A "Similar books" / "More by this author" / "In this series" item.
    init(_ item: RelatedItem) {
        self.init(
            id: Self.key(("brn", item.brn), url: item.bibUrl, title: item.title),
            title: Self.title(item.title),
            author: item.author.nonEmpty,
            coverUrl: item.coverUrls.first,
            url: item.bibUrl,
            isbn: item.isbn.nonEmpty,
            year: item.year.nonEmpty
        )
    }

    /// Just a URL (the `-LSRoute detail:<url>` hook, deep links).
    init(url: String, title: String = "") {
        self.init(id: Self.key(url: url, title: title), title: title, url: url)
    }

    /// This ref updated from its loaded record: stable bookmark URL, better cover/ISBN/title.
    /// Keeps `id` so a favourite toggled before and after loading stays the same entry.
    func updated(with detail: RecordDetail) -> BookRef {
        var copy = self
        if let bookmark = detail.bookmarkUrl.nonEmpty { copy.url = bookmark }
        if copy.title.isEmpty, let title = detail.title.nonEmpty { copy.title = title }
        if copy.coverUrl == nil { copy.coverUrl = detail.coverUrls.first }
        if copy.isbn == nil { copy.isbn = detail.isbn.nonEmpty }
        return copy
    }

    private static func key(_ irn: (prefix: String, value: String?)? = nil, url: String?, title: String?) -> String {
        if let irn, let value = irn.value.nonEmpty { return "\(irn.prefix):\(value)" }
        if let url = url.nonEmpty { return "url:\(url)" }
        return "title:\(title.nonEmpty?.lowercased() ?? UUID().uuidString)"
    }

    private static func title(_ title: String?) -> String {
        title.nonEmpty ?? "Untitled"
    }
}

/// `RecordFormat` (code, label, reserveUrl, availabilityUrl) in a Codable form for favourites.
struct FormatRef: Codable, Hashable, Sendable {
    var code: String
    var label: String?
    var reserveUrl: String?
    var availabilityUrl: String?

    init(code: String, label: String? = nil, reserveUrl: String? = nil, availabilityUrl: String? = nil) {
        self.code = code
        self.label = label
        self.reserveUrl = reserveUrl
        self.availabilityUrl = availabilityUrl
    }

    init(_ format: RecordFormat) {
        self.init(code: format.code, label: format.label, reserveUrl: format.reserveUrl,
                  availabilityUrl: format.availabilityUrl)
    }

    var record: RecordFormat {
        RecordFormat(code: code, label: label, reserveUrl: reserveUrl, availabilityUrl: availabilityUrl)
    }

    var displayLabel: String { label.nonEmpty ?? code }

    /// Ebook/eaudio/streaming: "Available online", no pickup UI.
    ///
    /// TODO(P): return the core's `isDigitalFormat(format: record)` once it lands (§4.1).
    var isDigital: Bool {
        let digitalCodes: Set<String> = ["EBK", "EAUD", "EAU", "EMAG", "EVID", "EMUS", "STRM", "EJNL", "ONLINE"]
        if digitalCodes.contains(code.uppercased()) { return true }
        let text = (label ?? "").lowercased()
        return ["ebook", "eaudio", "e-book", "e-audio", "emagazine", "online", "digital", "streaming"]
            .contains { text.contains($0) }
    }
}

extension Optional where Wrapped == String {
    /// The trimmed string, or nil when it's nil or blank.
    var nonEmpty: String? {
        guard let trimmed = self?.trimmingCharacters(in: .whitespacesAndNewlines), !trimmed.isEmpty else { return nil }
        return trimmed
    }
}
