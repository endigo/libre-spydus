import Foundation
import LibreSpydusCore
import Observation

/// One search, paged: the first page from `searchSorted` (or, for a results list given by URL,
/// `page(url)`), later pages from `nextPageUrl` → `page` (infinite scroll). Shared by the Search
/// tab, `SearchResultsView` and `ResultsPageView`.
///
/// Records are merged without duplicates (`workIrn`, else `detailUrl`); each page URL is fetched
/// at most once, and a result that arrives after a newer search started is dropped.
@MainActor
@Observable
final class SearchResultsModel {
    /// What a load is for. A view runs `load(_:)` from `.task(id: request)`, so any change
    /// (query, sort, library, verification, a retry's `attempt`) starts a fresh search, while the
    /// same request again (popping back from a detail screen) keeps the pages already loaded.
    struct Request: Hashable, Sendable {
        var query: String
        var sort: SearchSortOption
        /// A results-list URL to start from (`page(url)`) instead of searching `query`: a Featured
        /// saved search or collection section (§5.10).
        var url: String? = nil
        var origin: String
        var verification: Int
        var attempt = 0
    }

    enum Phase: Equatable {
        case idle
        case loading
        case loaded
        /// The OPAC's own "did not find any records" page.
        case noResults
        case failed(UserFacingError)
    }

    private(set) var phase: Phase = .idle
    private(set) var items: [SearchResultItem] = []
    /// `totalResults` from the first page.
    private(set) var total: Int64?
    private(set) var nextPageUrl: String?
    private(set) var isLoadingMore = false
    /// The last next-page failure; the footer offers Retry and automatic paging stops.
    private(set) var loadMoreError: UserFacingError?
    private(set) var request: Request?

    var hasMore: Bool { nextPageUrl != nil }

    @ObservationIgnored private var backend: (any LibraryBackend)?
    @ObservationIgnored private var report: @MainActor (any Error) -> UserFacingError? = { _ in nil }
    /// Bumped by every new search: late answers for an older one are dropped.
    @ObservationIgnored private var generation = 0
    @ObservationIgnored private var requestedPages: Set<String> = []
    /// The request whose load finished (success, no results or failure).
    @ObservationIgnored private var settledRequest: Request?

    /// Runs `request`'s first page, unless it's the request already shown.
    /// `report` is `AppModel.report`: it opens the verification sheet when needed.
    func load(_ request: Request, backend: any LibraryBackend,
              report: @escaping @MainActor (any Error) -> UserFacingError?) async {
        if request == settledRequest, phase != .idle { return }
        generation += 1
        let generation = generation
        self.request = request
        self.backend = backend
        self.report = report
        settledRequest = nil
        phase = .loading
        items = []
        total = nil
        nextPageUrl = nil
        isLoadingMore = false
        loadMoreError = nil
        requestedPages = []

        do {
            let page: SearchResults
            if let url = request.url {
                requestedPages.insert(url)
                page = try await backend.page(url: url)
            } else {
                page = try await backend.searchSorted(query: request.query, sort: request.sort)
            }
            guard generation == self.generation else { return }
            settledRequest = request
            if page.records.isEmpty {
                phase = page.isNoResults ? .noResults : .failed(Self.unrecognisedPage)
                return
            }
            items = Self.merge([], with: page.records)
            total = page.totalResults
            nextPageUrl = page.nextPageUrl.nonEmpty
            phase = .loaded
        } catch {
            guard generation == self.generation else { return }
            // A cancelled load (the view went away or the request changed) stays `.loading`
            // and runs again next time.
            guard let userError = report(error) else { return }
            settledRequest = request
            phase = .failed(userError)
        }
    }

    /// Fetches the next page, once. Ignored while a page is loading, after a page failed (use
    /// `retryLoadMore()`), and at the end of the results.
    func loadMore() async {
        guard loadMoreError == nil else { return }
        await fetchNextPage()
    }

    func retryLoadMore() async {
        loadMoreError = nil
        await fetchNextPage()
    }

    private func fetchNextPage() async {
        guard phase == .loaded, !isLoadingMore, let backend,
              let url = nextPageUrl, !requestedPages.contains(url) else { return }
        let generation = generation
        isLoadingMore = true
        requestedPages.insert(url)
        defer { if generation == self.generation { isLoadingMore = false } }

        do {
            let page = try await backend.page(url: url)
            guard generation == self.generation else { return }
            if page.records.isEmpty {
                if page.isNoResults {
                    nextPageUrl = nil
                } else {
                    requestedPages.remove(url)
                    loadMoreError = Self.unrecognisedPage
                }
                return
            }
            items = Self.merge(items, with: page.records)
            // A next link that points back at a page already fetched would loop forever.
            nextPageUrl = page.nextPageUrl.nonEmpty.flatMap { requestedPages.contains($0) ? nil : $0 }
        } catch {
            guard generation == self.generation else { return }
            requestedPages.remove(url)
            guard let userError = report(error) else { return }
            loadMoreError = userError
        }
    }

    // MARK: Merging

    /// `existing` followed by the records of `incoming` not already listed, in order.
    nonisolated static func merge(_ existing: [SearchResultItem], with incoming: [SearchRecord]) -> [SearchResultItem] {
        var merged = existing
        var seen = Set(existing.compactMap(\.key))
        for record in incoming {
            let key = dedupeKey(record)
            if let key {
                guard seen.insert(key).inserted else { continue }
            }
            merged.append(SearchResultItem(record, key: key, position: merged.count))
        }
        return merged
    }

    /// The same work on two pages: `workIrn`, else `detailUrl` (nil: never treated as a duplicate).
    nonisolated static func dedupeKey(_ record: SearchRecord) -> String? {
        if let irn = record.workIrn.nonEmpty { return "work:\(irn)" }
        if let url = record.detailUrl.nonEmpty { return "url:\(url)" }
        return nil
    }

    /// A page with no records that isn't the OPAC's "no records" page.
    nonisolated static let unrecognisedPage = UserFacingError(
        kind: .unexpected, message: UserFacingError.unexpectedMessage,
        detail: "The results page wasn't recognised (no records and no \"did not find any records\" message)."
    )
}

/// One result row.
struct SearchResultItem: Identifiable, Hashable, Sendable {
    /// Unique within a result list: the dedupe key, else the row's position.
    let id: String
    /// `SearchResultsModel.dedupeKey`.
    let key: String?
    let ref: BookRef
    /// Format chips, one per label; none for an edition-level item.
    let formats: [FormatRef]

    init(_ record: SearchRecord, key: String?, position: Int) {
        id = key ?? "row:\(position)"
        self.key = key
        ref = BookRef(record)
        var labels = Set<String>()
        // An edition-level item's one unnamed format (Featured lists, §5.10) gets no chip.
        formats = ref.formats.isEditionLevel ? [] : ref.formats.filter { labels.insert($0.displayLabel.lowercased()).inserted }
    }
}
