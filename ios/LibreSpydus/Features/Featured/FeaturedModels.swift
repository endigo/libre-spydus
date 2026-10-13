import Foundation
import LibreSpydusCore
import Observation

// The library's own "Featured" collections (docs/native-apps.md §5.10): Home's row of tiles, the
// Collection screen's shelves, and where a tile leads. Kept apart from the views so they're
// unit-tested (FeaturedTests).

// MARK: - Icons

enum FeaturedIcon {
    /// An SF Symbol for the library's Font Awesome icon name ("game-board", "fa-globe"); anything
    /// unknown gets sparkles.
    static func systemImage(for icon: String?) -> String {
        var name = (icon ?? "").trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        if name.hasPrefix("fa-") { name.removeFirst(3) }
        switch name {
        case "book": return "book.fill"
        case "balloons": return "party.popper.fill"
        case "computer": return "desktopcomputer"
        case "circle": return "circle.fill"
        case "globe": return "globe"
        case "game-board": return "puzzlepiece.fill"
        case "map-location-dot": return "map.fill"
        case "user-circle": return "person.crop.circle.fill"
        default: return fallback
        }
    }

    static let fallback = "sparkles"
}

// MARK: - Where a tile leads

enum FeaturedDestination: Equatable, Sendable {
    /// Pushed on the current tab: a collection's shelves, or a saved search's results.
    case route(Route)
    /// The in-app browser (`SFSafariViewController`).
    case web(URL)

    /// `.collection` → the Collection screen; `.search` → its results list, titled with the link;
    /// `.web` → the browser (nil for a URL the browser can't open).
    init?(_ link: FeaturedLink) {
        switch link.kind {
        case .collection: self = .route(.collection(link))
        case .search: self = .route(.resultsPage(url: link.url, title: link.title))
        case .web:
            guard let url = InAppBrowser.webURL(link.url) else { return nil }
            self = .web(url)
        }
    }
}

// MARK: - Home's row

/// Home's "Featured" row: `featured()`, loaded with Home but on its own. Nothing shows while it
/// loads, when the library has no such list, or when it fails (no error on Home; most libraries
/// have no list). A refresh that fails keeps the tiles already shown.
@MainActor
@Observable
final class FeaturedRowModel {
    /// What the row was loaded for: public data, so only a new library or verification reloads it.
    struct Key: Hashable, Sendable {
        var origin: String
        var verification: Int
    }

    private(set) var links: [FeaturedLink] = []

    @ObservationIgnored private var loadedKey: Key?
    /// The origin `links` belong to.
    @ObservationIgnored private var linksOrigin: String?
    /// Bumped by each load: a late answer for an older load is dropped.
    @ObservationIgnored private var generation = 0

    /// The row is shown only with tiles to show.
    var isVisible: Bool { !links.isEmpty }

    /// `.task(id:)` on Home: loads once per key (coming back from a pushed screen doesn't reload);
    /// a failed load tries again the next time Home appears.
    func loadIfNeeded(backend: any LibraryBackend, key: Key) async {
        guard key != loadedKey else { return }
        await load(backend: backend, key: key)
    }

    /// Pull to refresh.
    func refresh(backend: any LibraryBackend, key: Key) async {
        await load(backend: backend, key: key)
    }

    private func load(backend: any LibraryBackend, key: Key) async {
        // Another library's tiles never show while this one's load.
        if linksOrigin != key.origin { links = [] }
        generation += 1
        let generation = generation
        do {
            let loaded = try await backend.featured()
            guard generation == self.generation else { return }
            links = loaded
            linksOrigin = key.origin
            loadedKey = key
        } catch {
            guard generation == self.generation else { return }
            // Hidden, no error: Home's other shelves report verification and network problems.
        }
    }
}

// MARK: - Collection screen

/// The Collection screen (§5.10): `collection(url)`, then one shelf per section, each the first
/// `page(section.url)` (up to `shelfLimit` covers). Shelves load when they come into view
/// (`shelfAppeared`), at most `maxConcurrentShelves` at a time; one scrolled past before its turn
/// goes back to waiting (`shelfDisappeared`). A section that comes back empty is hidden; one that
/// fails shows Retry in its row.
@MainActor
@Observable
final class CollectionModel {
    /// What a load is for; any change (library, verification, a retry's `attempt`) loads afresh,
    /// the same request again (back from a book page) keeps what's loaded.
    struct Request: Hashable, Sendable {
        var origin: String
        var verification: Int
        var attempt = 0
    }

    enum Phase: Equatable {
        case loading
        /// One shelf per section.
        case shelves
        /// A page of links to outside services (eLibrary): "Open website".
        case noSections
        case failed(UserFacingError)
    }

    enum ShelfState: Equatable {
        /// Not in view yet.
        case idle
        /// In view, waiting for a free slot.
        case queued
        case loading
        case loaded([BookRef])
        case failed(UserFacingError)
    }

    struct Shelf: Identifiable, Equatable {
        /// Unique per load, so a reloaded collection's rows appear (and ask for their shelves) afresh.
        let id: String
        let index: Int
        let section: FeaturedSection
        var state: ShelfState = .idle

        /// "View All" → the section's results list.
        var route: Route { .resultsPage(url: section.url, title: section.title) }
    }

    /// How many covers a shelf shows (one results page).
    nonisolated static let shelfLimit = 20
    /// Shelves loading at once (What's New has 11 sections).
    nonisolated static let maxConcurrentShelves = 3

    let link: FeaturedLink
    private(set) var phase: Phase = .loading
    /// The page's title: the link's.
    var title: String { link.title }
    private(set) var shelves: [Shelf] = []

    @ObservationIgnored private var backend: (any LibraryBackend)?
    @ObservationIgnored private var report: @MainActor (any Error) -> UserFacingError? = { _ in nil }
    @ObservationIgnored private var settledRequest: Request?
    /// Bumped by every load: shelf answers for an older load are dropped.
    @ObservationIgnored private var generation = 0
    @ObservationIgnored private var queue: [Int] = []
    @ObservationIgnored private var running: [Int: Task<Void, Never>] = [:]
    /// The most shelves that were loading at once (tests).
    @ObservationIgnored private(set) var peakConcurrentShelves = 0

    init(link: FeaturedLink) {
        self.link = link
    }

    /// Shelves to draw: every section except those that came back empty.
    var visibleShelves: [Shelf] {
        shelves.filter { shelf in
            if case .loaded(let refs) = shelf.state { return !refs.isEmpty }
            return true
        }
    }

    /// Every section came back empty.
    var isEmpty: Bool { phase == .shelves && visibleShelves.isEmpty }

    /// Loads the collection page, unless it's the request already shown. `report` is
    /// `AppModel.report`: it opens the verification sheet when needed.
    func load(_ request: Request, backend: any LibraryBackend,
              report: @escaping @MainActor (any Error) -> UserFacingError?) async {
        if request == settledRequest, phase != .loading { return }
        generation += 1
        let generation = generation
        self.backend = backend
        self.report = report
        settledRequest = nil
        cancelShelves()
        phase = .loading
        shelves = []
        do {
            let collection = try await backend.collection(url: link.url)
            guard generation == self.generation else { return }
            settledRequest = request
            shelves = collection.sections.enumerated().map { index, section in
                Shelf(id: "\(generation)-\(index)", index: index, section: section)
            }
            phase = shelves.isEmpty ? .noSections : .shelves
        } catch {
            guard generation == self.generation else { return }
            // A cancelled load (the screen went away) stays `.loading` and runs again next time.
            guard let shown = report(error) else { return }
            settledRequest = request
            phase = .failed(shown)
        }
    }

    /// A shelf's row came into view: queue its first page (once; a failed one waits for Retry).
    func shelfAppeared(_ index: Int) {
        guard shelves.indices.contains(index), shelves[index].state == .idle else { return }
        enqueue(index)
    }

    /// A shelf's row scrolled out of view before its turn: it waits until it's back, so the shelves
    /// on screen go first.
    func shelfDisappeared(_ index: Int) {
        guard shelves.indices.contains(index), shelves[index].state == .queued else { return }
        shelves[index].state = .idle
        queue.removeAll { $0 == index }
    }

    /// A failed shelf's Retry.
    func retryShelf(_ index: Int) {
        guard shelves.indices.contains(index), case .failed = shelves[index].state else { return }
        enqueue(index)
    }

    /// Waits until no shelf is queued or loading (tests).
    func settle() async {
        while let task = running.values.first {
            await task.value
        }
    }

    // MARK: Private

    private func enqueue(_ index: Int) {
        shelves[index].state = .queued
        queue.append(index)
        startQueued()
    }

    private func startQueued() {
        guard let backend else { return }
        while running.count < Self.maxConcurrentShelves, !queue.isEmpty {
            let index = queue.removeFirst()
            guard shelves.indices.contains(index), shelves[index].state == .queued else { continue }
            shelves[index].state = .loading
            let generation = generation
            let url = shelves[index].section.url
            running[index] = Task { [weak self] in
                let result: Result<SearchResults, any Error>
                do { result = .success(try await backend.page(url: url)) } catch { result = .failure(error) }
                self?.finish(index, result: result, generation: generation)
            }
            peakConcurrentShelves = max(peakConcurrentShelves, running.count)
        }
    }

    private func finish(_ index: Int, result: Result<SearchResults, any Error>, generation: Int) {
        guard generation == self.generation else { return }
        running[index] = nil
        if shelves.indices.contains(index) {
            switch result {
            case .success(let page):
                if page.records.isEmpty, !page.isNoResults {
                    shelves[index].state = .failed(SearchResultsModel.unrecognisedPage)
                } else {
                    shelves[index].state = .loaded(Self.covers(page.records))
                }
            case .failure(let error):
                // Cancelled: back to waiting until the row appears again.
                shelves[index].state = report(error).map { .failed($0) } ?? .idle
            }
        }
        startQueued()
    }

    private func cancelShelves() {
        for task in running.values { task.cancel() }
        running = [:]
        queue = []
    }

    /// One cover per item, at most `shelfLimit`.
    nonisolated static func covers(_ records: [SearchRecord]) -> [BookRef] {
        var seen = Set<String>()
        let refs = SearchResultsModel.merge([], with: records).map(\.ref).filter { seen.insert($0.id).inserted }
        return Array(refs.prefix(shelfLimit))
    }
}
