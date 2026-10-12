import Foundation
import LibreSpydusCore
import Testing
@testable import LibreSpydus

/// An offline backend with scripted search pages (first page from `searchSorted`, later pages by
/// URL). `pageDelay` keeps a page request in flight so tests can race it.
final class PagingFakeBackend: LibraryBackend, @unchecked Sendable {
    let origin = "https://canning.spydus.com"
    var isDemo: Bool { false }

    private let lock = NSLock()
    private var _first: Result<SearchResults, SpydusError>
    private var _pages: [String: Result<SearchResults, SpydusError>]
    private var _searchCalls: [String] = []
    private var _pageCalls: [String] = []
    var pageDelay: Duration = .zero
    var loansResult: Result<PatronList, SpydusError> = .success(PatronList(total: 0, items: []))

    init(first: Result<SearchResults, SpydusError>, pages: [String: Result<SearchResults, SpydusError>] = [:]) {
        _first = first
        _pages = pages
    }

    var searchCalls: [String] { lock.withLock { _searchCalls } }
    var pageCalls: [String] { lock.withLock { _pageCalls } }

    func setFirst(_ result: Result<SearchResults, SpydusError>) { lock.withLock { _first = result } }
    func setPage(_ url: String, _ result: Result<SearchResults, SpydusError>) { lock.withLock { _pages[url] = result } }

    func setVerificationCookie(_ cookie: String?) {}
    func search(query: String) async throws -> SearchResults { try await searchSorted(query: query, sort: .relevance) }
    func searchSorted(query: String, sort: SearchSortOption) async throws -> SearchResults {
        let result = lock.withLock { () -> Result<SearchResults, SpydusError> in
            _searchCalls.append("\(query)|\(sort.rawValue)")
            return _first
        }
        return try result.get()
    }
    func page(url: String) async throws -> SearchResults {
        let result = lock.withLock { () -> Result<SearchResults, SpydusError>? in
            _pageCalls.append(url)
            return _pages[url]
        }
        if pageDelay > .zero { try? await Task.sleep(for: pageDelay) }
        guard let result else { throw SpydusError.Upstream(message: "no page \(url)") }
        return try result.get()
    }
    func availability(url: String) async throws -> Availability { throw SpydusError.Upstream(message: "fake") }
    func record(url: String) async throws -> RecordDetail { throw SpydusError.Upstream(message: "fake") }
    func related(url: String) async throws -> RelatedList { RelatedList(items: []) }
    func login(username: String, password: String) async throws -> String { throw SpydusError.CredentialsRejected(message: "") }
    func account(session: String) async throws -> AccountLinks {
        AccountLinks(name: "Borrower, Test", loansUrl: nil, reservationsReadyUrl: nil, reservationsPendingUrl: nil)
    }
    func loans(session: String) async throws -> PatronList { try loansResult.get() }
    func reservations(session: String) async throws -> Reservations {
        Reservations(ready: PatronList(total: 0, items: []), pending: PatronList(total: 0, items: []))
    }
    func accountSnapshot(session: String) async throws -> AccountSnapshot {
        AccountSnapshot(version: DemoBackend.snapshotVersion, accountName: "Borrower, Test", loans: try loansResult.get(),
                        reservations: try await reservations(session: session),
                        fetchedAtMs: Int64(Date.now.timeIntervalSince1970 * 1000))
    }
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

enum SearchFixtures {
    static let origin = "https://canning.spydus.com"

    static func record(_ irn: String?, url: String? = nil, title: String? = nil,
                       formats: [RecordFormat] = []) -> SearchRecord {
        SearchRecord(rank: nil, workIrn: irn, title: title ?? "Title \(irn ?? "?")", detailUrl: url, author: "Haig, Matt",
                     authorRole: nil, authorUrl: nil, year: "2024", language: nil, summary: nil, isbn: nil,
                     coverUrls: [], formats: formats)
    }

    static func page(_ records: [SearchRecord], next: String? = nil, total: Int64? = 25,
                     noResults: Bool = false) -> SearchResults {
        SearchResults(query: "q", setId: "1", totalResults: total, showing: nil, nextPageUrl: next,
                      prevPageUrl: nil, noResults: noResults, records: records)
    }

    static func pageURL(_ n: Int) -> String { "\(origin)/cgi-bin/spydus.exe/SET/OPAC/ALLWRKENQ/1/\(n)?NREC=10" }

    static func request(_ query: String = "matt haig", sort: SearchSortOption = .relevance, attempt: Int = 0)
        -> SearchResultsModel.Request {
        SearchResultsModel.Request(query: query, sort: sort, origin: origin, verification: 0, attempt: attempt)
    }
}

@MainActor
struct SearchPagingTests {
    private typealias F = SearchFixtures

    /// Mirrors `AppModel.report` for a non-routed error (no sheets).
    private let report: @MainActor (any Error) -> UserFacingError? = { UserFacingError($0, libraryName: "Canning") }

    // MARK: merge / dedupe

    @Test func mergeDropsWorksAlreadyListedByWorkIrnOrDetailUrl() {
        let first = SearchResultsModel.merge([], with: [
            F.record("1", url: "u1"), F.record(nil, url: "u2"), F.record("3"),
        ])
        let merged = SearchResultsModel.merge(first, with: [
            F.record("1", url: "u1-new-set-id"), // same work on the next page (new set id in its URL)
            F.record(nil, url: "u2"),            // same detail URL, no work IRN
            F.record("4"),
            F.record("4"),                       // repeated within one page
        ])
        #expect(merged.map(\.key) == ["work:1", "url:u2", "work:3", "work:4"])
        #expect(merged.prefix(3).map(\.id) == first.map(\.id))
    }

    @Test func recordsWithoutAKeyAreKeptWithUniqueIds() {
        let merged = SearchResultsModel.merge([], with: [F.record(nil), F.record(nil), F.record("9")])
        #expect(merged.count == 3)
        #expect(Set(merged.map(\.id)).count == 3)
        #expect(merged[0].key == nil && merged[1].key == nil)
        let more = SearchResultsModel.merge(merged, with: [F.record(nil)])
        #expect(Set(more.map(\.id)).count == 4)
    }

    @Test func formatChipsAreOnePerLabelAndDigitalIsFlagged() {
        let item = SearchResultsModel.merge([], with: [F.record("1", formats: [
            RecordFormat(code: "BK", label: "Book", reserveUrl: nil, availabilityUrl: nil),
            RecordFormat(code: "BK", label: "Book", reserveUrl: nil, availabilityUrl: nil),
            RecordFormat(code: "EBK", label: "eBook", reserveUrl: nil, availabilityUrl: nil),
        ])])[0]
        #expect(item.formats.map(\.displayLabel) == ["Book", "eBook"])
        #expect(item.formats.map(\.isDigital) == [false, true])
    }

    // MARK: paging

    @Test func loadsTheFirstPageThenAppendsTheNextWithoutDuplicates() async {
        let backend = PagingFakeBackend(
            first: .success(F.page([F.record("1"), F.record("2")], next: F.pageURL(2), total: 1234)),
            pages: [F.pageURL(2): .success(F.page([F.record("2"), F.record("3")], next: F.pageURL(3)))]
        )
        let model = SearchResultsModel()
        await model.load(F.request(sort: .newest), backend: backend, report: report)
        #expect(model.phase == .loaded)
        #expect(model.total == 1234)
        #expect(model.items.map(\.key) == ["work:1", "work:2"])
        #expect(backend.searchCalls == ["matt haig|newest"])

        await model.loadMore()
        #expect(model.items.map(\.key) == ["work:1", "work:2", "work:3"])
        #expect(model.nextPageUrl == F.pageURL(3))
        #expect(backend.pageCalls == [F.pageURL(2)])
    }

    @Test func concurrentLoadMoreFetchesEachPageOnce() async {
        let backend = PagingFakeBackend(
            first: .success(F.page([F.record("1")], next: F.pageURL(2))),
            pages: [F.pageURL(2): .success(F.page([F.record("2")], next: nil))]
        )
        backend.pageDelay = .milliseconds(50)
        let model = SearchResultsModel()
        await model.load(F.request(), backend: backend, report: report)
        async let a: Void = model.loadMore()
        async let b: Void = model.loadMore()
        async let c: Void = model.loadMore()
        _ = await (a, b, c)
        #expect(backend.pageCalls == [F.pageURL(2)])
        #expect(model.items.count == 2)
        #expect(!model.hasMore)
        // At the end, further calls do nothing.
        await model.loadMore()
        #expect(backend.pageCalls.count == 1)
    }

    @Test func aNextLinkBackToAFetchedPageEndsPaging() async {
        let backend = PagingFakeBackend(
            first: .success(F.page([F.record("1")], next: F.pageURL(2))),
            pages: [F.pageURL(2): .success(F.page([F.record("2")], next: F.pageURL(2)))]
        )
        let model = SearchResultsModel()
        await model.load(F.request(), backend: backend, report: report)
        await model.loadMore()
        #expect(model.nextPageUrl == nil)
        await model.loadMore()
        #expect(backend.pageCalls == [F.pageURL(2)])
    }

    @Test func noResultsAndUnrecognisedPagesAreDifferentStates() async {
        let empty = PagingFakeBackend(first: .success(F.page([], total: nil, noResults: true)))
        let model = SearchResultsModel()
        await model.load(F.request("zzqxjv"), backend: empty, report: report)
        #expect(model.phase == .noResults)

        let unrecognised = PagingFakeBackend(first: .success(F.page([], total: nil, noResults: false)))
        await model.load(F.request("other"), backend: unrecognised, report: report)
        #expect(model.phase == .failed(SearchResultsModel.unrecognisedPage))
        #expect(model.items.isEmpty)
    }

    @Test func aFailedNextPageStopsAutoPagingUntilRetried() async {
        let backend = PagingFakeBackend(
            first: .success(F.page([F.record("1")], next: F.pageURL(2))),
            pages: [F.pageURL(2): .success(F.page([], noResults: false))] // unrecognised page
        )
        let model = SearchResultsModel()
        await model.load(F.request(), backend: backend, report: report)
        await model.loadMore()
        #expect(model.loadMoreError == SearchResultsModel.unrecognisedPage)
        #expect(model.items.count == 1)
        #expect(model.hasMore)

        await model.loadMore() // scrolling doesn't retry by itself
        #expect(backend.pageCalls.count == 1)

        backend.setPage(F.pageURL(2), .failure(.Network(message: "timed out")))
        await model.retryLoadMore()
        #expect(model.loadMoreError?.kind == .network)

        backend.setPage(F.pageURL(2), .success(F.page([F.record("2")])))
        await model.retryLoadMore()
        #expect(model.loadMoreError == nil)
        #expect(model.items.map(\.key) == ["work:1", "work:2"])
        #expect(backend.pageCalls.count == 3)
    }

    @Test func aPageForAnOlderSearchIsDropped() async {
        let backend = PagingFakeBackend(
            first: .success(F.page([F.record("1")], next: F.pageURL(2))),
            pages: [F.pageURL(2): .success(F.page([F.record("old-2")]))]
        )
        backend.pageDelay = .milliseconds(100)
        let model = SearchResultsModel()
        await model.load(F.request("first"), backend: backend, report: report)
        async let stalePage: Void = model.loadMore()
        try? await Task.sleep(for: .milliseconds(20))
        backend.setFirst(.success(F.page([F.record("new-1")], next: nil)))
        await model.load(F.request("second"), backend: backend, report: report)
        await stalePage
        #expect(model.items.map(\.key) == ["work:new-1"])
        #expect(!model.isLoadingMore)
    }

    @Test func theSameRequestKeepsLoadedPagesAndANewAttemptReloads() async {
        let backend = PagingFakeBackend(
            first: .success(F.page([F.record("1")], next: F.pageURL(2))),
            pages: [F.pageURL(2): .success(F.page([F.record("2")]))]
        )
        let model = SearchResultsModel()
        await model.load(F.request(), backend: backend, report: report)
        await model.loadMore()
        // Coming back from a detail screen re-runs the view's task with the same request.
        await model.load(F.request(), backend: backend, report: report)
        #expect(model.items.count == 2)
        #expect(backend.searchCalls.count == 1)
        // Return pressed again / Retry: a new attempt searches again from page 1.
        await model.load(F.request(attempt: 1), backend: backend, report: report)
        #expect(model.items.count == 1)
        #expect(backend.searchCalls.count == 2)
    }

    @Test func errorsGoThroughReport() async {
        var reported: [String] = []
        let backend = PagingFakeBackend(first: .failure(.HumanVerificationRequired(message: "human verification required")))
        let model = SearchResultsModel()
        await model.load(F.request(), backend: backend) { error in
            reported.append(String(describing: error))
            return UserFacingError(error, libraryName: "Canning")
        }
        #expect(reported.count == 1)
        #expect(model.phase.failure?.kind == .verificationRequired)
    }
}

private extension SearchResultsModel.Phase {
    var failure: UserFacingError? {
        if case .failed(let error) = self { return error }
        return nil
    }
}

@MainActor
struct SearchRecentStoreTests {
    private func defaults() -> UserDefaults {
        UserDefaults(suiteName: "LibreSpydusTests-recents-\(UUID().uuidString)")!
    }

    @Test func recordsNewestFirstTrimmedAndWithoutCaseInsensitiveRepeats() {
        let store = RecentSearchStore(defaults: defaults())
        store.record("  harry   potter ")
        store.record("Matt Haig")
        store.record("HARRY POTTER")
        store.record("   ")
        #expect(store.items == ["HARRY POTTER", "Matt Haig"])
    }

    @Test func keepsTheLastEight() {
        let store = RecentSearchStore(defaults: defaults())
        for index in 1...10 { store.record("query \(index)") }
        #expect(store.items.count == RecentSearchStore.limit)
        #expect(store.items.first == "query 10")
        #expect(store.items.last == "query 3")
    }

    @Test func persistsRemovesAndClears() {
        let defaults = defaults()
        let store = RecentSearchStore(defaults: defaults)
        store.record("cooking")
        store.record("crime")
        store.record("history")
        #expect(RecentSearchStore(defaults: defaults).items == ["history", "crime", "cooking"])

        store.remove("crime")
        #expect(RecentSearchStore(defaults: defaults).items == ["history", "cooking"])

        store.clear()
        #expect(store.items.isEmpty)
        #expect(RecentSearchStore(defaults: defaults).items.isEmpty)
        #expect(defaults.object(forKey: RecentSearchStore.defaultsKey) == nil)
    }
}
