import LibreSpydusCore
import SwiftUI

// Renewing a loan from Your Books (docs/native-apps.md §5.12): the card's action, the
// confirmation's and toasts' wording, and the loan's new date until the next fetch. Pure, so
// RenewTests checks it.

/// What a loan card offers behind its swipe, in its long-press menu and as a VoiceOver action:
/// Renew (confirmed first), only when the library gave the loan a "Renew loan" link.
enum LoanAction: String, CaseIterable, SwipeTileAction {
    case renew

    var id: String { rawValue }
    var title: String { Copy.YourBooks.renew }
    var menuTitle: String { Copy.YourBooks.renewLoan }
    var systemImage: String { "arrow.triangle.2.circlepath" }
    var role: ButtonRole? { nil }
    var tint: Color { Theme.Colors.accent }

    /// Renew when the loan has its link; nothing while a renewal of it is in flight.
    static func available(for row: LoanRow, busy: Bool = false) -> [LoanAction] {
        guard !busy, row.renewHref != nil else { return [] }
        return [.renew]
    }
}

extension Array where Element == LoanRow {
    /// Whether any loan can be renewed (the swipe hint under the list).
    var hasActions: Bool {
        contains { !LoanAction.available(for: $0).isEmpty }
    }
}

/// The renewal's words, every piece from `Copy.YourBooks` (dates as `Copy.Loans.dueOn` writes them).
enum RenewText {
    /// The confirmation's message: "“Whistler” is due 15 Oct. Renewing asks the library for a later
    /// due date.", or without the date when the loan has none.
    static func confirmMessage(for row: LoanRow) -> String {
        guard let date = row.dueDateText else { return Copy.YourBooks.renewMessageNoDate(title: row.ref.title) }
        return Copy.YourBooks.renewMessage(title: row.ref.title, date: date)
    }

    /// The success toast: "Renewed · Due 3 Nov" from `RenewResult.due` as the library writes it
    /// ("3 Nov 2026"; kept as written when it can't be read), else "Renewed".
    static func renewed(due: String?, now: Date = .now) -> String {
        guard let due = due.nonEmpty else { return Copy.YourBooks.renewedNoDate }
        let date = DueDate.parse(due, calendar: .libraryDates).map { Formatting.shortDate($0, now: now) } ?? due
        return Copy.YourBooks.renewed(date: date)
    }

    /// The error toast when the library didn't renew: its reason (or the core's), else `renewFailed`.
    static func refused(_ result: RenewResult) -> String {
        result.message.nonEmpty ?? Copy.YourBooks.renewFailed
    }
}

extension AccountSnapshot {
    /// With the loan whose actions include `href` due on `due` (just renewed; the date as the
    /// library writes it), until the next fetch.
    func renewingLoan(actionHref href: String, due: String) -> AccountSnapshot {
        var snapshot = self
        snapshot.loans.items = loans.items.map { item in
            guard item.actions.contains(where: { $0.href == href }) else { return item }
            var item = item
            item.fields["Due"] = due
            return item
        }
        return snapshot
    }
}

extension View {
    /// "Renew this loan?" while `confirming` holds a loan: Renew runs `onConfirm`, Not now keeps
    /// the loan as it is. Renewing is a real change at the library, so it only runs after Renew.
    func renewLoanConfirmation(_ confirming: Binding<LoanRow?>, onConfirm: @escaping (LoanRow) -> Void) -> some View {
        dialogs {
            Color.clear.alert(
                Copy.YourBooks.renewTitle,
                isPresented: Binding(get: { confirming.wrappedValue != nil },
                                     set: { if !$0 { confirming.wrappedValue = nil } }),
                presenting: confirming.wrappedValue
            ) { row in
                Button(Copy.Common.notNow, role: .cancel) {}
                Button(Copy.YourBooks.renew) { onConfirm(row) }
                    .keyboardShortcut(.defaultAction)
                    .accessibilityHint(Copy.YourBooks.renewHint)
            } message: { row in
                Text(RenewText.confirmMessage(for: row))
            }
        }
    }
}

/// A renewal's toast above the tab bar: "Renewing…" until the result replaces it, then the result
/// for `duration`. Each is announced to VoiceOver.
struct RenewToastOverlay: View {
    @Binding var toast: BorrowedViewModel.ToastMessage?

    /// Long enough to read the library's reason for not renewing.
    static let duration: Duration = .seconds(4)

    var body: some View {
        VStack {
            if let toast {
                Toast(message: toast.message, icon: icon(toast.tone))
                    .transition(.move(edge: .bottom).combined(with: .opacity))
            }
        }
        .frame(maxWidth: .infinity)
        .padding(.horizontal, Theme.Spacing.l)
        .padding(.bottom, Theme.Spacing.m)
        // Slides in and out; the result replaces "Renewing…" in place.
        .animation(.spring(duration: 0.3), value: toast == nil)
        .allowsHitTesting(false)
        .task(id: toast?.id) {
            guard let shown = toast else { return }
            AccessibilityNotification.Announcement(shown.message).post()
            guard shown.tone != .progress else { return }
            try? await Task.sleep(for: Self.duration)
            guard !Task.isCancelled, toast?.id == shown.id else { return }
            toast = nil
        }
    }

    private func icon(_ tone: BorrowedViewModel.ToastMessage.Tone) -> Toast.Icon {
        switch tone {
        case .progress: .progress
        case .success: .symbol("checkmark.circle.fill", Theme.Colors.success)
        case .error: .symbol("exclamationmark.triangle.fill", Theme.Colors.warning)
        }
    }
}
