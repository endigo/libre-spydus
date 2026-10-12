import Foundation
import LibreSpydusCore
import Observation

/// State for one `BookDetailView`: the record, formats (given or re-found), availability per
/// format and the related panels. Every call goes through the `LibraryBackend` passed in, and
/// failures that need the app (verification, sign-in) go through `report`.
@MainActor
@Observable
final class BookDetailModel {
    typealias Reporter = @MainActor (any Error) -> UserFacingError?

    /// Whether the formats came with the ref or are being looked up (docs/native-apps.md §4:
    /// formats live on search records only).
    enum FormatLookup: Equatable {
        case provided
        /// Waiting for the record before searching.
        case pending
        case searching
        case found
        /// No confident match: availability and Borrow Now are hidden.
        case notFound
        case failed(UserFacingError)
    }

    /// The availability line under the byline.
    enum HeaderAvailability: Equatable {
        case loading(String)
        case line(AvailabilityLine)
        case availabilityFailed
        case lookupFailed
        /// No formats matched: "Check availability on the library website".
        case checkWebsite
        case none
    }

    /// The ref as opened (its URL picks which record to load).
    let opened: BookRef
    /// What the header draws: `opened`, filled in from the record (bookmark URL, cover, title).
    private(set) var ref: BookRef
    private(set) var record: Loadable<RecordDetail> = .idle
    private(set) var formats: [FormatRef]
    private(set) var selectedFormatIndex: Int?
    private(set) var formatLookup: FormatLookup
    /// Keyed by `availabilityUrl`, so switching back to a format doesn't fetch it again.
    private(set) var availability: [String: Loadable<AvailabilitySummary>] = [:]
    private(set) var related: [RelatedPanel: Loadable<[BookRef]>] = [:]
    /// The related links whose panels were requested (a record upgrade brings new ones).
    @ObservationIgnored private var relatedRequestedFor: RecordRelated?

    init(ref: BookRef) {
        opened = ref
        self.ref = ref
        formats = ref.formats
        selectedFormatIndex = FormatSelection.defaultIndex(ref.formats)
        formatLookup = ref.formats.isEmpty ? .pending : .provided
    }

    // MARK: Derived

    var selectedFormat: FormatRef? {
        selectedFormatIndex.flatMap { formats.indices.contains($0) ? formats[$0] : nil }
    }

    /// The format "Borrow Now" reserves: the selected one, when it has a reserve URL (physical, or
    /// digital at libraries that take ebook holds).
    var reservableFormat: FormatRef? {
        guard formatLookup == .provided || formatLookup == .found,
              let format = selectedFormat, format.reserveUrl.nonEmpty != nil else { return nil }
        return format
    }

    var headerAvailability: HeaderAvailability {
        switch formatLookup {
        case .pending:
            return record.error == nil ? .loading("Finding formats…") : .none
        case .searching:
            return .loading("Finding formats…")
        case .notFound:
            return .checkWebsite
        case .failed:
            return .lookupFailed
        case .provided, .found:
            break
        }
        guard let format = selectedFormat else { return .none }
        guard let url = format.availabilityUrl.nonEmpty else {
            return .line(AvailabilityLine(summary: nil, format: format))
        }
        switch availability[url] ?? .idle {
        case .idle, .loading: return .loading("Checking availability…")
        case .loaded(let summary): return .line(AvailabilityLine(summary: summary, format: format))
        case .failed: return .availabilityFailed
        }
    }

    /// What a favourite stores: the stable bookmark URL once the record is known, and the formats.
    var favouriteRef: BookRef {
        var saved = ref
        saved.formats = formats
        return saved
    }

    var subjects: [SubjectChip] {
        record.value.map { DetailSubjects.extract(from: $0.details) } ?? []
    }

    var facts: [DetailFact] {
        record.value.map { DetailFacts.rows(from: $0.details) } ?? []
    }

    var visiblePanels: [RelatedPanel] {
        record.value.map { RelatedPanel.visible(in: $0.related, states: related) } ?? []
    }

    var hasRelatedPanels: Bool {
        guard let related = record.value?.related else { return false }
        return RelatedPanel.allCases.contains { $0.url(in: related) != nil }
    }

    // MARK: Actions

    func select(formatAt index: Int) {
        guard formats.indices.contains(index) else { return }
        selectedFormatIndex = index
    }

    /// Loads what's missing: the record, then (for a ref without formats) its formats. Safe to call
    /// again after a failure or verification.
    func load(backend: any LibraryBackend, report: Reporter) async {
        if record.isLoading { return }
        if record.value == nil {
            await loadRecord(backend: backend, report: report)
        }
        switch formatLookup {
        case .pending, .failed: await refindFormats(backend: backend, report: report)
        default: break
        }
    }

    func loadRecord(backend: any LibraryBackend, report: Reporter) async {
        let urls = DetailRecordSource.candidateURLs(refURL: opened.url, format: selectedFormat)
        guard !urls.isEmpty else {
            record = .failed(UserFacingError(kind: .unexpected, message: "This book has no link to the library's record."))
            return
        }
        record = .loading
        var failure: (any Error)?
        for url in urls {
            do {
                let detail = try await backend.record(url: url)
                apply(detail)
                return
            } catch {
                failure = error
                // A challenge or a lost connection fails the other URL the same way.
                if let spydus = error as? SpydusError, case .HumanVerificationRequired = spydus { break }
                if let spydus = error as? SpydusError, case .Network = spydus { break }
                if error is CancellationError { break }
            }
        }
        if let failure, let shown = report(failure) {
            record = .failed(shown)
        } else {
            record = .idle
        }
    }

    /// Searches for the record's formats (title + author) and keeps them only on a confident match.
    func refindFormats(backend: any LibraryBackend, report: Reporter) async {
        guard formats.isEmpty, let detail = record.value else { return }
        guard let query = FormatRefinder.searchQuery(ref: ref, record: detail) else {
            formatLookup = .notFound
            return
        }
        formatLookup = .searching
        do {
            let results = try await backend.searchSorted(query: query, sort: .relevance)
            if let hit = FormatRefinder.match(ref: ref, record: detail, in: results.records) {
                formats = hit.formats.map(FormatRef.init)
                selectedFormatIndex = FormatSelection.defaultIndex(formats)
                if ref.coverUrl == nil { ref.coverUrl = hit.coverUrls.first }
                formatLookup = .found
                await upgradeWorkLevelRecord(backend: backend)
            } else {
                formatLookup = .notFound
            }
        } catch {
            formatLookup = report(error).map { .failed($0) } ?? .pending
        }
    }

    /// The selected format's availability (once per URL; again after a failure).
    func loadAvailability(backend: any LibraryBackend, report: Reporter) async {
        guard let url = selectedFormat?.availabilityUrl.nonEmpty else { return }
        switch availability[url] {
        case .loaded, .loading: return
        default: break
        }
        availability[url] = .loading
        do {
            let result = try await backend.availability(url: url)
            availability[url] = .loaded(availabilitySummary(availability: result))
        } catch {
            availability[url] = report(error).map { .failed($0) } ?? .idle
        }
    }

    /// Starts loading the related panels once per record, when the carousels scroll into view.
    func startRelated(backend: any LibraryBackend) {
        guard let links = record.value?.related, relatedRequestedFor != links else { return }
        relatedRequestedFor = links
        Task { await loadRelated(backend: backend) }
    }

    /// After verification or a retry: load panels that failed again (if they were requested).
    func retryFailedRelated(backend: any LibraryBackend) {
        guard relatedRequestedFor != nil, related.values.contains(where: { $0.error != nil }) else { return }
        Task { await loadRelated(backend: backend) }
    }

    /// Every related panel the record links to, concurrently (at most four GETs). A failed or
    /// empty panel is hidden, never reported.
    func loadRelated(backend: any LibraryBackend) async {
        guard let detail = record.value else { return }
        let pending = RelatedPanel.allCases.compactMap { panel -> (RelatedPanel, String)? in
            guard let url = panel.url(in: detail.related) else { return nil }
            switch related[panel] {
            case .loaded, .loading: return nil
            default: return (panel, url)
            }
        }
        guard !pending.isEmpty else { return }
        for (panel, _) in pending { related[panel] = .loading }
        let brn = detail.brn
        await withTaskGroup(of: (RelatedPanel, Loadable<[BookRef]>).self) { group in
            for (panel, url) in pending {
                group.addTask {
                    do {
                        let list = try await backend.related(url: url)
                        return (panel, .loaded(RelatedPanel.items(list, excludingBrn: brn)))
                    } catch {
                        return (panel, .failed(UserFacingError(error, libraryName: "")))
                    }
                }
            }
            for await (panel, state) in group where record.value?.related == detail.related {
                related[panel] = state
            }
        }
    }

    // MARK: Private

    /// A work-level record (opened by a work URL) has no subjects, imprint or similar titles; once
    /// formats are known, the selected one's `availabilityUrl` loads the full bib record. Kept as is
    /// if that fails.
    private func upgradeWorkLevelRecord(backend: any LibraryBackend) async {
        guard let current = record.value,
              DetailRecordSource.isWorkLevel(current.bookmarkUrl) || DetailRecordSource.isWorkLevel(opened.url),
              let url = selectedFormat?.availabilityUrl.nonEmpty,
              let detail = try? await backend.record(url: url) else { return }
        apply(detail)
    }

    private func apply(_ detail: RecordDetail) {
        if let current = record.value, current.related != detail.related { related = [:] }
        record = .loaded(detail)
        var updated = ref.updated(with: detail)
        if updated.title == "Untitled", let title = detail.title.nonEmpty { updated.title = title }
        if updated.author == nil {
            let row = detail.details.first { ["author", "creator"].contains($0.label.lowercased()) }
            updated.author = (row?.values.first?.text).nonEmpty
        }
        ref = updated
    }
}
