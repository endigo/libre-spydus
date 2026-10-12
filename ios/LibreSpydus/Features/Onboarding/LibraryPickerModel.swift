import Foundation
import LibreSpydusCore
import Observation

/// The library picker's state: the Civica directory (cached on disk), the search text and the
/// sections to show.
///
/// `load()` shows the cached list at once and refreshes it in the background when it's older
/// than a week; without a cache it fetches (one public GET) and shows loading / error + Retry.
@MainActor
@Observable
final class LibraryPickerModel {
    typealias Fetch = @Sendable () async throws -> [LibraryInfo]

    private(set) var libraries: [LibraryInfo] = []
    /// When `libraries` was fetched from the directory.
    private(set) var fetchedAt: Date?
    private(set) var isFetching = false
    /// The list couldn't be fetched and there's nothing cached to show.
    private(set) var loadError: UserFacingError?
    /// A refresh failed while an older list is on screen (shown as a banner).
    private(set) var refreshError: UserFacingError?
    var query = ""

    @ObservationIgnored private let fetch: Fetch
    @ObservationIgnored private let cache: LibraryDirectoryCache
    @ObservationIgnored private let now: () -> Date
    @ObservationIgnored private var started = false

    init(fetch: @escaping Fetch = { try await listLibraries(includeTest: false) },
         cache: LibraryDirectoryCache = .default,
         now: @escaping () -> Date = Date.init) {
        self.fetch = fetch
        self.cache = cache
        self.now = now
    }

    var sections: [LibraryDirectory.Section] {
        LibraryDirectory.sections(libraries, query: query)
    }

    /// Nothing to show yet and a fetch is running.
    var isLoading: Bool { libraries.isEmpty && isFetching }

    var isSearching: Bool { !query.trimmingCharacters(in: .whitespaces).isEmpty }

    /// First appearance: the cached list, refreshed when stale or missing. Later calls do nothing.
    func load() async {
        guard !started else { return }
        started = true
        if let snapshot = cache.load(), !snapshot.libraries.isEmpty {
            libraries = snapshot.libraries
            fetchedAt = snapshot.fetchedAt
            if LibraryDirectoryCache.isFresh(fetchedAt: snapshot.fetchedAt, now: now()) { return }
        }
        await refresh()
    }

    /// Fetches the directory (pull-to-refresh, Retry, stale cache) and caches it. On failure the
    /// list on screen stays.
    func refresh() async {
        guard !isFetching else { return }
        isFetching = true
        defer { isFetching = false }
        do {
            let fetched = try await fetch()
            let snapshot = LibraryDirectoryCache.Snapshot(fetchedAt: now(), libraries: fetched)
            libraries = fetched
            fetchedAt = snapshot.fetchedAt
            loadError = nil
            refreshError = nil
            try? cache.save(snapshot)
        } catch {
            if error is CancellationError { return }
            let userError = Self.userFacingError(error)
            if libraries.isEmpty {
                loadError = userError
            } else {
                refreshError = userError
            }
        }
    }

    func retry() async {
        loadError = nil
        await refresh()
    }

    /// The directory isn't a library, so the §5.7 messages name it instead.
    static func userFacingError(_ error: any Error) -> UserFacingError {
        var userError = UserFacingError(error, libraryName: "the library directory")
        switch userError.kind {
        case .network:
            userError.message = "Can't reach the library directory. Check your connection."
        case .unexpected, .verificationRequired, .verificationUnavailable, .notLoggedIn, .credentialsRejected:
            userError.kind = .unexpected
            userError.message = "The library directory returned something unexpected."
        }
        return userError
    }

    #if DEBUG
    /// `-LSPickerQuery <text>` (DEBUG, screenshots): start with this search text.
    func applyDebugQuery() {
        if let query = UserDefaults.standard.string(forKey: "LSPickerQuery"), self.query.isEmpty {
            self.query = query
        }
    }
    #endif
}
