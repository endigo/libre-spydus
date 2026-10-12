import Foundation
import LibreSpydusCore
import Testing
@testable import LibreSpydus

private func item(_ title: String, irn: String? = nil, fields: [String: String] = [:],
                  actions: [(String, String)] = []) -> PatronItem {
    PatronItem(irn: irn, bibIrn: nil, title: title, bibUrl: nil, author: "Haig, Matt, author", year: nil,
               coverUrls: [], isbn: nil, fields: fields,
               actions: actions.map { PatronAction(label: $0.0, href: $0.1) })
}

private let base = "https://canning.spydus.com/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/78500000/0"

// MARK: - Loans

struct BorrowedLoanTests {
    private let calendar = Calendar.libraryDates
    private var today: Date { DueDate.parse("12 Oct 2026", calendar: calendar)! }

    @Test func loansAreSortedByDueDateWithUnreadableDatesLast() {
        let items = [
            item("Later", irn: "1", fields: ["Due": "20 Oct 2026"]),
            item("No date", irn: "2"),
            item("Today", irn: "3", fields: ["Due": "12 Oct 2026"]),
            item("Overdue", irn: "4", fields: ["Due": "10/10/2026"]),
            item("Unreadable", irn: "5", fields: ["Due": "next week"]),
            item("Tomorrow", irn: "6", fields: ["Due": "2026-10-13"]),
        ]
        let rows = LoanRow.rows(from: items, today: today, calendar: calendar)
        #expect(rows.map(\.ref.title) == ["Overdue", "Today", "Tomorrow", "Later", "No date", "Unreadable"])
        #expect(rows.map(\.due?.daysLeft) == [-2, 0, 1, 8, nil, nil])
    }

    @Test func daysLeftWording() {
        func meta(_ due: String) -> String? {
            LoanRow(item: item("Book", fields: ["Due": due]), today: today, calendar: calendar).meta
        }
        func date(_ text: String) -> String {
            Formatting.shortDate(DueDate.parse(text, calendar: calendar)!)
        }
        #expect(meta("12 Oct 2026") == "Due today · " + date("12 Oct 2026"))
        #expect(meta("13 Oct 2026") == "Due tomorrow · " + date("13 Oct 2026"))
        #expect(meta("15 Oct 2026") == "Due " + date("15 Oct 2026") + " · 3 days left")
        #expect(meta("27 Oct 2026") == "Due " + date("27 Oct 2026") + " · 15 days left")
        #expect(meta("11 Oct 2026") == "Due " + date("11 Oct 2026") + " · 1 day overdue")
        #expect(meta("2 Oct 2026") == "Due " + date("2 Oct 2026") + " · 10 days overdue")
    }

    @Test func dangerColourWithinThreeDaysOrOverdue() {
        func urgent(_ due: String) -> Bool {
            LoanRow(item: item("Book", fields: ["Due": due]), today: today, calendar: calendar).isUrgent
        }
        #expect(urgent("10 Oct 2026"))
        #expect(urgent("12 Oct 2026"))
        #expect(urgent("15 Oct 2026"))
        #expect(!urgent("16 Oct 2026"))
        #expect(!urgent("whenever"))
    }

    @Test func unreadableDueDatesAreShownAsWritten() {
        func meta(_ fields: [String: String]) -> String? {
            LoanRow(item: item("Book", fields: fields), today: today, calendar: calendar).meta
        }
        #expect(meta(["Due": "next week"]) == "Due next week")
        #expect(meta(["Due": "Overdue - see library"]) == "Overdue - see library")
        #expect(meta(["Due": "  "]) == nil)
        #expect(meta([:]) == nil)
    }

    @Test func rowIdsStayUniqueWhenTheLibraryRepeatsAnIrn() {
        let rows = LoanRow.rows(from: [item("A", irn: "1"), item("B", irn: "1"), item("C")], today: today)
        #expect(Set(rows.map(\.id)).count == 3)
    }
}

// MARK: - Reservations

struct BorrowedReservationTests {
    @Test func actionHrefsMatchCancelAndEditLabels() {
        let reservation = item("Book", actions: [("Cancel reservation", "\(base)/C/5/RSVCENQ?SVL=1"),
                                                 ("EDIT RESERVATION", "\(base)/ER/1/RSVCENQ?SVL=1")])
        #expect(reservation.cancelHref == "\(base)/C/5/RSVCENQ?SVL=1")
        #expect(reservation.editHref == "\(base)/ER/1/RSVCENQ?SVL=1")

        let short = item("Book", actions: [("Edit", "\(base)/ER/1/RSVCENQ?SVL=2"), ("Cancel", "\(base)/C/5/RSVCENQ?SVL=2")])
        #expect(short.editHref == "\(base)/ER/1/RSVCENQ?SVL=2")
        #expect(short.cancelHref == "\(base)/C/5/RSVCENQ?SVL=2")

        let loan = item("Book", actions: [("Renew loan", "\(base)/RL/1/LOANRENQ?SVL=3")])
        #expect(loan.cancelHref == nil && loan.editHref == nil)
        #expect(item("Book", actions: [("Cancel reservation", " ")]).cancelHref == nil)
    }

    @Test func readyForPickupComesFirstAndAppearsOnce() {
        let ready = item("Ready", irn: "1", fields: ["Status": "Awaiting pickup", "Pickup location": "RIVERTON LIBRARY"])
        let reservations = Reservations(
            ready: PatronList(total: 1, items: [ready]),
            pending: PatronList(total: 3, items: [
                item("In transit", irn: "2", fields: ["Status": "In transit"]),
                ready,
                item("Queued", irn: "3", fields: ["Status": "Pending", "Rank": "4"]),
            ])
        )
        let sections = ReservationSections(reservations)
        #expect(sections.ready.map(\.ref.title) == ["Ready"])
        #expect(sections.ready.allSatisfy { $0.isReady })
        #expect(sections.pending.map(\.ref.title) == ["In transit", "Queued"])
        #expect(sections.pending.allSatisfy { !$0.isReady })
        #expect(sections.count == 3)
        #expect(sections.removing(id: "2").pending.map(\.ref.title) == ["Queued"])
    }

    @Test func cardWording() throws {
        let ready = ReservationRow(item: item("Ready", fields: ["Status": "Awaiting pickup",
                                                                "Pickup location": "WILLETTON & DISTRICT"]), kind: .ready)
        #expect(ready.statusLabel == "Ready for pickup")
        #expect(ready.meta == "Pick up at Willetton & District")

        let placed = try #require(DueDate.parse("02 Oct 2026", calendar: .libraryDates))
        let pending = ReservationRow(item: item("Pending", fields: ["Status": "Pending", "Rank": "4", "Placed": "02 Oct 2026",
                                                                    "Pickup location": "RIVERTON LIBRARY"]), kind: .pending)
        #expect(pending.statusLabel == "Pending")
        #expect(pending.meta == "#4 in queue · Placed \(Formatting.shortDate(placed))\nPick up at Riverton Library")

        let bare = ReservationRow(item: item("Bare", fields: ["Status": "In transit"]), kind: .pending)
        #expect(bare.statusLabel == "In transit")
        #expect(bare.meta == nil)
        #expect(ReservationRow(item: item("No status"), kind: .pending).statusLabel == "Pending")
        #expect(ReservationRow.displayName("Central Library") == "Central Library")
    }

    @Test func cancelConfirmationNamesTheBook() {
        let ready = ReservationRow(item: item("The midnight library", fields: ["Pickup location": "RIVERTON LIBRARY"]),
                                   kind: .ready)
        #expect(CancelConfirm.message(for: ready).contains("“The midnight library” is waiting for you at Riverton Library"))
        let pending = ReservationRow(item: item("Hollow bones"), kind: .pending)
        #expect(CancelConfirm.message(for: pending).contains("lose your place in the queue"))
    }

    @Test func demoReservationsSplitAsExpected() throws {
        let fixtures = try DemoFixtures.bundled()
        let sections = ReservationSections(Reservations(
            ready: PatronList(total: nil, items: fixtures.ready), pending: PatronList(total: nil, items: fixtures.pending)))
        #expect(sections.ready.map(\.ref.title) == ["The midnight library"])
        #expect(sections.ready[0].cancelHref != nil && sections.ready[0].editHref != nil)
        #expect(sections.pending.count == 3)
        // The in-transit row has no actions: no Cancel / Edit buttons.
        let inTransit = try #require(sections.pending.first { $0.status == "In transit" })
        #expect(inTransit.cancelHref == nil && inTransit.editHref == nil)
    }
}

// MARK: - Edit sheet → EditOverrides

struct EditDraftTests {
    private let calendar = Calendar.libraryDates
    private let choices = [PickupChoice(code: "25353", name: "CANNINGTON LIBRARY"),
                           PickupChoice(code: "25363", name: "RIVERTON LIBRARY"),
                           PickupChoice(code: "25379", name: "WILLETTON & DISTRICT")]

    private func options(selected: String? = "25363", delay: String? = "2026-11-01",
                         expiry: String? = nil) -> EditOptions {
        EditOptions(canEdit: true, pickupChoices: choices, pickupSelected: selected, delayUntil: delay,
                    notRequiredAfter: expiry, message: nil)
    }

    private func date(_ text: String) -> Date { DueDate.parse(text, calendar: calendar)! }

    @Test func untouchedFormSendsOnlyTheShownPickup() {
        let draft = EditDraft(options: options(), calendar: calendar)
        #expect(draft.pickup == "25363")
        #expect(draft.delay.isOn && draft.delay.date == date("2026-11-01"))
        #expect(!draft.expiry.isOn)
        #expect(!draft.hasChanges(calendar: calendar))
        #expect(draft.overrides(calendar: calendar)
            == EditOverrides(pickup: "25363", delayUntil: nil, notRequiredAfter: nil, clearDelay: false, clearExpiry: false))
    }

    @Test func changedPickupAndDatesAreSent() {
        var draft = EditDraft(options: options(), calendar: calendar)
        draft.pickup = "25353"
        draft.delay.date = date("2026-11-15")
        draft.expiry.isOn = true
        draft.expiry.date = date("2027-01-31")
        #expect(draft.hasChanges(calendar: calendar))
        #expect(draft.overrides(calendar: calendar)
            == EditOverrides(pickup: "25353", delayUntil: "2026-11-15", notRequiredAfter: "2027-01-31",
                             clearDelay: false, clearExpiry: false))
    }

    @Test func switchingOffAnExistingDateClearsIt() {
        var draft = EditDraft(options: options(expiry: "2027-03-01"), calendar: calendar)
        draft.delay.isOn = false
        draft.expiry.isOn = false
        #expect(draft.delay.isCleared && draft.expiry.isCleared)
        #expect(draft.overrides(calendar: calendar)
            == EditOverrides(pickup: "25363", delayUntil: nil, notRequiredAfter: nil, clearDelay: true, clearExpiry: true))

        // Switching off a date the form never had changes nothing.
        var fresh = EditDraft(options: options(delay: nil), calendar: calendar)
        fresh.delay.isOn = true
        fresh.delay.isOn = false
        #expect(!fresh.delay.isCleared)
        #expect(!fresh.hasChanges(calendar: calendar))
    }

    @Test func pickupFallsBackToTheListedBranchThenTheFirst() {
        let listed = EditDraft(options: options(selected: nil), currentPickup: "Willetton & District", calendar: calendar)
        #expect(listed.pickup == "25379")
        let first = EditDraft(options: options(selected: "99999"), currentPickup: nil, calendar: calendar)
        #expect(first.pickup == "25353")
        let digital = EditDraft(options: EditOptions(canEdit: true, pickupChoices: [], pickupSelected: nil, delayUntil: nil,
                                                     notRequiredAfter: nil, message: nil), calendar: calendar)
        #expect(digital.pickup == nil)
        #expect(digital.overrides(calendar: calendar).pickup == nil)
    }

    @Test func libraryDatesStayGregorianOnOtherDeviceCalendars() {
        // Parsed in the device's Buddhist calendar, "2026-11-01" would be year 2026 BE (1483 CE).
        let buddhist = Calendar(identifier: .buddhist)
        #expect(buddhist.component(.year, from: DueDate.parse("2026-11-01", calendar: buddhist)!) == 2026)
        #expect(Calendar.libraryDates.identifier == .gregorian)

        let draft = EditDraft(options: options())
        #expect(Calendar(identifier: .gregorian).component(.year, from: draft.delay.date) == 2026)
        #expect(!draft.hasChanges())
        #expect(draft.overrides().delayUntil == nil)
    }

    @Test func datesAreWrittenInTheFormsOwnFormat() {
        var draft = EditDraft(options: options(delay: "01/11/2026"), calendar: calendar)
        #expect(draft.delay.date == date("2026-11-01"))
        #expect(draft.overrides(calendar: calendar).delayUntil == nil)
        draft.delay.date = date("2026-11-20")
        #expect(draft.overrides(calendar: calendar).delayUntil == "20/11/2026")
        #expect(EditDraft.formValue(date("2026-12-05"), like: nil, calendar: calendar) == "2026-12-05")
    }
}

// MARK: - View model (demo backend, no network)

@MainActor
struct BorrowedViewModelTests {
    /// An AppModel on Canning whose backend is a demo backend wrapped in a call counter.
    private func makeModel(session: String? = nil, delay: Duration = .milliseconds(80)) throws -> (AppModel, CountingBackend) {
        let demo = DemoBackend(publicBackend: FakeBackend(), fixtures: try DemoFixtures.bundled(), mutationDelay: delay,
                               snapshotDelay: .milliseconds(20))
        let backend = CountingBackend(demo)
        let sessions = InMemorySecretStore()
        if let session { sessions.setValue(session, for: LibraryInfo.canning.origin) }
        let dependencies = AppModel.Dependencies(
            defaults: UserDefaults(suiteName: "LibreSpydusTests-\(UUID().uuidString)")!,
            sessions: sessions, verificationCookies: InMemorySecretStore(),
            favouritesDirectory: TestData.temporaryDirectory(), isDemo: true,
            makeBackend: { _ in backend }
        )
        let model = AppModel(dependencies: dependencies)
        model.selectLibrary(.canning)
        return (model, backend)
    }

    @Test func loadsSortedLoansAndSplitReservations() async throws {
        let (model, backend) = try makeModel()
        let viewModel = BorrowedViewModel()
        #expect(viewModel.lists(model) == nil)
        await viewModel.appear(model)
        let lists = try #require(viewModel.lists(model))
        #expect(lists.loans.compactMap(\.due?.daysLeft) == [-2, 2, 5, 12, 19])
        #expect(lists.reservations.ready.count == 1)
        #expect(lists.reservations.pending.count == 3)
        // One call for everything, and opening the screen again while it's fresh fetches nothing.
        #expect(backend.snapshotCalls == 1)
        await viewModel.appear(model)
        #expect(backend.snapshotCalls == 1)
        #expect(backend.accountCalls == 0)
    }

    @Test func cancelIsSubmittedOnceAndRemovesTheRow() async throws {
        let (model, backend) = try makeModel()
        let viewModel = BorrowedViewModel()
        await viewModel.appear(model)
        let row = try #require(viewModel.lists(model)?.reservations.ready.first)

        async let first: Void = viewModel.cancel(row, model: model)
        async let second: Void = viewModel.cancel(row, model: model)
        _ = await (first, second)

        #expect(backend.cancelCalls == 1)
        #expect(viewModel.notice?.message == "Reservation cancelled.")
        #expect(viewModel.notice?.isError == false)
        // Gone at once, then fetched again in the background.
        #expect(viewModel.lists(model)?.reservations.ready.isEmpty == true)
        #expect(model.account.isRefreshing)
        await model.account.refresh()
        #expect(backend.snapshotCalls == 2)
        #expect(viewModel.lists(model)?.reservations.ready.isEmpty == true)
        #expect(!viewModel.isBusy(row))
    }

    @Test func editResultIsShownAndRefreshes() async throws {
        let (model, backend) = try makeModel()
        let viewModel = BorrowedViewModel()
        await viewModel.appear(model)
        let row = try #require(viewModel.lists(model)?.reservations.pending.first { $0.editHref != nil })
        viewModel.edit(row)
        #expect(viewModel.editing == row)

        // What EditReservationSheet's Save does.
        let href = try #require(row.editHref)
        let result = try await model.account.performChange {
            try await backend.editReservation(
                url: href, session: DemoBackend.session,
                overrides: EditOverrides(pickup: "25353", delayUntil: nil, notRequiredAfter: nil, clearDelay: false, clearExpiry: false))
        } accepted: { $0.ok }
        await viewModel.finishEdit(.saved(result), row: row, model: model)
        #expect(viewModel.editing == nil)
        #expect(viewModel.notice?.message == "Reservation updated.")
        await model.account.refresh()
        #expect(viewModel.lists(model)?.reservations.row(id: row.id)?.pickup == "Cannington Library")
    }

    @Test func aLostSessionSignsOutAndAsksForLogin() async throws {
        let (model, _) = try makeModel(session: "expired")
        #expect(model.isSignedIn)
        let viewModel = BorrowedViewModel()
        await viewModel.appear(model)
        #expect(!model.isSignedIn)
        #expect(model.loginNeeded)
        #expect(model.account.snapshot == nil)
    }

    @Test func editSheetSignOutIsReportedAfterTheSheetCloses() async throws {
        let (model, _) = try makeModel()
        let viewModel = BorrowedViewModel()
        await viewModel.appear(model)
        let row = try #require(viewModel.lists(model)?.reservations.pending.first { $0.editHref != nil })
        viewModel.edit(row)
        await viewModel.finishEdit(.failed(SpydusError.NotLoggedIn(message: "not logged in")), row: row, model: model)
        #expect(viewModel.editing == nil)
        #expect(model.isSignedIn) // nothing reported until the sheet has gone
        viewModel.reportDeferredError(model)
        #expect(!model.isSignedIn)
        #expect(model.loginNeeded)
    }

    @Test func aSheetClosedWhileLoadingCantCloseAnother() async throws {
        let (model, _) = try makeModel()
        let viewModel = BorrowedViewModel()
        await viewModel.appear(model)
        let rows = try #require(viewModel.lists(model)).reservations.pending.filter { $0.editHref != nil }
        #expect(rows.count == 2)
        viewModel.edit(rows[1])
        await viewModel.finishEdit(.closed, row: rows[0], model: model)
        #expect(viewModel.editing == rows[1])
    }

    @Test func aBackgroundSignOutWaitsForTheEditSheet() async throws {
        let (model, backend) = try makeModel()
        let viewModel = BorrowedViewModel()
        await viewModel.appear(model)
        let editable = try #require(viewModel.lists(model)?.reservations.pending.first { $0.editHref != nil })
        viewModel.edit(editable)
        model.isPresentingFeatureSheet = true // BorrowedView, while the edit sheet is up
        backend.failSnapshot = SpydusError.NotLoggedIn(message: "not logged in")
        await model.refreshAccount(force: true)
        #expect(!model.isSignedIn && model.loginNeeded)
        #expect(model.rootSheet == nil) // login can't show over the edit sheet
        await viewModel.finishEdit(.closed, row: editable, model: model)
        model.isPresentingFeatureSheet = false // the sheet's onDismiss
        #expect(model.rootSheet == .login)
    }

    @Test func aRefreshFromBeforeACancelDoesntBringTheRowBack() async throws {
        let (model, backend) = try makeModel()
        let viewModel = BorrowedViewModel()
        await viewModel.appear(model)
        let row = try #require(viewModel.lists(model)?.reservations.ready.first)

        backend.delayNextSnapshot = .milliseconds(400) // fetched now, delivered late
        async let refresh: Void = model.account.refresh(force: true)
        try await Task.sleep(for: .milliseconds(50))
        await viewModel.cancel(row, model: model)
        await refresh

        #expect(backend.cancelCalls == 1)
        #expect(backend.snapshotCalls == 3) // the first load, the late one (dropped), one after the cancel
        #expect(viewModel.lists(model)?.reservations.row(id: row.id) == nil)
    }

    @Test func aFailedRefreshKeepsTheLists() async throws {
        let (model, backend) = try makeModel()
        let viewModel = BorrowedViewModel()
        await viewModel.appear(model)
        backend.failSnapshot = SpydusError.Network(message: "timed out")
        await model.refreshAccount(force: true)
        #expect(viewModel.lists(model)?.loans.count == 5)
        #expect(model.account.lastRefreshError?.kind == .network)
        #expect(model.isSignedIn)
    }

    @Test func signedOutLoadsNothing() async throws {
        let (model, backend) = try makeModel()
        model.signOut()
        let viewModel = BorrowedViewModel()
        await viewModel.appear(model)
        #expect(backend.accountCalls == 0)
        #expect(backend.snapshotCalls == 0)
        #expect(viewModel.lists(model) == nil)
    }
}

/// Forwards to a `DemoBackend`, counting account calls.
private final class CountingBackend: LibraryBackend, @unchecked Sendable {
    private let inner: DemoBackend
    private let lock = NSLock()
    private var _cancelCalls = 0
    private var _accountCalls = 0
    private var _snapshotCalls = 0
    private var _delayNextSnapshot: Duration?
    private var _failSnapshot: SpydusError?

    init(_ inner: DemoBackend) { self.inner = inner }

    var cancelCalls: Int { lock.withLock { _cancelCalls } }
    /// `account`, `loans` and `reservations` (the screens use `accountSnapshot` instead).
    var accountCalls: Int { lock.withLock { _accountCalls } }
    var snapshotCalls: Int { lock.withLock { _snapshotCalls } }
    /// The next `accountSnapshot` answers with what it fetched, but only after this delay.
    var delayNextSnapshot: Duration? {
        get { lock.withLock { _delayNextSnapshot } }
        set { lock.withLock { _delayNextSnapshot = newValue } }
    }
    var failSnapshot: SpydusError? {
        get { lock.withLock { _failSnapshot } }
        set { lock.withLock { _failSnapshot = newValue } }
    }
    private func countAccount() { lock.withLock { _accountCalls += 1 } }

    var origin: String { inner.origin }
    var isDemo: Bool { true }
    func setVerificationCookie(_ cookie: String?) { inner.setVerificationCookie(cookie) }
    func search(query: String) async throws -> SearchResults { try await inner.search(query: query) }
    func searchSorted(query: String, sort: SearchSortOption) async throws -> SearchResults {
        try await inner.searchSorted(query: query, sort: sort)
    }
    func page(url: String) async throws -> SearchResults { try await inner.page(url: url) }
    func availability(url: String) async throws -> Availability { try await inner.availability(url: url) }
    func record(url: String) async throws -> RecordDetail { try await inner.record(url: url) }
    func related(url: String) async throws -> RelatedList { try await inner.related(url: url) }
    func login(username: String, password: String) async throws -> String {
        try await inner.login(username: username, password: password)
    }
    func account(session: String) async throws -> AccountLinks {
        countAccount()
        return try await inner.account(session: session)
    }
    func loans(session: String) async throws -> PatronList {
        countAccount()
        return try await inner.loans(session: session)
    }
    func reservations(session: String) async throws -> Reservations {
        countAccount()
        return try await inner.reservations(session: session)
    }
    func accountSnapshot(session: String) async throws -> AccountSnapshot {
        let delay = lock.withLock {
            _snapshotCalls += 1
            defer { _delayNextSnapshot = nil }
            return _delayNextSnapshot
        }
        if let failure = failSnapshot { throw failure }
        let snapshot = try await inner.accountSnapshot(session: session)
        if let delay { try await Task.sleep(for: delay) }
        return snapshot
    }
    func reserveOptions(url: String, session: String) async throws -> ReserveOptions {
        try await inner.reserveOptions(url: url, session: session)
    }
    func placeReservation(url: String, session: String, xdays: String?, pickup: String?) async throws -> ReserveResult {
        try await inner.placeReservation(url: url, session: session, xdays: xdays, pickup: pickup)
    }
    func cancelReservation(url: String, session: String) async throws -> ReserveResult {
        lock.withLock { _cancelCalls += 1 }
        return try await inner.cancelReservation(url: url, session: session)
    }
    func editOptions(url: String, session: String) async throws -> EditOptions {
        try await inner.editOptions(url: url, session: session)
    }
    func editReservation(url: String, session: String, overrides: EditOverrides) async throws -> ReserveResult {
        try await inner.editReservation(url: url, session: session, overrides: overrides)
    }
}
