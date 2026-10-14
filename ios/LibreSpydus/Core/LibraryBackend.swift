import Foundation
import LibreSpydusCore

/// Everything the app asks of one library (docs/native-apps.md §5.1). Screens talk only to this,
/// never to `SpydusClient`: `CoreBackend` wraps the real core, `DemoBackend` serves account data
/// from bundled fixtures.
///
/// Errors are the core's `SpydusError`; route them through `AppModel.report(_:)`.
/// Mutations (`placeReservation`, `cancelReservation`, `editReservation`, `renewLoan`) return
/// `ok: false` results as data when the library refuses; that's not an error.
protocol LibraryBackend: Sendable {
    /// Normalised origin, e.g. `https://canning.spydus.com` (no trailing slash).
    var origin: String { get }
    /// True for `DemoBackend` (DEBUG demo mode).
    var isDemo: Bool { get }
    /// `"aws-waf-token=<value>"` after human verification, or nil to clear. Drops the core's caches.
    func setVerificationCookie(_ cookie: String?)

    // MARK: Public reads (GET only)

    func search(query: String) async throws -> SearchResults
    /// `search` with an explicit order (`.relevance` is the same request as `search`).
    func searchSorted(query: String, sort: SearchSortOption) async throws -> SearchResults
    /// Next/previous page: pass `SearchResults.nextPageUrl` / `prevPageUrl`.
    func page(url: String) async throws -> SearchResults
    /// A format's `availabilityUrl`.
    func availability(url: String) async throws -> Availability
    /// `SearchRecord.detailUrl`, `patronItemRecordUrl(item:)` for a loan or reservation (its own
    /// `bibUrl` needs the patron's OPAC session), `RelatedItem.bibUrl`, a format's
    /// `availabilityUrl`, or a stored `BookRef.url`.
    func record(url: String) async throws -> RecordDetail
    /// One of `RecordDetail.related.*`.
    func related(url: String) async throws -> RelatedList
    /// The symbology the library's card scanners read (§5.9): its mobile config's `barcodeType`,
    /// Code 128 when it has none or it can't be read. Never throws; cached in the core.
    func cardBarcodeSymbology() async -> BarcodeSymbology
    /// The library's "Featured" collections from its OPAC home page, in order (§5.10); empty for
    /// most libraries.
    func featured() async throws -> [FeaturedLink]
    /// A `.collection` link's page → its sections, each a results list for `page(url:)`; none for a
    /// page of outside links (eLibrary).
    func collection(url: String) async throws -> FeaturedCollection
    /// Every copy of a record: an `Availability.holdingsUrl`.
    func holdings(url: String) async throws -> [Holding]

    // MARK: Saved pages (§5.13: the device only, never the network)

    /// The book page saved under `key` (`BookRef.savedPageKey`), or nil.
    func savedBook(key: String) -> SavedBook?
    /// Saves a book page after a fresh load, replacing what was saved. Never throws.
    func saveBook(key: String, book: SavedBook) async
    /// A Home shelf's first results page saved under its `Shelf.key`, or nil.
    func savedShelf(key: String) -> SearchResults?
    func saveShelf(key: String, results: SearchResults) async
    /// The `featured()` links saved last (an empty list: the library has none), or nil.
    func savedFeatured() -> [FeaturedLink]?
    func saveFeatured(links: [FeaturedLink]) async

    // MARK: Account (needs a session from `login`)

    func login(username: String, password: String) async throws -> String
    func account(session: String) async throws -> AccountLinks
    func loans(session: String) async throws -> PatronList
    func reservations(session: String) async throws -> Reservations
    /// Name, loans and both reservation lists in one call (loans and reservations concurrently):
    /// what `AccountStore` shows and saves on the device.
    func accountSnapshot(session: String) async throws -> AccountSnapshot
    func reserveOptions(url: String, session: String) async throws -> ReserveOptions
    func placeReservation(url: String, session: String, xdays: String?, pickup: String?) async throws -> ReserveResult
    func cancelReservation(url: String, session: String) async throws -> ReserveResult
    func editOptions(url: String, session: String) async throws -> EditOptions
    func editReservation(url: String, session: String, overrides: EditOverrides) async throws -> ReserveResult
    /// MUTATING, only after the reader confirms (§5.12): renews the loan whose "Renew loan" action
    /// is `url` (`PatronItem.renewHref`). Takes a few seconds: the core reads the loans list before
    /// and after, and `ok` says whether the due date moved; `due` is the date after the attempt.
    func renewLoan(url: String, session: String) async throws -> RenewResult

    // MARK: My Picks (§5.11; needs a session, not cached)

    /// The reader's Pick list ("My Picks") as a results list: `url` nil for the first page, else its
    /// `nextPageUrl`. Empty → `noResults`. Each record's `workIrn` is the IRN to remove.
    func picks(session: String, url: String?) async throws -> SearchResults
    /// MUTATING, only on the reader's tap: saves a card's `workIrn` or a record's `bibIrn` to the
    /// list; returns the list's new size.
    func savePick(irn: String, session: String) async throws -> Int64
    /// MUTATING, only on the reader's tap: removes `irn` from the list; returns how many remain.
    func removePick(irn: String, session: String) async throws -> Int64
}

extension LibraryBackend {
    /// Code 128, the core's default, for a backend that doesn't ask the library (test fakes).
    func cardBarcodeSymbology() async -> BarcodeSymbology { .code128 }

    /// No Featured list, collections or copies, for a backend that doesn't ask the library (test fakes).
    func featured() async throws -> [FeaturedLink] { [] }
    func collection(url: String) async throws -> FeaturedCollection { FeaturedCollection(title: nil, sections: []) }
    func holdings(url: String) async throws -> [Holding] { [] }

    /// Nothing saved and nothing kept, for a backend that doesn't save pages (test fakes).
    func savedBook(key: String) -> SavedBook? { nil }
    func saveBook(key: String, book: SavedBook) async {}
    func savedShelf(key: String) -> SearchResults? { nil }
    func saveShelf(key: String, results: SearchResults) async {}
    func savedFeatured() -> [FeaturedLink]? { nil }
    func saveFeatured(links: [FeaturedLink]) async {}

    /// An empty Pick list that never changes, for a backend without one (test fakes).
    func picks(session: String, url: String?) async throws -> SearchResults {
        SearchResults(query: "", setId: nil, totalResults: 0, showing: nil, nextPageUrl: nil, prevPageUrl: nil,
                      noResults: true, records: [])
    }
    func savePick(irn: String, session: String) async throws -> Int64 { 0 }
    func removePick(irn: String, session: String) async throws -> Int64 { 0 }

    /// A library that never renews, for a backend without loans (test fakes).
    func renewLoan(url: String, session: String) async throws -> RenewResult {
        RenewResult(ok: false, message: nil, due: nil)
    }
}

/// Result ordering for search / `Route.results`; `newest` = most recently added to the catalogue.
enum SearchSortOption: String, CaseIterable, Codable, Hashable, Identifiable, Sendable {
    case relevance
    case newest

    var id: String { rawValue }
    var label: String {
        switch self {
        case .relevance: Copy.Search.relevance
        case .newest: Copy.Search.newest
        }
    }

    /// The core's `SearchSort`.
    var core: SearchSort {
        switch self {
        case .relevance: .relevance
        case .newest: .newest
        }
    }

    init(_ sort: SearchSort) {
        switch sort {
        case .relevance: self = .relevance
        case .newest: self = .newest
        }
    }
}

extension SearchResults {
    /// True only for a genuine "no records found" page; an unrecognised page should be an error.
    var isNoResults: Bool { noResults }
}
