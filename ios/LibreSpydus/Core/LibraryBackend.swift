import Foundation
import LibreSpydusCore

/// Everything the app asks of one library (docs/native-apps.md §5.1). Screens talk only to this,
/// never to `SpydusClient`: `CoreBackend` wraps the real core, `DemoBackend` serves account data
/// from bundled fixtures.
///
/// Errors are the core's `SpydusError`; route them through `AppModel.report(_:)`.
/// Mutations (`placeReservation`, `cancelReservation`, `editReservation`) return
/// `ReserveResult(ok: false, …)` as data when the library refuses; that's not an error.
protocol LibraryBackend: Sendable {
    /// Normalised origin, e.g. `https://canning.spydus.com` (no trailing slash).
    var origin: String { get }
    /// True for `DemoBackend` (DEBUG demo mode).
    var isDemo: Bool { get }
    /// `"aws-waf-token=<value>"` after human verification, or nil to clear. Drops the core's caches.
    func setVerificationCookie(_ cookie: String?)

    // MARK: Public reads (GET only)

    func search(query: String) async throws -> SearchResults
    /// TODO(P): `CoreBackend` falls back to `search(query:)` until the core's `searchSorted` lands.
    func searchSorted(query: String, sort: SearchSortOption) async throws -> SearchResults
    /// Next/previous page: pass `SearchResults.nextPageUrl` / `prevPageUrl`.
    func page(url: String) async throws -> SearchResults
    /// A format's `availabilityUrl`.
    func availability(url: String) async throws -> Availability
    /// `SearchRecord.detailUrl`, `PatronItem.bibUrl`, `RelatedItem.bibUrl`, a format's
    /// `availabilityUrl`, or a stored `BookRef.url`.
    func record(url: String) async throws -> RecordDetail
    /// One of `RecordDetail.related.*`.
    func related(url: String) async throws -> RelatedList

    // MARK: Account (needs a session from `login`)

    func login(username: String, password: String) async throws -> String
    func account(session: String) async throws -> AccountLinks
    func loans(session: String) async throws -> PatronList
    func reservations(session: String) async throws -> Reservations
    func reserveOptions(url: String, session: String) async throws -> ReserveOptions
    func placeReservation(url: String, session: String, xdays: String?, pickup: String?) async throws -> ReserveResult
    func cancelReservation(url: String, session: String) async throws -> ReserveResult
    func editOptions(url: String, session: String) async throws -> EditOptions
    func editReservation(url: String, session: String, overrides: EditOverrides) async throws -> ReserveResult
}

/// Result ordering for search / `Route.results`.
///
/// TODO(P): when the core's `enum SearchSort { relevance, newest }` lands (§4.1), add
/// `var core: SearchSort` here and call `client.searchSorted(query:sort:)` from `CoreBackend`.
enum SearchSortOption: String, CaseIterable, Codable, Hashable, Identifiable, Sendable {
    case relevance
    case newest

    var id: String { rawValue }
    var label: String {
        switch self {
        case .relevance: "Relevance"
        case .newest: "Newest"
        }
    }
}

extension SearchResults {
    /// True only for a genuine "no records found" page; an unrecognised page should be an error.
    ///
    /// TODO(P): return the core's `noResults` once it lands (§4.1). Until then: no records and
    /// a total of zero, or no total and no paging.
    var isNoResults: Bool {
        guard records.isEmpty else { return false }
        if let totalResults { return totalResults == 0 }
        return nextPageUrl == nil && prevPageUrl == nil
    }
}
