import Foundation
import LibreSpydusCore
import Observation

/// State for "Your Books": the cards built from the saved account (`AppModel.account`), cancel/edit
/// results, renewals (§5.12), and which rows have a mutation in flight (one at a time per row, so
/// nothing is submitted twice).
@MainActor
@Observable
final class BorrowedViewModel {
    enum Segment: String, CaseIterable, Identifiable {
        case loans
        case reservations

        var id: String { rawValue }
        var title: String {
            switch self {
            case .loans: Copy.YourBooks.loans
            case .reservations: Copy.YourBooks.reservations
            }
        }
    }

    /// A cancel/edit result shown as a banner above the list.
    struct Notice: Identifiable, Equatable {
        let id = UUID()
        var message: String
        var isError: Bool
    }

    /// A renewal's progress or result, shown as a toast above the tab bar (`RenewToastOverlay`).
    struct ToastMessage: Identifiable, Equatable {
        enum Tone: Equatable {
            /// Stays until the result replaces it.
            case progress
            case success
            case error
        }

        let id = UUID()
        var message: String
        var tone: Tone
    }

    /// The cards for one account snapshot.
    struct Lists: Equatable {
        var loans: [LoanRow]
        var reservations: ReservationSections
    }

    var segment: Segment = .loans
    /// Rows with a cancel or edit in flight.
    private(set) var busyRows: Set<String> = []
    /// Loans with a renewal in flight.
    private(set) var renewingRows: Set<String> = []
    var notice: Notice?
    /// "Renewing…", then the renewal's result.
    var toast: ToastMessage?
    /// The row whose "Cancel" was tapped, awaiting confirmation.
    var confirmingCancel: ReservationRow?
    /// The loan whose "Renew" was tapped, awaiting confirmation.
    var confirmingRenew: LoanRow?
    /// The card whose swipe actions are showing (one card at a time, in the segment on screen).
    var swipedRowID: String?
    /// The row whose edit sheet is shown.
    var editing: ReservationRow?
    /// A sign-in or verification error from the edit sheet, reported once the sheet is gone
    /// (those are root sheets and can't appear over it).
    @ObservationIgnored var deferredError: (any Error)?

    @ObservationIgnored private let today: () -> Date
    /// The last snapshot's cards, so a redraw doesn't rebuild them.
    @ObservationIgnored private var built: (snapshot: AccountSnapshot, lists: Lists)?

    init(today: @escaping () -> Date = { .now }) {
        self.today = today
    }

    // MARK: Lists

    /// The cards for `snapshot`; nil before the first one has loaded.
    func lists(for snapshot: AccountSnapshot?) -> Lists? {
        guard let snapshot else { return nil }
        if let built, built.snapshot == snapshot { return built.lists }
        let lists = Lists(loans: LoanRow.rows(from: snapshot.loans.items, today: today()),
                          reservations: ReservationSections(snapshot.reservations))
        built = (snapshot, lists)
        return lists
    }

    /// The cards for the account on screen.
    func lists(_ model: AppModel) -> Lists? {
        lists(for: model.account.snapshot)
    }

    /// The screen opened, or the library, session or verification changed: fetch the account if
    /// it's out of date (the saved one stays on screen meanwhile).
    func appear(_ model: AppModel) async {
        guard model.isSignedIn else {
            busyRows = []
            renewingRows = []
            swipedRowID = nil
            return
        }
        await model.refreshAccount(force: false)
    }

    // MARK: Cancel

    func isBusy(_ row: ReservationRow) -> Bool {
        busyRows.contains(row.id)
    }

    /// Asks to confirm cancelling `row` (not while it's busy). Its swipe actions stay out until
    /// the user answers.
    func requestCancel(_ row: ReservationRow) {
        guard row.cancelHref != nil, !busyRows.contains(row.id) else { return }
        confirmingCancel = row
    }

    /// Cancels `row` after the user confirmed. Ignored while that row is busy or once it's gone.
    /// On success the card goes at once and the account is fetched again.
    func cancel(_ row: ReservationRow, model: AppModel) async {
        guard let href = row.cancelHref, let session = model.session, !busyRows.contains(row.id),
              lists(model)?.reservations.row(id: row.id) != nil else { return }
        let key = model.reloadKey
        let backend = model.backend
        busyRows.insert(row.id)
        if swipedRowID == row.id { swipedRowID = nil }
        defer { busyRows.remove(row.id) }
        do {
            let result = try await model.account.performChange {
                try await backend.cancelReservation(url: href, session: session)
            } accepted: { $0.ok }
            guard model.reloadKey == key else { return }
            notice = Notice(message: result.message.nonEmpty ?? (result.ok ? Copy.YourBooks.cancelled : Copy.YourBooks.cancelFailed),
                            isError: !result.ok)
            if result.ok {
                model.account.update { $0 = $0.removingReservation(actionHref: href) }
            }
        } catch {
            guard model.reloadKey == key, let shown = report(error, model: model) else { return }
            notice = Notice(message: shown.message, isError: true)
        }
    }

    // MARK: Renew

    func isRenewing(_ row: LoanRow) -> Bool {
        renewingRows.contains(row.id)
    }

    /// Asks to confirm renewing `row` (not while it's renewing). Its swipe actions stay out until
    /// the user answers.
    func requestRenew(_ row: LoanRow) {
        guard row.renewHref != nil, !renewingRows.contains(row.id) else { return }
        confirmingRenew = row
    }

    /// Renews `row` after the user confirmed: the card shows progress and "Renewing…" while the
    /// library answers (a few seconds), then the result; the account is fetched again either way,
    /// so the card, Home's "Your loans" and the due-date reminders follow. A renewed loan shows its
    /// new date at once. Ignored while that loan is renewing or once it's gone.
    func renew(_ row: LoanRow, model: AppModel) async {
        guard let href = row.renewHref, let session = model.session, !renewingRows.contains(row.id),
              lists(model)?.loans.contains(where: { $0.id == row.id }) == true else { return }
        let key = model.reloadKey
        let backend = model.backend
        renewingRows.insert(row.id)
        if swipedRowID == row.id { swipedRowID = nil }
        let progress = ToastMessage(message: Copy.YourBooks.renewing, tone: .progress)
        toast = progress
        defer { renewingRows.remove(row.id) }
        do {
            let result = try await model.account.performChange {
                try await backend.renewLoan(url: href, session: session)
            } accepted: { _ in true }
            guard model.reloadKey == key else { return dismiss(progress) }
            if result.ok {
                if let due = result.due.nonEmpty {
                    model.account.update { $0 = $0.renewingLoan(actionHref: href, due: due) }
                }
                toast = ToastMessage(message: RenewText.renewed(due: result.due), tone: .success)
            } else {
                toast = ToastMessage(message: RenewText.refused(result), tone: .error)
            }
        } catch {
            guard model.reloadKey == key else { return dismiss(progress) }
            // A lost session signs out and asks to sign in again (§5.2): that says enough.
            guard report(error, model: model) != nil, !Self.isSessionExpired(error) else { return dismiss(progress) }
            toast = ToastMessage(message: Copy.YourBooks.renewFailed, tone: .error)
        }
    }

    // MARK: Edit

    /// Opens the edit sheet for `row` (not while it's busy).
    func edit(_ row: ReservationRow) {
        guard row.editHref != nil, !busyRows.contains(row.id) else { return }
        swipedRowID = nil
        editing = row
    }

    /// `row`'s edit sheet finished: show the result (the sheet's save already asked for a fresh
    /// account).
    func finishEdit(_ outcome: EditReservationSheet.Outcome, row: ReservationRow, model: AppModel) async {
        // A sheet that's already gone (closed while loading) can still finish late: keep only
        // its error, reported now or when the sheet on screen closes.
        guard editing?.id == row.id else {
            if case .failed(let error) = outcome, let shown = report(error, model: model) {
                notice = Notice(message: shown.message, isError: true)
            }
            return
        }
        editing = nil
        switch outcome {
        case .closed:
            break
        case .saved(let result):
            notice = Notice(message: result.message.nonEmpty ?? Copy.YourBooks.updated, isError: !result.ok)
        case .failed(let error):
            deferredError = error
        }
    }

    /// After the edit sheet is dismissed: route a deferred sign-in/verification error.
    func reportDeferredError(_ model: AppModel) {
        guard let error = deferredError else { return }
        deferredError = nil
        if let shown = model.report(error) {
            notice = Notice(message: shown.message, isError: true)
        }
    }

    // MARK: Private

    /// `AppModel.report`, except while the edit sheet is up: sign-in and verification are root
    /// sheets that can't appear over it, so those wait until it closes (`reportDeferredError`).
    private func report(_ error: any Error, model: AppModel) -> UserFacingError? {
        guard editing != nil, Self.needsRootSheet(error) else { return model.report(error) }
        deferredError = error
        return UserFacingError(error, libraryName: model.libraryName)
    }

    /// Takes `progress` down unless something has replaced it.
    private func dismiss(_ progress: ToastMessage) {
        if toast?.id == progress.id { toast = nil }
    }

    private static func isSessionExpired(_ error: any Error) -> Bool {
        if case .NotLoggedIn = error as? SpydusError { true } else { false }
    }

    /// Errors `AppModel.report` answers with a root sheet (login, verification).
    static func needsRootSheet(_ error: any Error) -> Bool {
        switch error as? SpydusError {
        case .NotLoggedIn, .HumanVerificationRequired: true
        default: false
        }
    }
}

extension AccountSnapshot {
    /// Without the reservation whose actions include `href` (just cancelled), until the next fetch.
    func removingReservation(actionHref href: String) -> AccountSnapshot {
        func removing(from list: PatronList) -> PatronList {
            let items = list.items.filter { item in !item.actions.contains { $0.href == href } }
            let removed = Int64(list.items.count - items.count)
            return PatronList(total: list.total.map { max(0, $0 - removed) }, items: items)
        }
        var snapshot = self
        snapshot.reservations = Reservations(ready: removing(from: reservations.ready),
                                             pending: removing(from: reservations.pending))
        return snapshot
    }
}
