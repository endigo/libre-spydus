import SwiftUI
import UIKit

/// UIKit appearance for the pieces SwiftUI styles through UIKit (navigation bars, segmented
/// controls), so every screen gets the navy look without per-screen modifiers.
@MainActor
enum Appearance {
    static func configure() {
        let navigation = UINavigationBarAppearance()
        navigation.configureWithOpaqueBackground()
        navigation.backgroundColor = UIColor(Theme.Colors.background)
        navigation.shadowColor = .clear
        navigation.titleTextAttributes = [.foregroundColor: UIColor.white]
        navigation.largeTitleTextAttributes = [.foregroundColor: UIColor.white]
        let bar = UINavigationBar.appearance()
        bar.standardAppearance = navigation
        bar.scrollEdgeAppearance = navigation
        bar.compactAppearance = navigation
        bar.tintColor = .white

        let segmented = UISegmentedControl.appearance()
        segmented.selectedSegmentTintColor = UIColor(Theme.Colors.primary)
        segmented.backgroundColor = UIColor(Theme.Colors.surface)
        segmented.setTitleTextAttributes([.foregroundColor: UIColor.white], for: .selected)
        segmented.setTitleTextAttributes([.foregroundColor: UIColor(Theme.Colors.textSecondary)], for: .normal)
    }
}
