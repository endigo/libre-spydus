import SwiftUI

/// A pulsing placeholder block (Home shelves, search rows) while content loads.
struct SkeletonBlock: View {
    var width: CGFloat?
    var height: CGFloat
    var radius: CGFloat = 6

    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var dimmed = false

    var body: some View {
        RoundedRectangle(cornerRadius: radius, style: .continuous)
            .fill(Theme.Colors.surfaceElevated)
            // Only the pulse repeats: a layout change while it runs (a grid finding its columns,
            // a rotation) would otherwise slide the block between its old and new places forever.
            .animation(reduceMotion ? nil : .easeInOut(duration: 0.9).repeatForever(autoreverses: true)) {
                $0.opacity(dimmed ? 0.45 : 1)
            }
            .frame(width: width, height: height)
            .frame(maxWidth: width == nil ? .infinity : nil, alignment: .leading)
            .onAppear { dimmed = !reduceMotion }
            .accessibilityHidden(true)
    }
}

/// A shelf of cover-shaped skeletons.
struct ShelfSkeleton: View {
    private let coverWidth = Theme.Cover.shelfWidth
    private let count = 4

    var body: some View {
        // A (non-scrolling) scroll view takes the offered width; a bare HStack of fixed-width
        // covers would widen the whole screen.
        ScrollView(.horizontal) {
            HStack(alignment: .top, spacing: Theme.Spacing.m) {
                ForEach(0..<count, id: \.self) { _ in
                    VStack(alignment: .leading, spacing: Theme.Spacing.s) {
                        SkeletonBlock(width: coverWidth, height: Theme.Cover.height(forWidth: coverWidth),
                                      radius: Theme.Radius.cover)
                        SkeletonBlock(width: coverWidth * 0.7, height: 10)
                    }
                }
            }
            .padding(.horizontal, Theme.Spacing.l)
        }
        .scrollDisabled(true)
        .scrollIndicators(.hidden)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(Copy.Common.loading)
    }
}

/// A section's inline failure: what went wrong and Retry, without taking over the screen.
struct SectionErrorCard: View {
    let error: UserFacingError
    let retry: () -> Void

    var body: some View {
        HStack(spacing: Theme.Spacing.m) {
            Image(systemName: error.systemImage)
                .font(.title3)
                .foregroundStyle(Theme.Colors.warning)
                .accessibilityHidden(true)
            VStack(alignment: .leading, spacing: Theme.Spacing.xs) {
                Text(error.title)
                    .themeFont(.bodyStrong)
                    .foregroundStyle(Theme.Colors.textPrimary)
                Text(error.message)
                    .themeFont(.caption)
                    .foregroundStyle(Theme.Colors.textSecondary)
                    .fixedSize(horizontal: false, vertical: true)
            }
            .accessibilityElement(children: .combine)
            Spacer(minLength: Theme.Spacing.s)
            if error.canRetry {
                Button(Copy.Common.retry, action: retry)
                    .themeFont(.bodyStrong)
                    .foregroundStyle(Theme.Colors.primary)
                    .buttonStyle(.borderless)
            }
        }
        .homeCard()
    }
}

/// A section's empty state: an icon and a line or two.
struct SectionMessageCard: View {
    let systemImage: String
    let title: String
    var message: String?

    var body: some View {
        HStack(spacing: Theme.Spacing.m) {
            Image(systemName: systemImage)
                .font(.title3)
                .foregroundStyle(Theme.Colors.textMuted)
                .accessibilityHidden(true)
            VStack(alignment: .leading, spacing: Theme.Spacing.xs) {
                Text(title)
                    .themeFont(.bodyStrong)
                    .foregroundStyle(Theme.Colors.textPrimary)
                if let message {
                    Text(message)
                        .themeFont(.caption)
                        .foregroundStyle(Theme.Colors.textSecondary)
                        .fixedSize(horizontal: false, vertical: true)
                }
            }
            Spacer(minLength: 0)
        }
        .accessibilityElement(children: .combine)
        .homeCard()
    }
}

/// Signed out, in place of "Your loans".
struct SignInCard: View {
    let libraryName: String
    let signIn: () -> Void

    var body: some View {
        HStack(spacing: Theme.Spacing.m) {
            Image(systemName: "books.vertical.fill")
                .font(.title3)
                .foregroundStyle(Theme.Colors.onAccent)
                .frame(width: 44, height: 44)
                .background(Circle().fill(Theme.Colors.accent))
                .accessibilityHidden(true)
            VStack(alignment: .leading, spacing: Theme.Spacing.xs) {
                Text(Copy.Home.signInCardTitle)
                    .themeFont(.bodyStrong)
                    .foregroundStyle(Theme.Colors.textPrimary)
                Text(Copy.Home.signInCardMessage(library: libraryName))
                    .themeFont(.caption)
                    .foregroundStyle(Theme.Colors.textSecondary)
                    .fixedSize(horizontal: false, vertical: true)
            }
            .accessibilityElement(children: .combine)
            Spacer(minLength: Theme.Spacing.s)
            Button(action: signIn) {
                Text(Copy.Account.signIn)
                    .themeFont(.bodyStrong)
                    .foregroundStyle(Theme.Colors.onPrimary)
                    .padding(.horizontal, Theme.Spacing.l)
                    .padding(.vertical, Theme.Spacing.s)
                    .background(Capsule().fill(Theme.Colors.primaryGradient))
                    .contentShape(Capsule())
            }
            .buttonStyle(PressScaleStyle())
        }
        .homeCard()
    }
}


extension View {
    /// Home's section card on `surface`.
    fileprivate func homeCard() -> some View {
        padding(Theme.Spacing.l)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(RoundedRectangle(cornerRadius: Theme.Radius.card, style: .continuous)
                .fill(Theme.Colors.surface))
            .padding(.horizontal, Theme.Spacing.l)
    }
}
