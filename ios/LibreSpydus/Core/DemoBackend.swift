import Foundation
import LibreSpydusCore

/// Demo mode (DEBUG, `-LSDemo YES`; docs/native-apps.md §5.4).
///
/// Public reads (search/page/availability/record/related) go to a real backend: public GETs are
/// allowed. The demo account's own books are the exception: `DemoCatalogue` answers for them
/// offline, the way the OPAC does (a loan's session-set link loads an empty record, the
/// session-free one the full record). Every account call is answered from `DemoFixtures` and never
/// touches the network.
/// `login("demo", "demo")` succeeds, anything else is `CredentialsRejected`. Mutations succeed
/// after `mutationDelay` and update the in-memory lists: cancel removes the row, reserve adds a
/// pending row, edit changes the pickup location. `accountSnapshot` takes `snapshotDelay`, so the
/// refreshing state is visible, and reflects those changes.
actor DemoBackend: LibraryBackend {
    static let username = "demo"
    static let password = "demo"
    static let session = "demo-session"
    /// The core's `ACCOUNT_SNAPSHOT_VERSION` (not exported; DemoBackendTests round-trips a demo
    /// snapshot through `decodeAccountSnapshot`, which fails if the core moves on).
    static let snapshotVersion: UInt32 = 1

    nonisolated let origin: String
    nonisolated var isDemo: Bool { true }

    private let publicBackend: any LibraryBackend
    private let mutationDelay: Duration
    private let snapshotDelay: Duration
    /// `-LSDemoRefreshFails YES`: every `accountSnapshot` fails, to show a failed refresh.
    private let failsAccountSnapshot: Bool
    private var fixtures: DemoFixtures
    /// Search records seen through this backend, by `RecordFormat.reserveUrl`, so a demo
    /// reservation can show the right title and link to the right bib.
    private var reservable: [String: (record: SearchRecord, format: RecordFormat)] = [:]
    private var reservedUrls: Set<String> = []
    private var nextIrn = 19_100_000

    init(publicBackend: any LibraryBackend, fixtures: DemoFixtures, mutationDelay: Duration = .milliseconds(600),
         snapshotDelay: Duration = .milliseconds(1500), failsAccountSnapshot: Bool = false) {
        self.publicBackend = publicBackend
        self.origin = publicBackend.origin
        self.fixtures = fixtures
        self.mutationDelay = mutationDelay
        self.snapshotDelay = snapshotDelay
        self.failsAccountSnapshot = failsAccountSnapshot
    }

    /// The demo backend the app uses: real public reads for `origin`, bundled account fixtures.
    static func live(origin: String?, failsAccountSnapshot: Bool = false) -> DemoBackend {
        let core = CoreBackend(origin: origin)
        let fixtures: DemoFixtures
        do {
            fixtures = try DemoFixtures.bundled(origin: core.origin)
        } catch {
            preconditionFailure("DemoData fixtures missing from the app bundle: \(error)")
        }
        return DemoBackend(publicBackend: core, fixtures: fixtures, failsAccountSnapshot: failsAccountSnapshot)
    }

    nonisolated func setVerificationCookie(_ cookie: String?) {
        publicBackend.setVerificationCookie(cookie)
    }

    // MARK: Public reads → real backend

    func search(query: String) async throws -> SearchResults {
        if let results = fixtures.catalogue.search(query: query) { return remember(results) }
        return remember(try await publicBackend.search(query: query))
    }

    func searchSorted(query: String, sort: SearchSortOption) async throws -> SearchResults {
        if let results = fixtures.catalogue.search(query: query) { return remember(results) }
        return remember(try await publicBackend.searchSorted(query: query, sort: sort))
    }

    func page(url: String) async throws -> SearchResults {
        remember(try await publicBackend.page(url: url))
    }

    func availability(url: String) async throws -> Availability {
        if let availability = fixtures.catalogue.availability(url: url) { return availability }
        return try await publicBackend.availability(url: url)
    }

    func record(url: String) async throws -> RecordDetail {
        if let record = fixtures.catalogue.record(url: url) { return record }
        return try await publicBackend.record(url: url)
    }

    func related(url: String) async throws -> RelatedList {
        try await publicBackend.related(url: url)
    }

    // MARK: Account → fixtures

    func login(username: String, password: String) async throws -> String {
        try await Task.sleep(for: mutationDelay)
        let user = username.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard user == Self.username, password == Self.password else {
            throw SpydusError.CredentialsRejected(message: "credentials rejected")
        }
        return Self.session
    }

    func account(session: String) async throws -> AccountLinks {
        try check(session)
        return fixtures.account
    }

    func loans(session: String) async throws -> PatronList {
        try check(session)
        return PatronList(total: Int64(fixtures.loans.count), items: fixtures.loans)
    }

    func reservations(session: String) async throws -> Reservations {
        try check(session)
        return Reservations(
            ready: PatronList(total: Int64(fixtures.ready.count), items: fixtures.ready),
            pending: PatronList(total: Int64(fixtures.pending.count), items: fixtures.pending)
        )
    }

    func accountSnapshot(session: String) async throws -> AccountSnapshot {
        try check(session)
        try await Task.sleep(for: snapshotDelay)
        if failsAccountSnapshot {
            throw SpydusError.Network(message: "demo: -LSDemoRefreshFails")
        }
        return AccountSnapshot(
            version: Self.snapshotVersion,
            accountName: fixtures.account.name,
            loans: try await loans(session: session),
            reservations: try await reservations(session: session),
            fetchedAtMs: Int64((Date.now.timeIntervalSince1970 * 1000).rounded(.down))
        )
    }

    func reserveOptions(url: String, session: String) async throws -> ReserveOptions {
        try check(session)
        return fixtures.reserveOptions
    }

    func placeReservation(url: String, session: String, xdays: String?, pickup: String?) async throws -> ReserveResult {
        try check(session)
        try await Task.sleep(for: mutationDelay)
        guard !reservedUrls.contains(url) else {
            return ReserveResult(ok: false, message: "You already have a reservation for this title.")
        }
        reservedUrls.insert(url)
        let record = reservable[url]?.record
        let irn = nextIrn
        nextIrn += 1
        // Like the library's own row: the reserved edition's bib, linked through the patron's set.
        let bibIrn = await editionIrn(of: reservable[url]?.format)
        let item = PatronItem(
            irn: String(irn), bibIrn: bibIrn, title: record?.title ?? "Reserved title",
            bibUrl: bibIrn.map { DemoCatalogue.sessionSetURL(origin: origin, irn: $0) } ?? record?.detailUrl,
            author: record?.author, year: record?.year, coverUrls: record?.coverUrls ?? [], isbn: record?.isbn,
            fields: [
                "Status": "Pending",
                "Rank": "1",
                "Placed": DueDate.format(.now),
                "Pickup location": pickup ?? fixtures.reserveOptions.pickupLocation ?? "Your library",
            ],
            actions: reservationActions(irn: irn)
        )
        fixtures.pending.insert(item, at: 0)
        return ReserveResult(ok: true, message: "Reservation placed.")
    }

    func cancelReservation(url: String, session: String) async throws -> ReserveResult {
        try check(session)
        try await Task.sleep(for: mutationDelay)
        let before = fixtures.ready.count + fixtures.pending.count
        fixtures.ready.removeAll { $0.hasAction(href: url) }
        fixtures.pending.removeAll { $0.hasAction(href: url) }
        guard fixtures.ready.count + fixtures.pending.count < before else {
            return ReserveResult(ok: false, message: "This reservation can't be cancelled.")
        }
        return ReserveResult(ok: true, message: "Reservation cancelled.")
    }

    func editOptions(url: String, session: String) async throws -> EditOptions {
        try check(session)
        var options = fixtures.editOptions
        if let row = (fixtures.ready + fixtures.pending).first(where: { $0.hasAction(href: url) }),
           let pickup = row.fields["Pickup location"],
           let choice = options.pickupChoices.first(where: { $0.name.caseInsensitiveCompare(pickup) == .orderedSame }) {
            options.pickupSelected = choice.code
        }
        return options
    }

    func editReservation(url: String, session: String, overrides: EditOverrides) async throws -> ReserveResult {
        try check(session)
        try await Task.sleep(for: mutationDelay)
        let pickupName = overrides.pickup.flatMap { code in
            fixtures.editOptions.pickupChoices.first { $0.code == code }?.name
        }
        var found = false
        for index in fixtures.ready.indices where fixtures.ready[index].hasAction(href: url) {
            found = true
            if let pickupName { fixtures.ready[index].fields["Pickup location"] = pickupName }
        }
        for index in fixtures.pending.indices where fixtures.pending[index].hasAction(href: url) {
            found = true
            if let pickupName { fixtures.pending[index].fields["Pickup location"] = pickupName }
        }
        guard found else {
            return ReserveResult(ok: false, message: "This reservation can't be edited.")
        }
        return ReserveResult(ok: true, message: "Reservation updated.")
    }

    // MARK: Helpers

    private func check(_ session: String) throws {
        guard session == Self.session else {
            throw SpydusError.NotLoggedIn(message: "not logged in")
        }
    }

    private func remember(_ results: SearchResults) -> SearchResults {
        for record in results.records {
            for format in record.formats {
                if let reserveUrl = format.reserveUrl { reservable[reserveUrl] = (record, format) }
            }
        }
        return results
    }

    /// The bib IRN `format` resolves to: the catalogue's, else the library's availability page
    /// (a public GET, usually cached by the detail page already); nil if that fails.
    private func editionIrn(of format: RecordFormat?) async -> String? {
        guard let url = format?.availabilityUrl.nonEmpty else { return nil }
        if let irn = fixtures.catalogue.bibIrn(availabilityUrl: url) { return irn }
        return (try? await publicBackend.availability(url: url))?.bibRecordIrn.nonEmpty
    }

    private func reservationActions(irn: Int) -> [PatronAction] {
        let base = "\(origin)/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/78500000/0"
        return [
            PatronAction(label: "Cancel reservation", href: "\(base)/C/5/RSVCENQ?SVL=\(irn)&SETDESC=x"),
            PatronAction(label: "Edit", href: "\(base)/ER/1/RSVCENQ?SVL=\(irn)&SETDESC=x"),
        ]
    }
}

private extension PatronItem {
    func hasAction(href: String) -> Bool {
        actions.contains { $0.href == href }
    }
}
