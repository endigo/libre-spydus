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
        try await timed("search") { try await client.search(query: query) }
    }

    func searchSorted(query: String, sort: SearchSortOption) async throws -> SearchResults {
        try await timed("searchSorted") { try await client.searchSorted(query: query, sort: sort.core) }
    }

    func page(url: String) async throws -> SearchResults {
        try await timed("page") { try await client.page(url: url) }
    }

    func availability(url: String) async throws -> Availability {
        try await timed("availability") { try await client.availability(url: url) }
    }

    func record(url: String) async throws -> RecordDetail {
        try await timed("record") { try await client.record(url: url) }
    }

    func related(url: String) async throws -> RelatedList {
        try await timed("related") { try await client.related(url: url) }
    }

    func cardBarcodeSymbology() async -> BarcodeSymbology {
        await timed("cardBarcodeSymbology") { await client.cardBarcodeSymbology() }
    }

    func featured() async throws -> [FeaturedLink] {
        try await timed("featured") { try await client.featured() }
    }

    func collection(url: String) async throws -> FeaturedCollection {
        try await timed("collection") { try await client.collection(url: url) }
    }

    func holdings(url: String) async throws -> [Holding] {
        try await timed("holdings") { try await client.holdings(url: url) }
    }

    func login(username: String, password: String) async throws -> String {
        try await timed("login") { try await client.login(username: username, password: password) }
    }

    func account(session: String) async throws -> AccountLinks {
        try await timed("account") { try await client.account(session: session) }
    }

    func loans(session: String) async throws -> PatronList {
        try await timed("loans") { try await client.loans(session: session) }
    }

    func reservations(session: String) async throws -> Reservations {
        try await timed("reservations") { try await client.reservations(session: session) }
    }

    func accountSnapshot(session: String) async throws -> AccountSnapshot {
        try await timed("accountSnapshot") { try await client.accountSnapshot(session: session) }
    }

    func reserveOptions(url: String, session: String) async throws -> ReserveOptions {
        try await timed("reserveOptions") { try await client.reserveOptions(url: url, session: session) }
    }

    func placeReservation(url: String, session: String, xdays: String?, pickup: String?) async throws -> ReserveResult {
        try await timed("placeReservation") { try await client.placeReservation(url: url, session: session, xdays: xdays, pickup: pickup) }
    }

    func cancelReservation(url: String, session: String) async throws -> ReserveResult {
        try await timed("cancelReservation") { try await client.cancelReservation(url: url, session: session) }
    }

    func editOptions(url: String, session: String) async throws -> EditOptions {
        try await timed("editOptions") { try await client.editOptions(url: url, session: session) }
    }

    func editReservation(url: String, session: String, overrides: EditOverrides) async throws -> ReserveResult {
        try await timed("editReservation") { try await client.editReservation(url: url, session: session, overrides: overrides) }
    }

    func picks(session: String, url: String?) async throws -> SearchResults {
        try await timed("picks") { try await client.picks(session: session, url: url) }
    }

    func savePick(irn: String, session: String) async throws -> Int64 {
        try await timed("savePick") { try await client.savePick(irn: irn, session: session) }
    }

    func removePick(irn: String, session: String) async throws -> Int64 {
        try await timed("removePick") { try await client.removePick(irn: irn, session: session) }
    }

    /// Times `call` for the debug timing log (`TimingLog`).
    private func timed<T>(_ operation: String, _ call: () async throws -> T) async rethrows -> T {
        #if DEBUG
        let started = ContinuousClock.now
        defer { TimingLog.app(operation, ContinuousClock.now - started) }
        #endif
        return try await call()
    }
}
