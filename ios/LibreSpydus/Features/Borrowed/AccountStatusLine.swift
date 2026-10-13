import SwiftUI

/// The muted line under Your Books' segments: when the saved account was fetched ("Updated 12 min
/// ago") with a small spinner while a fresh one loads, or, after a failed refresh,
/// "Couldn’t refresh · Showing the saved list" and Retry.
struct AccountStatusLine: View {
    let updatedAt: Date
    let isRefreshing: Bool
    let failed: Bool
    let retry: () -> Void

    static let failedText = Copy.YourBooks.refreshFailed

    var body: some View {
        Group {
            if failed && !isRefreshing {
                failedLine
            } else {
                updatedLine
            }
        }
        .themeFont(.caption)
        .foregroundStyle(Theme.Colors.textMuted)
        .frame(maxWidth: .infinity, minHeight: 22, alignment: .leading)
        .animation(.default, value: isRefreshing)
        .animation(.default, value: failed)
    }

    private var updatedLine: some View {
        // Re-rendered every half minute so "Updated 3 min ago" keeps up.
        TimelineView(.periodic(from: .now, by: 30)) { context in
            let text = Formatting.updated(updatedAt, now: context.date)
            HStack(spacing: Theme.Spacing.s) {
                Text(text)
                if isRefreshing {
                    ProgressView()
                        .controlSize(.mini)
                        .tint(Theme.Colors.textMuted)
                        .transition(.opacity)
                }
            }
            .accessibilityElement(children: .ignore)
            .accessibilityLabel(isRefreshing ? Copy.A11y.refreshing(status: text) : text)
        }
    }

    private var failedLine: some View {
        HStack(spacing: Theme.Spacing.s) {
            Label {
                Text(Self.failedText)
            } icon: {
                Image(systemName: "exclamationmark.triangle.fill")
                    .foregroundStyle(Theme.Colors.warning)
            }
            .labelStyle(CompactLabelStyle())
            Spacer(minLength: Theme.Spacing.s)
            Button(Copy.Common.retry, action: retry)
                .themeFont(.bodyStrong)
                .foregroundStyle(Theme.Colors.primary)
                .buttonStyle(.borderless)
                .accessibilityHint(Copy.YourBooks.refreshHint)
        }
    }
}

/// Icon and title close together, centred on each other (the default label spaces them widely).
private struct CompactLabelStyle: LabelStyle {
    func makeBody(configuration: Configuration) -> some View {
        HStack(spacing: Theme.Spacing.xs) {
            configuration.icon
            configuration.title
        }
    }
}
