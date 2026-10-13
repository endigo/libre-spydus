import Foundation
import LibreSpydusCore
import Testing
@testable import LibreSpydus

/// Edition-level items (docs/native-apps.md §5.10): a Featured list's cards, each one bib with one
/// unnamed format. No format chips, the record and counts through that format, and a "Copies" list.
@MainActor
struct FeaturedEditionTests {
    private typealias F = FeaturedFixtures
    private let report: BookDetailModel.Reporter = { UserFacingError($0, libraryName: "Canning") }

    // MARK: Detection

    @Test func oneFormatWithoutACodeIsEditionLevel() {
        let edition = FormatRef(F.editionFormat("10936881"))
        #expect(edition.isEditionLevel)
        #expect([edition].isEditionLevel)
        #expect(BookRef(F.item("10936881")).formats.isEditionLevel)

        #expect(![FormatRef(code: "BK", label: "Book")].isEditionLevel)
        #expect(![FormatRef(code: "", label: "Book")].isEditionLevel)
        #expect(![edition, edition].isEditionLevel)
        #expect(![FormatRef]().isEditionLevel)
        #expect(!BookRef(TestData.searchRecord).formats.isEditionLevel)
    }

    @Test func resultRowsShowNoChipForIt() {
        let items = SearchResultsModel.merge([], with: [F.item("10936881"), TestData.searchRecord])
        #expect(items[0].formats.isEmpty)
        #expect(items[0].ref.formats.count == 1) // the book page still gets the format
        #expect(!items[1].formats.isEmpty)
    }

    @Test func itsRecordLoadsThroughTheFormatBeforeTheCardsSetLink() {
        let ref = BookRef(F.item("10936881"))
        let format = ref.formats[0]
        #expect(DetailRecordSource.candidateURLs(refURL: ref.url, format: format) == [format.availabilityUrl!, ref.url!])
    }

    // MARK: Book page

    @Test func theBookPageLoadsRecordCountsAndCopiesWithoutAFormatSearch() async throws {
        let ref = BookRef(F.item("10936881", title: "Mini Keyboard : Musical Instruments"))
        let format = try #require(ref.formats.first)
        let recordURL = try #require(format.availabilityUrl)
        let holdingsURL = "\(F.base)/XHLD/OPAC/BIBENQ/78607062/10936881"
        let backend = EditionBackend(
            records: [recordURL: Self.miniKeyboard],
            availability: [recordURL: Availability(bibRecordIrn: "10936881", title: nil, publisher: nil,
                                                   counts: ["copies": 2, "available": 0], reserved: nil, copies: 2,
                                                   holdingsUrl: holdingsURL, recordUrl: nil)],
            holdings: [holdingsURL: .success([
                Holding(location: "RIVERTON LIBRARY", collection: "Library of Things", callNumber: "LOT MUS004",
                        status: "On loan - Due: 29 Oct 2026", description: nil, available: false),
                Holding(location: "CANNINGTON LIBRARY", collection: "Library of Things", callNumber: "LOTMUS009",
                        status: "Available", description: "Pink body, teal handle", available: true),
            ])]
        )
        let model = BookDetailModel(ref: ref)
        #expect(model.isEditionLevel)
        #expect(model.formatLookup == .provided)
        #expect(model.copiesSource == nil) // availability not known yet

        await model.load(backend: backend, report: report)
        await model.loadAvailability(backend: backend, report: report)
        await model.loadCopies(backend: backend, report: report)

        #expect(backend.recordURLs == [recordURL])
        #expect(backend.searchCount == 0)
        #expect(model.ref.author == nil) // Library of Things items have none
        #expect(model.headerAvailability == .line(AvailabilityLine(text: "0 of 2 available", tone: .normal)))
        #expect(model.reservableFormat == format)
        #expect(model.copiesSource == holdingsURL)
        #expect(model.copies.value == [
            CopyRow(location: "Riverton Library", status: "On loan - Due: 29 Oct 2026", available: false,
                    callNumber: "LOT MUS004"),
            CopyRow(location: "Cannington Library", status: "Available", available: true,
                    description: "Pink body, teal handle", callNumber: "LOTMUS009"),
        ])

        // Once loaded, copies aren't fetched again.
        await model.loadCopies(backend: backend, report: report)
        #expect(backend.holdingsCalls == 1)

        // A favourite keeps the item's id and its format, so it opens the same way later.
        #expect(model.favouriteRef.id == ref.id)
        #expect(model.favouriteRef.formats == ref.formats)
        #expect(model.favouriteRef.formats.isEditionLevel)
    }

    @Test func failedCopiesCanBeRetried() async throws {
        let ref = BookRef(F.item("9342794"))
        let recordURL = try #require(ref.formats.first?.availabilityUrl)
        let holdingsURL = "\(F.base)/XHLD/OPAC/BIBENQ/78607062/9342794"
        let backend = EditionBackend(
            records: [recordURL: Self.miniKeyboard],
            availability: [recordURL: Availability(bibRecordIrn: "9342794", title: nil, publisher: nil,
                                                   counts: ["copies": 1, "available": 1], reserved: nil, copies: 1,
                                                   holdingsUrl: holdingsURL, recordUrl: nil)],
            holdings: [holdingsURL: .failure(.Network(message: "timed out"))]
        )
        let model = BookDetailModel(ref: ref)
        await model.load(backend: backend, report: report)
        await model.loadAvailability(backend: backend, report: report)
        await model.loadCopies(backend: backend, report: report)
        #expect(model.copies.error?.kind == .network)

        backend.setHoldings(.success([Holding(location: "Riverton Library", collection: nil, callNumber: nil,
                                              status: "Available", description: nil, available: true)]), for: holdingsURL)
        await model.loadCopies(backend: backend, report: report)
        #expect(model.copies.value == [CopyRow(location: "Riverton Library", status: "Available", available: true)])
        #expect(model.headerAvailability == .line(AvailabilityLine(text: "1 of 1 available", tone: .available)))
    }

    @Test func otherBooksHaveNoCopiesList() async throws {
        let ref = BookRef(TestData.searchRecord)
        let bk = try #require(ref.formats.first?.availabilityUrl)
        var counts = DetailFixtures.availability(["copies": 2])
        counts.holdingsUrl = "\(F.base)/XHLD/OPAC/BIBENQ/1/8753080"
        let backend = EditionBackend(records: [bk: DetailFixtures.midnightLibrary], availability: [bk: counts], holdings: [:])
        let model = BookDetailModel(ref: ref)
        await model.load(backend: backend, report: report)
        await model.loadAvailability(backend: backend, report: report)
        await model.loadCopies(backend: backend, report: report)
        #expect(!model.isEditionLevel)
        #expect(model.copiesSource == nil)
        #expect(model.copies == .idle)
        #expect(backend.holdingsCalls == 0)
    }

    // MARK: Copy rows

    @Test func copyRowsTitleCaseTheBranchAndKeepWhatTheLibraryWrote() {
        let rows = CopyRow.rows([
            Holding(location: "RIVERTON LIBRARY", collection: "Library of Things", callNumber: " LOT MUS004 ",
                    status: "On loan - Due: 29 Oct 2026", description: "  ", available: false),
            Holding(location: "Willetton Library", collection: nil, callNumber: nil, status: nil, description: nil,
                    available: true),
            Holding(location: nil, collection: "Library of Things", callNumber: nil, status: "Available", description: nil,
                    available: true),
        ])
        #expect(rows[0] == CopyRow(location: "Riverton Library", status: "On loan - Due: 29 Oct 2026", available: false,
                                   description: nil, callNumber: "LOT MUS004"))
        #expect(rows[1].location == "Willetton Library")
        #expect(rows[1].status == nil)
        #expect(rows[2].location == "Library of Things")
        #expect(CopyRow.titleCased("ST. JAMES LIBRARY") == "St. James Library")
        #expect(CopyRow.titleCased("123") == "123")
    }

    // MARK: Fixtures

    /// `record(<RECFMT url>)` for a Library of Things item: no author row.
    static let miniKeyboard = RecordDetail(
        bibIrn: "10936881", brn: "307745", title: "Mini Keyboard : Musical Instruments", seriesTitle: "Musical Instruments",
        isbn: nil, coverUrls: [], summary: "Mini keys! Big fun!",
        details: [DetailFixtures.row("Main title", "Mini Keyboard : Musical Instruments"),
                  DetailFixtures.row("Subject", "Piano", "Musical Instrument")],
        bookmarkUrl: nil,
        related: RecordRelated(similar: nil, author: nil, series: nil, versions: nil)
    )
}

/// An offline backend for edition-level book pages: canned records, availability and holdings.
final class EditionBackend: LibraryBackend, @unchecked Sendable {
    let origin = SearchFixtures.origin
    var isDemo: Bool { false }

    private let lock = NSLock()
    private let records: [String: RecordDetail]
    private let availabilityByURL: [String: Availability]
    private var holdingsByURL: [String: Result<[Holding], SpydusError>]
    private var _recordURLs: [String] = []
    private var _searchCount = 0
    private var _holdingsCalls = 0

    init(records: [String: RecordDetail], availability: [String: Availability],
         holdings: [String: Result<[Holding], SpydusError>]) {
        self.records = records
        availabilityByURL = availability
        holdingsByURL = holdings
    }

    var recordURLs: [String] { lock.withLock { _recordURLs } }
    var searchCount: Int { lock.withLock { _searchCount } }
    var holdingsCalls: Int { lock.withLock { _holdingsCalls } }

    func setHoldings(_ result: Result<[Holding], SpydusError>, for url: String) {
        lock.withLock { holdingsByURL[url] = result }
    }

    func holdings(url: String) async throws -> [Holding] {
        let result = lock.withLock {
            _holdingsCalls += 1
            return holdingsByURL[url]
        }
        guard let result else { throw SpydusError.Upstream(message: "no holdings \(url)") }
        return try result.get()
    }

    func record(url: String) async throws -> RecordDetail {
        let record = lock.withLock {
            _recordURLs.append(url)
            return records[url]
        }
        guard let record else { throw SpydusError.Upstream(message: "no record \(url)") }
        return record
    }

    func availability(url: String) async throws -> Availability {
        guard let availability = availabilityByURL[url] else { throw SpydusError.Upstream(message: "no availability") }
        return availability
    }

    func search(query: String) async throws -> SearchResults {
        lock.withLock { _searchCount += 1 }
        return SearchFixtures.page([], total: 0, noResults: true)
    }
    func searchSorted(query: String, sort: SearchSortOption) async throws -> SearchResults { try await search(query: query) }
    func page(url: String) async throws -> SearchResults { try await search(query: url) }
    func setVerificationCookie(_ cookie: String?) {}
    func related(url: String) async throws -> RelatedList { RelatedList(items: []) }
    func login(username: String, password: String) async throws -> String { throw SpydusError.CredentialsRejected(message: "") }
    func account(session: String) async throws -> AccountLinks { throw SpydusError.NotLoggedIn(message: "") }
    func loans(session: String) async throws -> PatronList { throw SpydusError.NotLoggedIn(message: "") }
    func reservations(session: String) async throws -> Reservations { throw SpydusError.NotLoggedIn(message: "") }
    func accountSnapshot(session: String) async throws -> AccountSnapshot { throw SpydusError.NotLoggedIn(message: "") }
    func reserveOptions(url: String, session: String) async throws -> ReserveOptions { throw SpydusError.NotLoggedIn(message: "") }
    func placeReservation(url: String, session: String, xdays: String?, pickup: String?) async throws -> ReserveResult {
        throw SpydusError.NotLoggedIn(message: "")
    }
    func cancelReservation(url: String, session: String) async throws -> ReserveResult { throw SpydusError.NotLoggedIn(message: "") }
    func editOptions(url: String, session: String) async throws -> EditOptions { throw SpydusError.NotLoggedIn(message: "") }
    func editReservation(url: String, session: String, overrides: EditOverrides) async throws -> ReserveResult {
        throw SpydusError.NotLoggedIn(message: "")
    }
}
