import Foundation
import LibreSpydusCore
import Testing
@testable import LibreSpydus

/// Book pages opened from Your Books (session-free record links, the patron's edition selected)
/// and the "On loan" / "Reserved" pill in Reserve's place (docs/native-apps.md §6 "Already yours").
@MainActor
struct HeldDetailTests {
    private let report: BookDetailModel.Reporter = { UserFacingError($0, libraryName: "Canning") }

    // MARK: Which URL loads the record

    @Test func aRefFromALoanLoadsTheSessionFreeRecordAndKeepsItsBib() {
        let ref = BookRef(HeldFixtures.item(bibIrn: HeldFixtures.longNightLP))
        #expect(ref.id == "bib:19244682")
        #expect(ref.bibIrn == "19244682")
        #expect(ref.url == HeldFixtures.freeURL("19244682"))

        #expect(BookRef(TestData.searchRecord).bibIrn == nil)
        #expect(BookRef(id: "bib: ", title: "x").bibIrn == nil)
    }

    @Test func aSavedSetLinkTriesTheSessionFreeFormFirst() {
        let set = HeldFixtures.setURL("8753080")
        let free = HeldFixtures.freeURL("8753080")
        #expect(DetailRecordSource.candidateURLs(refURL: set, bibIrn: "8753080", format: nil) == [free, set])
        // Not for refs that aren't loans or reservations, nor for links that already work.
        #expect(DetailRecordSource.candidateURLs(refURL: set, bibIrn: nil, format: nil) == [set])
        let bookmark = DetailFixtures.midnightLibrary.bookmarkUrl!
        #expect(DetailRecordSource.candidateURLs(refURL: bookmark, bibIrn: "8753080", format: nil) == [bookmark])
        #expect(DetailRecordSource.candidateURLs(refURL: free, bibIrn: "8753080", format: nil) == [free])
    }

    @Test func emptyRecordsCountAsFailures() {
        #expect(DemoCatalogue.emptyRecord.isEmptyRecord)
        #expect(!DetailFixtures.midnightLibrary.isEmptyRecord)
        var titleOnly = DemoCatalogue.emptyRecord
        titleOnly.title = "Whistler"
        #expect(!titleOnly.isEmptyRecord)
    }

    @Test func anEmptyRecordFallsThroughToTheNextCandidate() async {
        // A favourite saved from a loan before this fix: its URL is the session-set link.
        let set = HeldFixtures.setURL("8753080")
        let free = HeldFixtures.freeURL("8753080")
        let ref = BookRef(id: "bib:8753080", title: "The midnight library", url: set)
        let backend = DetailBackend(records: [free: .success(DemoCatalogue.emptyRecord),
                                              set: .success(DetailFixtures.midnightLibrary)])
        let model = BookDetailModel(ref: ref)
        await model.load(backend: backend, report: report)
        #expect(backend.recordURLs.prefix(2) == [free, set])
        #expect(model.record.value == DetailFixtures.midnightLibrary)
    }

    @Test func onlyEmptyRecordsShowTheErrorStateAndRetry() async {
        let ref = BookRef(HeldFixtures.item(bibIrn: "8753080", title: "The midnight library"))
        let free = HeldFixtures.freeURL("8753080")
        let backend = DetailBackend(records: [free: .success(DemoCatalogue.emptyRecord)])
        let model = BookDetailModel(ref: ref)
        await model.load(backend: backend, report: report)
        #expect(backend.recordURLs == [free])
        #expect(model.record.error?.kind == .unexpected)
        #expect(model.record.error?.message == UserFacingError.unexpectedMessage)

        backend.setRecord(.success(DetailFixtures.midnightLibrary), for: free)
        await model.load(backend: backend, report: report)
        #expect(model.record.value == DetailFixtures.midnightLibrary)
    }

    // MARK: The patron's edition

    @Test func holdingIrnsPreferTheSelectedFormatsOwn() {
        #expect(HoldingIrns.forPage(selectedFormatIrn: "2", recordIrn: "1", openedIrn: "1") == ["2"])
        #expect(HoldingIrns.forPage(selectedFormatIrn: nil, recordIrn: "1", openedIrn: "3") == ["1", "3"])
        #expect(HoldingIrns.forPage(selectedFormatIrn: " ", recordIrn: "1", openedIrn: "1") == ["1"])
        #expect(HoldingIrns.forPage(selectedFormatIrn: nil, recordIrn: nil, openedIrn: nil).isEmpty)
        // The record came through the book format; the large print is still loading.
        #expect(HoldingIrns.forPage(selectedFormatIrn: nil, recordIrn: "1", openedIrn: nil, otherFormatIrns: ["1"]).isEmpty)
    }

    @Test func aLoanOpenedFromYourBooksSelectsItsEditionAndShowsOnLoan() async {
        let loan = HeldFixtures.item(bibIrn: HeldFixtures.longNightLP, fields: ["Due": "14 Oct 2026"])
        let backend = HeldFixtures.longNightBackend()
        let model = BookDetailModel(ref: BookRef(loan))
        // Before anything loads, the opened row's IRN already finds the loan.
        let snapshot = HeldFixtures.snapshot(loans: [loan])
        #expect(model.holding(in: snapshot)?.kind == .loan)

        await model.load(backend: backend, report: report)

        #expect(backend.recordURLs == [HeldFixtures.freeURL(HeldFixtures.longNightLP)])
        #expect(model.formats.map(\.code) == ["BK", "LP"])
        #expect(model.selectedFormat?.code == "LP")
        #expect(model.holdingIrns == [HeldFixtures.longNightLP])
        #expect(model.holding(in: snapshot)?.item == loan)
        #expect(model.headerAvailability == .line(AvailabilityLine(text: "1 copy", tone: .normal)))

        // The book format is another edition: Reserve, not the pill.
        model.select(formatAt: 0)
        #expect(model.holdingIrns == [HeldFixtures.longNightBK])
        #expect(model.holding(in: snapshot) == nil)
        #expect(model.reservableFormat?.code == "BK")
    }

    @Test func noMatchingFormatKeepsTheDefault() async {
        let reservation = HeldFixtures.item(bibIrn: "99", fields: ["Status": "Pending"])
        let backend = HeldFixtures.longNightBackend(recordIrn: "99")
        let model = BookDetailModel(ref: BookRef(reservation))
        await model.load(backend: backend, report: report)
        #expect(model.formats.map(\.code) == ["BK", "LP"])
        #expect(model.selectedFormat?.code == "BK")
        #expect(model.holding(in: HeldFixtures.snapshot(pending: [reservation])) == nil)
    }

    @Test func aFormatPickedOnScreenIsKept() async {
        // A favourite saved from the loan's page, formats included.
        var saved = BookRef(HeldFixtures.item(bibIrn: HeldFixtures.longNightLP))
        saved.formats = HeldFixtures.longNightWork.formats.map(FormatRef.init)
        let model = BookDetailModel(ref: saved)
        #expect(model.formatLookup == .provided)
        model.select(formatAt: 0)
        await model.load(backend: HeldFixtures.longNightBackend(), report: report)
        #expect(model.selectedFormat?.code == "BK")
    }

    @Test func aBookOpenedFromSearchShowsThePillOnTheHeldFormatOnly() async {
        let ref = BookRef(HeldFixtures.longNightWork)
        let backend = HeldFixtures.longNightBackend()
        let model = BookDetailModel(ref: ref)
        let snapshot = HeldFixtures.snapshot(ready: [HeldFixtures.item(bibIrn: HeldFixtures.longNightBK,
                                                                          fields: ["Pickup location": "RIVERTON LIBRARY"])])
        await model.load(backend: backend, report: report)
        // The record came through the book format; its availability is still unknown.
        #expect(model.holdingIrns == [HeldFixtures.longNightBK])
        await model.loadAvailability(backend: backend, report: report)
        #expect(model.holdingIrns == [HeldFixtures.longNightBK])
        #expect(model.holding(in: snapshot)?.kind == .reservationReady)
        #expect(model.holding(in: nil) == nil)

        model.select(formatAt: 1)
        // The large print's availability is loading: the book's IRN is known to be the other format's.
        #expect(model.holdingIrns.isEmpty)
        await model.loadAvailability(backend: backend, report: report)
        #expect(model.holdingIrns == [HeldFixtures.longNightLP])
        #expect(model.holding(in: snapshot) == nil)
        #expect(model.reservableFormat?.code == "LP")
    }

    // MARK: The pill

    @Test func loanPillShowsTheDueDate() throws {
        let today = try #require(DueDate.parse("12 Oct 2026", calendar: .libraryDates))
        let due = try #require(DueDate.parse("14 Oct 2026", calendar: .libraryDates))
        let status = HeldStatus(AccountHolding(kind: .loan, item: HeldFixtures.item(bibIrn: "1", fields: ["Due": "14 Oct 2026"])),
                                today: today, calendar: .libraryDates)
        #expect(status.text == "On loan · Due \(Formatting.shortDate(due))")
        #expect(status.accessibilityLabel == "On loan, due \(due.formatted(.dateTime.day().month(.wide)))")
        #expect(status.tone == .loan)
        #expect(status.segment == .loans)
    }

    @Test func overdueLoanPillIsDanger() throws {
        let today = try #require(DueDate.parse("12 Oct 2026", calendar: .libraryDates))
        let status = HeldStatus(AccountHolding(kind: .loan, item: HeldFixtures.item(bibIrn: "1", fields: ["Due": "10 Oct 2026"])),
                                today: today, calendar: .libraryDates)
        #expect(status.text == "On loan · Overdue")
        #expect(status.tone == .overdue)
        #expect(status.accessibilityLabel.hasPrefix("On loan, overdue, was due "))

        let unreadable = HeldStatus(AccountHolding(kind: .loan, item: HeldFixtures.item(bibIrn: "1", fields: ["Due": "soon"])))
        #expect(unreadable.text == "On loan · Due soon")
        #expect(HeldStatus(AccountHolding(kind: .loan, item: HeldFixtures.item(bibIrn: "1"))).text == "On loan")
    }

    @Test func readyPillNamesThePickupBranch() {
        func ready(_ fields: [String: String]) -> HeldStatus {
            HeldStatus(AccountHolding(kind: .reservationReady, item: HeldFixtures.item(bibIrn: "1", fields: fields)))
        }
        let branch = ready(["Pickup location": "RIVERTON LIBRARY"])
        #expect(branch.text == "Ready to collect · Riverton Library")
        #expect(branch.accessibilityLabel == "Ready to collect at Riverton Library")
        #expect(branch.tone == .ready)
        #expect(branch.segment == .reservations)
        // Canning writes the branch into Status.
        #expect(ready(["Status": "Please pickup from: CANNINGTON LIBRARY by 17 Oct 2026"]).text
            == "Ready to collect · Cannington Library")
        #expect(ready([:]).text == "Ready to collect")
    }

    @Test func pendingPillNamesTheBranchOrTheQueue() {
        func pending(_ fields: [String: String]) -> HeldStatus {
            HeldStatus(AccountHolding(kind: .reservationPending, item: HeldFixtures.item(bibIrn: "1", fields: fields)))
        }
        let branch = pending(["Status": "Pending", "Rank": "4", "Pickup location": "RIVERTON LIBRARY"])
        #expect(branch.text == "Reserved · Riverton Library")
        #expect(branch.tone == .reserved)
        #expect(branch.segment == .reservations)
        #expect(pending(["Status": "Requested pickup location: RIVERTON LIBRARY"]).text == "Reserved · Riverton Library")
        let queue = pending(["Rank": "2"])
        #expect(queue.text == "Reserved · #2 in queue")
        #expect(queue.accessibilityLabel == "Reserved, number 2 in the queue")
        #expect(pending(["Status": "In transit"]).text == "Reserved")
    }

    @Test func thePillOpensYourBooksOnItsSegment() {
        let app = AppModel(dependencies: .testing())
        app.selectLibrary(.canning)
        app.push(.detail(BookRef(HeldFixtures.item(bibIrn: "1"))), on: .borrowed)
        app.selectedTab = .home
        app.openYourBooks(on: .reservations)
        #expect(app.selectedTab == .borrowed)
        #expect(app.path(for: .borrowed).isEmpty)
        #expect(app.takePendingBorrowedSegment() == .reservations)
        #expect(app.takePendingBorrowedSegment() == nil)
    }

    // MARK: Demo mode

    @Test func demoAccountRowsLoadTheirFullRecordOnlyWithoutTheSessionSet() async throws {
        let demo = try HeldFixtures.demo()
        let snapshot = try await demo.accountSnapshot(session: DemoBackend.session)
        let rows = snapshot.loans.items + snapshot.reservations.ready.items + snapshot.reservations.pending.items
        #expect(rows.count == 9)
        for row in rows {
            let irn = try #require(row.bibIrn)
            #expect(row.bibUrl == DemoCatalogue.sessionSetURL(origin: demo.origin, irn: irn))
            #expect(try await demo.record(url: row.bibUrl!).isEmptyRecord)
            let full = try await demo.record(url: try #require(patronItemRecordUrl(item: row)))
            #expect(full.bibIrn == irn)
            #expect(full.title != nil && full.summary != nil)
            #expect(!DetailSubjects.extract(from: full.details).isEmpty)
        }
    }

    @Test func demoSearchGivesTheMidnightLibraryTwoEditions() async throws {
        let demo = try HeldFixtures.demo()
        let results = try await demo.searchSorted(query: "the midnight library", sort: .relevance)
        let work = try #require(results.records.first)
        #expect(results.records.count == 1)
        #expect(work.formats.map(\.code) == ["BK", "LP"])
        var irns: [String] = []
        for format in work.formats {
            irns.append(try #require(try await demo.availability(url: format.availabilityUrl!).bibRecordIrn))
            #expect(try await demo.record(url: format.availabilityUrl!).summary != nil)
        }
        let snapshot = try await demo.accountSnapshot(session: DemoBackend.session)
        #expect(accountHolding(snapshot: snapshot, bibIrns: [irns[0]])?.kind == .reservationReady)
        #expect(accountHolding(snapshot: snapshot, bibIrns: [irns[1]]) == nil)
        // Other searches still go to the library.
        let fake = FakeBackend()
        let other = try HeldFixtures.demo(publicBackend: fake)
        _ = try await other.search(query: "library")
        #expect(fake.searches == ["library"])
    }

    @Test func demoOverdueLoanOpensWithItsLargePrintSelected() async throws {
        let demo = try HeldFixtures.demo()
        let snapshot = try await demo.accountSnapshot(session: DemoBackend.session)
        let loan = try #require(snapshot.loans.items.first { $0.title == "The long night" })
        let model = BookDetailModel(ref: BookRef(loan))
        await model.load(backend: demo, report: report)
        #expect(model.record.value?.summary != nil)
        #expect(!model.subjects.isEmpty)
        #expect(model.selectedFormat?.code == "LP")
        let held = try #require(model.holding(in: snapshot))
        #expect(HeldStatus(held).text == "On loan · Overdue")
    }

    @Test func aDemoReservationLinksToTheReservedEdition() async throws {
        let demo = try HeldFixtures.demo()
        let work = try #require(try await demo.search(query: "midnight library").records.first)
        let largePrint = try #require(work.formats.last)
        let result = try await demo.placeReservation(url: largePrint.reserveUrl!, session: DemoBackend.session,
                                                     xdays: nil, pickup: nil)
        #expect(result.ok)
        let snapshot = try await demo.accountSnapshot(session: DemoBackend.session)
        let irn = try #require(try await demo.availability(url: largePrint.availabilityUrl!).bibRecordIrn)
        let holding = try #require(accountHolding(snapshot: snapshot, bibIrns: [irn]))
        #expect(holding.kind == .reservationPending)
        #expect(holding.item.bibUrl == DemoCatalogue.sessionSetURL(origin: demo.origin, irn: irn))
        #expect(HeldStatus(holding).text == "Reserved · Riverton Library")
    }
}

enum HeldFixtures {
    static let longNightBK = "17648631"
    static let longNightLP = "19244682"

    static func setURL(_ irn: String) -> String {
        "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/78500000?SETLVL=&QRY=IRN(\(irn))&NAVLVL=SET"
    }

    static func freeURL(_ irn: String) -> String {
        "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&IRN=\(irn)"
    }

    /// A loan or reservation row as the OPAC lists it.
    static func item(bibIrn: String, title: String = "The long night", fields: [String: String] = [:]) -> PatronItem {
        PatronItem(irn: "18000001", bibIrn: bibIrn, title: title, bibUrl: setURL(bibIrn), author: "White, Christian",
                   year: "2026", coverUrls: [], isbn: nil, fields: fields, actions: [])
    }

    static func snapshot(loans: [PatronItem] = [], ready: [PatronItem] = [], pending: [PatronItem] = []) -> AccountSnapshot {
        AccountSnapshot(version: DemoBackend.snapshotVersion, accountName: "Test Borrower",
                        loans: PatronList(total: Int64(loans.count), items: loans),
                        reservations: Reservations(ready: PatronList(total: Int64(ready.count), items: ready),
                                                   pending: PatronList(total: Int64(pending.count), items: pending)),
                        fetchedAtMs: 0)
    }

    /// "The Long Night": a book and a large print edition (separate bibs).
    static let longNightWork: SearchRecord = {
        var work = DetailFixtures.search(work: "15975964", title: "The Long Night", author: "White, Christian")
        let base = "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/78563304?SETLVL=SET&FMT=RECFMT&QRY=WRK01%5C15975964"
        work.formats.append(RecordFormat(
            code: "LP", label: "Large Print",
            reserveUrl: "https://canning.spydus.com/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/78563304/0/V/0/R/9/ALLWRKENQ?SVL=15975964&RECFMT=LP",
            availabilityUrl: "\(base)\\LP"))
        return work
    }()

    static func record(irn: String) -> RecordDetail {
        RecordDetail(bibIrn: irn, brn: "467086", title: "The long night", seriesTitle: nil, isbn: "9781399197809", coverUrls: [],
                     summary: "Em has lived a quiet life…",
                     details: [
                        DetailFixtures.row("Author", "White, Christian, author"),
                        DetailRow(label: "Work", values: [DetailValue(
                            text: "The Long Night",
                            url: "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/WRKENQ/78563304?SETLVL=&IRN=15975964")]),
                        DetailFixtures.row("Subject", "Kidnapping -- Fiction"),
                     ],
                     bookmarkUrl: "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&BRN=467086",
                     related: RecordRelated(similar: nil, author: nil, series: nil, versions: nil))
    }

    /// The loan's record by its session-free link (bib `recordIrn`) and through the book format;
    /// the work's search; each format's availability.
    static func longNightBackend(recordIrn: String = longNightLP) -> DetailBackend {
        let bk = longNightWork.formats[0].availabilityUrl!
        let lp = longNightWork.formats[1].availabilityUrl!
        return DetailBackend(
            records: [freeURL(recordIrn): .success(record(irn: recordIrn)), bk: .success(record(irn: longNightBK))],
            searchRecords: [longNightWork],
            availability: [bk: availability(irn: longNightBK, copies: 2), lp: availability(irn: longNightLP, copies: 1)]
        )
    }

    static func availability(irn: String, copies: Int64) -> Availability {
        Availability(bibRecordIrn: irn, title: nil, publisher: nil, counts: ["copies": copies], reserved: nil,
                     copies: copies, holdingsUrl: nil, recordUrl: nil)
    }

    static func demo(publicBackend: FakeBackend = FakeBackend()) throws -> DemoBackend {
        DemoBackend(publicBackend: publicBackend, fixtures: try DemoFixtures.bundled(), mutationDelay: .zero,
                    snapshotDelay: .zero)
    }
}
