import SwiftUI

/// The custom tab bar: white bar, five items, Search as a raised white circle in the centre.
struct LSTabBar: View {
    let selection: AppTab
    let select: (AppTab) -> Void

    var body: some View {
        HStack(spacing: 0) {
            ForEach(AppTab.allCases) { tab in
                item(tab)
            }
        }
        .frame(height: Theme.TabBar.height)
        .background(alignment: .top) {
            Theme.Colors.tabBar
                .ignoresSafeArea(edges: .bottom)
                .shadow(color: Theme.Colors.tabActive.opacity(0.08), radius: 6, y: -2)
        }
        .overlay(alignment: .top) {
            centerButton
                .offset(y: -Theme.TabBar.centerButtonLift)
        }
    }

    private func item(_ tab: AppTab) -> some View {
        let isSelected = tab == selection
        return Button {
            select(tab)
        } label: {
            VStack(spacing: Theme.Spacing.xs) {
                if tab == .search {
                    // The raised centre button sits above this slot; keep the label on the baseline.
                    Color.clear.frame(height: 24)
                } else {
                    Image(systemName: isSelected ? tab.selectedSystemImage : tab.systemImage)
                        .font(.system(size: 20, weight: .regular))
                        .frame(height: 24)
                }
                Text(tab.title)
                    .themeFont(.tab)
                    .lineLimit(1)
                    .minimumScaleFactor(0.8)
            }
            .padding(.vertical, Theme.Spacing.s)
            .foregroundStyle(isSelected ? Theme.Colors.tabActive : Theme.Colors.tabInactive)
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityLabel(tab.title)
        .accessibilityAddTraits(isSelected ? [.isSelected, .isButton] : .isButton)
        .accessibilityIdentifier("tab.\(tab.rawValue)")
    }

    private var centerButton: some View {
        let isSelected = selection == .search
        let diameter = Theme.TabBar.centerButtonDiameter
        return Button {
            select(.search)
        } label: {
            Image(systemName: "magnifyingglass")
                .font(.system(size: 24, weight: .semibold))
                .foregroundStyle(isSelected ? Theme.Colors.primary : Theme.Colors.tabActive)
                .frame(width: diameter, height: diameter)
                .background(Circle().fill(Theme.Colors.tabBar))
                .overlay(Circle().strokeBorder(Theme.Colors.tabInactive.opacity(0.25), lineWidth: 1))
                .themeShadow(.tabCenter)
                .contentShape(Circle())
        }
        .buttonStyle(PressScaleStyle())
        .accessibilityHidden(true)
    }
}

extension View {
    /// Keeps scrolling content clear of the custom tab bar. RootView adds the bar with
    /// `.safeAreaInset`, but on iOS 27 that inset doesn't reach content inside the tabs'
    /// `NavigationStack`s (their safe area ends at the home indicator), so `TabStack` applies this
    /// to every tab root and pushed screen. Screens don't add it themselves.
    func tabBarClearance() -> some View {
        safeAreaPadding(.bottom, Theme.TabBar.height)
    }
}
