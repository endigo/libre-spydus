import Foundation
import LibreSpydusCore
import Testing
@testable import LibreSpydus

private let canning = "https://canning.spydus.com"
private let hutt = "https://hutt.spydus.co.nz"

/// A backend whose `accountSnapshot` answers are scripted: each call takes the next result (the
/// last one repeats) and answers after `delay`.
private final class SnapshotBackend: LibraryBackend, @unchecked Sendable {
    let origin: String
    var isDemo: Bool { false }

    private let lock = NSLock()
    private var _results: [Result<AccountSnapshot, SpydusError>]
    private var _calls = 0
    private let delay: Duration

    init(origin: String = canning, delay: Duration = .zero, _ results: Result<AccountSnapshot, SpydusError>...) {
        self.origin = origin
        self.delay = delay
        _results = results
    }

    var calls: Int { lock.withLock { _calls } }

    func setResults(_ results: Result<AccountSnapshot, SpydusError>...) {
        lock.withLock { _results = results }
    }

    func accountSnapshot(session: String) async throws -> AccountSnapshot {
        let result = lock.withLock {
            _calls += 1
            return _results.count > 1 ? _results.removeFirst() : _results[0]
        }
        if delay > .zero { try await Task.sleep(for: delay) }
        return try result.get()
    }

    func setVerificationCookie(_ cookie: String?) {}
    func search(query: String) async throws -> SearchResults { throw SpydusError.Upstream(message: "unused") }
    func searchSorted(query: String, sort: SearchSortOption) async throws -> SearchResults {
        throw SpydusError.Upstream(message: "unused")
    }
    func page(url: String) async throws -> SearchResults { throw SpydusError.Upstream(message: "unused") }
    func availability(url: String) async throws -> Availability { throw SpydusError.Upstream(message: "unused") }
    func record(url: String) async throws -> RecordDetail { throw SpydusError.Upstream(message: "unused") }
    func related(url: String) async throws -> RelatedList { throw SpydusError.Upstream(message: "unused") }
    func login(username: String, password: String) async throws -> String { throw SpydusError.Upstream(message: "unused") }
    func account(session: String) async throws -> AccountLinks { throw SpydusError.Upstream(message: "unused") }
    func loans(session: String) async throws -> PatronList { throw SpydusError.Upstream(message: "unused") }
    func reservations(session: String) async throws -> Reservations { throw SpydusError.Upstream(message: "unused") }
    func reserveOptions(url: String, session: String) async throws -> ReserveOptions { throw SpydusError.Upstream(message: "unused") }
    func placeReservation(url: String, session: String, xdays: String?, pickup: String?) async throws -> ReserveResult {
        throw SpydusError.Upstream(message: "unused")
    }
    func cancelReservation(url: String, session: String) async throws -> ReserveResult { throw SpydusError.Upstream(message: "unused") }
    func editOptions(url: String, session: String) async throws -> EditOptions { throw SpydusError.Upstream(message: "unused") }
    func editReservation(url: String, session: String, overrides: EditOverrides) async throws -> ReserveResult {
        throw SpydusError.Upstream(message: "unused")
    }
}

/// Files in `directory` (none when it doesn't exist).
private func files(in directory: URL) -> [String] {
    ((try? FileManager.default.contentsOfDirectory(atPath: directory.path)) ?? []).filter { $0.hasSuffix(".json") }
}

// MARK: - Saved file

struct AccountSnapshotCacheTests {
    @Test func savesLoadsAndRemovesEachLibrarySeparately() throws {
        let cache = AccountSnapshotCache(directory: TestData.temporaryDirectory().appending(path: "AccountCache"))
        let mine = TestData.snapshot(loans: ["Dune"], reservations: ["Emma"])
        let theirs = TestData.snapshot(name: "Other", loans: ["Hamlet"])
        try cache.save(mine, origin: canning)
        try cache.save(theirs, origin: hutt)

        #expect(cache.load(origin: canning) == mine)
        #expect(cache.load(origin: hutt) == theirs)
        #expect(cache.load(origin: "https://other.spydus.com") == nil)

        cache.remove(origin: canning)
        #expect(cache.load(origin: canning) == nil)
        #expect(cache.load(origin: hutt) == theirs)
    }

    @Test func theFileIsNamedByHashAndKeptOutOfBackups() throws {
        let cache = AccountSnapshotCache(directory: TestData.temporaryDirectory().appending(path: "AccountCache"))
        try cache.save(TestData.snapshot(), origin: canning)
        let file = cache.fileURL(for: canning)
        // sha256("https://canning.spydus.com")
        #expect(file.lastPathComponent == "e20e5e3f49866bfbb2a5eeaa2e2a33ca4ee95304db1f2738eca4584b1bc229c6.json")
        #expect(try file.resourceValues(forKeys: [.isExcludedFromBackupKey]).isExcludedFromBackup == true)
        #expect(try cache.directory.resourceValues(forKeys: [.isExcludedFromBackupKey]).isExcludedFromBackup == true)
    }

    @Test func aCorruptFileIsDiscarded() throws {
        let cache = AccountSnapshotCache(directory: TestData.temporaryDirectory())
        try Data("{\"version\": 1, \"loans\": ".utf8).write(to: cache.fileURL(for: canning))
        #expect(cache.load(origin: canning) == nil)
        #expect(!FileManager.default.fileExists(atPath: cache.fileURL(for: canning).path))
    }

    @Test func anotherSnapshotVersionIsDiscarded() throws {
        let cache = AccountSnapshotCache(directory: TestData.temporaryDirectory())
        let json = encodeAccountSnapshot(snapshot: TestData.snapshot(loans: ["Dune"]))
        let future = json.replacingOccurrences(of: #""version"\s*:\s*1"#, with: "\"version\":99", options: .regularExpression)
        #expect(future != json)
        try Data(future.utf8).write(to: cache.fileURL(for: canning))
        #expect(cache.load(origin: canning) == nil)
        #expect(!FileManager.default.fileExists(atPath: cache.fileURL(for: canning).path))
    }
}

// MARK: - Store

@MainActor
struct AccountStoreTests {
    private func makeStore(cache: AccountSnapshotCache? = nil, now: @escaping () -> Date = { .now }) -> AccountStore {
        let store = AccountStore(cache: cache, now: now)
        store.report = { UserFacingError($0, libraryName: "Canning") }
        return store
    }

    @Test func concurrentRefreshesShareOneFetch() async {
        let backend = SnapshotBackend(delay: .milliseconds(200), .success(TestData.snapshot(loans: ["Dune"])))
        let store = makeStore()
        store.use(origin: canning, session: "s", backend: backend)

        async let first: Void = store.refresh(force: true)
        async let second: Void = store.refresh()
        async let third: Void = store.refresh(force: true)
        try? await Task.sleep(for: .milliseconds(50))
        #expect(store.isRefreshing)
        _ = await (first, second, third)

        #expect(backend.calls == 1)
        #expect(store.snapshot?.loans.items.map(\.title) == ["Dune"])
        #expect(!store.isRefreshing)
    }

    @Test func onlyAnOutOfDateSnapshotIsFetchedAgain() async {
        let fetchedAt = Date(timeIntervalSince1970: 1_800_000_000)
        var clock = fetchedAt
        let backend = SnapshotBackend(.success(TestData.snapshot(fetchedAt: fetchedAt)))
        let store = makeStore { clock }
        #expect(!store.needsRefresh) // signed out
        store.use(origin: canning, session: "s", backend: backend)
        #expect(store.needsRefresh) // nothing yet
        await store.refresh()
        #expect(backend.calls == 1)

        clock = fetchedAt.addingTimeInterval(AccountStore.staleAfter - 1)
        #expect(!store.needsRefresh)
        await store.refresh()
        #expect(backend.calls == 1)
        await store.refresh(force: true) // pull to refresh
        #expect(backend.calls == 2)

        clock = fetchedAt.addingTimeInterval(AccountStore.staleAfter)
        #expect(store.needsRefresh)
        await store.refresh()
        #expect(backend.calls == 3)

        // A snapshot from the future (the clock was changed) is out of date too.
        clock = fetchedAt.addingTimeInterval(-AccountStore.staleAfter - 1)
        #expect(store.needsRefresh)
    }

    @Test func aFailedRefreshKeepsTheData() async {
        let saved = TestData.snapshot(loans: ["Dune"])
        let backend = SnapshotBackend(.success(saved), .failure(.Network(message: "timed out")))
        let store = makeStore()
        store.use(origin: canning, session: "s", backend: backend)
        await store.refresh()

        await store.refresh(force: true)
        #expect(store.snapshot == saved)
        #expect(store.lastRefreshError?.kind == .network)
        #expect(!store.isRefreshing)
        // A failure is retried the next time a screen opens, and a success clears it.
        #expect(store.needsRefresh)
        backend.setResults(.success(TestData.snapshot(loans: ["Dune", "Emma"])))
        await store.refresh()
        #expect(store.lastRefreshError == nil)
        #expect(store.snapshot?.loans.items.count == 2)
    }

    @Test func aResultForAnotherLibraryOrSessionIsDropped() async {
        let backend = SnapshotBackend(delay: .milliseconds(150), .success(TestData.snapshot(loans: ["Canning loan"])))
        let store = makeStore()
        store.use(origin: canning, session: "s", backend: backend)
        let late = Task { await store.refresh() }
        try? await Task.sleep(for: .milliseconds(30))

        let other = SnapshotBackend(origin: hutt, .success(TestData.snapshot(loans: ["Hutt loan"])))
        store.use(origin: hutt, session: "h", backend: other)
        #expect(!store.isRefreshing)
        await late.value
        #expect(store.snapshot == nil)
        await store.refresh()
        #expect(store.snapshot?.loans.items.map(\.title) == ["Hutt loan"])

        // Signed out while fetching: nothing comes back either.
        let slow = SnapshotBackend(origin: hutt, delay: .milliseconds(100), .success(TestData.snapshot(loans: ["Late"])))
        store.use(origin: hutt, session: "h2", backend: slow)
        let signedOut = Task { await store.refresh() }
        try? await Task.sleep(for: .milliseconds(20))
        store.clear(origin: hutt)
        await signedOut.value
        #expect(store.snapshot == nil)
    }

    @Test func aFetchOverlappingAChangeIsDroppedAndFetchedAgain() async throws {
        let before = TestData.snapshot(reservations: ["Emma"])
        let after = TestData.snapshot(reservations: [])
        let backend = SnapshotBackend(delay: .milliseconds(150), .success(before), .success(before), .success(after))
        let store = makeStore()
        store.use(origin: canning, session: "s", backend: backend)
        await store.refresh()

        let refresh = Task { await store.refresh(force: true) } // fetched before the cancel lands
        try await Task.sleep(for: .milliseconds(30))
        let accepted = try await store.performChange {
            try await Task.sleep(for: .milliseconds(30))
            return true
        } accepted: { $0 }
        #expect(accepted)
        await refresh.value
        #expect(backend.calls == 3)
        #expect(store.snapshot == after)

        // Refused changes don't fetch again.
        _ = try await store.performChange { false } accepted: { $0 }
        #expect(!store.isRefreshing)
        #expect(backend.calls == 3)
    }

    @Test func aHoldPlacedElsewhereFetchesAgain() async {
        let backend = SnapshotBackend(.success(TestData.snapshot()), .success(TestData.snapshot(reservations: ["Emma"])))
        let store = makeStore()
        store.use(origin: canning, session: "s", backend: backend)
        await store.refresh()
        store.libraryDidChange()
        #expect(store.isRefreshing)
        await store.refresh()
        #expect(store.snapshot?.reservations.pending.items.map(\.title) == ["Emma"])
    }

    @Test func anEditInPlaceIsSaved() async throws {
        let cache = AccountSnapshotCache(directory: TestData.temporaryDirectory())
        let backend = SnapshotBackend(.success(TestData.snapshot(reservations: ["Emma", "Dune"])))
        let store = makeStore(cache: cache)
        store.use(origin: canning, session: "s", backend: backend)
        await store.refresh()
        let href = try #require(store.snapshot?.reservations.pending.items.first?.actions.first?.href)
        store.update { $0 = $0.removingReservation(actionHref: href) }
        #expect(store.snapshot?.reservations.pending.items.map(\.title) == ["Dune"])
        #expect(store.snapshot?.reservations.pending.total == 1)
        #expect(cache.load(origin: canning)?.reservations.pending.items.map(\.title) == ["Dune"])
    }

    @Test func eachLibraryShowsItsOwnSavedAccount() async {
        let cache = AccountSnapshotCache(directory: TestData.temporaryDirectory())
        let store = makeStore(cache: cache)
        store.use(origin: canning, session: "c", backend: SnapshotBackend(.success(TestData.snapshot(loans: ["Canning loan"]))))
        await store.refresh()
        store.use(origin: hutt, session: "h", backend: SnapshotBackend(origin: hutt, .success(TestData.snapshot(loans: ["Hutt loan"]))))
        #expect(store.snapshot == nil)
        await store.refresh()

        store.use(origin: canning, session: "c", backend: SnapshotBackend(.failure(.Network(message: "offline"))))
        #expect(store.snapshot?.loans.items.map(\.title) == ["Canning loan"]) // straight away, from the file
        #expect(cache.load(origin: hutt)?.loans.items.map(\.title) == ["Hutt loan"])
    }

    @Test func anotherSignInAtTheSameLibraryForgetsTheOldCard() async {
        let cache = AccountSnapshotCache(directory: TestData.temporaryDirectory())
        let store = makeStore(cache: cache)
        store.use(origin: canning, session: "first", backend: SnapshotBackend(.success(TestData.snapshot(loans: ["Mine"]))))
        await store.refresh()
        store.use(origin: canning, session: "second", backend: SnapshotBackend(.failure(.Network(message: "offline"))))
        #expect(store.snapshot == nil)
        #expect(cache.load(origin: canning) == nil)
    }
}

// MARK: - "Updated …"

struct AccountUpdatedWordingTests {
    private let calendar: Calendar = {
        var calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = TimeZone(identifier: "Australia/Perth")!
        return calendar
    }()
    private let locale = Locale(identifier: "en_AU")

    private func date(_ year: Int, _ month: Int, _ day: Int, _ hour: Int, _ minute: Int, _ second: Int = 0) -> Date {
        calendar.date(from: DateComponents(year: year, month: month, day: day, hour: hour, minute: minute, second: second))!
    }

    private func updated(_ date: Date, now: Date) -> String {
        Formatting.updated(date, now: now, calendar: calendar, locale: locale)
    }

    /// The time as this locale writes it (ICU may put a narrow no-break space before "pm").
    private func plain(_ text: String) -> String {
        text.replacingOccurrences(of: "\u{202F}", with: " ").replacingOccurrences(of: "\u{00A0}", with: " ")
    }

    @Test func recentUpdatesCountMinutes() {
        let now = date(2026, 10, 12, 10, 0)
        #expect(updated(now, now: now) == "Updated just now")
        #expect(updated(now.addingTimeInterval(-59), now: now) == "Updated just now")
        #expect(updated(now.addingTimeInterval(30), now: now) == "Updated just now") // clock skew
        #expect(updated(now.addingTimeInterval(-60), now: now) == "Updated 1 min ago")
        #expect(updated(now.addingTimeInterval(-12 * 60 - 40), now: now) == "Updated 12 min ago")
        #expect(updated(now.addingTimeInterval(-59 * 60), now: now) == "Updated 59 min ago")
    }

    @Test func olderUpdatesGiveTheDayAndTime() {
        let now = date(2026, 10, 12, 10, 0)
        #expect(plain(updated(date(2026, 10, 12, 8, 5), now: now)) == "Updated today at 8:05 am")
        #expect(plain(updated(date(2026, 10, 11, 16, 20), now: now)) == "Updated yesterday at 4:20 pm")
        // Just after midnight, an update from late last night is "yesterday", not minutes ago.
        #expect(plain(updated(date(2026, 10, 11, 22, 30), now: date(2026, 10, 12, 0, 10))) == "Updated yesterday at 10:30 pm")
        #expect(plain(updated(date(2026, 10, 3, 16, 20), now: now)) == "Updated 3 Oct at 4:20 pm")
        #expect(plain(updated(date(2025, 12, 30, 9, 0), now: now)).hasPrefix("Updated 30 Dec 2025 at 9:00"))
    }

    @Test func theTimeFollowsTheDevicesLocale() {
        let now = date(2026, 10, 12, 10, 0)
        let text = Formatting.updated(date(2026, 10, 11, 16, 20), now: now, calendar: calendar, locale: Locale(identifier: "en_GB"))
        #expect(text == "Updated yesterday at 16:20")
    }
}

// MARK: - App model

@MainActor
struct AccountAppModelTests {
    @Test func launchShowsTheSavedAccountAtOnce() async throws {
        let defaults = UserDefaults(suiteName: "LibreSpydusTests-\(UUID().uuidString)")!
        let sessions = InMemorySecretStore([canning: "fake-session"])
        let cacheDirectory = TestData.temporaryDirectory()
        let first = AppModel(dependencies: .testing(defaults: defaults, sessions: sessions, accountCache: cacheDirectory))
        first.selectLibrary(.canning)
        await first.refreshAccount()
        #expect(first.accountName == "Fake Borrower")
        #expect(files(in: cacheDirectory).count == 1)

        // A cold launch: the saved account is there before any fetch.
        let relaunched = AppModel(dependencies: .testing(defaults: defaults, sessions: sessions, accountCache: cacheDirectory) {
            $0.accountResult = .failure(.Network(message: "offline"))
        })
        #expect(relaunched.accountName == "Fake Borrower")
        #expect(relaunched.account.snapshot != nil)
        await relaunched.refreshAccount()
        #expect(relaunched.accountName == "Fake Borrower") // kept after a failed refresh
        #expect(relaunched.account.lastRefreshError?.kind == .network)
    }

    @Test func demoModeNeverSavesTheAccount() async {
        let cacheDirectory = TestData.temporaryDirectory()
        let model = AppModel(dependencies: .testing(demo: true, accountCache: cacheDirectory))
        model.selectLibrary(.canning)
        await model.refreshAccount()
        #expect(model.account.snapshot != nil)
        #expect(!model.account.persists)
        #expect(files(in: cacheDirectory).isEmpty)

        // Only the DEBUG launch flag (`-LSPersistDemoAccount`) saves it, for screenshots.
        let flagged = AppModel(dependencies: .testing(demo: true, accountCache: cacheDirectory, persistsDemoAccount: true))
        flagged.selectLibrary(.canning)
        await flagged.refreshAccount()
        #expect(files(in: cacheDirectory).count == 1)
    }

    @Test func liveDemoDependenciesKeepTheAccountInMemory() {
        let demo = AppModel.Dependencies.live(demo: true)
        #expect(!demo.persistsDemoAccount)
        let flagged = AppModel.Dependencies.live(demo: true, persistDemoAccount: true)
        #expect(flagged.accountCacheDirectory != AppModel.Dependencies.live(demo: false).accountCacheDirectory)
        #expect(AppModel.Dependencies.live(demo: false).accountCacheDirectory == AccountSnapshotCache.defaultDirectory)
    }

    @Test func signingOutForgetsTheSavedAccount() async {
        let cacheDirectory = TestData.temporaryDirectory()
        let sessions = InMemorySecretStore([canning: "fake-session"])
        let model = AppModel(dependencies: .testing(sessions: sessions, accountCache: cacheDirectory))
        model.selectLibrary(.canning)
        await model.refreshAccount()
        #expect(files(in: cacheDirectory).count == 1)
        model.signOut()
        #expect(model.account.snapshot == nil)
        #expect(model.accountName == nil)
        #expect(files(in: cacheDirectory).isEmpty)
    }

    @Test func anExpiredSessionForgetsTheSavedAccount() async {
        let cacheDirectory = TestData.temporaryDirectory()
        let sessions = InMemorySecretStore([canning: "fake-session"])
        var backends: [FakeBackend] = []
        let model = AppModel(dependencies: .testing(sessions: sessions, accountCache: cacheDirectory) { backends.append($0) })
        model.selectLibrary(.canning)
        await model.refreshAccount()
        #expect(files(in: cacheDirectory).count == 1)

        backends.last?.accountResult = .failure(.NotLoggedIn(message: "not logged in"))
        await model.refreshAccount()
        #expect(!model.isSignedIn)
        #expect(model.loginNeeded)
        #expect(model.account.snapshot == nil)
        #expect(model.account.lastRefreshError == nil) // the sign-in sheet says it
        #expect(files(in: cacheDirectory).isEmpty)
    }

    @Test func changingLibraryKeepsEachLibrarysSavedAccount() async {
        let cacheDirectory = TestData.temporaryDirectory()
        let sessions = InMemorySecretStore([canning: "c-session", hutt: "h-session"])
        let model = AppModel(dependencies: .testing(sessions: sessions, accountCache: cacheDirectory) { backend in
            if backend.origin == hutt {
                backend.accountResult = .success(AccountLinks(name: "Hutt Reader", loansUrl: nil, reservationsReadyUrl: nil,
                                                              reservationsPendingUrl: nil))
            }
        })
        model.selectLibrary(.canning)
        await model.refreshAccount()
        model.selectLibrary(origin: hutt)
        #expect(model.accountName == nil)
        await model.refreshAccount()
        #expect(model.accountName == "Hutt Reader")

        model.selectLibrary(.canning)
        #expect(model.accountName == "Fake Borrower") // at once, from its file
        #expect(files(in: cacheDirectory).count == 2)
    }

    @Test func signInFetchesTheAccountAtOnce() async throws {
        let cacheDirectory = TestData.temporaryDirectory()
        let model = AppModel(dependencies: .testing(accountCache: cacheDirectory))
        model.selectLibrary(.canning)
        #expect(model.account.snapshot == nil)
        try await model.signIn(username: "card", password: "ok")
        #expect(model.account.snapshot?.accountName == "Fake Borrower")
        #expect(files(in: cacheDirectory).count == 1)
    }

    @Test func resetDeletesSavedAccounts() async {
        let cacheDirectory = TestData.temporaryDirectory().appending(path: "AccountCache")
        let dependencies = AppModel.Dependencies.testing(sessions: InMemorySecretStore([canning: "s"]), accountCache: cacheDirectory)
        let model = AppModel(dependencies: dependencies)
        model.selectLibrary(.canning)
        await model.refreshAccount()
        #expect(files(in: cacheDirectory).count == 1)
        dependencies.resetPersistentState()
        #expect(!FileManager.default.fileExists(atPath: cacheDirectory.path))
    }
}

// MARK: - Demo

struct DemoAccountSnapshotTests {
    private func makeDemo(fails: Bool = false) throws -> DemoBackend {
        DemoBackend(publicBackend: FakeBackend(), fixtures: try DemoFixtures.bundled(), mutationDelay: .zero,
                    snapshotDelay: .zero, failsAccountSnapshot: fails)
    }

    @Test func theDemoSnapshotDecodesLikeTheCoresAndFollowsChanges() async throws {
        let demo = try makeDemo()
        let snapshot = try await demo.accountSnapshot(session: DemoBackend.session)
        #expect(snapshot.accountName == "Test Borrower")
        #expect(snapshot.loans.items.count == 5)
        #expect(snapshot.reservations.ready.items.count == 1)
        // Same version as the core's: it survives the round trip through the saved JSON.
        #expect(try decodeAccountSnapshot(json: encodeAccountSnapshot(snapshot: snapshot)) == snapshot)
        #expect(abs(Date.now.timeIntervalSince1970 - Double(snapshot.fetchedAtMs) / 1000) < 5)

        let cancel = try #require(snapshot.reservations.ready.items[0].cancelHref)
        _ = try await demo.cancelReservation(url: cancel, session: DemoBackend.session)
        #expect(try await demo.accountSnapshot(session: DemoBackend.session).reservations.ready.items.isEmpty)

        await #expect(throws: SpydusError.NotLoggedIn(message: "not logged in")) {
            try await demo.accountSnapshot(session: "someone-else")
        }
    }

    @Test func theDebugFlagMakesEveryRefreshFail() async throws {
        let demo = try makeDemo(fails: true)
        await #expect(throws: SpydusError.self) {
            try await demo.accountSnapshot(session: DemoBackend.session)
        }
    }

    @Test func launchFlagsParse() {
        let options = LaunchOptions.parse(["app", "-LSDemo", "YES", "-LSPersistDemoAccount", "YES", "-LSDemoRefreshFails", "1"])
        #expect(options.demo && options.persistDemoAccount && options.demoRefreshFails)
        let plain = LaunchOptions.parse(["app", "-LSDemo", "YES"])
        #expect(!plain.persistDemoAccount && !plain.demoRefreshFails)
    }
}
