import SwiftUI

/// A short line in a capsule above the tab bar: a confirmation, a failure, or work in progress
/// (My Picks' notices, a loan's renewal). The overlay that shows it decides how long it stays.
struct Toast: View {
    enum Icon: Equatable {
        /// A small spinner: the work is still running.
        case progress
        case symbol(String, Color)
    }

    let message: String
    let icon: Icon

    var body: some View {
        HStack(spacing: Theme.Spacing.s) {
            switch icon {
            case .progress:
                ProgressView()
                    .controlSize(.small)
                    .tint(Theme.Colors.textSecondary)
            case .symbol(let name, let color):
                Image(systemName: name).foregroundStyle(color)
            }
            Text(message)
                .themeFont(.bodyStrong)
                .foregroundStyle(Theme.Colors.textPrimary)
                .lineLimit(2)
                .fixedSize(horizontal: false, vertical: true)
        }
        .padding(.horizontal, Theme.Spacing.l)
        .padding(.vertical, Theme.Spacing.m)
        .background(Capsule().fill(Theme.Colors.surfaceElevated))
        .overlay(Capsule().strokeBorder(Theme.Colors.textMuted.opacity(0.25), lineWidth: 1))
        .shadow(color: .black.opacity(0.35), radius: 12, y: 4)
        .accessibilityElement(children: .combine)
    }
}
