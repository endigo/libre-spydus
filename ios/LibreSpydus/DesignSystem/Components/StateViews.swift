import SwiftUI

/// Loading state for a screen or section.
struct LoadingView: View {
    var message: String?

    var body: some View {
        VStack(spacing: Theme.Spacing.m) {
            ProgressView()
                .controlSize(.large)
                .tint(Theme.Colors.textSecondary)
            if let message {
                Text(message)
                    .themeFont(.body)
                    .foregroundStyle(Theme.Colors.textSecondary)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding(Theme.Spacing.xl)
        .accessibilityElement(children: .combine)
        .accessibilityLabel(message ?? "Loading")
    }
}

/// Empty state: icon, title, message, optional action ("No results for …").
struct EmptyStateView: View {
    let systemImage: String
    let title: String
    var message: String?
    var actionTitle: String?
    var action: (() -> Void)?

    var body: some View {
        VStack(spacing: Theme.Spacing.m) {
            Image(systemName: systemImage)
                .font(.system(size: 44, weight: .light))
                .foregroundStyle(Theme.Colors.textMuted)
                .accessibilityHidden(true)
            Text(title)
                .themeFont(.section)
                .foregroundStyle(Theme.Colors.textPrimary)
                .multilineTextAlignment(.center)
            if let message {
                Text(message)
                    .themeFont(.body)
                    .foregroundStyle(Theme.Colors.textSecondary)
                    .multilineTextAlignment(.center)
            }
            if let actionTitle, let action {
                PrimaryPillButton(title: actionTitle, action: action)
                    .padding(.top, Theme.Spacing.s)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding(Theme.Spacing.xl)
    }
}

/// Error state with Retry (when it makes sense) and the technical detail in a disclosure.
struct ErrorStateView: View {
    let error: UserFacingError
    var retry: (() -> Void)?

    @State private var showsDetail = false

    var body: some View {
        VStack(spacing: Theme.Spacing.m) {
            Image(systemName: error.systemImage)
                .font(.system(size: 40, weight: .light))
                .foregroundStyle(Theme.Colors.warning)
                .accessibilityHidden(true)
            Text(error.title)
                .themeFont(.section)
                .foregroundStyle(Theme.Colors.textPrimary)
            Text(error.message)
                .themeFont(.body)
                .foregroundStyle(Theme.Colors.textSecondary)
                .multilineTextAlignment(.center)
            if let retry, error.canRetry {
                PrimaryPillButton(title: "Retry", systemImage: "arrow.clockwise", action: retry)
                    .padding(.top, Theme.Spacing.s)
            }
            if let detail = error.detail {
                DisclosureGroup("Details", isExpanded: $showsDetail) {
                    Text(detail)
                        .themeFont(.caption)
                        .foregroundStyle(Theme.Colors.textMuted)
                        .textSelection(.enabled)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(.top, Theme.Spacing.xs)
                }
                .themeFont(.caption)
                .tint(Theme.Colors.textSecondary)
                .foregroundStyle(Theme.Colors.textSecondary)
                .padding(.top, Theme.Spacing.s)
                .frame(maxWidth: 320)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding(Theme.Spacing.xl)
    }
}

/// A navy placeholder screen: what wave-1 feature stubs show until their screen is built.
struct PlaceholderScreen<Actions: View>: View {
    let title: String
    let systemImage: String
    var message: String?
    @ViewBuilder var actions: () -> Actions

    var body: some View {
        VStack(spacing: Theme.Spacing.l) {
            Image(systemName: systemImage)
                .font(.system(size: 52, weight: .light))
                .foregroundStyle(Theme.Colors.primary)
                .frame(width: 104, height: 104)
                .background(Circle().fill(Theme.Colors.surface))
                .accessibilityHidden(true)
            Text(title)
                .themeFont(.title)
                .foregroundStyle(Theme.Colors.textPrimary)
                .multilineTextAlignment(.center)
            if let message {
                Text(message)
                    .themeFont(.body)
                    .foregroundStyle(Theme.Colors.textSecondary)
                    .multilineTextAlignment(.center)
            }
            actions()
        }
        .padding(Theme.Spacing.xxl)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .screenBackground()
    }
}

extension PlaceholderScreen where Actions == EmptyView {
    init(title: String, systemImage: String, message: String? = nil) {
        self.init(title: title, systemImage: systemImage, message: message) { EmptyView() }
    }
}
