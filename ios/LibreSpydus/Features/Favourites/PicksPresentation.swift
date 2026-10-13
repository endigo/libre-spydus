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
    /// library account?" with Add / Not now, once the sign-in sheet has gone (and no other sheet is up).
    func picksCopyOfferAlert() -> some View {
        modifier(PicksCopyOfferAlert())
    }
}

/// iOS may put its own "Save Password?" prompt up right after a sign-in, over this alert or
/// instead of it: the alert waits while the app isn't active, and comes back (up to
/// `maxInterruptions` times) when something closed it without an answer.
private struct PicksCopyOfferAlert: ViewModifier {
    @Environment(AppModel.self) private var model
    @Environment(\.scenePhase) private var scenePhase
    @State private var shown: PicksCopyOffer?
    /// Times the alert was closed without an answer.
    @State private var interruptions = 0

    private static let maxInterruptions = 3

    private struct Trigger: Equatable {
        var offer: PicksCopyOffer?
        var canPresent: Bool
        var interruptions: Int
    }

    func body(content: Content) -> some View {
        content
            .task(id: Trigger(offer: model.picksCopyOffer, canPresent: canPresent, interruptions: interruptions)) {
                guard let offer = model.picksCopyOffer, canPresent, shown == nil,
                      interruptions <= Self.maxInterruptions else { return }
                // Let the sign-in sheet (or whatever closed the alert) finish going away: two
                // presentations can't overlap.
                try? await Task.sleep(for: .milliseconds(700))
                guard !Task.isCancelled, model.picksCopyOffer == offer, canPresent else { return }
                shown = offer
            }
            .dialogs {
                Color.clear.alert(Copy.Picks.offerTitle, isPresented: isPresented, presenting: shown) { _ in
                    Button(Copy.Common.notNow, role: .cancel) { answer(add: false) }
                    Button(Copy.Common.add) { answer(add: true) }
                        .keyboardShortcut(.defaultAction)
                } message: { offer in
                    Text(Copy.Picks.offerMessage(count: offer.count))
                }
            }
    }

    private var canPresent: Bool {
        model.rootSheet == nil && !model.isPresentingFeatureSheet && scenePhase == .active
    }

    private var isPresented: Binding<Bool> {
        Binding(get: { shown != nil }, set: { presented in
            guard !presented, shown != nil else { return }
            // Closed, maybe without an answer: if the offer is still open once things settle, it
            // comes back (an answer clears it first).
            shown = nil
            interruptions += 1
        })
    }

    private func answer(add: Bool) {
        shown = nil
        Task { await model.answerPicksCopyOffer(add: add) }
    }
}
