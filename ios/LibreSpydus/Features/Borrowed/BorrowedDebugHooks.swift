import SwiftUI

/// Screenshot hooks for "Your Books" and Favourites (DEBUG builds only), read from launch
/// arguments like the app's other `-LS…` hooks (README "Debug launch hooks"):
///
/// - `-LSScrollToBottom YES` opens Borrowed and Favourites scrolled to the end, to check the last
///   card clears the tab bar.
/// - `-LSBorrowedSegment reservations` opens on Reservations.
/// - `-LSBorrowedScript <step>` (demo mode only) runs one step once the lists have loaded:
///   `cancel` (the confirmation for the first ready reservation), `cancel-confirmed` (cancels it
///   and shows the result), `edit` (the edit sheet for the first editable pending reservation),
///   `edit-saved` (that sheet picks another branch and saves), `swipe` (the first ready
///   reservation's swipe actions showing), `swipe-cancel` (its confirmation over the open card);
///   for loans (§5.12) `swipe-renew` (the first loan's Renew showing), `renew` (its confirmation),
///   `renew-confirmed` (renews it: "Renewing…", then the result) and `renew-refused` (renews the
///   demo loan the library refuses).
///
/// Mutating steps only ever reach `DemoBackend`, never a real library.
enum BorrowedDebugHooks {
    /// Nil in release builds and without `-LSScrollToBottom`.
    static var initialScrollAnchor: UnitPoint? {
        #if DEBUG
        UserDefaults.standard.bool(forKey: "LSScrollToBottom") ? .bottom : nil
        #else
        nil
        #endif
    }
}

#if DEBUG
extension BorrowedDebugHooks {
    enum Script: String {
        case cancel
        case cancelConfirmed = "cancel-confirmed"
        case edit
        case editSaved = "edit-saved"
        case swipe
        case swipeCancel = "swipe-cancel"
        case swipeRenew = "swipe-renew"
        case renew
        case renewConfirmed = "renew-confirmed"
        case renewRefused = "renew-refused"
    }

    static var segment: BorrowedViewModel.Segment? {
        UserDefaults.standard.string(forKey: "LSBorrowedSegment").flatMap(BorrowedViewModel.Segment.init(rawValue:))
    }

    static var script: Script? {
        UserDefaults.standard.string(forKey: "LSBorrowedScript").flatMap(Script.init(rawValue:))
    }

    /// `edit-saved`: the edit sheet changes the pickup branch and saves by itself.
    static var autoSavesEdit: Bool { script == .editSaved }

    /// Runs the launch script once, after the account first shows in demo mode.
    @MainActor
    static func run(_ viewModel: BorrowedViewModel, model: AppModel) async {
        guard model.isDemo, model.backend.isDemo, !didRun else { return }
        didRun = true
        if let segment { viewModel.segment = segment }
        guard let script, let lists = viewModel.lists(model) else { return }
        let sections = lists.reservations
        let renewable = lists.loans.first { $0.renewHref != nil }
        let ready = sections.ready.first { $0.cancelHref != nil }
        let editable = sections.pending.first { $0.editHref != nil }
        switch script {
        case .cancel:
            viewModel.confirmingCancel = ready
        case .cancelConfirmed:
            if let ready { await viewModel.cancel(ready, model: model) }
        case .edit, .editSaved:
            if let editable { viewModel.edit(editable) }
        case .swipe, .swipeCancel:
            guard let ready else { return }
            viewModel.segment = .reservations
            try? await Task.sleep(for: .milliseconds(400))
            viewModel.swipedRowID = ready.id
            if script == .swipeCancel {
                try? await Task.sleep(for: .milliseconds(600))
                viewModel.requestCancel(ready)
            }
        case .swipeRenew:
            guard let renewable else { return }
            viewModel.segment = .loans
            try? await Task.sleep(for: .milliseconds(400))
            viewModel.swipedRowID = renewable.id
        case .renew:
            viewModel.segment = .loans
            if let renewable { viewModel.requestRenew(renewable) }
        case .renewConfirmed:
            viewModel.segment = .loans
            if let renewable { await viewModel.renew(renewable, model: model) }
        case .renewRefused:
            viewModel.segment = .loans
            if let refused = lists.loans.first(where: { $0.item.irn == DemoBackend.unrenewableLoanIrn }) {
                await viewModel.renew(refused, model: model)
            }
        }
    }

    @MainActor private static var didRun = false
}
#endif
