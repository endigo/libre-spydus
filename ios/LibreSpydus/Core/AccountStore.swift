import Foundation
import LibreSpydusCore
import Observation

/// The signed-in account (name, loans, reservations), stale-while-revalidate
/// (docs/native-apps.md §5.2a). `AppModel` owns it and points it at the current origin and session;
/// screens show `snapshot` at once (the saved one at launch) while `refresh` fetches a new one in
/// the background with `accountSnapshot`.
///
/// One fetch runs at a time and callers share it. A result for an origin or session that is no
/// longer current is dropped, and so is one that may predate a reserve / cancel / edit
/// (`performChange`, `libraryDidChange`). A failed refresh keeps the snapshot and sets
/// `lastRefreshError`.
@MainActor
@Observable
final class AccountStore {
    /// Older than this, opening a screen or returning to the app fetches a new snapshot.
    static let staleAfter: TimeInterval = 2 * 60

    /// The account as last fetched (or saved on this device); nil before the first one.
    private(set) var snapshot: AccountSnapshot?
    private(set) var isRefreshing = false
    /// Why the last refresh failed; nil after a success. With a `snapshot` the screens keep
    /// showing it ("Couldn’t refresh · Showing the saved list").
    private(set) var lastRefreshError: UserFacingError?

    /// Routes a failed fetch (`AppModel.report`): returns what to show, nil for a cancellation.
    /// `NotLoggedIn` signs out, which `clear`s this store.
    @ObservationIgnored var report: @MainActor (any Error) -> UserFacingError? = { _ in nil }
    /// Runs after every fetched snapshot is saved, before `refresh` returns: the alerts (§5.8)
    /// process it (`AlertCenter.process`).
    @ObservationIgnored var didFetch: @MainActor (_ snapshot: AccountSnapshot, _ origin: String) async -> Void = { _, _ in }

    private struct Account {
        var origin: String
        var session: String
        var backend: any LibraryBackend
    }

    @ObservationIgnored private var account: Account?
    /// Nil: in memory only (demo mode).
    @ObservationIgnored private let cache: AccountSnapshotCache?
    @ObservationIgnored private let now: () -> Date
    @ObservationIgnored private var refreshTask: Task<Void, Never>?
    /// Bumped when the origin or session changes: a fetch for the old one is dropped.
    @ObservationIgnored private var generation = 0
    /// Bumped when a change to the account starts or ends: a fetch from before is dropped.
    @ObservationIgnored private var version = 0
    @ObservationIgnored private var changesInFlight = 0
    /// Fetch again once the changes in flight have ended (one was accepted, or a fetch was
    /// dropped meanwhile).
    @ObservationIgnored private var refetchAfterChanges = false
    /// The running fetch may be out of date: fetch again when it ends.
    @ObservationIgnored private var fetchAgain = false

    init(cache: AccountSnapshotCache?, now: @escaping () -> Date = { .now }) {
        self.cache = cache
        self.now = now
    }

    var persists: Bool { cache != nil }

    /// When `snapshot` was fetched.
    var updatedAt: Date? {
        snapshot.map { Date(timeIntervalSince1970: TimeInterval($0.fetchedAtMs) / 1000) }
    }

    /// Whether opening a screen should fetch: nothing yet, the last refresh failed, or the
    /// snapshot is more than `staleAfter` old.
    var needsRefresh: Bool {
        guard account != nil else { return false }
        guard let updatedAt, lastRefreshError == nil else { return true }
        let age = now().timeIntervalSince(updatedAt)
        return age >= Self.staleAfter || age < -Self.staleAfter
    }

    // MARK: Origin and session

    /// The current library and session (library change, launch, sign-in). Shows `origin`'s saved
    /// snapshot straight away; a library's saved data stays when another is selected. A nil
    /// `origin` (no library chosen yet) forgets the account without touching saved files.
    func use(origin: String?, session: String?, backend: any LibraryBackend) {
        guard let origin else {
            reset()
            account = nil
            return
        }
        guard let session else {
            clear(origin: origin)
            return
        }
        if let account, account.origin == origin, account.session == session {
            self.account?.backend = backend
            return
        }
        let newSession = account?.origin == origin
        reset()
        account = Account(origin: origin, session: session, backend: backend)
        if newSession {
            // Signed in again at the same library, maybe with another card: the old data goes.
            cache?.remove(origin: origin)
        } else {
            snapshot = cache?.load(origin: origin)
        }
    }

    /// Signed out or the session expired: forgets `origin`'s account, on the device too.
    func clear(origin: String) {
        reset()
        account = nil
        cache?.remove(origin: origin)
    }

    // MARK: Refresh

    /// Fetches a new snapshot when `force` or `needsRefresh`; joins the fetch already running
    /// instead of starting another. Returns once it has settled.
    func refresh(force: Bool = false) async {
        if let refreshTask {
            await refreshTask.value
            return
        }
        guard account != nil, force || needsRefresh else { return }
        await startRefresh().value
    }

    /// A fetch that starts now, however new the snapshot (the morning check): one already running
    /// may have started too early, so this waits for it and then fetches again.
    func refreshNow() async {
        if let refreshTask { await refreshTask.value }
        await refresh(force: true)
    }

    /// A hold was placed (or the account otherwise changed on the library): a fetch in flight may
    /// predate it, so it's dropped and the account fetched again.
    func libraryDidChange() {
        version += 1
        fetchAgainNow()
    }

    /// Runs a reserve / cancel / edit. A fetch that overlaps it may not include it, so its result
    /// is dropped; once the library has `accepted` the change, the account is fetched again.
    func performChange<Value>(_ change: () async throws -> Value, accepted: (Value) -> Bool) async throws -> Value {
        let generation = self.generation
        changesInFlight += 1
        version += 1
        do {
            let value = try await change()
            endChange(changed: accepted(value), generation: generation)
            return value
        } catch {
            endChange(changed: false, generation: generation)
            throw error
        }
    }

    /// Edits the snapshot in place (a cancelled reservation goes at once) until the next fetch
    /// replaces it; saved like a fetched one.
    func update(_ edit: (inout AccountSnapshot) -> Void) {
        guard var snapshot, let account else { return }
        edit(&snapshot)
        self.snapshot = snapshot
        save(snapshot, origin: account.origin)
    }

    // MARK: Private

    private func endChange(changed: Bool, generation: Int) {
        changesInFlight = max(0, changesInFlight - 1)
        version += 1
        if changed, generation == self.generation { refetchAfterChanges = true }
        guard changesInFlight == 0, refetchAfterChanges else { return }
        refetchAfterChanges = false
        fetchAgainNow()
    }

    /// A fresh fetch, even while one is running (that one may be out of date).
    private func fetchAgainNow() {
        guard account != nil else { return }
        if refreshTask != nil {
            fetchAgain = true
        } else {
            startRefresh()
        }
    }

    @discardableResult
    private func startRefresh() -> Task<Void, Never> {
        let generation = self.generation
        isRefreshing = true
        let task = Task { await self.run(generation: generation) }
        refreshTask = task
        return task
    }

    private func run(generation: Int) async {
        repeat {
            fetchAgain = false
            guard generation == self.generation else { return }
            guard let account else { break }
            let version = self.version
            let result: Result<AccountSnapshot, any Error>
            do {
                result = .success(try await account.backend.accountSnapshot(session: account.session))
            } catch {
                result = .failure(error)
            }
            // Another origin or session took over (and owns the flags now).
            guard generation == self.generation else { return }
            guard version == self.version, changesInFlight == 0 else {
                // Possibly from before a change: fetch again, now or once the changes end.
                if changesInFlight > 0 { refetchAfterChanges = true } else { fetchAgain = true }
                continue
            }
            switch result {
            case .success(let snapshot):
                self.snapshot = snapshot
                lastRefreshError = nil
                save(snapshot, origin: account.origin)
                await didFetch(snapshot, account.origin)
                // Signed out or another library meanwhile (the new one owns the flags now).
                guard generation == self.generation else { return }
            case .failure(let error):
                let shown = report(error)
                // `NotLoggedIn` signed out (a new generation); a cancellation shows nothing.
                guard generation == self.generation else { return }
                if let shown { lastRefreshError = shown }
            }
        } while fetchAgain
        refreshTask = nil
        isRefreshing = false
    }

    private func save(_ snapshot: AccountSnapshot, origin: String) {
        // Losing the saved copy only costs a slower next launch; the screens still have it.
        try? cache?.save(snapshot, origin: origin)
    }

    /// Forgets the in-memory account and lets a running fetch finish unseen.
    private func reset() {
        generation += 1
        refreshTask = nil
        isRefreshing = false
        fetchAgain = false
        refetchAfterChanges = false
        snapshot = nil
        lastRefreshError = nil
    }
}
