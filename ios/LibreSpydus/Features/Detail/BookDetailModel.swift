import Foundation
import LibreSpydusCore
import Observation

/// State for one `BookDetailView`: the record, formats (given or re-found), availability per
/// format, the related panels and, for an edition-level item, its copies. Every call goes through the `LibraryBackend` passed in, and
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
        /// No confident match: availability and Reserve are hidden.
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
    private(set) var availability: [String: Loadable<FormatAvailability>] = [:]
    private(set) var related: [RelatedPanel: Loadable<[BookRef]>] = [:]
    /// An edition-level item's copies, from its availability's `holdingsUrl` (the "Copies" list,
    /// §5.10); `.idle` for any other book.
    private(set) var copies: Loadable<[CopyRow]> = .idle
    /// The holdings URL `copies` was loaded from.
    @ObservationIgnored private var copiesURL: String?
    /// The related links whose panels were requested (a record upgrade brings new ones).
    @ObservationIgnored private var relatedRequestedFor: RecordRelated?
    /// A format was picked on screen: the patron's edition is no longer selected for them.
    @ObservationIgnored private var formatPickedOnScreen = false
    /// Picking the patron's edition among the formats (`selectHeldFormat`).
    private var heldFormatSearch = HeldFormatSearch.idle

    private enum HeldFormatSearch { case idle, running, done }

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

    /// A Featured list's item (§5.10): one format without a code. No format chips; the record and
    /// counts load through that format, and the page lists its copies.
    var isEditionLevel: Bool { formats.isEditionLevel }

    /// Where an edition-level item's copies are listed, once its availability has come back (nil
    /// before, and for any other book).
    var copiesSource: String? {
        guard isEditionLevel, let url = selectedFormat?.availabilityUrl.nonEmpty else { return nil }
        return availability[url]?.value?.holdingsUrl
    }

    /// The format "Reserve" reserves: the selected one, when it has a reserve URL (physical, or
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
        case .loaded(let loaded): return .line(AvailabilityLine(summary: loaded.summary, format: format))
        case .failed: return .availabilityFailed
        }
    }

    /// The bib IRNs of the edition on screen, to find it in the account (`HoldingIrns`). While the
    /// patron's edition is being picked, the default format isn't the page's yet: the opened row's
    /// IRN stands, so the pill doesn't flicker to Reserve.
    var holdingIrns: [String] {
        let known = { (format: FormatRef) in format.availabilityUrl.flatMap { self.availability[$0]?.value?.bibRecordIrn } }
        let others = formats.indices.filter { $0 != selectedFormatIndex }.compactMap { known(formats[$0]) }
        let selected = heldFormatSearch == .running ? nil : selectedFormat.flatMap(known)
        return HoldingIrns.forPage(selectedFormatIrn: selected, recordIrn: record.value?.bibIrn,
                                   openedIrn: opened.bibIrn, otherFormatIrns: Set(others))
    }

    /// The page can't tell yet which edition is on screen: the patron's is being picked, or the
    /// selected format's availability (without one, the record) hasn't come back. A patron with
    /// loans or reservations sees no Reserve until then, rather than Reserve flashing before
    /// "On loan" / "Reserved" replaces it.
    var isIdentifyingEdition: Bool {
        if heldFormatSearch == .running { return true }
        if let url = selectedFormat?.availabilityUrl.nonEmpty {
            switch availability[url] ?? .idle {
            case .loaded, .failed: return false
            case .idle, .loading: return true
            }
        }
        switch record {
        case .loaded, .failed: return false
        case .idle, .loading: return true
        }
    }

    /// The patron's loan or reservation of the edition on screen, from the saved account (nil when
    /// signed out or before the account has loaded): "On loan" / "Reserved" instead of Reserve.
    func holding(in snapshot: AccountSnapshot?) -> AccountHolding? {
        let irns = holdingIrns
        guard let snapshot, !irns.isEmpty else { return nil }
        return accountHolding(snapshot: snapshot, bibIrns: irns)
    }

    /// What My Picks keys this book on (§5.11): the opened card's or row's own IRN (`work:` / `bib:`),
    /// else the loaded record's `bibIrn`; nil until the record has loaded (the bookmark waits).
    var pickIrn: String? {
        PickIrn.of(opened, record: record.value)
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

    /// A format chip was tapped.
    func select(formatAt index: Int) {
        guard formats.indices.contains(index) else { return }
        formatPickedOnScreen = true
        selectedFormatIndex = index
    }

    /// Loads what's missing: the record, then (for a ref without formats) its formats, then for a
    /// loan or reservation the format that is the patron's edition. Safe to call again after a
    /// failure or verification.
    func load(backend: any LibraryBackend, report: Reporter) async {
        if record.isLoading { return }
        if record.value == nil {
            await loadRecord(backend: backend, report: report)
        }
        switch formatLookup {
        case .pending, .failed: await refindFormats(backend: backend, report: report)
        default: break
        }
        await selectHeldFormat(backend: backend, report: report)
    }

    func loadRecord(backend: any LibraryBackend, report: Reporter) async {
        let urls = DetailRecordSource.candidateURLs(refURL: opened.url, bibIrn: opened.bibIrn, format: selectedFormat)
        guard !urls.isEmpty else {
            record = .failed(UserFacingError(kind: .unexpected, message: "This book has no link to the library's record."))
            return
        }
        record = .loading
        var failure: (any Error)?
        for url in urls {
            do {
                let detail = try await backend.record(url: url)
                // The OPAC's answer to a link it won't show (a loan's set without the session).
                guard !detail.isEmptyRecord else {
                    failure = SpydusError.Upstream(message: "empty record from \(url)")
                    continue
                }
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
        guard let url = selectedFormat?.availabilityUrl.nonEmpty, needsAvailability(url) else { return }
        availability[url] = .loading
        do {
            setAvailability(.success(try await backend.availability(url: url)), url: url, report: report)
        } catch {
            setAvailability(.failure(error), url: url, report: report)
        }
    }

    /// An edition-level item's copies (`holdings(url:)` of `copiesSource`), once per URL; again
    /// after a failure.
    func loadCopies(backend: any LibraryBackend, report: Reporter) async {
        guard let url = copiesSource else { return }
        if url == copiesURL {
            switch copies {
            case .loaded, .loading: return
            case .idle, .failed: break
            }
        }
        copiesURL = url
        copies = .loading
        do {
            let holdings = try await backend.holdings(url: url)
            guard url == copiesURL else { return }
            copies = .loaded(CopyRow.rows(holdings))
        } catch {
            guard url == copiesURL else { return }
            copies = report(error).map { .failed($0) } ?? .idle
        }
    }

    /// Opened from Your Books or Home's borrowed shelf (a `bib:` ref): once the formats are known,
    /// selects the one that is the patron's edition (its `bibRecordIrn` is the opened IRN), from
    /// every format's availability, at most 6 at a time and each fetched once. No match keeps the
    /// default; neither does a format picked on screen meanwhile.
    func selectHeldFormat(backend: any LibraryBackend, report: Reporter) async {
        guard let irn = opened.bibIrn, heldFormatSearch == .idle, formats.count > 1,
              formatLookup == .provided || formatLookup == .found else { return }
        heldFormatSearch = .running
        let urls = formats.compactMap { $0.availabilityUrl.nonEmpty }
        let missing = urls.filter(needsAvailability)
        for url in missing { availability[url] = .loading }
        let fetch: @Sendable (String) async -> (String, Result<Availability, any Error>) = { url in
            do { return (url, .success(try await backend.availability(url: url))) } catch { return (url, .failure(error)) }
        }
        await withTaskGroup(of: (String, Result<Availability, any Error>).self) { group in
            var next = missing.makeIterator()
            for _ in 0..<Self.maxConcurrentAvailability {
                guard let url = next.next() else { break }
                group.addTask { await fetch(url) }
            }
            for await (url, result) in group {
                setAvailability(result, url: url, report: report)
                if let url = next.next() { group.addTask { await fetch(url) } }
            }
        }
        let match = formats.firstIndex { format in
            format.availabilityUrl.flatMap { availability[$0]?.value?.bibRecordIrn } == irn
        }
        if let match, !formatPickedOnScreen { selectedFormatIndex = match }
        // A format that failed (or was cancelled) may be the one: look again on the next load.
        let settled = urls.allSatisfy { availability[$0]?.value != nil || availability[$0]?.isLoading == true }
        heldFormatSearch = match != nil || settled ? .done : .idle
    }

    /// The library's limit for one page's concurrent requests (docs/native-apps.md §2).
    static let maxConcurrentAvailability = 6

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
              let detail = try? await backend.record(url: url), !detail.isEmptyRecord else { return }
        apply(detail)
    }

    /// Not loaded or loading yet (each format's availability is fetched once; again after a failure).
    private func needsAvailability(_ url: String) -> Bool {
        switch availability[url] {
        case .loaded, .loading: false
        default: true
        }
    }

    private func setAvailability(_ result: Result<Availability, any Error>, url: String, report: Reporter) {
        switch result {
        case .success(let loaded): availability[url] = .loaded(FormatAvailability(loaded))
        case .failure(let error): availability[url] = report(error).map { .failed($0) } ?? .idle
        }
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
