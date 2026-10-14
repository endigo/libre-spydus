import LibreSpydusCore
import SwiftUI

/// Profile (docs/native-apps.md §6): the library card + Change library, the account (Sign in /
/// Sign out), Notifications while signed in (§5.8), human verification for libraries that needed
/// it, About, and (DEBUG) demo mode.
/// The borrower's name comes from the saved account (`AppModel.account`).
struct ProfileView: View {
    @Environment(AppModel.self) private var model
    @State private var confirmingSignOut = false
    @State private var confirmingClearVerification = false

    var body: some View {
        ScrollViewReader { proxy in
            ScrollView {
                content
            }
            #if DEBUG
            .onAppear {
                // `-LSProfileAnchor bottom` (DEBUG, screenshots): start scrolled to the end.
                if UserDefaults.standard.string(forKey: "LSProfileAnchor") == "bottom" {
                    proxy.scrollTo(Self.bottomAnchor, anchor: .bottom)
                }
            }
            #endif
        }
        .screenBackground()
        .toolbar(.hidden, for: .navigationBar)
        .dialogs {
            Color.clear
                .confirmationDialog(Copy.Profile.signOutTitle(library: model.libraryName), isPresented: $confirmingSignOut,
                                    titleVisibility: .visible) {
                    Button(Copy.Account.signOut, role: .destructive) { model.signOut() }
                } message: {
                    Text(Copy.Profile.signOutMessage)
                }
                .confirmationDialog(Copy.Profile.clearVerificationTitle, isPresented: $confirmingClearVerification,
                                    titleVisibility: .visible) {
                    Button(Copy.Profile.clearVerification, role: .destructive) { clearVerification() }
                } message: {
                    Text(Copy.Profile.clearVerificationMessage(library: model.libraryName).capitalisedFirst)
                }
        }
    }

    private static let bottomAnchor = "profile-bottom"

    private var content: some View {
        VStack(alignment: .leading, spacing: Theme.Spacing.xl) {
            Text(Copy.Profile.title)
                .themeFont(.greeting)
                .foregroundStyle(Theme.Colors.textPrimary)
                .accessibilityAddTraits(.isHeader)
                .padding(.top, Theme.Spacing.l)
            if let library = model.library {
                libraryCard(library)
                accountSection
                if model.isSignedIn { NotificationsSection() }
                if model.isVerified { verificationSection }
            } else {
                EmptyStateView(systemImage: "building.columns", title: Copy.Profile.noLibraryTitle,
                               message: Copy.Profile.noLibraryMessage,
                               actionTitle: Copy.Profile.chooseLibrary) { model.libraryPickerPresented = true }
            }
            aboutSection
            #if DEBUG
            developerSection
            #endif
            Color.clear.frame(height: 0).id(Self.bottomAnchor)
        }
        .padding(.horizontal, Theme.Spacing.l)
        .padding(.bottom, Theme.Spacing.xxl)
    }

    // MARK: Library

    private func libraryCard(_ library: LibraryInfo) -> some View {
        let region = LibraryDirectory.Region(code: library.region)
        return ProfileCard {
            HStack(alignment: .center, spacing: Theme.Spacing.l) {
                LibraryLogo(library, size: 60)
                VStack(alignment: .leading, spacing: Theme.Spacing.xs) {
                    Text(library.name)
                        .themeFont(.section)
                        .foregroundStyle(Theme.Colors.textPrimary)
                        .fixedSize(horizontal: false, vertical: true)
                    Text(LibraryDirectory.host(of: library.origin))
                        .themeFont(.caption)
                        .foregroundStyle(Theme.Colors.textSecondary)
                        .lineLimit(1)
                        .truncationMode(.middle)
                    Chip(text: library.country.flatMap { $0.isEmpty ? nil : $0 } ?? region.title,
                         systemImage: "mappin.and.ellipse")
                        .padding(.top, 2)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .accessibilityElement(children: .combine)
            ProfileDivider()
            ProfileRowButton(title: Copy.Profile.changeLibrary, systemImage: "arrow.left.arrow.right") {
                model.libraryPickerPresented = true
            }
        }
    }

    // MARK: Account

    private var accountSection: some View {
        VStack(alignment: .leading, spacing: Theme.Spacing.m) {
            ProfileSectionHeader(title: Copy.Profile.account)
            ProfileCard {
                if model.isSignedIn {
                    signedInContent
                } else {
                    signedOutContent
                }
            }
        }
    }

    @ViewBuilder
    private var signedInContent: some View {
        HStack(spacing: Theme.Spacing.l) {
            Text(initials)
                .font(.system(size: 20, weight: .bold, design: .rounded))
                .foregroundStyle(Theme.Colors.onAccent)
                .frame(width: 52, height: 52)
                .background(Circle().fill(Theme.Colors.accent))
                .accessibilityHidden(true)
            VStack(alignment: .leading, spacing: Theme.Spacing.xs) {
                if let name = model.accountName {
                    Text(name)
                        .themeFont(.bodyStrong)
                        .foregroundStyle(Theme.Colors.textPrimary)
                } else if model.account.snapshot == nil, model.account.isRefreshing {
                    HStack(spacing: Theme.Spacing.s) {
                        ProgressView().tint(Theme.Colors.textSecondary)
                        Text(Copy.Profile.loadingAccount)
                            .themeFont(.body)
                            .foregroundStyle(Theme.Colors.textSecondary)
                    }
                } else {
                    Text(Copy.Account.signedIn)
                        .themeFont(.bodyStrong)
                        .foregroundStyle(Theme.Colors.textPrimary)
                }
                Text(Copy.Profile.signedInTo(library: model.libraryName))
                    .themeFont(.caption)
                    .foregroundStyle(Theme.Colors.textSecondary)
                    .fixedSize(horizontal: false, vertical: true)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .accessibilityElement(children: .combine)
        // A library page without a name isn't an error: Profile then just says "Signed in".
        if model.account.snapshot == nil, model.account.lastRefreshError != nil, !model.account.isRefreshing {
            HStack(spacing: Theme.Spacing.s) {
                Text(Copy.Profile.accountFailed)
                    .themeFont(.caption)
                    .foregroundStyle(Theme.Colors.warning)
                    .frame(maxWidth: .infinity, alignment: .leading)
                Button(Copy.Common.retry) { Task { await model.refreshAccount(force: true) } }
                    .themeFont(.bodyStrong)
                    .foregroundStyle(Theme.Colors.primary)
                    .buttonStyle(.borderless)
            }
        }
        if let biometry = model.biometrics.biometry {
            ProfileDivider()
            ProfileSwitchRow(title: Copy.Biometrics.signInWith(method: biometry.name),
                             subtitle: Copy.Biometrics.switchSubtitle(library: model.libraryName),
                             systemImage: biometry.systemImage, isOn: biometricSignIn)
        }
        ProfileDivider()
        ProfileRowButton(title: Copy.Account.signOut, systemImage: "rectangle.portrait.and.arrow.right", tint: Theme.Colors.danger) {
            confirmingSignOut = true
        }
    }

    /// On while this library has Face ID's details kept, or while turning it on is under way (§5.14).
    private var biometricSignIn: Binding<Bool> {
        Binding(get: { model.biometrics.isSwitchedOn }, set: { model.setBiometricSignIn($0) })
    }

    @ViewBuilder
    private var signedOutContent: some View {
        HStack(alignment: .top, spacing: Theme.Spacing.l) {
            Image(systemName: "person.crop.circle")
                .font(.system(size: 40, weight: .light))
                .foregroundStyle(Theme.Colors.textMuted)
                .accessibilityHidden(true)
            VStack(alignment: .leading, spacing: Theme.Spacing.xs) {
                Text(Copy.Profile.notSignedInTitle)
                    .themeFont(.bodyStrong)
                    .foregroundStyle(Theme.Colors.textPrimary)
                Text(Copy.Profile.notSignedInMessage)
                    .themeFont(.caption)
                    .foregroundStyle(Theme.Colors.textSecondary)
                    .fixedSize(horizontal: false, vertical: true)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .accessibilityElement(children: .combine)
        PrimaryPillButton(title: Copy.Account.signIn, systemImage: "person.badge.key", fillsWidth: true) {
            model.loginNeeded = true
        }
        .padding(.top, Theme.Spacing.xs)
    }

    private var initials: String {
        guard let name = model.accountName else { return "" }
        // Library names often come as "Surname, Given".
        let parts = name.split(separator: ",").map { $0.trimmingCharacters(in: .whitespaces) }
        let ordered = parts.count == 2 ? "\(parts[1]) \(parts[0])" : name
        return LibraryDirectory.monogram(for: ordered)
    }

    // MARK: Verification

    private var verificationSection: some View {
        VStack(alignment: .leading, spacing: Theme.Spacing.m) {
            ProfileSectionHeader(title: Copy.Profile.humanVerification)
            ProfileCard {
                HStack(alignment: .top, spacing: Theme.Spacing.l) {
                    Image(systemName: "checkmark.shield.fill")
                        .font(.system(size: 26, weight: .semibold))
                        .foregroundStyle(Theme.Colors.success)
                        .accessibilityHidden(true)
                    VStack(alignment: .leading, spacing: Theme.Spacing.xs) {
                        Text(Copy.Profile.verified)
                            .themeFont(.bodyStrong)
                            .foregroundStyle(Theme.Colors.textPrimary)
                        Text(Copy.Profile.verifiedMessage(library: model.libraryName).capitalisedFirst)
                            .themeFont(.caption)
                            .foregroundStyle(Theme.Colors.textSecondary)
                            .fixedSize(horizontal: false, vertical: true)
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
                .accessibilityElement(children: .combine)
                ProfileDivider()
                ProfileRowButton(title: Copy.Profile.clearVerification, systemImage: "xmark.shield") {
                    confirmingClearVerification = true
                }
            }
        }
    }

    private func clearVerification() {
        let origin = model.backend.origin
        model.clearVerification()
        Task { await VerificationSession.clearStoredToken(origin: origin) }
    }

    // MARK: About

    private var aboutSection: some View {
        VStack(alignment: .leading, spacing: Theme.Spacing.m) {
            ProfileSectionHeader(title: Copy.Profile.about)
            ProfileCard {
                HStack {
                    Text(Copy.Profile.appName)
                        .themeFont(.bodyStrong)
                        .foregroundStyle(Theme.Colors.textPrimary)
                    Spacer()
                    Text(Self.versionText)
                        .themeFont(.caption)
                        .foregroundStyle(Theme.Colors.textSecondary)
                }
                .accessibilityElement(children: .combine)
                ProfileDivider()
                Text(Copy.Profile.notAffiliated)
                    .themeFont(.caption)
                    .foregroundStyle(Theme.Colors.textSecondary)
                    .fixedSize(horizontal: false, vertical: true)
            }
        }
    }

    /// "Version 0.1.0 (1)".
    static var versionText: String {
        let info = Bundle.main.infoDictionary ?? [:]
        let version = info["CFBundleShortVersionString"] as? String ?? "?"
        let build = info["CFBundleVersion"] as? String ?? "?"
        return Copy.Profile.version(version: version, build: build)
    }

    // MARK: Developer (DEBUG)

    #if DEBUG
    private var developerSection: some View {
        VStack(alignment: .leading, spacing: Theme.Spacing.m) {
            ProfileSectionHeader(title: "Developer")
            ProfileCard {
                HStack {
                    Label("Demo mode", systemImage: "hammer")
                        .themeFont(.bodyStrong)
                        .foregroundStyle(Theme.Colors.textPrimary)
                    Spacer()
                    Chip(text: model.isDemo ? "On" : "Off", tone: model.isDemo ? .success : .neutral)
                }
                .accessibilityElement(children: .combine)
                Text(model.isDemo
                     ? "Account data is sample data and never reaches the library. Sign in with demo / demo. Relaunch without -LSDemo to turn it off."
                     : "Relaunch with -LSDemo YES to use sample account data (debug builds only).")
                    .themeFont(.caption)
                    .foregroundStyle(Theme.Colors.textSecondary)
                    .fixedSize(horizontal: false, vertical: true)
                if model.isSignedIn {
                    ProfileDivider()
                    // Face ID sign-in (§5.14): the next refresh finds the session ended.
                    ProfileRowButton(title: "Expire session", systemImage: "clock.badge.xmark") {
                        Task { await model.expireSessionForDebugging() }
                    }
                }
            }
        }
    }
    #endif
}

// MARK: - Pieces

struct ProfileSectionHeader: View {
    let title: String

    var body: some View {
        Text(title)
            .themeFont(.section)
            .foregroundStyle(Theme.Colors.textPrimary)
            .accessibilityAddTraits(.isHeader)
    }
}

/// A `surface` card like Borrowed's.
struct ProfileCard<Content: View>: View {
    @ViewBuilder var content: Content

    var body: some View {
        VStack(alignment: .leading, spacing: Theme.Spacing.m) {
            content
        }
        .padding(Theme.Spacing.l)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(RoundedRectangle(cornerRadius: Theme.Radius.card, style: .continuous).fill(Theme.Colors.surface))
    }
}

struct ProfileDivider: View {
    var body: some View {
        Rectangle()
            .fill(Theme.Colors.divider)
            .frame(height: 1)
    }
}

/// A full-width text action inside a card ("Change library", "Sign out").
private struct ProfileRowButton: View {
    let title: String
    let systemImage: String
    var tint: Color = Theme.Colors.primary
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: Theme.Spacing.m) {
                Image(systemName: systemImage)
                    .frame(width: 24)
                    .accessibilityHidden(true)
                Text(title)
                Spacer()
                Image(systemName: "chevron.right")
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundStyle(Theme.Colors.textMuted)
                    .accessibilityHidden(true)
            }
            .themeFont(.bodyStrong)
            .foregroundStyle(tint)
            .frame(minHeight: 32)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityLabel(title)
    }
}
