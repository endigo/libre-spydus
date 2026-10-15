import SwiftUI

/// The wide layout's sidebar (docs/native-apps.md §6.1), in place of the tab bar: the library and
/// Change library at the top, the five sections in the tab bar's order, and the account at the
/// bottom (its name, which opens Profile, and Sign out; or Sign in). The desktop app's sidebar, in
/// navy: an iPad's status bar sits over it, and its text is white.
struct LSSidebar: View {
    let selection: AppTab
    let select: (AppTab) -> Void
    @Environment(AppModel.self) private var model
    @State private var confirmingSignOut = false

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            library
                .padding(.horizontal, Theme.Spacing.l)
                .padding(.top, Theme.Spacing.l)
                .padding(.bottom, Theme.Spacing.xl)
            VStack(spacing: Theme.Spacing.xs) {
                ForEach(AppTab.allCases) { tab in
                    item(tab)
                }
            }
            .padding(.horizontal, Theme.Spacing.s)
            Spacer(minLength: Theme.Spacing.l)
            Rectangle()
                .fill(Theme.Colors.divider)
                .frame(height: 1)
            account
                .padding(Theme.Spacing.l)
        }
        .frame(width: PageLayout.sidebarWidth)
        .frame(maxHeight: .infinity)
        .background(Theme.Colors.background.ignoresSafeArea())
        .overlay(alignment: .trailing) {
            Rectangle()
                .fill(Theme.Colors.divider)
                .frame(width: 1)
                .ignoresSafeArea()
        }
        .confirmationDialog(Copy.Profile.signOutTitle(library: model.libraryName), isPresented: $confirmingSignOut,
                            titleVisibility: .visible) {
            Button(Copy.Account.signOut, role: .destructive) { model.signOut() }
        } message: {
            Text(Copy.Profile.signOutMessage)
        }
    }

    private var library: some View {
        HStack(alignment: .center, spacing: Theme.Spacing.m) {
            LibraryLogo(name: model.libraryName, logoUrl: model.library?.logoUrl, size: 40)
                .accessibilityHidden(true)
            VStack(alignment: .leading, spacing: Theme.Spacing.xs / 2) {
                Text(model.libraryName)
                    .themeFont(.bodyStrong)
                    .foregroundStyle(Theme.Colors.textPrimary)
                    .lineLimit(2)
                    .accessibilityAddTraits(.isHeader)
                Button(Copy.Profile.changeLibrary) { model.libraryPickerPresented = true }
                    .themeFont(.caption)
                    .foregroundStyle(Theme.Colors.textSecondary)
                    .buttonStyle(.borderless)
                    .accessibilityIdentifier("sidebar.changeLibrary")
            }
        }
    }

    private func item(_ tab: AppTab) -> some View {
        let isSelected = tab == selection
        return Button {
            select(tab)
        } label: {
            HStack(spacing: Theme.Spacing.m) {
                Image(systemName: isSelected ? tab.selectedSystemImage : tab.systemImage)
                    .font(.system(size: 17, weight: isSelected ? .semibold : .regular))
                    .frame(width: 24)
                Text(tab.title)
                    .themeFont(isSelected ? .bodyStrong : .body)
                    .lineLimit(1)
                Spacer(minLength: 0)
            }
            .foregroundStyle(isSelected ? Theme.Colors.tabActive : Theme.Colors.textSecondary)
            .padding(.horizontal, Theme.Spacing.m)
            .frame(minHeight: 44)
            .background {
                if isSelected {
                    RoundedRectangle(cornerRadius: Theme.Radius.field, style: .continuous)
                        .fill(Theme.Colors.tabBar)
                }
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .hoverEffect(.highlight)
        .accessibilityLabel(tab.title)
        .accessibilityAddTraits(isSelected ? [.isSelected, .isButton] : .isButton)
        .accessibilityIdentifier("tab.\(tab.rawValue)")
    }

    @ViewBuilder
    private var account: some View {
        if model.isSignedIn {
            VStack(alignment: .leading, spacing: Theme.Spacing.s) {
                Button {
                    select(.profile)
                } label: {
                    Label {
                        Text(model.accountName ?? Copy.Account.signedIn)
                            .themeFont(.bodyStrong)
                            .lineLimit(1)
                    } icon: {
                        Image(systemName: "person.crop.circle")
                    }
                    .foregroundStyle(Theme.Colors.textPrimary)
                }
                .buttonStyle(.plain)
                .accessibilityIdentifier("sidebar.account")
                Button(Copy.Account.signOut) { confirmingSignOut = true }
                    .themeFont(.caption)
                    .foregroundStyle(Theme.Colors.textSecondary)
                    .buttonStyle(.borderless)
                    .accessibilityIdentifier("sidebar.signOut")
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        } else {
            PrimaryPillButton(title: Copy.Account.signIn, fillsWidth: true) { model.loginNeeded = true }
                .accessibilityIdentifier("sidebar.signIn")
        }
    }
}
