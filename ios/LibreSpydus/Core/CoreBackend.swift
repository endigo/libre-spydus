import Foundation
import LibreSpydusCore

/// The real backend: one `SpydusClient` for one library origin, kept while that library is
/// selected (AppModel rebuilds it on a library change).
final class CoreBackend: LibraryBackend {
    let client: SpydusClient
    let origin: String
    var isDemo: Bool { false }

    /// `origin` is a `LibraryInfo.origin`; nil means Canning.
    init(origin: String?) {
        client = SpydusClient(origin: origin)
        self.origin = client.origin()
    }

    func setVerificationCookie(_ cookie: String?) {
        client.setVerificationCookie(cookie: cookie)
    }

    func search(query: String) async throws -> SearchResults {
        try await client.search(query: query)
    }

    func searchSorted(query: String, sort: SearchSortOption) async throws -> SearchResults {
        try await client.searchSorted(query: query, sort: sort.core)
    }

    func page(url: String) async throws -> SearchResults {
        try await client.page(url: url)
    }

    func availability(url: String) async throws -> Availability {
        try await client.availability(url: url)
    }

    func record(url: String) async throws -> RecordDetail {
        try await client.record(url: url)
    }

    func related(url: String) async throws -> RelatedList {
        try await client.related(url: url)
    }

    func login(username: String, password: String) async throws -> String {
        try await client.login(username: username, password: password)
    }

    func account(session: String) async throws -> AccountLinks {
        try await client.account(session: session)
    }

    func loans(session: String) async throws -> PatronList {
        try await client.loans(session: session)
    }

    func reservations(session: String) async throws -> Reservations {
        try await client.reservations(session: session)
    }

    func reserveOptions(url: String, session: String) async throws -> ReserveOptions {
        try await client.reserveOptions(url: url, session: session)
    }

    func placeReservation(url: String, session: String, xdays: String?, pickup: String?) async throws -> ReserveResult {
        try await client.placeReservation(url: url, session: session, xdays: xdays, pickup: pickup)
    }

    func cancelReservation(url: String, session: String) async throws -> ReserveResult {
        try await client.cancelReservation(url: url, session: session)
    }

    func editOptions(url: String, session: String) async throws -> EditOptions {
        try await client.editOptions(url: url, session: session)
    }

    func editReservation(url: String, session: String, overrides: EditOverrides) async throws -> ReserveResult {
        try await client.editReservation(url: url, session: session, overrides: overrides)
    }
}
