import Foundation
import LibreSpydusCore
import Testing
@testable import LibreSpydus

// Renewing a loan from Your Books (docs/native-apps.md §5.12): the action, the wording, the
// view model's flow and the demo backend. No network: the demo backend serves the account.

private let base = "https://canning.spydus.com/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/78500000/0"

private func loan(_ title: String = "Whistler", irn: String = "1", due: String? = "15 Oct 2026",
                  actions: [(String, String)] = [("Renew loan", "\(base)/N/3/LOANRENQ?SVL=1&SETDESC=Current%20loans")]) -> PatronItem {
    PatronItem(irn: irn, bibIrn: nil, title: title, bibUrl: nil, author: "Patchett, Ann", year: nil,
               coverUrls: [], isbn: nil, fields: due.map { ["Due": $0] } ?? [:],
               actions: actions.map { PatronAction(label: $0.0, href: $0.1) })
}

// MARK: - Action and wording

struct RenewActionTests {
    private let calendar = Calendar.libraryDates
    private var today: Date { DueDate.parse("12 Oct 2026", calendar: calendar)! }

    @Test func theRenewLinkIsFoundByItsLabel() {
        let href = "\(base)/N/3/LOANRENQ?SVL=1&SETDESC=Current%20loans"
        #expect(loan(actions: [("Renew loan", href)]).renewHref == href)
        #expect(loan(actions: [("RENEW", href)]).renewHref == href)
        #expect(loan(actions: [("Cancel reservation", "\(base)/C/5/RSVCENQ?SVL=1")]).renewHref == nil)
        #expect(loan(actions: [("Renew loan", " ")]).renewHref == nil)
        #expect(loan(actions: []).renewHref == nil)

        let row = LoanRow(item: loan(actions: [("Renew loan", href)]), today: today, calendar: calendar)
        #expect(row.renewHref == href)
        #expect(LoanAction.available(for: row) == [.renew])
        #expect(LoanAction.available(for: row, busy: true).isEmpty)
        #expect([row].hasActions)

        let noRenew = LoanRow(item: loan(actions: []), today: today, calendar: calendar)
        #expect(LoanAction.available(for: noRenew).isEmpty)
        #expect(![noRenew].hasActions)
    }

    @Test func theActionSaysRenewOnTheTileAndRenewLoanInTheMenu() {
        #expect(LoanAction.renew.title == Copy.YourBooks.renew)
        #expect(LoanAction.renew.menuTitle == Copy.YourBooks.renewLoan)
        #expect(LoanAction.renew.title == "Renew")
        #expect(LoanAction.renew.menuTitle == "Renew loan")
        #expect(LoanAction.renew.role == nil)
    }

    @Test func theLoansFootnoteSaysWhereRenewIs() {
        #expect(SwipeReveal.hint(for: .loans, swipes: true).text == Copy.YourBooks.loanSwipeHint)
        #expect(SwipeReveal.hint(for: .loans, swipes: false).text == Copy.YourBooks.loanTouchAndHoldHint)
        #expect(Copy.YourBooks.loanSwipeHint == "Swipe left on a loan to renew it.")
        // The reservations keep theirs.
        #expect(SwipeReveal.hint(swipes: true).text == Copy.YourBooks.swipeHint)
        #expect(SwipeReveal.hint(for: .reservations, swipes: false).text == Copy.YourBooks.touchAndHoldHint)
    }

    @Test func theConfirmationNamesTheBookAndItsDueDate() {
        let dated = LoanRow(item: loan(due: "15 Oct 2026"), today: today, calendar: calendar)
        let date = Formatting.shortDate(DueDate.parse("15 Oct 2026", calendar: calendar)!)
        #expect(RenewText.confirmMessage(for: dated) == Copy.YourBooks.renewMessage(title: "Whistler", date: date))
        #expect(RenewText.confirmMessage(for: dated).hasPrefix("“Whistler” is due "))

        // A date the core can't read is shown as the library wrote it.
        let unreadable = LoanRow(item: loan(due: "end of term"), today: today, calendar: calendar)
        #expect(RenewText.confirmMessage(for: unreadable) == Copy.YourBooks.renewMessage(title: "Whistler", date: "end of term"))

        let undated = LoanRow(item: loan(due: nil), today: today, calendar: calendar)
        #expect(RenewText.confirmMessage(for: undated) == Copy.YourBooks.renewMessageNoDate(title: "Whistler"))
        #expect(RenewText.confirmMessage(for: undated) == "Renewing “Whistler” asks the library for a later due date.")
    }

    @Test func theResultToastsWording() {
        #expect(RenewText.renewed(due: "3 Nov 2026", now: today) == Copy.YourBooks.renewed(date: "3 Nov"))
        #expect(RenewText.renewed(due: "3 Nov 2026", now: today) == "Renewed · Due 3 Nov")
        #expect(RenewText.renewed(due: "5 Jan 2027", now: today) == "Renewed · Due 5 Jan 2027")
        #expect(RenewText.renewed(due: "soon", now: today) == Copy.YourBooks.renewed(date: "soon"))
        #expect(RenewText.renewed(due: nil, now: today) == Copy.YourBooks.renewedNoDate)
        #expect(RenewText.renewed(due: " ", now: today) == "Renewed")

        let reason = "This item cannot be renewed as it is reserved by another borrower."
        #expect(RenewText.refused(RenewResult(ok: false, message: reason, due: "15 Oct 2026")) == reason)
        #expect(RenewText.refused(RenewResult(ok: false, message: nil, due: nil)) == Copy.YourBooks.renewFailed)
    }

    @Test func aRenewedLoanTakesItsNewDateUntilTheNextFetch() {
        let href = "\(base)/N/3/LOANRENQ?SVL=2&SETDESC=Current%20loans"
        let snapshot = AccountSnapshot(
            version: DemoBackend.snapshotVersion, accountName: "Test Borrower",
            loans: PatronList(total: 2, items: [loan(irn: "1"), loan("The long night", irn: "2", due: "10 Oct 2026",
                                                                     actions: [("Renew loan", href)])]),
            reservations: Reservations(ready: PatronList(total: 0, items: []), pending: PatronList(total: 0, items: [])),
            fetchedAtMs: 0)
        let renewed = snapshot.renewingLoan(actionHref: href, due: "31 Oct 2026")
        #expect(renewed.loans.items.map { $0.fields["Due"] } == ["15 Oct 2026", "31 Oct 2026"])
        #expect(renewed.loans.total == 2)
        #expect(snapshot.renewingLoan(actionHref: "elsewhere", due: "31 Oct 2026") == snapshot)
    }
}

// MARK: - Demo backend

struct RenewDemoBackendTests {
    private func makeDemo(delay: Duration = .zero) throws -> DemoBackend {
        DemoBackend(publicBackend: FakeBackend(), fixtures: try DemoFixtures.bundled(), renewDelay: delay)
    }

    private func renewHref(_ demo: DemoBackend, irn: String) async throws -> String {
        let loans = try await demo.loans(session: DemoBackend.session)
        return try #require(loans.items.first { $0.irn == irn }?.renewHref)
    }

    @Test func everyDemoLoanCanBeRenewed() async throws {
        let loans = try await makeDemo().loans(session: DemoBackend.session)
        #expect(loans.items.count == 5)
        #expect(loans.items.allSatisfy { $0.actions.contains { $0.label == "Renew loan" } && $0.renewHref != nil })
    }

    @Test func aRenewalMovesTheDueDateThreeWeeksLater() async throws {
        let demo = try makeDemo(delay: .milliseconds(120))
        let before = try await demo.loans(session: DemoBackend.session)
        let overdue = try #require(before.items.first { $0.irn == "18000001" })
        let href = try #require(overdue.renewHref)
        let due = try #require(overdue.fields["Due"].flatMap { DueDate.parse($0) })

        let clock = ContinuousClock()
        let start = clock.now
        let result = try await demo.renewLoan(url: href, session: DemoBackend.session)
        #expect(clock.now - start >= .milliseconds(120))

        let expected = DueDate.format(Calendar.library.date(byAdding: .day, value: DemoBackend.renewalDays, to: due)!)
        #expect(result == RenewResult(ok: true, message: nil, due: expected))
        let after = try await demo.loans(session: DemoBackend.session)
        #expect(after.items.first { $0.irn == "18000001" }?.fields["Due"] == expected)
        // The snapshot the app saves says the same.
        let snapshot = try await demo.accountSnapshot(session: DemoBackend.session)
        #expect(snapshot.loans.items.first { $0.irn == "18000001" }?.fields["Due"] == expected)
    }

    @Test func oneDemoLoanIsRefusedWithTheLibrarysReason() async throws {
        let demo = try makeDemo()
        let before = try await demo.loans(session: DemoBackend.session)
        let refused = try #require(before.items.first { $0.irn == DemoBackend.unrenewableLoanIrn })
        #expect(refused.title == "Whistler")

        let result = try await demo.renewLoan(url: try #require(refused.renewHref), session: DemoBackend.session)
        #expect(!result.ok)
        #expect(result.message == "This item cannot be renewed as it is reserved by another borrower.")
        #expect(result.due == refused.fields["Due"])
        let after = try await demo.loans(session: DemoBackend.session)
        #expect(after == before)
    }

    @Test func aLinkThatIsntOnTheListRenewsNothing() async throws {
        let demo = try makeDemo()
        let before = try await demo.loans(session: DemoBackend.session)
        let result = try await demo.renewLoan(url: "\(base)/N/3/LOANRENQ?SVL=999", session: DemoBackend.session)
        #expect(!result.ok)
        #expect(result.due == nil)
        #expect(try await demo.loans(session: DemoBackend.session) == before)
    }

    @Test func renewingNeedsTheDemoSession() async throws {
        let demo = try makeDemo()
        let href = try await renewHref(demo, irn: "18000003")
        await #expect(throws: SpydusError.NotLoggedIn(message: "not logged in")) {
            try await demo.renewLoan(url: href, session: "someone-else")
        }
    }

    @Test func aBackendWithoutLoansNeverRenews() async throws {
        let result = try await FakeBackend().renewLoan(url: "\(base)/N/3/LOANRENQ?SVL=1", session: "s")
        #expect(result == RenewResult(ok: false, message: nil, due: nil))
    }
}

// MARK: - View model

@MainActor
struct RenewViewModelTests {
    /// An AppModel on Canning, signed in to a demo account (renewals answer after `delay`) that
    /// `RenewBackend` wraps. `persists`: the account is saved in `cacheDirectory`, as outside demo mode.
    private func makeModel(delay: Duration = .milliseconds(40), persists: Bool = false,
                           cacheDirectory: URL? = nil) throws -> (AppModel, RenewBackend) {
        let demo = DemoBackend(publicBackend: FakeBackend(), fixtures: try DemoFixtures.bundled(), mutationDelay: delay,
                               renewDelay: delay, snapshotDelay: .milliseconds(10))
        let backend = RenewBackend(demo)
        let sessions = InMemorySecretStore()
        sessions.setValue(DemoBackend.session, for: LibraryInfo.canning.origin)
        let dependencies = AppModel.Dependencies(
            defaults: UserDefaults(suiteName: "LibreSpydusTests-\(UUID().uuidString)")!,
            sessions: sessions, verificationCookies: InMemorySecretStore(),
            favouritesDirectory: TestData.temporaryDirectory(), isDemo: !persists,
            accountCacheDirectory: cacheDirectory,
            makeBackend: { _ in backend }
        )
        let model = AppModel(dependencies: dependencies)
        model.selectLibrary(.canning)
        return (model, backend)
    }

    private func loadedRow(_ viewModel: BorrowedViewModel, _ model: AppModel, irn: String) async throws -> LoanRow {
        await viewModel.appear(model)
        return try #require(viewModel.lists(model)?.loans.first { $0.item.irn == irn })
    }

    /// Waits (briefly) until `condition` holds.
    private func waitUntil(_ condition: () -> Bool) async throws {
        var tries = 0
        while !condition(), tries < 400 {
            tries += 1
            try await Task.sleep(for: .milliseconds(5))
        }
        #expect(condition())
    }

    @Test func renewingAsksFirst() async throws {
        let (model, backend) = try makeModel()
        let viewModel = BorrowedViewModel()
        let row = try await loadedRow(viewModel, model, irn: "18000003")

        viewModel.requestRenew(row)
        #expect(viewModel.confirmingRenew == row)
        #expect(backend.renewCalls == 0)
        // A loan without the link has nothing to confirm.
        let bare = LoanRow(item: loan(actions: []))
        viewModel.confirmingRenew = nil
        viewModel.requestRenew(bare)
        #expect(viewModel.confirmingRenew == nil)
    }

    @Test func aRenewalShowsProgressAndRunsOnceAtATime() async throws {
        let (model, backend) = try makeModel()
        let viewModel = BorrowedViewModel()
        let row = try await loadedRow(viewModel, model, irn: "18000001")
        backend.holdRenewals()
        viewModel.swipedRowID = row.id

        let first = Task { await viewModel.renew(row, model: model) }
        try await waitUntil { backend.renewCalls == 1 }
        #expect(viewModel.isRenewing(row))
        #expect(LoanAction.available(for: row, busy: viewModel.isRenewing(row)).isEmpty)
        #expect(viewModel.toast?.message == Copy.YourBooks.renewing)
        #expect(viewModel.toast?.message == "Renewing…")
        #expect(viewModel.toast?.tone == .progress)
        #expect(viewModel.swipedRowID == nil)
        // A second tap (swipe, menu or VoiceOver) while it runs sends nothing, and can't confirm again.
        await viewModel.renew(row, model: model)
        viewModel.requestRenew(row)
        #expect(viewModel.confirmingRenew == nil)
        #expect(backend.renewCalls == 1)

        backend.releaseRenewals()
        await first.value
        #expect(!viewModel.isRenewing(row))
        #expect(backend.renewCalls == 1)
        #expect(viewModel.toast?.tone == .success)
    }

    @Test func aRenewedLoanShowsItsNewDateAndTheAccountIsFetchedAgain() async throws {
        let (model, backend) = try makeModel()
        let viewModel = BorrowedViewModel()
        let row = try await loadedRow(viewModel, model, irn: "18000001")
        #expect(row.due?.daysLeft == -2)
        #expect(backend.snapshotCalls == 1)
        var fetched: [AccountSnapshot] = []
        model.account.didFetch = { snapshot, _ in fetched.append(snapshot) }

        await viewModel.renew(row, model: model)

        let renewedDue = DueDate.format(Calendar.library.date(byAdding: .day, value: 19, to: Calendar.library.startOfDay(for: .now))!)
        #expect(viewModel.toast?.message == RenewText.renewed(due: renewedDue))
        #expect(viewModel.toast?.tone == .success)
        // The card moves to its new date at once…
        let renewed = try #require(viewModel.lists(model)?.loans.first { $0.item.irn == "18000001" })
        #expect(renewed.due?.daysLeft == 19)
        #expect(renewed.dueText == renewedDue)
        // …and the account is fetched again, which the alerts (due-date reminders) process.
        #expect(model.account.isRefreshing)
        await model.account.refresh()
        #expect(backend.snapshotCalls == 2)
        #expect(fetched.last?.loans.items.first { $0.irn == "18000001" }?.fields["Due"] == renewedDue)
        #expect(viewModel.lists(model)?.loans.first { $0.item.irn == "18000001" }?.due?.daysLeft == 19)
        // Home's "Your loans" reads the same snapshot.
        guard case .loaded(let shelf) = HomeViewModel().loans(model) else {
            Issue.record("Home's loans didn't load")
            return
        }
        let homeLoan = try #require(shelf.first { $0.ref.title == renewed.ref.title })
        #expect(homeLoan.dueDate == renewed.due?.dueDate)
    }

    @Test func theBookPagesOnLoanPillRenewsTheSameWay() async throws {
        let (model, backend) = try makeModel()
        let row = try await loadedRow(BorrowedViewModel(), model, irn: "18000001")
        let snapshot = try #require(model.account.snapshot)
        let bibIrn = try #require(row.item.bibIrn)
        let holding = try #require(accountHolding(snapshot: snapshot, bibIrns: [bibIrn]))
        let pill = HeldStatus(holding)
        let loan = try #require(pill.renewableLoan)
        #expect(pill.accessibilityHint == Copy.YourBooks.renewHint)

        let renewal = try await model.renew(LoanRow(item: loan))
        let result = try #require(renewal)

        #expect(result.ok)
        #expect(backend.renewCalls == 1)
        #expect(BorrowedViewModel.ToastMessage(renewal: result).message == RenewText.renewed(due: result.due))
        // The pill follows the saved account to the new due date at once, and the account is
        // fetched again.
        let after = try #require(model.account.snapshot)
        let renewed = try #require(accountHolding(snapshot: after, bibIrns: [bibIrn]))
        #expect(renewed.item.fields["Due"] == result.due)
        #expect(HeldStatus(renewed).text != pill.text)
        #expect(model.account.isRefreshing)
    }

    @Test func aRefusedRenewalShowsTheLibrarysReasonAndRefreshes() async throws {
        let (model, backend) = try makeModel()
        let viewModel = BorrowedViewModel()
        let row = try await loadedRow(viewModel, model, irn: DemoBackend.unrenewableLoanIrn)

        await viewModel.renew(row, model: model)

        #expect(viewModel.toast?.message == "This item cannot be renewed as it is reserved by another borrower.")
        #expect(viewModel.toast?.tone == .error)
        #expect(!viewModel.isRenewing(row))
        await model.account.refresh()
        #expect(backend.snapshotCalls == 2)
        #expect(viewModel.lists(model)?.loans.first { $0.id == row.id }?.dueText == row.dueText)
    }

    @Test func aRefusalWithoutAReasonSaysItCouldntRenew() async throws {
        let (model, backend) = try makeModel()
        let viewModel = BorrowedViewModel()
        let row = try await loadedRow(viewModel, model, irn: "18000003")
        backend.renewOutcome = .success(RenewResult(ok: false, message: nil, due: row.dueText))

        await viewModel.renew(row, model: model)

        #expect(viewModel.toast?.message == Copy.YourBooks.renewFailed)
        #expect(viewModel.toast?.tone == .error)
    }

    @Test func aFailedRenewalSaysItCouldntRenew() async throws {
        let (model, backend) = try makeModel()
        let viewModel = BorrowedViewModel()
        let row = try await loadedRow(viewModel, model, irn: "18000003")
        backend.renewOutcome = .failure(SpydusError.Network(message: "timed out"))

        await viewModel.renew(row, model: model)

        #expect(viewModel.toast?.message == Copy.YourBooks.renewFailed)
        #expect(viewModel.toast?.message == "Couldn’t renew this loan")
        #expect(viewModel.toast?.tone == .error)
        #expect(!viewModel.isRenewing(row))
        #expect(model.isSignedIn)
        #expect(viewModel.lists(model)?.loans.first { $0.id == row.id }?.dueText == row.dueText)
    }

    @Test func aLostSessionSignsOutAndAsksToSignInAgain() async throws {
        let (model, backend) = try makeModel()
        let viewModel = BorrowedViewModel()
        let row = try await loadedRow(viewModel, model, irn: "18000003")
        backend.renewOutcome = .failure(SpydusError.NotLoggedIn(message: "not logged in"))

        await viewModel.renew(row, model: model)

        #expect(!model.isSignedIn)
        #expect(model.loginNeeded)
        #expect(model.account.snapshot == nil)
        #expect(viewModel.toast == nil)
        #expect(!viewModel.isRenewing(row))
    }

    @Test func theSavedAccountKeepsTheNewDate() async throws {
        let directory = TestData.temporaryDirectory()
        let (model, _) = try makeModel(persists: true, cacheDirectory: directory)
        let viewModel = BorrowedViewModel()
        let row = try await loadedRow(viewModel, model, irn: "18000004")

        await viewModel.renew(row, model: model)
        await model.account.refresh()

        let saved = try #require(AccountSnapshotCache(directory: directory).load(origin: LibraryInfo.canning.origin))
        let expected = DueDate.format(Calendar.library.date(byAdding: .day, value: 12 + 21, to: Calendar.library.startOfDay(for: .now))!)
        #expect(saved.loans.items.first { $0.irn == "18000004" }?.fields["Due"] == expected)
    }
}

/// Forwards to a `DemoBackend`, counting renewals and snapshots; renewals can be held in flight or
/// answered with `renewOutcome` instead.
private final class RenewBackend: LibraryBackend, @unchecked Sendable {
    private let inner: DemoBackend
    private let lock = NSLock()
    private var _renewCalls = 0
    private var _snapshotCalls = 0
    private var _renewOutcome: Result<RenewResult, SpydusError>?
    private var held = false
    private var waiting: [CheckedContinuation<Void, Never>] = []

    init(_ inner: DemoBackend) { self.inner = inner }

    var renewCalls: Int { lock.withLock { _renewCalls } }
    var snapshotCalls: Int { lock.withLock { _snapshotCalls } }
    var renewOutcome: Result<RenewResult, SpydusError>? {
        get { lock.withLock { _renewOutcome } }
        set { lock.withLock { _renewOutcome = newValue } }
    }

    /// Renewals wait until `releaseRenewals()`.
    func holdRenewals() { lock.withLock { held = true } }

    func releaseRenewals() {
        let continuations = lock.withLock {
            held = false
            defer { waiting = [] }
            return waiting
        }
        continuations.forEach { $0.resume() }
    }

    func renewLoan(url: String, session: String) async throws -> RenewResult {
        lock.withLock { _renewCalls += 1 }
        await withCheckedContinuation { continuation in
            let wait = lock.withLock {
                if held { waiting.append(continuation) }
                return held
            }
            if !wait { continuation.resume() }
        }
        if let outcome = renewOutcome { return try outcome.get() }
        return try await inner.renewLoan(url: url, session: session)
    }

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
    func account(session: String) async throws -> AccountLinks { try await inner.account(session: session) }
    func loans(session: String) async throws -> PatronList { try await inner.loans(session: session) }
    func reservations(session: String) async throws -> Reservations { try await inner.reservations(session: session) }
    func accountSnapshot(session: String) async throws -> AccountSnapshot {
        lock.withLock { _snapshotCalls += 1 }
        return try await inner.accountSnapshot(session: session)
    }
    func reserveOptions(url: String, session: String) async throws -> ReserveOptions {
        try await inner.reserveOptions(url: url, session: session)
    }
    func placeReservation(url: String, session: String, xdays: String?, pickup: String?) async throws -> ReserveResult {
        try await inner.placeReservation(url: url, session: session, xdays: xdays, pickup: pickup)
    }
    func cancelReservation(url: String, session: String) async throws -> ReserveResult {
        try await inner.cancelReservation(url: url, session: session)
    }
    func editOptions(url: String, session: String) async throws -> EditOptions {
        try await inner.editOptions(url: url, session: session)
    }
    func editReservation(url: String, session: String, overrides: EditOverrides) async throws -> ReserveResult {
        try await inner.editReservation(url: url, session: session, overrides: overrides)
    }
}
