import Foundation
import LibreSpydusCore
import Testing
@testable import LibreSpydus

/// In-memory saved pages for test fakes (docs/native-apps.md §5.13): what each key holds, and every
/// save made.
final class SavedPagesFake: @unchecked Sendable {
    private let lock = NSLock()
    private var books: [String: SavedBook] = [:]
    private var shelves: [String: SearchResults] = [:]
    private var links: [FeaturedLink]?
    private var _bookSaves: [String] = []
    private var _shelfSaves: [String] = []
    private var _featuredSaves = 0

    var bookSaves: [String] { lock.withLock { _bookSaves } }
    var shelfSaves: [String] { lock.withLock { _shelfSaves } }
    var featuredSaves: Int { lock.withLock { _featuredSaves } }

    func book(_ key: String) -> SavedBook? { lock.withLock { books[key] } }
    func shelf(_ key: String) -> SearchResults? { lock.withLock { shelves[key] } }
    var featured: [FeaturedLink]? { lock.withLock { links } }

    /// Saved by an earlier run of the app (not counted as a save).
    func setBook(_ book: SavedBook, key: String) { lock.withLock { books[key] = book } }
    func setShelf(_ results: SearchResults, key: String) { lock.withLock { shelves[key] = results } }
    func setFeatured(_ featured: [FeaturedLink]) { lock.withLock { links = featured } }

    func saveBook(_ book: SavedBook, key: String) {
        lock.withLock {
            books[key] = book
            _bookSaves.append(key)
        }
    }

    func saveShelf(_ results: SearchResults, key: String) {
        lock.withLock {
            shelves[key] = results
            _shelfSaves.append(key)
        }
    }

    func saveFeatured(_ featured: [FeaturedLink]) {
        lock.withLock {
            links = featured
            _featuredSaves += 1
        }
    }
}

/// A test fake that keeps saved pages in its `SavedPagesFake` instead of saving nothing.
protocol SavesPagesInMemory: LibraryBackend {
    var saved: SavedPagesFake { get }
}

extension SavesPagesInMemory {
    func savedBook(key: String) -> SavedBook? { saved.book(key) }
    func saveBook(key: String, book: SavedBook) async { saved.saveBook(book, key: key) }
    func savedShelf(key: String) -> SearchResults? { saved.shelf(key) }
    func saveShelf(key: String, results: SearchResults) async { saved.saveShelf(results, key: key) }
    func savedFeatured() -> [FeaturedLink]? { saved.featured }
    func saveFeatured(links: [FeaturedLink]) async { saved.saveFeatured(links) }
}

extension DetailBackend: SavesPagesInMemory {}
extension PagingFakeBackend: SavesPagesInMemory {}
extension FeaturedFakeBackend: SavesPagesInMemory {}

/// What the book page showed when its `record` request went out.
@MainActor
final class SeenOnScreen {
    var asked = false
    var summary: String?
    var similar: [String]?
}

/// Polls `condition` (for work a model starts in its own task) for up to two seconds.
@MainActor
private func eventually(_ condition: () -> Bool) async -> Bool {
    for _ in 0..<200 where !condition() {
        try? await Task.sleep(for: .milliseconds(10))
    }
    return condition()
}

// MARK: - Book page

@MainActor
struct SavedBookPageTests {
    private typealias D = DetailFixtures

    private let key = "work:15974354"
    private let report: BookDetailModel.Reporter = { UserFacingError($0, libraryName: "Canning") }
    private let freshSimilar = RelatedList(items: [D.related(brn: "2", title: "Fresh similar")])
    private let freshAuthor = RelatedList(items: [D.related(brn: "3", title: "Fresh by author")])
    private let savedSimilar = RelatedList(items: [D.related(brn: "1", title: "Saved similar")])

    /// The Midnight Library as saved by an earlier visit: an older summary, and related links from
    /// an older search set.
    private var savedRecord: RecordDetail {
        var record = D.midnightLibrary
        record.summary = "Saved summary"
        record.related = RecordRelated(similar: "https://canning.spydus.com/old-similar",
                                       author: "https://canning.spydus.com/old-author", series: nil, versions: nil)
        return record
    }

    private var savedPage: SavedBook {
        SavedBook(record: savedRecord, similar: savedSimilar, author: nil, series: nil, versions: nil)
    }

    private func watch(_ model: BookDetailModel, on backend: DetailBackend) -> SeenOnScreen {
        let seen = SeenOnScreen()
        backend.beforeRecord = {
            await MainActor.run {
                guard !seen.asked else { return }
                seen.asked = true
                seen.summary = model.record.value?.summary
                seen.similar = model.related[.similar]?.value?.map(\.title)
            }
        }
        return seen
    }

    @Test func theSavedPageShowsBeforeAnyRequestAndTheFreshOneReplacesIt() async {
        let ref = BookRef(TestData.searchRecord)
        let bk = ref.formats[0].availabilityUrl!
        let backend = DetailBackend(records: [bk: .success(D.midnightLibrary)], related: [
            "https://canning.spydus.com/similar": .success(freshSimilar),
            "https://canning.spydus.com/author": .success(freshAuthor),
        ])
        backend.saved.setBook(savedPage, key: key)
        let model = BookDetailModel(ref: ref)
        let seen = watch(model, on: backend)

        await model.load(backend: backend, report: report)

        #expect(seen.summary == "Saved summary")
        #expect(seen.similar == ["Saved similar"])
        #expect(model.record.value == D.midnightLibrary)
        // The fresh record's links are new: its carousel keeps the saved items until they load.
        #expect(model.related[.similar]?.value?.map(\.title) == ["Saved similar"])
        #expect(backend.saved.bookSaves == [key])
        #expect(backend.saved.book(key) == SavedBook(record: D.midnightLibrary, similar: savedSimilar, author: nil,
                                                     series: nil, versions: nil))

        await model.loadRelated(backend: backend)

        #expect(Set(backend.relatedURLs) == ["https://canning.spydus.com/similar", "https://canning.spydus.com/author",
                                             "https://canning.spydus.com/versions"])
        #expect(model.related[.similar]?.value?.map(\.title) == ["Fresh similar"])
        #expect(model.related[.author]?.value?.map(\.title) == ["Fresh by author"])
        #expect(model.visiblePanels == [.similar, .author]) // "versions" failed: hidden
        #expect(backend.saved.bookSaves == [key, key])
        #expect(backend.saved.book(key) == SavedBook(record: D.midnightLibrary, similar: freshSimilar,
                                                     author: freshAuthor, series: nil, versions: nil))
    }

    @Test func aFailedRefreshKeepsTheSavedPageWithoutAnError() async {
        let ref = BookRef(TestData.searchRecord)
        let bk = ref.formats[0].availabilityUrl!
        let backend = DetailBackend(records: [bk: .failure(.Network(message: "offline"))], related: [
            "https://canning.spydus.com/similar": .success(freshSimilar),
        ])
        backend.saved.setBook(savedPage, key: key)
        let reported = SeenOnScreen()
        let model = BookDetailModel(ref: ref)

        await model.load(backend: backend, report: { error in
            reported.asked = true
            return UserFacingError(error, libraryName: "Canning")
        })

        #expect(model.record.value == savedRecord)
        #expect(model.record.error == nil)
        #expect(reported.asked) // still reported: verification would still ask
        #expect(backend.saved.bookSaves.isEmpty)
        // The carousels came into view: the saved record's links are from an old search set, so
        // they wait for a fresh record.
        model.startRelated(backend: backend)
        #expect(backend.relatedURLs.isEmpty)
        #expect(model.related[.similar]?.value?.map(\.title) == ["Saved similar"])

        // The library answers again (e.g. after verification): the page refreshes, and so do the
        // carousels that came into view, from the fresh record's links.
        backend.setRecord(.success(D.midnightLibrary), for: bk)
        await model.load(backend: backend, report: report)
        #expect(model.record.value == D.midnightLibrary)
        #expect(await eventually { model.related[.similar]?.value?.map(\.title) == ["Fresh similar"] })
        #expect(!backend.relatedURLs.contains("https://canning.spydus.com/old-similar"))
    }

    @Test func aWorkLevelRecordDoesNotReplaceASavedBibRecord() async {
        var record = TestData.searchRecord
        record.formats = []
        let ref = BookRef(record) // work-level URL, no formats
        var workLevel = D.midnightLibrary
        workLevel.bookmarkUrl = "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/WRKENQ?SETLVL=&IRN=15642035"
        workLevel.details = []
        workLevel.related = RecordRelated(similar: nil, author: nil, series: nil, versions: nil)
        let backend = DetailBackend(records: [ref.url!: .success(workLevel)])
        backend.saved.setBook(SavedBook(record: D.midnightLibrary, similar: nil, author: nil, series: nil, versions: nil),
                              key: key)
        let model = BookDetailModel(ref: ref)

        await model.load(backend: backend, report: report)

        #expect(backend.recordURLs == [ref.url!])
        #expect(model.record.value == D.midnightLibrary)
        #expect(backend.saved.bookSaves.isEmpty)
    }

    @Test func aFirstVisitSavesTheRecordThenItsLists() async {
        let ref = BookRef(TestData.searchRecord)
        let bk = ref.formats[0].availabilityUrl!
        let backend = DetailBackend(records: [bk: .success(D.midnightLibrary)], related: [
            "https://canning.spydus.com/similar": .success(freshSimilar),
        ])
        let model = BookDetailModel(ref: ref)

        await model.load(backend: backend, report: report)
        #expect(backend.saved.book(key) == SavedBook(record: D.midnightLibrary, similar: nil, author: nil, series: nil,
                                                     versions: nil))
        await model.loadRelated(backend: backend)
        #expect(backend.saved.bookSaves == [key, key])
        #expect(backend.saved.book(key)?.similar == freshSimilar)
        #expect(backend.saved.book(key)?.author == nil) // failed: not saved
    }

    @Test func refsWithoutAStableIdAreNeitherReadNorSaved() async {
        let url = "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/78554992?SETLVL=&BRN=275039"
        let ref = BookRef(url: url)
        #expect(ref.savedPageKey == nil)
        let backend = DetailBackend(records: [url: .success(D.midnightLibrary)])
        backend.saved.setBook(savedPage, key: ref.id)
        let model = BookDetailModel(ref: ref)
        let seen = watch(model, on: backend)

        await model.load(backend: backend, report: report)

        #expect(seen.asked)
        #expect(seen.summary == nil)
        #expect(model.record.value == D.midnightLibrary)
        #expect(backend.saved.bookSaves.isEmpty)
    }

    @Test func savedPageKeys() {
        #expect(BookRef(TestData.searchRecord).savedPageKey == "work:15974354")
        #expect(BookRef(D.related(brn: "467086", title: "The long night")).savedPageKey == "brn:467086")
        let loan = PatronItem(irn: "1", bibIrn: "8753080", title: "The Midnight Library", bibUrl: nil, author: nil,
                              year: nil, coverUrls: [], isbn: nil, fields: [:], actions: [])
        #expect(BookRef(loan).savedPageKey == "bib:8753080")
        #expect(BookRef(url: "https://canning.spydus.com/x").savedPageKey == nil)
        #expect(BookRef(id: "title:untitled", title: "Untitled").savedPageKey == nil)
    }
}

// MARK: - Home

@MainActor
struct SavedHomeTests {
    private typealias S = SearchFixtures
    private let shelfKey = homeShelves()[0].key

    private func app(_ backend: PagingFakeBackend) -> AppModel {
        let dependencies = AppModel.Dependencies(
            defaults: UserDefaults(suiteName: "LibreSpydusTests-saved-home-\(UUID().uuidString)")!,
            sessions: InMemorySecretStore(), verificationCookies: InMemorySecretStore(),
            favouritesDirectory: TestData.temporaryDirectory(), isDemo: false,
            makeBackend: { _ in backend }
        )
        let model = AppModel(dependencies: dependencies)
        model.selectLibrary(.canning)
        return model
    }

    @Test func freshArrivalsReplaceTheSavedOnesAndAreSaved() async {
        let fresh = S.page([S.record("1"), S.record("2")])
        let backend = PagingFakeBackend(first: .success(fresh))
        backend.saved.setShelf(S.page([S.record("9")]), key: shelfKey)
        let home = HomeViewModel()

        await home.loadIfNeeded(app(backend))

        #expect(home.arrivals.value?.map(\.title) == ["Title 1", "Title 2"])
        #expect(backend.saved.shelfSaves == [shelfKey])
        #expect(backend.saved.shelf(shelfKey) == fresh)
    }

    @Test func savedArrivalsStayWhenTheFreshOnesFail() async {
        let backend = PagingFakeBackend(first: .failure(.Network(message: "offline")))
        backend.saved.setShelf(S.page([S.record("9")]), key: shelfKey)
        let home = HomeViewModel()

        await home.loadIfNeeded(app(backend))

        #expect(backend.searchCalls.count == 1)
        #expect(home.arrivals.value?.map(\.title) == ["Title 9"])
        #expect(backend.saved.shelfSaves.isEmpty)
    }

    @Test func anUnrecognisedPageDoesNotReplaceSavedArrivals() async {
        let backend = PagingFakeBackend(first: .success(S.page([], noResults: false)))
        backend.saved.setShelf(S.page([S.record("9")]), key: shelfKey)
        let home = HomeViewModel()

        await home.loadIfNeeded(app(backend))

        #expect(home.arrivals.value?.map(\.title) == ["Title 9"])
        #expect(backend.saved.shelfSaves.isEmpty)
    }

    @Test func withoutSavedArrivalsAFailureShowsAsBefore() async {
        let backend = PagingFakeBackend(first: .failure(.Network(message: "offline")))
        let home = HomeViewModel()

        await home.loadIfNeeded(app(backend))

        #expect(home.arrivals.error?.kind == .network)
    }
}

// MARK: - Featured row

@MainActor
struct SavedFeaturedTests {
    private typealias F = FeaturedFixtures
    private let key = FeaturedRowModel.Key(origin: SearchFixtures.origin, verification: 0)

    @Test func savedTilesStayWhenTheLoadFails() async {
        let backend = FeaturedFakeBackend(featured: .failure(.Network(message: "offline")))
        backend.saved.setFeatured([F.link("Library of Things")])
        let row = FeaturedRowModel()

        await row.loadIfNeeded(backend: backend, key: key)

        #expect(row.links.map(\.title) == ["Library of Things"])
        #expect(backend.saved.featuredSaves == 0)
    }

    @Test func freshTilesReplaceTheSavedOnesAndAreSaved() async {
        let fresh = [F.link("What's New"), F.link("Coming Soon", kind: .search)]
        let backend = FeaturedFakeBackend(featured: .success(fresh))
        backend.saved.setFeatured([F.link("Library of Things")])
        let row = FeaturedRowModel()

        await row.loadIfNeeded(backend: backend, key: key)

        #expect(row.links == fresh)
        #expect(backend.saved.featured == fresh)
    }

    @Test func aLibraryWithoutAListSavesItsEmptyList() async {
        let backend = FeaturedFakeBackend(featured: .success([]))
        backend.saved.setFeatured([F.link("Library of Things")])
        let row = FeaturedRowModel()

        await row.loadIfNeeded(backend: backend, key: key)

        #expect(!row.isVisible)
        #expect(backend.saved.featured == [])
        #expect(backend.saved.featuredSaves == 1)
    }
}
