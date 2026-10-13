import Foundation
import LibreSpydusCore
import Observation

/// My Picks (docs/native-apps.md §5.11): the reader's Pick list on the library account, which
/// the My Picks tab shows while signed in. `AppModel` owns one and points it at the current origin and
/// session, like `AccountStore`; sign-out, an expired session and a library change clear it.
///
/// - **The list and the picked set** come from the same pages: `refresh()` loads the first page,
///   then the rest in the background up to `maxPages` in all, collecting every record's `workIrn`
///   into `irns`, so a book page knows whether it's picked. Later pages load as the list scrolls.
/// - **`setPicked`** (a bookmark tap, a swipe in the list) flips the IRN at once, calls `savePick` /
///   `removePick` and flips back on failure, with a `notice` either way. A load that overlaps a change
///   may predate it, so the change is laid over what it brings back.
/// - **The copy-once offer:** whether a library's local favourites were offered (`wasOffered`, kept
///   in UserDefaults `picks.copyOffered.<sha256(origin)>`; demo mode `picks.demo.…`) and `copy(_:)`.
///
/// Saves and removes are MUTATING: only ever started by the reader. Nothing else is stored.
@MainActor
@Observable
final class PicksStore {
    /// The picked set loads at most this many pages (20 records each).
    static let maxPages = 10
    nonisolated static let namespace = "picks."
    nonisolated static let demoNamespace = "picks.demo."

    enum Phase: Equatable {
        /// Signed out, or not loaded yet.
        case idle
        /// The first page, with nothing on screen yet.
        case loading
        case loaded
        /// The library's "you do not have any items in your Pick list" page.
        case empty
        case failed(UserFacingError)
    }

    private(set) var phase: Phase = .idle
    /// The list's rows, as search results show them.
    private(set) var items: [SearchResultItem] = []
    private(set) var nextPageUrl: String?
    private(set) var isLoadingMore = false
    /// The last next-page failure: the footer offers Retry.
    private(set) var loadMoreError: UserFacingError?
    /// The IRNs on the list (`workIrn` of every record loaded), with changes in flight laid over.
    private(set) var irns: Set<String> = []
    /// A book was saved that the list doesn't show yet: the My Picks tab loads it again when it's next
    /// on screen.
    private(set) var isStale = false
    /// The latest confirmation or failure, shown briefly (`PicksNoticeOverlay`).
    private(set) var notice: PicksNotice?

    /// Routes a failed call (`AppModel.report`): `NotLoggedIn` signs out, which clears this store.
    @ObservationIgnored var report: @MainActor (any Error) -> UserFacingError? = { _ in nil }

    private struct Account {
        var origin: String
        var session: String
        var backend: any LibraryBackend
    }

    @ObservationIgnored private let defaults: UserDefaults
    @ObservationIgnored private let offeredPrefix: String
    @ObservationIgnored private var account: Account?
    /// Bumped when the origin or session changes: anything in flight for the old one is dropped.
    @ObservationIgnored private var generation = 0
    /// Bumped by every load from the first page: pages of an older load are dropped.
    @ObservationIgnored private var listGeneration = 0
    @ObservationIgnored private var firstPageTask: Task<Void, Never>?
    @ObservationIgnored private var morePagesTask: Task<Void, Never>?
    @ObservationIgnored private var nextPageTask: Task<Void, Never>?
    @ObservationIgnored private var requestedPages: Set<String> = []
    @ObservationIgnored private var pagesLoaded = 0
    /// The IRNs of the pages the current load has brought back.
    @ObservationIgnored private var loadedIrns: Set<String> = []
    /// Saves (true) and removes (false) in flight.
    @ObservationIgnored private var pending: [String: Bool] = [:]
    /// Saves and removes done since the current load began: its pages may predate them.
    @ObservationIgnored private var committed: [String: Bool] = [:]

    init(defaults: UserDefaults, namespace: String = PicksStore.namespace) {
        self.defaults = defaults
        offeredPrefix = namespace + "copyOffered."
    }

    /// Signed in to a library (a session to use).
    var isActive: Bool { account != nil }

    /// Whether `irn` is on the list (or being saved).
    func contains(_ irn: String) -> Bool {
        irns.contains(irn)
    }

    /// A save or remove of `irn` is in flight.
    func isChanging(_ irn: String) -> Bool {
        pending[irn] != nil
    }

    // MARK: Origin and session

    /// The current library and session (launch, library change, sign-in, sign-out). A new origin
    /// or session starts empty; nil for either forgets the list.
    func use(origin: String?, session: String?, backend: any LibraryBackend) {
        if let account, let origin, let session, account.origin == origin, account.session == session {
            self.account?.backend = backend
            return
        }
        reset()
        if let origin, let session {
            account = Account(origin: origin, session: session, backend: backend)
        }
    }

    // MARK: Loading

    /// Loads the list when it hasn't been, the last load failed, or a pick saved since is missing
    /// from it; joins a load already running.
    func loadIfNeeded() async {
        guard account != nil else { return }
        switch phase {
        case .idle, .loading, .failed: await refresh()
        case .loaded, .empty: if isStale { await refresh() }
        }
    }

    /// Loads the list again from its first page (pull to refresh, Retry, after the copy), or joins
    /// the load already running. Returns once the first page has settled; the rest of the picked
    /// set (up to `maxPages`) keeps loading in the background (`settle()` waits for it).
    func refresh() async {
        guard account != nil else { return }
        if let firstPageTask {
            await firstPageTask.value
            return
        }
        listGeneration += 1
        let list = listGeneration
        let task = Task { await loadFirstPage(list: list) }
        firstPageTask = task
        await task.value
    }

    /// Waits for the load in flight, the background pages included (tests, the copy).
    func settle() async {
        await firstPageTask?.value
        await morePagesTask?.value
        await nextPageTask?.value
    }

    /// The next page, as the list scrolls (or joins the one loading). Ignored after a page failed
    /// (`retryLoadMore()`) and at the end of the list.
    func loadMore() async {
        guard loadMoreError == nil else { return }
        await fetchNextPage()
    }

    func retryLoadMore() async {
        loadMoreError = nil
        await fetchNextPage()
    }

    // MARK: Changes

    /// Saves (`picked`) or removes `irn` for the reader: flips at once (a removed row leaves the
    /// list), calls the library, and flips back on failure. Shows "Saved to My Picks" / "Removed
    /// from My Picks", or "Couldn't update My Picks." (a lost session goes the usual way instead).
    /// Ignored while a change to the same IRN is in flight. Returns whether it went through.
    @discardableResult
    func setPicked(_ picked: Bool, irn: String) async -> Bool {
        guard let account, pending[irn] == nil else { return false }
        let generation = self.generation
        let wasPicked = irns.contains(irn)
        pending[irn] = picked
        if picked { irns.insert(irn) } else { irns.remove(irn) }
        let removedRows = picked ? [] : removeRows(irn: irn)
        do {
            if picked {
                _ = try await account.backend.savePick(irn: irn, session: account.session)
            } else {
                _ = try await account.backend.removePick(irn: irn, session: account.session)
            }
            guard generation == self.generation else { return true }
            pending[irn] = nil
            committed[irn] = picked
            if picked, !items.contains(where: { PickIrn.of($0.ref) == irn }) { isStale = true }
            notice = PicksNotice(message: picked ? Copy.Picks.saved : Copy.Picks.removed, tone: .success)
            return true
        } catch {
            guard generation == self.generation else { return false }
            pending[irn] = nil
            if wasPicked { irns.insert(irn) } else { irns.remove(irn) }
            restore(removedRows)
            let shown = report(error)
            // A lost session signed out (and cleared this store): the login sheet says it all.
            guard generation == self.generation, shown != nil else { return false }
            notice = PicksNotice(message: Copy.Picks.updateFailed, tone: .error)
            return false
        }
    }

    /// A row's remove (swipe, menu): the row leaves at once and comes back if the library refuses.
    func remove(_ item: SearchResultItem) async {
        guard let irn = PickIrn.of(item.ref) else { return }
        await setPicked(false, irn: irn)
    }

    // MARK: Copy-once offer

    /// Whether this library's local favourites were offered already (either answer counts).
    func wasOffered(origin: String) -> Bool {
        defaults.bool(forKey: offeredKey(origin))
    }

    func markOffered(origin: String) {
        defaults.set(true, forKey: offeredKey(origin))
    }

    /// Saves `refs` to the list one at a time, in order (the caller passes the oldest first). A ref
    /// keyed `work:` / `bib:` saves that IRN; another loads its record for the `bibIrn`, else is
    /// skipped. A lost session, a verification challenge or a lost connection stops the copy (the
    /// rest count as not added). Then shows "Added {k} books to My Picks" (" · {m} couldn't be
    /// added") and loads the list again. Returns the summary, nil when signed out.
    @discardableResult
    func copy(_ refs: [BookRef]) async -> CopySummary? {
        guard let account, !refs.isEmpty else { return nil }
        let generation = self.generation
        notice = PicksNotice(message: Copy.Picks.adding(count: refs.count), tone: .progress)
        var summary = CopySummary(added: 0, failed: 0)
        for (index, ref) in refs.enumerated() {
            guard generation == self.generation else { break }
            var irn = PickIrn.of(ref)
            if irn == nil, let url = ref.url.nonEmpty {
                // A related item or a link: its record says which bib it is. A failure just skips it.
                irn = PickIrn.validated((try? await account.backend.record(url: url))?.bibIrn)
            }
            guard let irn else {
                summary.failed += 1
                continue
            }
            do {
                _ = try await account.backend.savePick(irn: irn, session: account.session)
                guard generation == self.generation else { break }
                summary.added += 1
                irns.insert(irn)
                committed[irn] = true
            } catch {
                summary.failed += 1
                if Self.stopsCopy(error) {
                    summary.failed += refs.count - index - 1
                    _ = report(error)
                    break
                }
            }
        }
        notice = PicksNotice(message: PicksText.copied(added: summary.added, failed: summary.failed),
                             tone: summary.added == 0 ? .error : summary.failed == 0 ? .success : .info)
        if generation == self.generation, summary.added > 0 {
            isStale = true
            await refresh()
        }
        return summary
    }

    struct CopySummary: Equatable, Sendable {
        var added: Int
        var failed: Int
    }

    /// Deletes every library's "offered" flag, demo mode's too (`-LSReset`).
    nonisolated static func deleteAll(in defaults: UserDefaults) {
        for key in defaults.dictionaryRepresentation().keys where key.hasPrefix(namespace) {
            defaults.removeObject(forKey: key)
        }
    }

    // MARK: Private

    private func offeredKey(_ origin: String) -> String {
        offeredPrefix + AlertContent.originHash(origin)
    }

    /// Errors after which the next save would fail the same way.
    private static func stopsCopy(_ error: any Error) -> Bool {
        if error is CancellationError { return true }
        switch error as? SpydusError {
        case .NotLoggedIn, .HumanVerificationRequired, .Network: return true
        default: return false
        }
    }

    private func loadFirstPage(list: Int) async {
        defer { if list == listGeneration { firstPageTask = nil } }
        guard let account else { return }
        let generation = self.generation
        committed = [:]
        if items.isEmpty { phase = .loading }
        do {
            let page = try await account.backend.picks(session: account.session, url: nil)
            guard generation == self.generation, list == listGeneration else { return }
            loadMoreError = nil
            isLoadingMore = false
            requestedPages = []
            pagesLoaded = 1
            loadedIrns = Self.irns(page)
            isStale = false
            if page.records.isEmpty, !page.isNoResults {
                items = []
                nextPageUrl = nil
                phase = .failed(SearchResultsModel.unrecognisedPage)
                return
            }
            items = SearchResultsModel.merge([], with: page.records).filter(isKept)
            nextPageUrl = page.nextPageUrl.nonEmpty
            markStaleIfMissingSaves()
            updatePhase()
            irns.formUnion(overlaid(loadedIrns))
            if nextPageUrl == nil {
                irns = overlaid(loadedIrns)
            } else {
                morePagesTask = Task { await loadMorePages(list: list) }
            }
        } catch {
            guard generation == self.generation, list == listGeneration else { return }
            let shown = report(error)
            // NotLoggedIn signed out; a cancellation shows nothing.
            guard generation == self.generation else { return }
            guard let shown else {
                if items.isEmpty { phase = .idle }
                return
            }
            // Rows on screen (pull to refresh) stay; the failure shows as a notice instead.
            if items.isEmpty {
                phase = .failed(shown)
            } else {
                notice = PicksNotice(message: shown.message, tone: .error)
            }
        }
    }

    /// The rest of the picked set after the first page, up to `maxPages` in all; the set is exact
    /// once they're in (picks removed on the website leave it).
    private func loadMorePages(list: Int) async {
        defer { if list == listGeneration { morePagesTask = nil } }
        while list == listGeneration, account != nil, pagesLoaded < Self.maxPages, nextPageUrl != nil,
              loadMoreError == nil {
            let before = pagesLoaded
            await fetchNextPage()
            // Nothing came back (a cancelled fetch): stop here; scrolling asks again.
            if pagesLoaded == before { return }
        }
        guard list == listGeneration, account != nil, loadMoreError == nil else { return }
        irns = overlaid(loadedIrns)
    }

    /// Fetches `nextPageUrl` once (joining a fetch already running).
    private func fetchNextPage() async {
        if let nextPageTask {
            await nextPageTask.value
            return
        }
        // While the list loads again from its first page, the old next link waits.
        guard let account, firstPageTask == nil, phase == .loaded, let url = nextPageUrl,
              !requestedPages.contains(url) else { return }
        let generation = self.generation
        let list = listGeneration
        isLoadingMore = true
        requestedPages.insert(url)
        let task = Task {
            defer {
                if list == listGeneration {
                    isLoadingMore = false
                    nextPageTask = nil
                }
            }
            do {
                let page = try await account.backend.picks(session: account.session, url: url)
                guard generation == self.generation, list == listGeneration else { return }
                pagesLoaded += 1
                let pageIrns = Self.irns(page)
                loadedIrns.formUnion(pageIrns)
                irns.formUnion(overlaid(pageIrns))
                items = SearchResultsModel.merge(items, with: page.records).filter(isKept)
                // A next link back to a page already fetched would loop forever.
                nextPageUrl = page.nextPageUrl.nonEmpty.flatMap { requestedPages.contains($0) ? nil : $0 }
            } catch {
                guard generation == self.generation, list == listGeneration else { return }
                requestedPages.remove(url)
                guard let shown = report(error), generation == self.generation else { return }
                loadMoreError = shown
            }
        }
        nextPageTask = task
        await task.value
    }

    /// `base` with the changes in flight and those done since the load began laid over it.
    private func overlaid(_ base: Set<String>) -> Set<String> {
        var set = base
        for (irn, picked) in committed.merging(pending, uniquingKeysWith: { $1 }) {
            if picked { set.insert(irn) } else { set.remove(irn) }
        }
        return set
    }

    /// A row stays unless it's being (or was, since the load began) removed.
    private func isKept(_ item: SearchResultItem) -> Bool {
        guard let irn = PickIrn.of(item.ref) else { return true }
        return pending[irn] ?? committed[irn] ?? true
    }

    /// A save done since the load began that the first page doesn't show: load again later.
    private func markStaleIfMissingSaves() {
        let shown = Set(items.compactMap { PickIrn.of($0.ref) })
        if committed.contains(where: { $0.value && !shown.contains($0.key) }) { isStale = true }
    }

    private func updatePhase() {
        phase = items.isEmpty && nextPageUrl == nil ? .empty : .loaded
    }

    /// Takes `irn`'s rows out of the list at once; returns them with their positions.
    private func removeRows(irn: String) -> [(index: Int, item: SearchResultItem)] {
        let removed = items.enumerated().filter { PickIrn.of($0.element.ref) == irn }.map { (index: $0.offset, item: $0.element) }
        guard !removed.isEmpty else { return [] }
        items.removeAll { PickIrn.of($0.ref) == irn }
        if phase == .loaded { updatePhase() }
        return removed
    }

    private func restore(_ rows: [(index: Int, item: SearchResultItem)]) {
        guard !rows.isEmpty else { return }
        for row in rows where !items.contains(where: { $0.id == row.item.id }) {
            items.insert(row.item, at: min(row.index, items.count))
        }
        if phase == .empty { phase = .loaded }
    }

    private static func irns(_ page: SearchResults) -> Set<String> {
        Set(page.records.compactMap { PickIrn.validated($0.workIrn) })
    }

    /// Forgets the list and lets anything in flight finish unseen.
    private func reset() {
        generation += 1
        listGeneration += 1
        account = nil
        firstPageTask = nil
        morePagesTask = nil
        nextPageTask = nil
        requestedPages = []
        pagesLoaded = 0
        loadedIrns = []
        pending = [:]
        committed = [:]
        phase = .idle
        items = []
        nextPageUrl = nil
        isLoadingMore = false
        loadMoreError = nil
        irns = []
        isStale = false
    }
}

// MARK: - The IRN a pick is keyed on

/// What a pick is keyed on (§5.11): a results card's `workIrn` (a work; an edition on Featured and
/// Pick lists), a ref's own IRN, else the book page's loaded record's `bibIrn`.
enum PickIrn {
    /// A ref keyed `work:<IRN>` (a results card) or `bib:<IRN>` (a loan or reservation; saved
    /// favourites keep their key) → that IRN; nil for `brn:` / `url:` / `title:` refs.
    static func of(_ ref: BookRef) -> String? {
        for prefix in ["work:", "bib:"] where ref.id.hasPrefix(prefix) {
            return validated(String(ref.id.dropFirst(prefix.count)))
        }
        return nil
    }

    /// The ref's own IRN, else the loaded record's `bibIrn` (a related item or a link). Nil while
    /// the record is still loading: the bookmark waits.
    static func of(_ ref: BookRef, record: RecordDetail?) -> String? {
        of(ref) ?? validated(record?.bibIrn)
    }

    /// The IRN when it's all digits (the core refuses anything else), trimmed.
    static func validated(_ irn: String?) -> String? {
        guard let irn = irn.nonEmpty, irn.allSatisfy({ $0.isASCII && $0.isNumber }) else { return nil }
        return irn
    }
}

// MARK: - Copy and notices

/// A short confirmation or failure (RootView's toast), or the copy's progress.
struct PicksNotice: Identifiable, Equatable, Sendable {
    enum Tone: Equatable, Sendable {
        case success
        case info
        case error
        /// Stays until the next notice replaces it.
        case progress
    }

    let id = UUID()
    var message: String
    var tone: Tone
}

/// The copy-once offer after an interactive sign-in: this many local favourites for `origin`.
struct PicksCopyOffer: Identifiable, Hashable, Sendable {
    var origin: String
    var count: Int

    var id: String { origin }
}

/// My Picks' composed wording (docs/native-apps.md §5.11); every piece is from `Copy.Picks`.
enum PicksText {
    /// "Added 3 books to My Picks", plus " · 1 couldn’t be added".
    static func copied(added: Int, failed: Int) -> String {
        let text = Copy.Picks.added(count: added)
        return failed > 0 ? text + Copy.Common.separator + Copy.Picks.notAdded(count: failed) : text
    }
}
