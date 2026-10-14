import SwiftUI

/// An offer after an interactive sign-in, as an alert with **Not now** and one action, once the
/// sign-in sheet has gone (and no other sheet is up): "Sign in with Face ID next time?" (§5.14),
/// then My Picks' copy offer (§5.11).
///
/// iOS may put its own "Save Password?" prompt up right after a sign-in, over this alert or
/// instead of it: the alert waits while the app isn't active, and comes back (up to
/// `maxInterruptions` times) when something closed it without an answer.
struct AfterSignInOffer<Offer: Equatable>: ViewModifier {
    let offer: Offer?
    /// Another offer is still open: this one waits for it.
    var waits = false
    let title: (Offer) -> String
    let message: (Offer) -> String
    let actionTitle: (Offer) -> String
    /// False: the reader already said yes elsewhere, so the action runs without the alert (still
    /// once the sheet has gone and the app is active).
    var asks: (Offer) -> Bool = { _ in true }
    /// The answer: true for the action, false for Not now.
    let answer: (Bool) -> Void

    @Environment(AppModel.self) private var model
    @Environment(\.scenePhase) private var scenePhase
    @State private var shown: Offer?
    /// Times the alert was closed without an answer.
    @State private var interruptions = 0

    private static var maxInterruptions: Int { 3 }

    private struct Trigger: Equatable {
        var offer: Offer?
        var canPresent: Bool
        var interruptions: Int
    }

    func body(content: Content) -> some View {
        content
            .task(id: Trigger(offer: offer, canPresent: canPresent, interruptions: interruptions)) {
                guard let offer, canPresent, shown == nil, interruptions <= Self.maxInterruptions else { return }
                // Let the sign-in sheet (or whatever closed the alert) finish going away: two
                // presentations can't overlap.
                try? await Task.sleep(for: .milliseconds(700))
                guard !Task.isCancelled, self.offer == offer, canPresent else { return }
                if asks(offer) {
                    shown = offer
                } else {
                    answer(true)
                }
            }
            .dialogs {
                Color.clear.alert(shown.map(title) ?? "", isPresented: isPresented, presenting: shown) { offer in
                    Button(Copy.Common.notNow, role: .cancel) { reply(false) }
                    Button(actionTitle(offer)) { reply(true) }
                        .keyboardShortcut(.defaultAction)
                } message: { offer in
                    Text(message(offer))
                }
            }
    }

    private var canPresent: Bool {
        !waits && model.rootSheet == nil && !model.isPresentingFeatureSheet && scenePhase == .active
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

    private func reply(_ accepted: Bool) {
        shown = nil
        answer(accepted)
    }
}

extension View {
    /// "Sign in with Face ID next time?" after a typed sign-in (§5.14): Use Face ID / Not now. Turned
    /// on from Profile, the check runs without the question.
    func biometricOfferAlert(_ model: AppModel) -> some View {
        modifier(AfterSignInOffer(
            offer: model.biometrics.offer,
            title: { Copy.Biometrics.offerTitle(method: $0.method) },
            message: { Copy.Biometrics.offerMessage(library: model.libraryName, method: $0.method) },
            actionTitle: { Copy.Biometrics.use(method: $0.method) },
            asks: \.asks,
            answer: { use in Task { await model.answerBiometricOffer(use: use) } }
        ))
    }
}
