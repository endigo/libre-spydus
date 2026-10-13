import Foundation
import LibreSpydusCore

/// Demo mode (DEBUG, `-LSDemo YES`; docs/native-apps.md §5.4).
///
/// Public reads (search/page/availability/record/related, the Featured collections and copies)
/// go to a real backend: public GETs are
/// allowed. The demo account's own books are the exception: `DemoCatalogue` answers for them
/// offline, the way the OPAC does (a loan's session-set link loads an empty record, the
/// session-free one the full record). Every account call is answered from `DemoFixtures` and never
/// touches the network.
/// `login("demo", "demo")` succeeds, anything else is `CredentialsRejected`. Mutations succeed
/// after `mutationDelay` and update the in-memory lists: cancel removes the row, reserve adds a
/// pending row, edit changes the pickup location. `accountSnapshot` takes `snapshotDelay`, so the
/// refreshing state is visible, and reflects those changes. With `addsReadyOnNextRefresh` (DEBUG
/// `-LSDemoNewReady`) one more reservation is ready from the second `accountSnapshot` on, so a
/// "Ready to collect" alert can be shown (docs/native-apps.md §5.8).
///
/// My Picks (§5.11) is an in-memory list seeded with two demo records (`seedPickIrns`: "The Midnight
/// Library" as a work with two formats, "The Love Library" as an edition-level card the way Featured
/// lists show them); `savePick` adds a card for any IRN this backend has seen (search, page and
/// record results, the demo catalogue), `removePick` takes it out, both after `mutationDelay`, and
/// an emptied list is a genuine `noResults` page.
actor DemoBackend: LibraryBackend {
    static let username = "demo"
    static let password = "demo"
    static let session = "demo-session"
    /// The demo account's library card number and expiry (§5.9), in its snapshot like a real
    /// account's Personal details.
    static let cardNumber = "DEMO0012345"
    static let cardExpires = "12 Mar 2027"
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
    /// `-LSDemoNewReady YES`: a pending reservation comes in on the next refresh.
    private var addsReadyOnNextRefresh: Bool
    private var snapshotsServed = 0
    private var fixtures: DemoFixtures
    /// Search records seen through this backend, by `RecordFormat.reserveUrl`, so a demo
    /// reservation can show the right title and link to the right bib.
    private var reservable: [String: (record: SearchRecord, format: RecordFormat)] = [:]
    private var reservedUrls: Set<String> = []
    private var nextIrn = 19_100_000
    /// The demo account's Pick list, in the order saved.
    private var pickList: [SearchRecord]
    /// Search records seen through this backend, by `workIrn`, and records by `bibIrn`: what
    /// `savePick` lists for an IRN.
    private var seenRecords: [String: SearchRecord] = [:]
    private var seenDetails: [String: RecordDetail] = [:]

    /// The demo Pick list's first entries: a work ("The Midnight Library", Book and Large Print) and
    /// an edition ("The Love Library", edition-level like a Featured list's card).
    static let seedPickIrns = (work: "15642035", edition: "19242648")
    /// Records per Pick list page, as the library's `NRECS=20`.
    static let picksPageSize = 20

    init(publicBackend: any LibraryBackend, fixtures: DemoFixtures, mutationDelay: Duration = .milliseconds(600),
         snapshotDelay: Duration = .milliseconds(1500), failsAccountSnapshot: Bool = false,
         addsReadyOnNextRefresh: Bool = false) {
        self.publicBackend = publicBackend
        self.origin = publicBackend.origin
        self.fixtures = fixtures
        self.mutationDelay = mutationDelay
        self.snapshotDelay = snapshotDelay
        self.failsAccountSnapshot = failsAccountSnapshot
        self.addsReadyOnNextRefresh = addsReadyOnNextRefresh
        pickList = Self.seedPicks(fixtures.catalogue, origin: publicBackend.origin)
    }

    /// The demo backend the app uses: real public reads for `origin`, bundled account fixtures.
    static func live(origin: String?, failsAccountSnapshot: Bool = false, addsReadyOnNextRefresh: Bool = false) -> DemoBackend {
        let core = CoreBackend(origin: origin)
        let fixtures: DemoFixtures
        do {
            fixtures = try DemoFixtures.bundled(origin: core.origin)
        } catch {
            preconditionFailure("DemoData fixtures missing from the app bundle: \(error)")
        }
        return DemoBackend(publicBackend: core, fixtures: fixtures, failsAccountSnapshot: failsAccountSnapshot,
                           addsReadyOnNextRefresh: addsReadyOnNextRefresh)
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
        let record: RecordDetail
        if let demo = fixtures.catalogue.record(url: url) {
            record = demo
        } else {
            record = try await publicBackend.record(url: url)
        }
        if let irn = record.bibIrn.nonEmpty { seenDetails[irn] = record }
        return record
    }

    func related(url: String) async throws -> RelatedList {
        try await publicBackend.related(url: url)
    }

    /// The library's own symbology: a public GET of its mobile config (Canning Code 128,
    /// Cumberland Code 39), like the real app.
    func cardBarcodeSymbology() async -> BarcodeSymbology {
        await publicBackend.cardBarcodeSymbology()
    }

    func featured() async throws -> [FeaturedLink] {
        try await publicBackend.featured()
    }

    func collection(url: String) async throws -> FeaturedCollection {
        try await publicBackend.collection(url: url)
    }

    func holdings(url: String) async throws -> [Holding] {
        try await publicBackend.holdings(url: url)
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
        snapshotsServed += 1
        if addsReadyOnNextRefresh, snapshotsServed > 1 {
            addsReadyOnNextRefresh = false
            fixtures.makeNewReady()
        }
        return AccountSnapshot(
            version: Self.snapshotVersion,
            accountName: fixtures.account.name,
            loans: try await loans(session: session),
            reservations: try await reservations(session: session),
            fetchedAtMs: Int64((Date.now.timeIntervalSince1970 * 1000).rounded(.down)),
            cardNumber: Self.cardNumber,
            cardExpires: Self.cardExpires
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

    // MARK: My Picks → in memory

    func picks(session: String, url: String?) async throws -> SearchResults {
        try check(session)
        try await Task.sleep(for: mutationDelay)
        let size = Self.picksPageSize
        let pageNumber = url.flatMap(Self.picksPage) ?? 1
        let start = min((pageNumber - 1) * size, pickList.count)
        let records = pickList[start..<min(start + size, pickList.count)].enumerated().map { offset, record in
            var record = record
            record.rank = Int64(start + offset + 1)
            return record
        }
        let next = start + size < pickList.count ? Self.picksURL(origin: origin, page: pageNumber + 1) : nil
        let previous = pageNumber > 1 ? Self.picksURL(origin: origin, page: pageNumber - 1) : nil
        return remember(SearchResults(query: nil, setId: DemoCatalogue.setId, totalResults: Int64(pickList.count),
                                      showing: nil, nextPageUrl: next, prevPageUrl: previous,
                                      noResults: pickList.isEmpty, records: records))
    }

    func savePick(irn: String, session: String) async throws -> Int64 {
        try check(session)
        try await Task.sleep(for: mutationDelay)
        guard PickIrn.validated(irn) == irn else { throw SpydusError.Parse(message: "not an IRN: \(irn)") }
        if !pickList.contains(where: { $0.workIrn == irn }) {
            // The library's answer for an IRN it doesn't have.
            guard let card = pickCard(irn: irn) else { throw SpydusError.Upstream(message: "None found") }
            pickList.append(card)
        }
        return Int64(pickList.count)
    }

    func removePick(irn: String, session: String) async throws -> Int64 {
        try check(session)
        try await Task.sleep(for: mutationDelay)
        guard PickIrn.validated(irn) == irn else { throw SpydusError.Parse(message: "not an IRN: \(irn)") }
        pickList.removeAll { $0.workIrn == irn }
        return Int64(pickList.count)
    }

    /// The card the Pick list shows for `irn`: a results card seen with that `workIrn` (a work, or a
    /// Featured list's edition), a demo work, else an edition-level card for a demo edition or a
    /// record seen with that `bibIrn`.
    private func pickCard(irn: String) -> SearchRecord? {
        if let record = seenRecords[irn] { return record }
        let catalogue = fixtures.catalogue
        if let work = catalogue.works.first(where: { $0.record.workIrn == irn }) { return work.record }
        if let work = catalogue.works.first(where: { $0.editions.contains { $0.bibIrn == irn } }),
           let edition = work.editions.first(where: { $0.bibIrn == irn }) {
            return Self.editionCard(irn: irn, record: edition.record, work: work.record, origin: origin)
        }
        return seenDetails[irn].map { Self.editionCard(irn: irn, record: $0, work: nil, origin: origin) }
    }

    private static func seedPicks(_ catalogue: DemoCatalogue, origin: String) -> [SearchRecord] {
        let work = catalogue.works.first { $0.record.workIrn == seedPickIrns.work }?.record
        let edition = catalogue.works.lazy.compactMap { work in
            work.editions.first { $0.bibIrn == seedPickIrns.edition }.map { (work, $0) }
        }.first.map { work, edition in
            editionCard(irn: edition.bibIrn, record: edition.record, work: work.record, origin: origin)
        }
        return [work, edition].compactMap { $0 }
    }

    /// An edition as Featured and Pick lists show it (§5.10): its `workIrn` is the bib's IRN, and its
    /// one format has no code or label, the card's reserve link and `…/BIBENQ?SETLVL=&IRN=…&FMT=RECFMT`.
    static func editionCard(irn: String, record: RecordDetail, work: SearchRecord?, origin: String) -> SearchRecord {
        let author = record.details.first { ["author", "creator"].contains($0.label.lowercased()) }?.values.first?.text
        let base = "\(origin)/cgi-bin/spydus.exe"
        return SearchRecord(
            rank: 1, workIrn: irn, title: record.title.nonEmpty ?? work?.title,
            detailUrl: "\(base)/FULL/OPAC/ALLENQ/\(DemoCatalogue.setId)/\(irn),1",
            author: work?.author ?? author, authorRole: work?.authorRole, authorUrl: nil, year: work?.year,
            language: work?.language, summary: nil, isbn: record.isbn ?? work?.isbn,
            coverUrls: record.coverUrls.isEmpty ? work?.coverUrls ?? [] : record.coverUrls,
            formats: [RecordFormat(code: "", label: nil,
                                   reserveUrl: "\(base)/PGM/OPAC/CCOPT/\(DemoCatalogue.setId)/0/V/0/R/9/ALLENQ?SVL=\(irn)",
                                   availabilityUrl: "\(base)/ENQ/OPAC/BIBENQ?SETLVL=&IRN=\(irn)&FMT=RECFMT")]
        )
    }

    /// The demo list's page `page` (the library's own list is `SVL/OPAC/ALLENQ?ISGLB=1&NRECS=20`).
    static func picksURL(origin: String, page: Int) -> String {
        "\(origin)/cgi-bin/spydus.exe/SVL/OPAC/ALLENQ?ISGLB=1&NRECS=\(picksPageSize)&DEMOPAGE=\(page)"
    }

    private static func picksPage(_ url: String) -> Int? {
        URLComponents(string: url)?.queryItems?.first { $0.name == "DEMOPAGE" }?.value.flatMap(Int.init)
    }

    // MARK: Helpers

    private func check(_ session: String) throws {
        guard session == Self.session else {
            throw SpydusError.NotLoggedIn(message: "not logged in")
        }
    }

    private func remember(_ results: SearchResults) -> SearchResults {
        for record in results.records {
            if let irn = record.workIrn.nonEmpty { seenRecords[irn] = record }
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
