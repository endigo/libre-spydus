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
            .frame(width: width, height: height)
            .frame(maxWidth: width == nil ? .infinity : nil, alignment: .leading)
            .opacity(dimmed ? 0.45 : 1)
            .animation(reduceMotion ? nil : .easeInOut(duration: 0.9).repeatForever(autoreverses: true), value: dimmed)
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
        .accessibilityLabel("Loading")
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
                Button("Retry", action: retry)
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

/// Signed out, in place of "Your borrowed books".
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
                Text("See your loans here")
                    .themeFont(.bodyStrong)
                    .foregroundStyle(Theme.Colors.textPrimary)
                Text("Sign in with your \(libraryName) card.")
                    .themeFont(.caption)
                    .foregroundStyle(Theme.Colors.textSecondary)
                    .fixedSize(horizontal: false, vertical: true)
            }
            .accessibilityElement(children: .combine)
            Spacer(minLength: Theme.Spacing.s)
            Button(action: signIn) {
                Text("Sign in")
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

/// The selected library's logo on a small white tile (its initial when there's no logo).
struct LibraryLogo: View {
    let name: String
    let logoUrl: String?
    var size: CGFloat = 44

    var body: some View {
        let shape = RoundedRectangle(cornerRadius: size * 0.28, style: .continuous)
        ZStack {
            shape.fill(Color.white)
            if let url = logoUrl.nonEmpty.flatMap(URL.init(string:)) {
                AsyncImage(url: url) { phase in
                    if let image = phase.image {
                        image.resizable().scaledToFit().padding(size * 0.12)
                    } else {
                        initial
                    }
                }
            } else {
                initial
            }
        }
        .frame(width: size, height: size)
        .clipShape(shape)
        .themeShadow(.card)
    }

    private var initial: some View {
        Text(name.first.map { String($0).uppercased() } ?? "L")
            .font(.system(size: size * 0.45, weight: .bold, design: .rounded))
            .foregroundStyle(Theme.Colors.background)
    }
}

extension View {
    /// Keeps scrolling content clear of the custom tab bar. RootView adds the bar with
    /// `.safeAreaInset`, but that inset doesn't reach content inside the tabs' `NavigationStack`s
    /// (the safe area there ends at the home indicator), so the Home and Search screens add it.
    func tabBarClearance() -> some View {
        safeAreaPadding(.bottom, Theme.TabBar.height)
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
