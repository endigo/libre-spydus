import Foundation
import LibreSpydusCore
import Testing
@testable import LibreSpydus

/// Counts directory fetches and answers with a canned result.
private final class FetchStub: @unchecked Sendable {
    private let lock = NSLock()
    private var _calls = 0
    var result: Result<[LibraryInfo], SpydusError>

    init(_ result: Result<[LibraryInfo], SpydusError>) {
        self.result = result
    }

    var calls: Int { lock.withLock { _calls } }

    func fetch() async throws -> [LibraryInfo] {
        let result = lock.withLock {
            _calls += 1
            return self.result
        }
        return try result.get()
    }
}

@MainActor
struct OnboardingLibraryPickerTests {
    let now = Date(timeIntervalSince1970: 1_800_000_000)
    let day: TimeInterval = 24 * 60 * 60

    static let cached = [OnboardingLibraryDirectoryTests.library("City of Canning", region: "AU", country: "Australia")]
    static let fetched = OnboardingLibraryDirectoryTests.sample

    func temporaryCache() -> LibraryDirectoryCache {
        LibraryDirectoryCache(fileURL: TestData.temporaryDirectory().appending(path: "nested/library-directory.json"))
    }

    // MARK: Freshness

    @Test func freshForUpToSevenDays() {
        #expect(LibraryDirectoryCache.isFresh(fetchedAt: now, now: now))
        #expect(LibraryDirectoryCache.isFresh(fetchedAt: now - 6 * day, now: now))
        #expect(LibraryDirectoryCache.isFresh(fetchedAt: now - 7 * day + 1, now: now))
        #expect(!LibraryDirectoryCache.isFresh(fetchedAt: now - 7 * day, now: now))
        #expect(!LibraryDirectoryCache.isFresh(fetchedAt: now - 30 * day, now: now))
    }

    @Test func aFetchDateInTheFutureIsStale() {
        #expect(!LibraryDirectoryCache.isFresh(fetchedAt: now + 60, now: now))
    }

    // MARK: Disk cache

    @Test func cacheRoundTripsEveryField() throws {
        let cache = temporaryCache()
        #expect(cache.load() == nil)
        let library = LibraryInfo(id: "x", name: "Ünïcode Library", origin: "https://x.spydus.co.uk",
                                  logoUrl: "https://x.spydus.co.uk/logo.png", country: nil, region: "UK", isTest: true)
        try cache.save(.init(fetchedAt: now, libraries: [library]))
        let loaded = try #require(cache.load())
        #expect(loaded.fetchedAt == now)
        #expect(loaded.libraries == [library])
        cache.remove()
        #expect(cache.load() == nil)
    }

    @Test func unreadableCacheIsIgnored() throws {
        let cache = temporaryCache()
        try FileManager.default.createDirectory(at: cache.fileURL.deletingLastPathComponent(), withIntermediateDirectories: true)
        try Data("not json".utf8).write(to: cache.fileURL)
        #expect(cache.load() == nil)
    }

    // MARK: Picker model

    @Test func freshCacheOpensWithoutFetching() async throws {
        let cache = temporaryCache()
        try cache.save(.init(fetchedAt: now - day, libraries: Self.cached))
        let stub = FetchStub(.success(Self.fetched))
        let picker = LibraryPickerModel(fetch: stub.fetch, cache: cache, now: { now })
        await picker.load()
        #expect(stub.calls == 0)
        #expect(picker.libraries == Self.cached)
        await picker.load()
        #expect(stub.calls == 0)
    }

    @Test func staleCacheIsShownThenRefreshedAndSaved() async throws {
        let cache = temporaryCache()
        try cache.save(.init(fetchedAt: now - 8 * day, libraries: Self.cached))
        let stub = FetchStub(.success(Self.fetched))
        let picker = LibraryPickerModel(fetch: stub.fetch, cache: cache, now: { now })
        await picker.load()
        #expect(stub.calls == 1)
        #expect(picker.libraries == Self.fetched)
        #expect(picker.fetchedAt == now)
        #expect(cache.load() == .init(fetchedAt: now, libraries: Self.fetched))
    }

    @Test func noCacheFetchesAndCaches() async {
        let cache = temporaryCache()
        let stub = FetchStub(.success(Self.fetched))
        let picker = LibraryPickerModel(fetch: stub.fetch, cache: cache, now: { now })
        #expect(picker.libraries.isEmpty)
        await picker.load()
        #expect(stub.calls == 1)
        #expect(picker.sections.map(\.region) == [.au, .nz, .us, .uk, .sg, .other])
        #expect(cache.load()?.libraries == Self.fetched)
    }

    @Test func firstFetchFailureShowsAnErrorAndRetryRecovers() async {
        let stub = FetchStub(.failure(.Network(message: "connect timed out after 10s")))
        let picker = LibraryPickerModel(fetch: stub.fetch, cache: temporaryCache(), now: { now })
        await picker.load()
        let error = picker.loadError
        #expect(error?.kind == .network)
        #expect(error?.message == "Can’t reach the library directory. Check your connection.")
        #expect(error?.detail == "connect timed out after 10s")
        #expect(!picker.isLoading)

        stub.result = .success(Self.fetched)
        await picker.retry()
        #expect(picker.loadError == nil)
        #expect(picker.libraries == Self.fetched)
    }

    @Test func refreshFailureKeepsTheCachedList() async throws {
        let cache = temporaryCache()
        try cache.save(.init(fetchedAt: now - 8 * day, libraries: Self.cached))
        let stub = FetchStub(.failure(.Upstream(message: "HTTP 500")))
        let picker = LibraryPickerModel(fetch: stub.fetch, cache: cache, now: { now })
        await picker.load()
        #expect(picker.libraries == Self.cached)
        #expect(picker.loadError == nil)
        #expect(picker.refreshError?.message == "The library directory returned something unexpected.")
        // The stale snapshot isn't overwritten.
        #expect(cache.load()?.fetchedAt == now - 8 * day)

        stub.result = .success(Self.fetched)
        await picker.refresh()
        #expect(picker.refreshError == nil)
        #expect(picker.libraries == Self.fetched)
    }

    @Test func searchingFiltersTheSections() async {
        let picker = LibraryPickerModel(fetch: FetchStub(.success(Self.fetched)).fetch, cache: temporaryCache(),
                                        now: { now })
        await picker.load()
        picker.query = "kingdom"
        #expect(picker.isSearching)
        #expect(picker.sections.map(\.region) == [.uk])
        picker.query = " "
        #expect(!picker.isSearching)
        #expect(picker.sections.count == 6)
    }
}
