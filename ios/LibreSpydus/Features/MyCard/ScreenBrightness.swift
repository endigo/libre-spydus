import SwiftUI
import UIKit

/// Full screen brightness while the card is up, so desk and self-checkout scanners read it
/// (docs/native-apps.md §5.9); the patron's own level comes back afterwards.
@MainActor
final class BrightnessBoost {
    private let read: () -> CGFloat
    private let write: (CGFloat) -> Void
    /// The level before `raise`, while raised.
    private(set) var saved: CGFloat?

    init(read: @escaping () -> CGFloat, write: @escaping (CGFloat) -> Void) {
        self.read = read
        self.write = write
    }

    /// The screen showing the app (the foreground window scene's).
    static func activeScreen() -> BrightnessBoost {
        let scenes = UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }
        let scene = scenes.first { $0.activationState == .foregroundActive } ?? scenes.first
        let screen = scene?.screen
        return BrightnessBoost(read: { screen?.brightness ?? 1 }, write: { screen?.brightness = $0 })
    }

    var isRaised: Bool { saved != nil }

    /// Full brightness, remembering the current level (once: raising again keeps the first).
    func raise() {
        guard saved == nil else { return }
        let level = read()
        saved = level
        write(1)
        #if DEBUG
        print("🔆 card brightness \(level) → 1")
        #endif
    }

    /// Back to the level before `raise`.
    func restore() {
        guard let saved else { return }
        write(saved)
        self.saved = nil
        #if DEBUG
        print("🔆 card brightness restored to \(saved)")
        #endif
    }
}

extension View {
    /// Full screen brightness while this view is on screen and the app is in the foreground.
    func fullScreenBrightness() -> some View {
        modifier(FullScreenBrightness())
    }
}

private struct FullScreenBrightness: ViewModifier {
    @Environment(\.scenePhase) private var scenePhase
    @State private var boost: BrightnessBoost?

    func body(content: Content) -> some View {
        content
            .onAppear {
                if boost == nil { boost = .activeScreen() }
                if scenePhase == .active { boost?.raise() }
            }
            .onDisappear { boost?.restore() }
            .onChange(of: scenePhase) { _, phase in
                // Leaving the foreground (app switcher, home, lock) gives the patron's level back.
                if phase == .active { boost?.raise() } else { boost?.restore() }
            }
    }
}
