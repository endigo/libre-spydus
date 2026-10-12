import Foundation
import LibreSpydusCore
import Observation

/// State for "Your Books": loans and reservations from the core, cancel/edit results, and which
/// rows have a mutation in flight (one at a time per row, so nothing is submitted twice).
@MainActor
@Observable
final class BorrowedViewModel {
    enum Segment: String, CaseIterable, Identifiable {
        case loans
        case reservations

        var id: String { rawValue }
        var title: String {
            switch self {
            case .loans: "Loans"
            case .reservations: "Reservations"
            }
        }
    }

    /// A cancel/edit result shown as a banner above the list.
    struct Notice: Identifiable, Equatable {
        let id = UUID()
        var message: String
        var isError: Bool
    }

    var segment: Segment = .loans
    private(set) var loans: Loadable<[LoanRow]> = .idle
    private(set) var reservations: Loadable<ReservationSections> = .idle
    /// Rows with a cancel or edit in flight.
    private(set) var busyRows: Set<String> = []
    var notice: Notice?
    /// The row whose "Cancel" was tapped, awaiting confirmation.
    var confirmingCancel: ReservationRow?
    /// The reservation whose swipe actions are showing (one card at a time).
    var swipedRowID: String?
    /// The row whose edit sheet is shown.
    var editing: ReservationRow?
    /// A sign-in or verification error from the edit sheet, reported once the sheet is gone
    /// (those are root sheets and can't appear over it).
    @ObservationIgnored var deferredError: (any Error)?

    @ObservationIgnored private let today: () -> Date
    /// Bumped when a cancel or edit starts and ends: a reservations fetch that began before is
    /// out of date and is dropped (it could bring a cancelled reservation back).
    @ObservationIgnored private var reservationsVersion = 0

    init(today: @escaping () -> Date = { .now }) {
        self.today = today
    }

    // MARK: Loading

    /// Loads loans and reservations together. `keepContent` (pull to refresh, after a cancel)
    /// leaves the current lists on screen until the new ones arrive.
    func load(_ model: AppModel, keepContent: Bool = false) async {
        guard let session = model.session else {
            loans = .idle
            reservations = .idle
            busyRows = []
            swipedRowID = nil
            return
        }
        let key = model.reloadKey
        let version = reservationsVersion
        let backend = model.backend
        if !keepContent || loans.value == nil { loans = .loading }
        if !keepContent || reservations.value == nil { reservations = .loading }

        async let loanList = Self.capture { try await backend.loans(session: session) }
        async let reservationLists = Self.capture { try await backend.reservations(session: session) }
        let (loanResult, reservationResult) = await (loanList, reservationLists)
        // A sign-out, library switch or verification while loading: the next load takes over.
        guard model.reloadKey == key, model.session == session else { return }

        let now = today()
        apply(loanResult.map { LoanRow.rows(from: $0.items, today: now) }, to: \.loans, model: model)
        // A lost session was just reported (sign-in sheet); don't report it twice.
        guard model.session == session, reservationsVersion == version else { return }
        apply(reservationResult.map { ReservationSections($0) }, to: \.reservations, model: model)
    }

    /// Reloads only the reservations, keeping the list on screen (after a cancel or edit).
    func reloadReservations(_ model: AppModel) async {
        guard let session = model.session else { return }
        let key = model.reloadKey
        let version = reservationsVersion
        let backend = model.backend
        let result = await Self.capture { try await backend.reservations(session: session) }
        guard model.reloadKey == key, model.session == session, reservationsVersion == version else { return }
        apply(result.map { ReservationSections($0) }, to: \.reservations, model: model)
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
    func cancel(_ row: ReservationRow, model: AppModel) async {
        guard let href = row.cancelHref, let session = model.session, !busyRows.contains(row.id),
              reservations.value?.row(id: row.id) != nil else { return }
        let key = model.reloadKey
        busyRows.insert(row.id)
        if swipedRowID == row.id { swipedRowID = nil }
        reservationsVersion += 1
        defer { busyRows.remove(row.id) }
        do {
            let result = try await model.backend.cancelReservation(url: href, session: session)
            reservationsVersion += 1
            guard model.reloadKey == key else { return }
            notice = Notice(message: result.message.nonEmpty ?? (result.ok ? "Reservation cancelled." : "The library didn't cancel this reservation."),
                            isError: !result.ok)
            if result.ok {
                if let sections = reservations.value {
                    reservations = .loaded(sections.removing(id: row.id))
                }
                await reloadReservations(model)
            }
        } catch {
            reservationsVersion += 1
            guard model.reloadKey == key, let shown = report(error, model: model) else { return }
            notice = Notice(message: shown.message, isError: true)
        }
    }

    // MARK: Edit

    /// Opens the edit sheet for `row` (not while it's busy).
    func edit(_ row: ReservationRow) {
        guard row.editHref != nil, !busyRows.contains(row.id) else { return }
        reservationsVersion += 1
        swipedRowID = nil
        editing = row
    }

    /// `row`'s edit sheet finished: show the result and refresh on success.
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
        reservationsVersion += 1
        switch outcome {
        case .closed:
            break
        case .saved(let result):
            notice = Notice(message: result.message.nonEmpty ?? "Reservation updated.", isError: !result.ok)
            if result.ok { await reloadReservations(model) }
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

    private func apply<Value>(_ result: Result<Value, any Error>,
                              to keyPath: ReferenceWritableKeyPath<BorrowedViewModel, Loadable<Value>>,
                              model: AppModel) {
        switch result {
        case .success(let value):
            self[keyPath: keyPath] = .loaded(value)
        case .failure(let error):
            if let shown = report(error, model: model) {
                self[keyPath: keyPath] = .failed(shown)
            } else if self[keyPath: keyPath].isLoading {
                self[keyPath: keyPath] = .idle // cancelled
            }
        }
    }

    /// `AppModel.report`, except while the edit sheet is up: sign-in and verification are root
    /// sheets that can't appear over it, so those wait until it closes (`reportDeferredError`).
    private func report(_ error: any Error, model: AppModel) -> UserFacingError? {
        guard editing != nil, Self.needsRootSheet(error) else { return model.report(error) }
        deferredError = error
        return UserFacingError(error, libraryName: model.libraryName)
    }

    /// Errors `AppModel.report` answers with a root sheet (login, verification).
    static func needsRootSheet(_ error: any Error) -> Bool {
        switch error as? SpydusError {
        case .NotLoggedIn, .HumanVerificationRequired: true
        default: false
        }
    }

    private nonisolated static func capture<Value: Sendable>(
        _ work: @Sendable () async throws -> Value
    ) async -> Result<Value, any Error> {
        do {
            return .success(try await work())
        } catch {
            return .failure(error)
        }
    }
}
