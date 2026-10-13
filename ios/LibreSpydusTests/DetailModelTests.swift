import Foundation
import LibreSpydusCore
import Testing
@testable import LibreSpydus

/// `BookDetailModel` and `ReserveModel` against an offline backend.
@MainActor
struct DetailModelTests {
    private let noReport: BookDetailModel.Reporter = { UserFacingError($0, libraryName: "Canning") }

    @Test func searchRefsLoadTheBibRecordThroughTheSelectedFormat() async {
        let ref = BookRef(TestData.searchRecord)
        let bk = ref.formats[0].availabilityUrl!
        let backend = DetailBackend(records: [bk: .success(DetailFixtures.midnightLibrary)],
                                    availability: [bk: DetailFixtures.availability(["copies": 2])])
        let model = BookDetailModel(ref: ref)
        #expect(model.selectedFormat?.code == "BK")
        #expect(model.headerAvailability == .loading("Checking availability…"))

        await model.load(backend: backend, report: noReport)
        await model.loadAvailability(backend: backend, report: noReport)

        #expect(backend.recordURLs == [bk])
        #expect(backend.searches.isEmpty)
        #expect(model.headerAvailability == .line(AvailabilityLine(text: "2 copies", tone: .normal)))
        #expect(model.reservableFormat?.code == "BK")
        #expect(model.subjects.count == 6)

        model.select(formatAt: 1) // eBook: no reserve URL
        #expect(model.reservableFormat == nil)
    }

    @Test func aFailedFormatUrlFallsBackToTheRefUrl() async {
        let ref = BookRef(TestData.searchRecord)
        let backend = DetailBackend(records: [ref.url!: .success(DetailFixtures.midnightLibrary)])
        let model = BookDetailModel(ref: ref)
        await model.load(backend: backend, report: noReport)
        #expect(backend.recordURLs == [ref.formats[0].availabilityUrl!, ref.url!])
        #expect(model.record.value == DetailFixtures.midnightLibrary)
    }

    @Test func favouriteKeepsTheIdStoresTheBookmarkAndFormats() async {
        let ref = BookRef(TestData.searchRecord)
        let backend = DetailBackend(records: [ref.formats[0].availabilityUrl!: .success(DetailFixtures.midnightLibrary)])
        let model = BookDetailModel(ref: ref)
        await model.load(backend: backend, report: noReport)
        let saved = model.favouriteRef
        #expect(saved.id == ref.id)
        #expect(saved.url == DetailFixtures.midnightLibrary.bookmarkUrl)
        #expect(saved.formats == ref.formats)
    }

    @Test func refsWithoutFormatsReFindThemBySearch() async {
        let item = DetailFixtures.related(brn: "467086", title: "The long night", author: "White, Christian",
                                          isbn: "9781399197809")
        let ref = BookRef(item)
        let backend = DetailBackend(records: [ref.url!: .success(DetailFixtures.longNight)],
                                    searchRecords: DetailFixtures.longNightSearch)
        let model = BookDetailModel(ref: ref)
        #expect(model.formatLookup == .pending)
        #expect(model.reservableFormat == nil)

        await model.load(backend: backend, report: noReport)

        #expect(backend.searches == ["The long night white"])
        #expect(model.formatLookup == .found)
        #expect(model.formats.map(\.code) == ["BK"])
        #expect(model.reservableFormat?.code == "BK")
        #expect(model.favouriteRef.formats.map(\.code) == ["BK"])
        #expect(model.favouriteRef.url == DetailFixtures.longNight.bookmarkUrl)
    }

    @Test func aWorkLevelRecordIsUpgradedToTheBibRecordOnceFormatsAreFound() async {
        let workURL = "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/WRKENQ?SETLVL=&IRN=15642035"
        let work = RecordDetail(bibIrn: "15642035", brn: nil, title: "The Midnight Library", seriesTitle: nil,
                                isbn: "1786892707", coverUrls: [], summary: "Somewhere out beyond…",
                                details: [DetailFixtures.row("Creator", "Haig, Matt, 1975-, author")], bookmarkUrl: workURL,
                                related: RecordRelated(similar: nil, author: "https://canning.spydus.com/works-by", series: nil,
                                                       versions: nil))
        let hit = DetailFixtures.search(work: "15642035", title: "The Midnight Library", author: "Haig, Matt, 1975-")
        let backend = DetailBackend(records: [workURL: .success(work),
                                              hit.formats[0].availabilityUrl!: .success(DetailFixtures.midnightLibrary)],
                                    searchRecords: [hit])
        let model = BookDetailModel(ref: BookRef(url: workURL))
        await model.load(backend: backend, report: noReport)

        #expect(backend.recordURLs == [workURL, hit.formats[0].availabilityUrl!])
        #expect(model.formatLookup == .found)
        #expect(model.record.value == DetailFixtures.midnightLibrary)
        #expect(model.favouriteRef.url == DetailFixtures.midnightLibrary.bookmarkUrl)
        #expect(model.visiblePanels == [.similar, .author, .versions])
    }

    @Test func noConfidentMatchHidesAvailabilityAndBorrow() async {
        let ref = BookRef(id: "bib:1", title: "Hollow bones", author: "Mayne, Andrew",
                          url: "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&BRN=465783")
        let record = RecordDetail(bibIrn: nil, brn: "465783", title: "Hollow bones", seriesTitle: nil, isbn: nil,
                                  coverUrls: [], summary: "A thriller.", details: [], bookmarkUrl: ref.url,
                                  related: RecordRelated(similar: nil, author: nil, series: nil, versions: nil))
        let backend = DetailBackend(records: [ref.url!: .success(record)],
                                    searchRecords: [DetailFixtures.search(work: "9", title: "Hollow Bones", author: "Smith, Jo")])
        let model = BookDetailModel(ref: ref)
        await model.load(backend: backend, report: noReport)
        #expect(model.formatLookup == .notFound)
        #expect(model.headerAvailability == .checkWebsite)
        #expect(model.reservableFormat == nil)
    }

    @Test func aFailedRecordShowsAnErrorAndRetries() async {
        let ref = BookRef(url: "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&BRN=275039")
        let backend = DetailBackend(records: [:])
        let model = BookDetailModel(ref: ref)
        await model.load(backend: backend, report: noReport)
        #expect(model.record.error?.kind == .unexpected)
        #expect(model.headerAvailability == .none)

        backend.setRecord(.success(DetailFixtures.midnightLibrary), for: ref.url!)
        await model.load(backend: backend, report: noReport)
        #expect(model.record.value != nil)
        #expect(model.ref.title == "The Midnight Library")
        #expect(model.ref.author == "Haig, Matt, 1975-, author")
    }

    @Test func relatedPanelsLoadConcurrentlyAndHideEmptyOrFailedOnes() async {
        let ref = BookRef(TestData.searchRecord)
        let backend = DetailBackend(
            records: [ref.formats[0].availabilityUrl!: .success(DetailFixtures.midnightLibrary)],
            related: [
                "https://canning.spydus.com/similar": .success(RelatedList(items: [
                    DetailFixtures.related(brn: "467086", title: "The long night"),
                ])),
                "https://canning.spydus.com/author": .success(RelatedList(items: [
                    DetailFixtures.related(brn: "275039", title: "The Midnight Library"),
                ])),
            ]
        )
        let model = BookDetailModel(ref: ref)
        await model.load(backend: backend, report: noReport)
        #expect(model.visiblePanels == [.similar, .author, .versions])

        await model.loadRelated(backend: backend)

        // "More by this author" only had this book; "Other editions" failed.
        #expect(model.visiblePanels == [.similar])
        #expect(model.related[.similar]?.value?.map(\.id) == ["brn:467086"])
        #expect(Set(backend.relatedURLs) == ["https://canning.spydus.com/similar", "https://canning.spydus.com/author",
                                             "https://canning.spydus.com/versions"])
    }

    // MARK: Reserve sheet

    @Test func reserveDefaultsToTheFirstExpiryAndSubmitsOnce() async throws {
        let demo = DemoBackend(publicBackend: FakeBackend(), fixtures: try DemoFixtures.bundled(), mutationDelay: .zero)
        let model = ReserveModel()
        var rootSheetErrors: [any Error] = []
        await model.loadOptions(url: TestData.reserveUrl, backend: demo, session: DemoBackend.session,
                                libraryName: "Canning") { rootSheetErrors.append($0) }
        let options = try #require(model.options.value)
        #expect(options.canReserve)
        #expect(model.selectedDays == options.expiryChoices.first?.days)

        await model.confirm(url: TestData.reserveUrl, backend: demo, session: DemoBackend.session,
                            libraryName: "Canning") { rootSheetErrors.append($0) }
        #expect(model.result?.ok == true)
        #expect(model.isComplete)

        // Confirm again does nothing: the demo backend would answer "already reserved".
        await model.confirm(url: TestData.reserveUrl, backend: demo, session: DemoBackend.session,
                            libraryName: "Canning") { rootSheetErrors.append($0) }
        #expect(model.result?.ok == true)
        #expect(try await demo.reservations(session: DemoBackend.session).pending.items.count(where: {
            $0.fields["Status"] == "Pending" && $0.irn == "19100000"
        }) == 1)
        #expect(rootSheetErrors.isEmpty)
    }

    @Test func reserveWithoutASessionAsksForSignIn() async {
        let model = ReserveModel()
        var rootSheetErrors: [any Error] = []
        await model.loadOptions(url: TestData.reserveUrl, backend: FakeBackend(), session: nil,
                                libraryName: "Canning") { rootSheetErrors.append($0) }
        #expect(rootSheetErrors.count == 1)
        #expect((rootSheetErrors.first as? SpydusError) == .NotLoggedIn(message: "not logged in"))

        // A session the library no longer accepts goes the same way (FakeBackend throws NotLoggedIn).
        await model.loadOptions(url: TestData.reserveUrl, backend: FakeBackend(), session: "stale",
                                libraryName: "Canning") { rootSheetErrors.append($0) }
        #expect(rootSheetErrors.count == 2)
        #expect(model.options.error == nil)
    }
}

/// An offline backend for detail tests: canned records, searches, availability and related lists;
/// it records the URLs asked for. Account calls are unused.
final class DetailBackend: LibraryBackend, @unchecked Sendable {
    let origin = "https://canning.spydus.com"
    var isDemo: Bool { false }

    private let lock = NSLock()
    private var records: [String: Result<RecordDetail, SpydusError>]
    private let searchRecords: [SearchRecord]
    private let availabilityByURL: [String: Availability]
    private let relatedByURL: [String: Result<RelatedList, SpydusError>]
    private var _recordURLs: [String] = []
    private var _searches: [String] = []
    private var _relatedURLs: [String] = []

    init(records: [String: Result<RecordDetail, SpydusError>], searchRecords: [SearchRecord] = [],
         availability: [String: Availability] = [:], related: [String: Result<RelatedList, SpydusError>] = [:]) {
        self.records = records
        self.searchRecords = searchRecords
        availabilityByURL = availability
        relatedByURL = related
    }

    var recordURLs: [String] { lock.withLock { _recordURLs } }
    var searches: [String] { lock.withLock { _searches } }
    var relatedURLs: [String] { lock.withLock { _relatedURLs } }

    func setRecord(_ result: Result<RecordDetail, SpydusError>, for url: String) {
        lock.withLock { records[url] = result }
    }

    func setVerificationCookie(_ cookie: String?) {}

    func search(query: String) async throws -> SearchResults {
        lock.withLock { _searches.append(query) }
        return SearchResults(query: query, setId: "1", totalResults: Int64(searchRecords.count), showing: nil,
                             nextPageUrl: nil, prevPageUrl: nil, noResults: searchRecords.isEmpty, records: searchRecords)
    }

    func searchSorted(query: String, sort: SearchSortOption) async throws -> SearchResults {
        try await search(query: query)
    }

    func page(url: String) async throws -> SearchResults { try await search(query: url) }

    func availability(url: String) async throws -> Availability {
        guard let availability = availabilityByURL[url] else { throw SpydusError.Upstream(message: "no availability") }
        return availability
    }

    func record(url: String) async throws -> RecordDetail {
        let result = lock.withLock {
            _recordURLs.append(url)
            return records[url]
        }
        guard let result else { throw SpydusError.Upstream(message: "no bib record link in format fragment") }
        return try result.get()
    }

    func related(url: String) async throws -> RelatedList {
        lock.withLock { _relatedURLs.append(url) }
        guard let result = relatedByURL[url] else { throw SpydusError.Network(message: "timed out") }
        return try result.get()
    }

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
