import SwiftUI

/// My Picks' short confirmations (docs/native-apps.md §5.11) as a toast above the tab bar, on
/// whatever screen is showing: "Saved to My Picks", "Removed from My Picks", "Couldn’t update My
/// Picks", the copy's progress and its summary. Each shows for a few seconds (progress until the
/// next one) and is announced to VoiceOver. RootView puts it over the tab stacks.
struct PicksNoticeOverlay: View {
    @Environment(AppModel.self) private var model
    @State private var shown: PicksNotice?

    static let duration: Duration = .seconds(2.5)

    var body: some View {
        VStack {
            if let shown {
                PicksToast(notice: shown)
                    .transition(.move(edge: .bottom).combined(with: .opacity))
            }
        }
        .frame(maxWidth: .infinity)
        .padding(.horizontal, Theme.Spacing.l)
        .padding(.bottom, Theme.Spacing.m)
        // Slides in and out; a notice that replaces another (the copy's progress → its summary)
        // swaps in place.
        .animation(.spring(duration: 0.3), value: shown == nil)
        .allowsHitTesting(false)
        .task(id: model.picks.notice) {
            guard let notice = model.picks.notice else { return }
            shown = notice
            AccessibilityNotification.Announcement(notice.message).post()
            guard notice.tone != .progress else { return }
            try? await Task.sleep(for: Self.duration)
            guard !Task.isCancelled else { return }
            shown = nil
        }
    }
}

private struct PicksToast: View {
    let notice: PicksNotice

    var body: some View {
        Toast(message: notice.message, icon: icon)
    }

    private var icon: Toast.Icon {
        switch notice.tone {
        case .progress: .progress
        case .success: .symbol("bookmark.fill", Theme.Colors.primary)
        case .info: .symbol("info.circle.fill", Theme.Colors.accent)
        case .error: .symbol("exclamationmark.triangle.fill", Theme.Colors.warning)
        }
    }
}

extension View {
    /// The copy-once offer after an interactive sign-in (§5.11): "Add this phone’s picks to your
    /// library account?" with Add / Not now, once the sign-in sheet has gone (and no other sheet is
    /// up), after "Sign in with Face ID next time?" has been answered (§5.14).
    func picksCopyOfferAlert(_ model: AppModel) -> some View {
        modifier(AfterSignInOffer(
            offer: model.picksCopyOffer,
            waits: model.biometrics.offer != nil,
            title: { _ in Copy.Picks.offerTitle },
            message: { Copy.Picks.offerMessage(count: $0.count) },
            actionTitle: { _ in Copy.Common.add },
            answer: { add in Task { await model.answerPicksCopyOffer(add: add) } }
        ))
    }
}
