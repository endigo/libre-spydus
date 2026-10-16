import Foundation
import LibreSpydusCore
import Testing
@testable import LibreSpydus

// MARK: - Support

/// Holds a call until the test releases it, so the state in between can be checked.
actor Gate {
    private var isOpen = false
    private var waiters: [CheckedContinuation<Void, Never>] = []

    func wait() async {
        guard !isOpen else { return }
        await withCheckedContinuation { waiters.append($0) }
    }

    func release() {
        isOpen = true
        waiters.forEach { $0.resume() }
        waiters = []
    }
}

/// An offline library with a Pick list that behaves like the website's: `savePick` adds a card,
/// `removePick` takes it out, `picks` pages the list `pageSize` at a time. Records every call
/// ("picks:<page>", "save:<irn>", "remove:<irn>", "record:<url>").
actor PicksFake: LibraryBackend {
    static let session = "fake-session"

    nonisolated let origin: String
    nonisolated var isDemo: Bool { false }

    private(set) var list: [SearchRecord]
    private(set) var calls: [String] = []
    private let pageSize: Int
    private var listError: SpydusError?
    private var unrecognisedPage = false
    /// By IRN; "*" for every IRN.
    private var changeErrors: [String: SpydusError] = [:]
    private var records: [String: RecordDetail] = [:]
    private var gates: [String: Gate] = [:]

    init(origin: String = "https://canning.spydus.com", irns: [String] = [], pageSize: Int = 20) {
        self.origin = origin
        list = irns.map(Self.card)
        self.pageSize = pageSize
    }

    var irns: [String] { list.compactMap(\.workIrn) }

    func failList(with error: SpydusError?) { listError = error }
    func answerUnrecognisedPage() { unrecognisedPage = true }
    func failChanges(with error: SpydusError?, irn: String = "*") { changeErrors[irn] = error }
    func setRecord(_ record: RecordDetail, for url: String) { records[url] = record }

    /// Holds every later `operation` ("picks", "save", "remove") until the gate is released.
    func hold(_ operation: String) -> Gate {
        let gate = Gate()
        gates[operation] = gate
        return gate
    }

    static func pageURL(_ page: Int) -> String { "https://canning.spydus.com/picks?page=\(page)" }

    static func card(_ irn: String) -> SearchRecord {
        SearchRecord(rank: 1, workIrn: irn, title: "Book \(irn)",
                     detailUrl: "https://canning.spydus.com/cgi-bin/spydus.exe/FULL/OPAC/ALLWRKENQ/1/\(irn),1",
                     author: "Writer, A.", authorRole: nil, authorUrl: nil, year: "2024", language: nil, summary: nil,
                     isbn: nil, coverUrls: [],
                     formats: [RecordFormat(code: "BK", label: "Book", reserveUrl: nil, availabilityUrl: "avail-\(irn)")])
    }

    // MARK: My Picks

    /// The page is read when the call comes in; a held call answers with it later (a page that
    /// predates a change made meanwhile).
    func picks(session: String, url: String?) async throws -> SearchResults {
        let page = url.flatMap { $0.split(separator: "=").last.flatMap { Int($0) } } ?? 1
        calls.append("picks:\(page)")
        let answer = pickPage(page, session: session)
        await gates["picks"]?.wait()
        return try answer.get()
    }

    private func pickPage(_ page: Int, session: String) -> Result<SearchResults, SpydusError> {
        guard session == Self.session else { return .failure(.NotLoggedIn(message: "not logged in")) }
        if let listError { return .failure(listError) }
        if unrecognisedPage {
            return .success(SearchResults(query: nil, setId: nil, totalResults: nil, showing: nil, nextPageUrl: nil,
                                          prevPageUrl: nil, noResults: false, records: []))
        }
        let start = min((page - 1) * pageSize, list.count)
        let records = Array(list[start..<min(start + pageSize, list.count)])
        return .success(SearchResults(query: nil, setId: "1", totalResults: Int64(list.count), showing: nil,
                                      nextPageUrl: start + pageSize < list.count ? Self.pageURL(page + 1) : nil,
                                      prevPageUrl: nil, noResults: list.isEmpty, records: records))
    }

    func savePick(irn: String, session: String) async throws -> Int64 {
        calls.append("save:\(irn)")
        await gates["save"]?.wait()
        guard session == Self.session else { throw SpydusError.NotLoggedIn(message: "not logged in") }
        if let error = changeErrors[irn] ?? changeErrors["*"] { throw error }
        if !list.contains(where: { $0.workIrn == irn }) { list.append(Self.card(irn)) }
        return Int64(list.count)
    }

    func removePick(irn: String, session: String) async throws -> Int64 {
        calls.append("remove:\(irn)")
        await gates["remove"]?.wait()
        guard session == Self.session else { throw SpydusError.NotLoggedIn(message: "not logged in") }
        if let error = changeErrors[irn] ?? changeErrors["*"] { throw error }
        list.removeAll { $0.workIrn == irn }
        return Int64(list.count)
    }

    // MARK: The rest

    nonisolated func setVerificationCookie(_ cookie: String?) {}
    func search(query: String) async throws -> SearchResults { try await picks(session: Self.session, url: nil) }
    func searchSorted(query: String, sort: SearchSortOption) async throws -> SearchResults { try await search(query: query) }
    func page(url: String) async throws -> SearchResults { throw SpydusError.Upstream(message: "fake") }
    func availability(url: String) async throws -> Availability { throw SpydusError.Upstream(message: "fake") }
    func record(url: String) async throws -> RecordDetail {
        calls.append("record:\(url)")
        guard let record = records[url] else { throw SpydusError.Upstream(message: "fake") }
        return record
    }
    func related(url: String) async throws -> RelatedList { RelatedList(items: []) }
    func login(username: String, password: String) async throws -> String {
        guard password == "ok" else { throw SpydusError.CredentialsRejected(message: "credentials rejected") }
        return Self.session
    }
    func account(session: String) async throws -> AccountLinks {
        AccountLinks(name: "Fake Borrower", loansUrl: nil, reservationsReadyUrl: nil, reservationsPendingUrl: nil)
    }
    func loans(session: String) async throws -> PatronList { PatronList(total: 0, items: []) }
    func reservations(session: String) async throws -> Reservations {
        Reservations(ready: PatronList(total: 0, items: []), pending: PatronList(total: 0, items: []))
    }
    func accountSnapshot(session: String) async throws -> AccountSnapshot { TestData.snapshot() }
    func reserveOptions(url: String, session: String) async throws -> ReserveOptions { throw SpydusError.NotLoggedIn(message: "") }
    func placeReservation(url: String, session: String, xdays: String?, pickup: String?) async throws -> ReserveResult {
        ReserveResult(ok: false, message: nil)
    }
    func cancelReservation(url: String, session: String) async throws -> ReserveResult { ReserveResult(ok: false, message: nil) }
    func editOptions(url: String, session: String) async throws -> EditOptions { throw SpydusError.NotLoggedIn(message: "") }
    func editReservation(url: String, session: String, overrides: EditOverrides) async throws -> ReserveResult {
        ReserveResult(ok: false, message: nil)
    }
}

enum PicksTestData {
    static func record(bibIrn: String?, title: String = "A record") -> RecordDetail {
        RecordDetail(bibIrn: bibIrn, brn: nil, title: title, seriesTitle: nil, isbn: nil, coverUrls: [], summary: nil,
                     details: [], bookmarkUrl: nil,
                     related: RecordRelated(similar: nil, author: nil, series: nil, versions: nil))
    }

    static func defaults() -> UserDefaults {
        UserDefaults(suiteName: "LibreSpydusTests-\(UUID().uuidString)")!
    }
}

/// Polls `condition` (every 5 ms, up to 2 s): for state that settles on another task.
@MainActor
func eventually(_ condition: () async -> Bool) async -> Bool {
    for _ in 0..<400 {
        if await condition() { return true }
        try? await Task.sleep(for: .milliseconds(5))
    }
    return await condition()
}

// MARK: - Tests

@MainActor
struct PicksStoreTests {
    private let origin = "https://canning.spydus.com"

    /// A store signed in to `backend`, recording what it reports.
    private func makeStore(_ backend: PicksFake, reported: Reported = Reported()) -> PicksStore {
        let store = PicksStore(defaults: PicksTestData.defaults())
        store.report = { error in
            reported.errors.append(error)
            if error is CancellationError { return nil }
            return UserFacingError(error, libraryName: "Canning")
        }
        store.use(origin: origin, session: PicksFake.session, backend: backend)
        return store
    }

    final class Reported {
        var errors: [any Error] = []
    }

    // MARK: The IRN

    @Test func pickIrnComesFromWorkAndBibKeys() {
        #expect(PickIrn.of(BookRef(TestData.searchRecord)) == "15974354")
        #expect(PickIrn.of(BookRef(id: "bib:8753080", title: "Loan")) == "8753080")
        #expect(PickIrn.of(BookRef(id: "work: 42 ", title: "Padded")) == "42")
        #expect(PickIrn.of(BookRef(id: "brn:275039", title: "Related")) == nil)
        #expect(PickIrn.of(BookRef(url: "https://canning.spydus.com/x")) == nil)
        #expect(PickIrn.of(BookRef(id: "title:a book", title: "A book")) == nil)
        #expect(PickIrn.of(BookRef(id: "work:12a", title: "Not an IRN")) == nil)
    }

    @Test func otherRefsUseTheLoadedRecordsBibIrn() {
        let related = BookRef(id: "brn:275039", title: "Related", url: "https://canning.spydus.com/rec")
        #expect(PickIrn.of(related, record: nil) == nil)
        #expect(PickIrn.of(related, record: PicksTestData.record(bibIrn: "8753080")) == "8753080")
        #expect(PickIrn.of(related, record: PicksTestData.record(bibIrn: nil)) == nil)
        // A ref's own IRN wins over the record's.
        #expect(PickIrn.of(BookRef(id: "bib:1", title: "Loan"), record: PicksTestData.record(bibIrn: "2")) == "1")
    }

    @Test func bookPageWaitsForTheRecordBeforeItKnowsTheIrn() async {
        let url = "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&BRN=465787"
        let backend = PicksFake()
        await backend.setRecord(PicksTestData.record(bibIrn: "19243435", title: "Whistler"), for: url)
        let detail = BookDetailModel(ref: BookRef(url: url))
        #expect(detail.pickIrn == nil)
        await detail.loadRecord(backend: backend) { _ in nil }
        #expect(detail.pickIrn == "19243435")

        // A search result knows it at once.
        #expect(BookDetailModel(ref: BookRef(TestData.searchRecord)).pickIrn == "15974354")
    }

    // MARK: Saving and removing

    @Test func saveFlipsAtOnceAndConfirms() async throws {
        let backend = PicksFake(irns: ["1"])
        let store = makeStore(backend)
        await store.refresh()
        let gate = await backend.hold("save")

        let save = Task { await store.setPicked(true, irn: "2") }
        #expect(await eventually { await backend.calls.contains("save:2") })
        #expect(store.contains("2"))
        #expect(store.isChanging("2"))
        // A second tap while it's in flight does nothing.
        #expect(await store.setPicked(false, irn: "2") == false)

        await gate.release()
        #expect(await save.value)
        #expect(store.contains("2"))
        #expect(!store.isChanging("2"))
        #expect(store.notice?.message == "Saved to My Picks")
        #expect(store.notice?.tone == .success)
        // The list doesn't show it yet: Favourites loads it again when it's next on screen.
        #expect(store.isStale)
        await store.loadIfNeeded()
        #expect(store.items.map(\.ref.id) == ["work:1", "work:2"])
        #expect(!store.isStale)
    }

    @Test func failedSaveFlipsBackWithAnError() async {
        let backend = PicksFake()
        let reported = Reported()
        let store = makeStore(backend, reported: reported)
        await store.refresh()
        await backend.failChanges(with: .Network(message: "offline"))

        #expect(await store.setPicked(true, irn: "7") == false)
        #expect(!store.contains("7"))
        #expect(store.notice?.message == "Couldn’t update My Picks")
        #expect(store.notice?.tone == .error)
        #expect(reported.errors.count == 1)
    }

    @Test func removeTakesTheRowOutAtOnceAndPutsItBackOnFailure() async throws {
        let backend = PicksFake(irns: ["1", "2", "3"])
        let store = makeStore(backend)
        await store.refresh()
        let row = try #require(store.items.first { $0.ref.id == "work:2" })
        await backend.failChanges(with: .Upstream(message: "None found"))
        let gate = await backend.hold("remove")

        let remove = Task { await store.remove(row) }
        #expect(await eventually { await backend.calls.contains("remove:2") })
        #expect(store.items.map(\.ref.id) == ["work:1", "work:3"])
        #expect(!store.contains("2"))

        await gate.release()
        await remove.value
        #expect(store.items.map(\.ref.id) == ["work:1", "work:2", "work:3"])
        #expect(store.contains("2"))
        #expect(store.notice?.message == "Couldn’t update My Picks")
    }

    @Test func removingTheLastPickShowsTheEmptyState() async {
        let backend = PicksFake(irns: ["1"])
        let store = makeStore(backend)
        await store.refresh()
        #expect(store.phase == .loaded)

        await store.remove(store.items[0])
        #expect(store.items.isEmpty)
        #expect(store.phase == .empty)
        #expect(store.irns.isEmpty)
        #expect(store.notice?.message == "Removed from My Picks")
        #expect(await backend.irns.isEmpty)
    }

    @Test func notLoggedInIsReportedWithoutItsOwnError() async {
        let backend = PicksFake(irns: ["1"])
        let reported = Reported()
        let store = makeStore(backend, reported: reported)
        await store.refresh()
        await backend.failChanges(with: .NotLoggedIn(message: "not logged in"))
        // AppModel.report signs out on NotLoggedIn, which clears the store.
        store.report = { error in
            reported.errors.append(error)
            store.use(origin: nil, session: nil, backend: backend)
            return UserFacingError(error, libraryName: "Canning")
        }

        #expect(await store.setPicked(false, irn: "1") == false)
        #expect(reported.errors.count == 1)
        #expect(store.notice == nil)
        #expect(store.phase == .idle)
        #expect(store.items.isEmpty)
    }

    // MARK: Loading

    @Test func loadsThePickedSetAcrossAtMostTenPages() async {
        let irns = (1...12).map(String.init)
        let backend = PicksFake(irns: irns, pageSize: 1)
        let store = makeStore(backend)

        await store.refresh()
        #expect(store.phase == .loaded)
        await store.settle()
        #expect(await backend.calls == (1...10).map { "picks:\($0)" })
        #expect(store.irns == Set(irns.prefix(10)))
        #expect(store.items.count == 10)
        #expect(store.contains("10"))
        #expect(!store.contains("11"))
        // The list itself goes on as it scrolls.
        #expect(store.nextPageUrl == PicksFake.pageURL(11))
        await store.loadMore()
        #expect(store.items.count == 11)
        #expect(store.contains("11"))
    }

    @Test func aRefreshDropsPicksRemovedElsewhere() async {
        let backend = PicksFake(irns: ["1", "2"])
        let store = makeStore(backend)
        await store.refresh()
        #expect(store.irns == ["1", "2"])
        // Removed on the website.
        _ = try? await backend.removePick(irn: "2", session: PicksFake.session)
        await store.refresh()
        await store.settle()
        #expect(store.irns == ["1"])
        #expect(store.items.map(\.ref.id) == ["work:1"])
    }

    @Test func aChangeDuringALoadWinsOverWhatTheLoadBringsBack() async {
        let backend = PicksFake(irns: ["1", "2"])
        let store = makeStore(backend)
        await store.refresh()
        let gate = await backend.hold("picks")
        let load = Task { await store.refresh() }
        #expect(await eventually { await backend.calls.filter { $0 == "picks:1" }.count == 2 })

        // Removed while the (older) page is on its way.
        #expect(await store.setPicked(false, irn: "2"))
        await gate.release()
        await load.value
        await store.settle()
        #expect(store.irns == ["1"])
        #expect(store.items.map(\.ref.id) == ["work:1"])
    }

    @Test func emptyListIsTheEmptyStateAndAnUnknownPageAnError() async {
        let store = makeStore(PicksFake())
        await store.refresh()
        #expect(store.phase == .empty)
        #expect(store.items.isEmpty)

        let odd = PicksFake(irns: ["1"])
        await odd.answerUnrecognisedPage()
        let failing = makeStore(odd)
        await failing.refresh()
        #expect(failing.phase == .failed(SearchResultsModel.unrecognisedPage))
    }

    @Test func failedLoadShowsTheErrorAndRetries() async {
        let backend = PicksFake(irns: ["1"])
        await backend.failList(with: .Network(message: "offline"))
        let store = makeStore(backend)
        await store.refresh()
        guard case .failed(let error) = store.phase else {
            Issue.record("expected a failure, got \(store.phase)")
            return
        }
        #expect(error.kind == .network)

        await backend.failList(with: nil)
        await store.loadIfNeeded()
        #expect(store.phase == .loaded)
        #expect(store.contains("1"))
    }

    @Test func aFailedRefreshKeepsTheRows() async {
        let backend = PicksFake(irns: ["1"])
        let store = makeStore(backend)
        await store.refresh()
        await backend.failList(with: .Network(message: "offline"))
        await store.refresh()
        #expect(store.phase == .loaded)
        #expect(store.items.count == 1)
        #expect(store.notice?.message == "Can’t reach Canning. Check your connection.")
        #expect(store.notice?.tone == .error)
    }

    @Test func aNewSessionOrSignOutClearsTheList() async {
        let backend = PicksFake(irns: ["1"])
        let store = makeStore(backend)
        await store.refresh()
        #expect(store.contains("1"))

        store.use(origin: origin, session: "another-session", backend: backend)
        #expect(store.irns.isEmpty)
        #expect(store.phase == .idle)

        store.use(origin: origin, session: PicksFake.session, backend: backend)
        await store.refresh()
        store.use(origin: nil, session: nil, backend: backend)
        #expect(!store.isActive)
        #expect(store.items.isEmpty)
        #expect(store.irns.isEmpty)
        await store.refresh()
        #expect(store.phase == .idle)
    }

    // MARK: Copy

    @Test func copySavesOneAtATimeAndSummarises() async {
        let backend = PicksFake(irns: ["9"])
        let store = makeStore(backend)
        await store.refresh()
        let relatedURL = "https://canning.spydus.com/related"
        await backend.setRecord(PicksTestData.record(bibIrn: "3"), for: relatedURL)
        let refs = [
            BookRef(id: "work:1", title: "Work"),
            BookRef(id: "bib:2", title: "Loan"),
            BookRef(id: "brn:30", title: "Related", url: relatedURL),
            BookRef(id: "url:https://canning.spydus.com/gone", title: "Gone", url: "https://canning.spydus.com/gone"),
            BookRef(id: "title:no link", title: "No link"),
        ]

        let summary = await store.copy(refs)
        #expect(summary == PicksStore.CopySummary(added: 3, failed: 2))
        #expect(store.notice?.tone == .info)
        let calls = await backend.calls
        #expect(calls.filter { $0.hasPrefix("save:") || $0.hasPrefix("record:") } == [
            "save:1", "save:2", "record:\(relatedURL)", "save:3", "record:https://canning.spydus.com/gone",
        ])
        #expect(store.notice?.message == "Added 3 books to My Picks · 2 couldn’t be added")
        // The set (and list) loads again afterwards.
        #expect(calls.last == "picks:1")
        #expect(store.irns == ["9", "1", "2", "3"])
        #expect(store.items.count == 4)
    }

    @Test func copyStopsWhenTheSessionEnds() async {
        let backend = PicksFake()
        let reported = Reported()
        let store = makeStore(backend, reported: reported)
        await backend.failChanges(with: .NotLoggedIn(message: "not logged in"))
        let summary = await store.copy([BookRef(id: "work:1", title: "A"), BookRef(id: "work:2", title: "B")])
        #expect(summary == PicksStore.CopySummary(added: 0, failed: 2))
        #expect(store.notice?.message == "Added 0 books to My Picks · 2 couldn’t be added")
        #expect(store.notice?.tone == .error)
        #expect(await backend.calls.filter { $0.hasPrefix("save:") } == ["save:1"])
        #expect(reported.errors.count == 1)
    }

    @Test func copyWording() {
        CopyDevice.$isPadOverride.withValue(false) {
            #expect(Copy.Picks.offerTitle == "Add this phone’s picks to your library account?")
            #expect(Copy.Picks.offerMessage(count: 3) == "3 books saved on this phone can be added to My Picks on your library account.")
            #expect(Copy.Picks.offerMessage(count: 1) == "1 book saved on this phone can be added to My Picks on your library account.")
        }
        #expect(PicksText.copied(added: 4, failed: 0) == "Added 4 books to My Picks")
        #expect(PicksText.copied(added: 1, failed: 1) == "Added 1 book to My Picks · 1 couldn’t be added")
        #expect(Copy.Picks.save == "Save to My Picks")
        #expect(Copy.Picks.remove == "Remove from My Picks")
        #expect(Copy.Picks.emptyTitle == "No picks yet")
        #expect(Copy.Picks.emptyAccountMessage == "Tap the bookmark on a book to save it to your library account.")
    }

    @Test func offeredFlagIsPerLibraryAndReset() {
        let defaults = PicksTestData.defaults()
        let store = PicksStore(defaults: defaults)
        let demo = PicksStore(defaults: defaults, namespace: PicksStore.demoNamespace)
        store.markOffered(origin: origin)
        #expect(store.wasOffered(origin: origin))
        #expect(!store.wasOffered(origin: "https://hutt.spydus.co.nz"))
        #expect(!demo.wasOffered(origin: origin))
        demo.markOffered(origin: origin)

        PicksStore.deleteAll(in: defaults)
        #expect(!store.wasOffered(origin: origin))
        #expect(!demo.wasOffered(origin: origin))
    }
}
