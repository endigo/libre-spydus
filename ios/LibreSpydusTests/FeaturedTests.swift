import Foundation
import LibreSpydusCore
import Testing
import UIKit
@testable import LibreSpydus

/// An offline backend for the Featured screens: a scripted `featured()` list, collection pages and
/// results pages (each held for `pageDelay`, so concurrent loads overlap); it records the calls
/// and the most pages in flight at once.
final class FeaturedFakeBackend: LibraryBackend, @unchecked Sendable {
    let origin: String
    var isDemo: Bool { false }

    private let lock = NSLock()
    private var _featured: Result<[FeaturedLink], SpydusError>
    private var _collections: [String: Result<FeaturedCollection, SpydusError>]
    private var _pages: [String: Result<SearchResults, SpydusError>]
    private var _featuredCalls = 0
    private var _collectionCalls: [String] = []
    private var _pageCalls: [String] = []
    private var _inFlight = 0
    private var _peakInFlight = 0
    var pageDelay: Duration = .zero
    /// Saved pages (§5.13), in memory.
    let saved = SavedPagesFake()

    init(origin: String = SearchFixtures.origin, featured: Result<[FeaturedLink], SpydusError> = .success([]),
         collections: [String: Result<FeaturedCollection, SpydusError>] = [:],
         pages: [String: Result<SearchResults, SpydusError>] = [:]) {
        self.origin = origin
        _featured = featured
        _collections = collections
        _pages = pages
    }

    var featuredCalls: Int { lock.withLock { _featuredCalls } }
    var collectionCalls: [String] { lock.withLock { _collectionCalls } }
    var pageCalls: [String] { lock.withLock { _pageCalls } }
    var peakInFlight: Int { lock.withLock { _peakInFlight } }

    func setFeatured(_ result: Result<[FeaturedLink], SpydusError>) { lock.withLock { _featured = result } }
    func setPage(_ url: String, _ result: Result<SearchResults, SpydusError>) { lock.withLock { _pages[url] = result } }

    func featured() async throws -> [FeaturedLink] {
        try lock.withLock {
            _featuredCalls += 1
            return _featured
        }.get()
    }

    func collection(url: String) async throws -> FeaturedCollection {
        let result = lock.withLock {
            _collectionCalls.append(url)
            return _collections[url]
        }
        guard let result else { throw SpydusError.Upstream(message: "no collection \(url)") }
        return try result.get()
    }

    func page(url: String) async throws -> SearchResults {
        let result = lock.withLock {
            _pageCalls.append(url)
            _inFlight += 1
            _peakInFlight = max(_peakInFlight, _inFlight)
            return _pages[url]
        }
        if pageDelay > .zero { try? await Task.sleep(for: pageDelay) }
        lock.withLock { _inFlight -= 1 }
        guard let result else { throw SpydusError.Upstream(message: "no page \(url)") }
        return try result.get()
    }

    func setVerificationCookie(_ cookie: String?) {}
    func search(query: String) async throws -> SearchResults { throw SpydusError.Upstream(message: "no search") }
    func searchSorted(query: String, sort: SearchSortOption) async throws -> SearchResults {
        throw SpydusError.Upstream(message: "no search")
    }
    func availability(url: String) async throws -> Availability { throw SpydusError.Upstream(message: "fake") }
    func record(url: String) async throws -> RecordDetail { throw SpydusError.Upstream(message: "fake") }
    func related(url: String) async throws -> RelatedList { RelatedList(items: []) }
    func login(username: String, password: String) async throws -> String { throw SpydusError.CredentialsRejected(message: "") }
    func account(session: String) async throws -> AccountLinks { throw SpydusError.NotLoggedIn(message: "") }
    func loans(session: String) async throws -> PatronList { throw SpydusError.NotLoggedIn(message: "") }
    func reservations(session: String) async throws -> Reservations { throw SpydusError.NotLoggedIn(message: "") }
    func accountSnapshot(session: String) async throws -> AccountSnapshot { throw SpydusError.NotLoggedIn(message: "") }
    func reserveOptions(url: String, session: String) async throws -> ReserveOptions { throw SpydusError.NotLoggedIn(message: "") }
    func placeReservation(url: String, session: String, xdays: String?, pickup: String?) async throws -> ReserveResult {
        throw SpydusError.NotLoggedIn(message: "")
    }
    func cancelReservation(url: String, session: String) async throws -> ReserveResult { throw SpydusError.NotLoggedIn(message: "") }
    func editOptions(url: String, session: String) async throws -> EditOptions { throw SpydusError.NotLoggedIn(message: "") }
    func editReservation(url: String, session: String, overrides: EditOverrides) async throws -> ReserveResult {
        throw SpydusError.NotLoggedIn(message: "")
    }
}

enum FeaturedFixtures {
    static let base = "https://canning.spydus.com/cgi-bin/spydus.exe"

    static func link(_ title: String, kind: FeaturedKind = .collection, icon: String? = nil, url: String? = nil) -> FeaturedLink {
        let slug = title.uppercased().filter(\.isLetter)
        let defaultURL = switch kind {
        case .collection: "\(base)/MSGTRNGEN/OPAC/\(slug)?HOMEPRMS=UD_\(slug)"
        case .search: "\(base)/RUNSQRY/OPAC/BIBENQ?SQRYIRN=4050924&QRYTEXT=\(slug)&NRECS=20"
        case .web: "https://www.example.com/\(slug.lowercased())"
        }
        return FeaturedLink(title: title, description: nil, icon: icon, kind: kind, url: url ?? defaultURL)
    }

    static func sectionURL(_ index: Int) -> String {
        "\(base)/RUNSQRY/OPAC/BIBENQ?SQRYIRN=40503\(index)&QRYTEXT=S\(index)&SETLVL=SET&ISGLB=0&NRECS=20"
    }

    static func collection(sections count: Int, title: String = "Library of Things") -> FeaturedCollection {
        FeaturedCollection(title: title, sections: (0..<count).map { FeaturedSection(title: "Section \($0)", url: sectionURL($0)) })
    }

    /// An edition-level saved-search card: one format with no code or label (core/README.md).
    static func item(_ irn: String, title: String? = nil) -> SearchRecord {
        SearchRecord(rank: nil, workIrn: irn, title: title ?? "Item \(irn) : Musical Instruments",
                     detailUrl: "\(base)/FULL/OPAC/BIBENQ/78607053/\(irn),1", author: nil, authorRole: nil,
                     authorUrl: nil, year: nil, language: nil, summary: "A fun thing to borrow.", isbn: nil,
                     coverUrls: ["https://stspydusproduction.blob.core.windows.net/canninglive-wqup-pub/\(irn)_lt.jpg"],
                     formats: [editionFormat(irn)])
    }

    static func editionFormat(_ irn: String) -> RecordFormat {
        RecordFormat(code: "", label: nil,
                     reserveUrl: "\(base)/PGM/OPAC/CCOPT/78607053/0/V/0/R/9/BIBENQ?SVL=\(irn)&RECFMT=",
                     availabilityUrl: "\(base)/ENQ/OPAC/BIBENQ?SETLVL=&IRN=\(irn)&FMT=RECFMT")
    }
}

// MARK: - Home's row

@MainActor
struct FeaturedRowTests {
    private typealias F = FeaturedFixtures
    private let key = FeaturedRowModel.Key(origin: SearchFixtures.origin, verification: 0)

    @Test func hiddenWhenTheLibraryHasNoFeaturedList() async {
        let backend = FeaturedFakeBackend(featured: .success([]))
        let row = FeaturedRowModel()
        await row.loadIfNeeded(backend: backend, key: key)
        #expect(backend.featuredCalls == 1)
        #expect(!row.isVisible)
        #expect(row.links.isEmpty)
    }

    @Test func hiddenWhenItFails() async {
        let backend = FeaturedFakeBackend(featured: .failure(.Network(message: "timed out")))
        let row = FeaturedRowModel()
        await row.loadIfNeeded(backend: backend, key: key)
        #expect(!row.isVisible)
        // A failed load tries again the next time Home appears.
        backend.setFeatured(.success([F.link("Library of Things")]))
        await row.loadIfNeeded(backend: backend, key: key)
        #expect(row.links.map(\.title) == ["Library of Things"])
    }

    @Test func loadsOncePerKeyAndKeepsTilesWhenARefreshFails() async {
        let links = [F.link("What's New", icon: "book"), F.link("Coming Soon", kind: .search, icon: "balloons")]
        let backend = FeaturedFakeBackend(featured: .success(links))
        let row = FeaturedRowModel()
        await row.loadIfNeeded(backend: backend, key: key)
        #expect(row.isVisible)
        #expect(row.links == links)

        // Back from a pushed screen: nothing reloads.
        await row.loadIfNeeded(backend: backend, key: key)
        #expect(backend.featuredCalls == 1)

        // Pull to refresh that fails keeps the tiles.
        backend.setFeatured(.failure(.Network(message: "timed out")))
        await row.refresh(backend: backend, key: key)
        #expect(backend.featuredCalls == 2)
        #expect(row.links == links)
    }

    @Test func anotherLibrarysTilesNeverShow() async {
        let canning = FeaturedFakeBackend(featured: .success([F.link("Library of Things")]))
        let row = FeaturedRowModel()
        await row.loadIfNeeded(backend: canning, key: key)
        #expect(row.isVisible)

        let hutt = FeaturedFakeBackend(origin: "https://hutt.spydus.co.nz", featured: .failure(.Network(message: "timed out")))
        await row.loadIfNeeded(backend: hutt, key: FeaturedRowModel.Key(origin: hutt.origin, verification: 0))
        #expect(!row.isVisible)
    }

    // MARK: Tap routing

    @Test func aCollectionOpensTheCollectionScreen() {
        let link = F.link("Library of Things", icon: "game-board")
        #expect(FeaturedDestination(link) == .route(.collection(link)))
    }

    @Test func aSavedSearchOpensItsResultsListTitledWithTheLink() {
        let link = F.link("Coming Soon", kind: .search)
        #expect(FeaturedDestination(link) == .route(.resultsPage(url: link.url, title: "Coming Soon")))
    }

    @Test func aWebLinkOpensTheBrowser() throws {
        let link = F.link("Kanopy", kind: .web, url: "https://canning.kanopy.com/")
        #expect(FeaturedDestination(link) == .web(try #require(URL(string: "https://canning.kanopy.com/"))))
        // The browser opens http(s) pages only.
        #expect(FeaturedDestination(F.link("Mail", kind: .web, url: "mailto:library@canning.wa.gov.au")) == nil)
        #expect(FeaturedDestination(F.link("Broken", kind: .web, url: "not a url")) == nil)
    }

    // MARK: Icons

    @Test(arguments: [
        ("book", "book.fill"),
        ("balloons", "party.popper.fill"),
        ("computer", "desktopcomputer"),
        ("circle", "circle.fill"),
        ("globe", "globe"),
        ("game-board", "puzzlepiece.fill"),
        ("map-location-dot", "map.fill"),
        ("user-circle", "person.crop.circle.fill"),
    ])
    func canningsIconsMapToSymbols(icon: String, symbol: String) {
        #expect(FeaturedIcon.systemImage(for: icon) == symbol)
        #expect(UIImage(systemName: symbol) != nil, "missing SF Symbol \(symbol)")
    }

    @Test func iconNamesAreNormalisedAndUnknownOnesGetSparkles() {
        #expect(FeaturedIcon.systemImage(for: "fa-globe") == "globe")
        #expect(FeaturedIcon.systemImage(for: " Game-Board ") == "puzzlepiece.fill")
        #expect(FeaturedIcon.systemImage(for: "rocket") == "sparkles")
        #expect(FeaturedIcon.systemImage(for: "") == "sparkles")
        #expect(FeaturedIcon.systemImage(for: nil) == "sparkles")
        #expect(UIImage(systemName: FeaturedIcon.fallback) != nil)
    }
}

// MARK: - Collection screen

@MainActor
struct CollectionModelTests {
    private typealias F = FeaturedFixtures
    private let report: @MainActor (any Error) -> UserFacingError? = { UserFacingError($0, libraryName: "Canning") }
    private let request = CollectionModel.Request(origin: SearchFixtures.origin, verification: 0)

    private func pages(_ count: Int, items: Int = 3) -> [String: Result<SearchResults, SpydusError>] {
        Dictionary(uniqueKeysWithValues: (0..<count).map { index in
            (F.sectionURL(index), .success(SearchFixtures.page((0..<items).map { F.item("\(index)0\($0)") }, total: Int64(items))))
        })
    }

    @Test func oneShelfPerSectionLoadedOnlyWhenItComesIntoView() async {
        let link = F.link("Library of Things")
        let backend = FeaturedFakeBackend(collections: [link.url: .success(F.collection(sections: 9))], pages: pages(9))
        let model = CollectionModel(link: link)
        await model.load(request, backend: backend, report: report)

        #expect(model.title == "Library of Things")
        #expect(model.phase == .shelves)
        #expect(model.shelves.map(\.section.title) == (0..<9).map { "Section \($0)" })
        #expect(model.shelves.allSatisfy { $0.state == .idle })
        #expect(backend.pageCalls.isEmpty)

        model.shelfAppeared(0)
        model.shelfAppeared(1)
        await model.settle()
        #expect(Set(backend.pageCalls) == [F.sectionURL(0), F.sectionURL(1)])
        #expect(model.shelves[0].state == .loaded(F.itemsAsRefs(["000", "001", "002"])))
        #expect(model.shelves[2].state == .idle)

        // Appearing again doesn't fetch again.
        model.shelfAppeared(0)
        await model.settle()
        #expect(backend.pageCalls.count == 2)
        #expect(model.shelves[4].route == .resultsPage(url: F.sectionURL(4), title: "Section 4"))
    }

    @Test func atMostThreeShelvesLoadAtOnce() async {
        let link = F.link("What's New")
        let backend = FeaturedFakeBackend(collections: [link.url: .success(F.collection(sections: 11))], pages: pages(11))
        backend.pageDelay = .milliseconds(40)
        let model = CollectionModel(link: link)
        await model.load(request, backend: backend, report: report)

        for index in model.shelves.indices { model.shelfAppeared(index) }
        #expect(model.shelves.filter { $0.state == .loading }.count == CollectionModel.maxConcurrentShelves)
        #expect(model.shelves.filter { $0.state == .queued }.count == 8)
        await model.settle()

        #expect(backend.pageCalls.count == 11)
        #expect(backend.peakInFlight == 3)
        #expect(model.peakConcurrentShelves == 3)
        #expect(model.shelves.allSatisfy { $0.state.isLoaded })
        #expect(Set(backend.pageCalls) == Set((0..<11).map(F.sectionURL)))
    }

    @Test func aShelfScrolledPastBeforeItsTurnWaitsUntilItIsBack() async {
        let link = F.link("What's New")
        let backend = FeaturedFakeBackend(collections: [link.url: .success(F.collection(sections: 6))], pages: pages(6))
        backend.pageDelay = .milliseconds(20)
        let model = CollectionModel(link: link)
        await model.load(request, backend: backend, report: report)

        for index in 0..<5 { model.shelfAppeared(index) }
        model.shelfDisappeared(3)
        model.shelfDisappeared(0) // already loading: carries on
        await model.settle()
        #expect(Set(backend.pageCalls) == Set([0, 1, 2, 4].map(F.sectionURL)))
        #expect(model.shelves[3].state == .idle)
        #expect(model.shelves[0].state.isLoaded)
    }

    @Test func anEmptySectionIsHiddenAndAFailedOneOffersRetry() async {
        let link = F.link("Local History")
        var scripted = pages(3)
        scripted[F.sectionURL(1)] = .success(SearchFixtures.page([], total: 0, noResults: true))
        scripted[F.sectionURL(2)] = .failure(.Network(message: "timed out"))
        let backend = FeaturedFakeBackend(collections: [link.url: .success(F.collection(sections: 3))], pages: scripted)
        let model = CollectionModel(link: link)
        await model.load(request, backend: backend, report: report)
        for index in model.shelves.indices { model.shelfAppeared(index) }
        await model.settle()

        #expect(model.visibleShelves.map(\.index) == [0, 2])
        #expect(model.shelves[2].state == .failed(UserFacingError(SpydusError.Network(message: "timed out"), libraryName: "Canning")))
        #expect(!model.isEmpty)

        // Appearing doesn't retry a failure on its own; Retry does.
        model.shelfAppeared(2)
        await model.settle()
        #expect(backend.pageCalls.count == 3)
        backend.setPage(F.sectionURL(2), .success(SearchFixtures.page([F.item("9")], total: 1)))
        model.retryShelf(2)
        await model.settle()
        #expect(backend.pageCalls.count == 4)
        #expect(model.shelves[2].state == .loaded(F.itemsAsRefs(["9"])))
    }

    @Test func anUnrecognisedSectionPageIsAFailure() async {
        let link = F.link("LOTE")
        var scripted = pages(1)
        scripted[F.sectionURL(0)] = .success(SearchFixtures.page([], total: nil, noResults: false))
        let backend = FeaturedFakeBackend(collections: [link.url: .success(F.collection(sections: 1))], pages: scripted)
        let model = CollectionModel(link: link)
        await model.load(request, backend: backend, report: report)
        model.shelfAppeared(0)
        await model.settle()
        #expect(model.shelves[0].state == .failed(SearchResultsModel.unrecognisedPage))
    }

    @Test func everySectionEmptyIsAnEmptyCollection() async {
        let link = F.link("More to Celebrate")
        let empty = SearchFixtures.page([], total: 0, noResults: true)
        let backend = FeaturedFakeBackend(collections: [link.url: .success(F.collection(sections: 2))],
                                          pages: [F.sectionURL(0): .success(empty), F.sectionURL(1): .success(empty)])
        let model = CollectionModel(link: link)
        await model.load(request, backend: backend, report: report)
        #expect(!model.isEmpty) // not known yet
        model.shelfAppeared(0)
        model.shelfAppeared(1)
        await model.settle()
        #expect(model.visibleShelves.isEmpty)
        #expect(model.isEmpty)
    }

    @Test func aPageWithoutSectionsIsOnTheLibrarysWebsite() async {
        let link = F.link("eLibrary", icon: "computer")
        let backend = FeaturedFakeBackend(collections: [link.url: .success(FeaturedCollection(title: "eLibrary", sections: []))])
        let model = CollectionModel(link: link)
        await model.load(request, backend: backend, report: report)
        #expect(model.phase == .noSections)
        #expect(model.visibleShelves.isEmpty)
        #expect(InAppBrowser.webURL(link.url)?.absoluteString == link.url)
    }

    @Test func aFailedCollectionIsAnErrorAndTheSameRequestDoesNotReload() async {
        let link = F.link("First Nations")
        let backend = FeaturedFakeBackend(collections: [link.url: .failure(.Network(message: "timed out"))])
        let model = CollectionModel(link: link)
        await model.load(request, backend: backend, report: report)
        #expect(model.phase == .failed(UserFacingError(SpydusError.Network(message: "timed out"), libraryName: "Canning")))
        await model.load(request, backend: backend, report: report)
        #expect(backend.collectionCalls.count == 1)

        // Retry (a new attempt) loads again.
        var retry = request
        retry.attempt += 1
        await model.load(retry, backend: backend, report: report)
        #expect(backend.collectionCalls.count == 2)
    }

    @Test func aShelfShowsAtMostOnePageOfCoversWithoutDuplicates() {
        let records = (0..<25).map { F.item(String($0)) } + [F.item("3")]
        let covers = CollectionModel.covers(records)
        #expect(covers.count == CollectionModel.shelfLimit)
        #expect(Set(covers.map(\.id)).count == covers.count)
    }
}

// MARK: - Results list by URL

@MainActor
struct ResultsPageTests {
    private typealias F = FeaturedFixtures
    private let report: @MainActor (any Error) -> UserFacingError? = { UserFacingError($0, libraryName: "Canning") }

    private func request(_ url: String) -> SearchResultsModel.Request {
        SearchResultsModel.Request(query: "Coming Soon", sort: .relevance, url: url, origin: SearchFixtures.origin,
                                   verification: 0)
    }

    @Test func startsFromTheURLAndPagesThroughNextPageUrl() async {
        let first = F.link("Coming Soon", kind: .search).url
        let second = "\(F.base)/RUNSQRY/OPAC/BIBENQ/78607100?SETLVL=SET&PAGE=2"
        let backend = PagingFakeBackend(first: .failure(.Upstream(message: "no search")), pages: [
            first: .success(SearchFixtures.page((0..<20).map { F.item("1\($0)") }, next: second, total: 25)),
            second: .success(SearchFixtures.page((0..<5).map { F.item("2\($0)") } + [F.item("10")], next: first, total: 25)),
        ])
        let results = SearchResultsModel()
        await results.load(request(first), backend: backend, report: report)

        #expect(backend.searchCalls.isEmpty)
        #expect(backend.pageCalls == [first])
        #expect(results.phase == .loaded)
        #expect(results.total == 25)
        #expect(results.items.count == 20)
        #expect(results.hasMore)

        await results.loadMore()
        #expect(backend.pageCalls == [first, second])
        #expect(results.items.count == 25) // the repeat of item 10 is dropped
        // A next link back to the first page would loop: the list ends.
        #expect(!results.hasMore)
    }

    @Test func anEmptySavedSearchIsNoResults() async {
        let url = F.link("Coming Soon", kind: .search).url
        let backend = PagingFakeBackend(first: .failure(.Upstream(message: "no search")),
                                        pages: [url: .success(SearchFixtures.page([], total: 0, noResults: true))])
        let results = SearchResultsModel()
        await results.load(request(url), backend: backend, report: report)
        #expect(results.phase == .noResults)
    }

    @Test func theSameURLAgainKeepsThePagesAlreadyLoaded() async {
        let url = F.link("Coming Soon", kind: .search).url
        let backend = PagingFakeBackend(first: .failure(.Upstream(message: "no search")),
                                        pages: [url: .success(SearchFixtures.page([F.item("1")], total: 1))])
        let results = SearchResultsModel()
        await results.load(request(url), backend: backend, report: report)
        await results.load(request(url), backend: backend, report: report)
        #expect(backend.pageCalls == [url])
    }
}

extension FeaturedFixtures {
    /// The covers a shelf shows for these edition-level items.
    static func itemsAsRefs(_ irns: [String]) -> [BookRef] {
        irns.map { BookRef(item($0)) }
    }
}

private extension CollectionModel.ShelfState {
    var isLoaded: Bool {
        if case .loaded = self { return true }
        return false
    }
}
