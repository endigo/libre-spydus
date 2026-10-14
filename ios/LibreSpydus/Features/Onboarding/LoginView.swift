import SwiftUI

/// Sign-in sheet (docs/native-apps.md §6 "Login"): library card number + PIN → `AppModel.signIn`.
/// Closes itself on success (`signIn` clears `loginNeeded`); errors show inline.
struct LoginView: View {
    @Environment(AppModel.self) private var model
    @Environment(\.dismiss) private var dismiss
    @State private var form = LoginFormModel()
    @FocusState private var focus: Field?

    private enum Field: Hashable { case cardNumber, pin }

    var body: some View {
        @Bindable var form = form
        ScrollView {
            VStack(alignment: .leading, spacing: Theme.Spacing.l) {
                libraryHeader
                if model.biometrics.isTurningOn, let method = model.biometrics.method {
                    Banner(message: Copy.Biometrics.turnOnNote(method: method), tone: .info)
                }
                VStack(alignment: .leading, spacing: Theme.Spacing.m) {
                    labelled(Copy.Login.cardLabel) {
                        TextField(text: $form.cardNumber) { placeholder(Copy.Login.cardPlaceholder) }
                            .textContentType(.username)
                            .keyboardType(.asciiCapable)
                            .submitLabel(.next)
                            .focused($focus, equals: .cardNumber)
                            .onSubmit { focus = .pin }
                    }
                    labelled(Copy.Login.pinLabel) {
                        SecureField(text: $form.pin) { placeholder(Copy.Login.pinPlaceholder) }
                            .textContentType(.password)
                            .submitLabel(.go)
                            .focused($focus, equals: .pin)
                            .onSubmit(submit)
                    }
                }
                .disabled(form.isSigningIn)
                .onChange(of: form.cardNumber) { form.clearError() }
                .onChange(of: form.pin) { form.clearError() }

                if let error = form.error {
                    Banner(message: error.message, tone: .error)
                        .transition(.opacity)
                }

                PrimaryPillButton(title: form.isSigningIn ? Copy.Account.signingIn : Copy.Account.signIn, isLoading: form.isSigningIn,
                                  fillsWidth: true, action: submit)
                    .disabled(!form.canSubmit && !form.isSigningIn)
                    .padding(.top, Theme.Spacing.xs)

                if model.biometrics.isAvailable, let biometry = model.biometrics.biometry {
                    SecondaryPillButton(title: Copy.Biometrics.signInWith(method: biometry.name),
                                        systemImage: biometry.systemImage, fillsWidth: true, action: signInWithBiometrics)
                        .disabled(form.isSigningIn)
                }

                Text(privacyNote)
                    .themeFont(.caption)
                    .foregroundStyle(Theme.Colors.textMuted)
                    .fixedSize(horizontal: false, vertical: true)

                #if DEBUG
                if model.isDemo {
                    Banner(message: "Demo: demo / demo", tone: .info)
                        .accessibilityLabel("Demo mode. Card number demo, PIN demo.")
                }
                #endif
            }
            .padding(.horizontal, Theme.Spacing.xl)
            .padding(.bottom, Theme.Spacing.xxl)
            .animation(.easeOut(duration: 0.2), value: form.error)
        }
        .scrollDismissesKeyboard(.interactively)
        .safeAreaInset(edge: .top, spacing: 0) { topBar }
        .screenBackground()
        .presentationBackground(Theme.Colors.background)
        .presentationDragIndicator(.visible)
        .interactiveDismissDisabled(form.isSigningIn)
        .task {
            form.appear(model)
            // Face ID kept for this library: ask for it at once, unless a check was just
            // cancelled or failed (§5.14).
            if model.biometrics.takeAutomaticCheck() {
                // Let the sheet finish coming up first.
                try? await Task.sleep(for: .milliseconds(400))
                guard !Task.isCancelled else { return }
                focus = nil
                await form.signInWithBiometrics(model)
            }
            #if DEBUG
            await applyDebugHooks()
            #endif
        }
    }

    private var topBar: some View {
        HStack {
            Spacer()
            Button {
                dismiss()
            } label: {
                Image(systemName: "xmark")
                    .font(.system(size: 15, weight: .bold))
                    .foregroundStyle(Theme.Colors.textSecondary)
                    .frame(width: 34, height: 34)
                    .background(Circle().fill(Theme.Colors.surface))
                    .frame(width: 44, height: 44)
                    .contentShape(Circle())
            }
            .buttonStyle(.plain)
            .accessibilityLabel(Copy.Common.close)
        }
        .padding(.horizontal, Theme.Spacing.m)
        .padding(.top, Theme.Spacing.s)
    }

    private var libraryHeader: some View {
        VStack(alignment: .leading, spacing: Theme.Spacing.m) {
            if let library = model.library {
                LibraryLogo(library, size: 64)
            }
            VStack(alignment: .leading, spacing: Theme.Spacing.xs) {
                Text(Copy.Login.title)
                    .themeFont(.greeting)
                    .foregroundStyle(Theme.Colors.textPrimary)
                    .accessibilityAddTraits(.isHeader)
                Text(model.libraryName)
                    .themeFont(.section)
                    .foregroundStyle(Theme.Colors.textSecondary)
                    .fixedSize(horizontal: false, vertical: true)
            }
        }
        .padding(.bottom, Theme.Spacing.s)
    }

    /// A caption over a rounded navy field (card number / PIN).
    private func labelled(_ title: String, @ViewBuilder field: () -> some View) -> some View {
        VStack(alignment: .leading, spacing: Theme.Spacing.s) {
            Text(title)
                .themeFont(.caption)
                .foregroundStyle(Theme.Colors.textSecondary)
                .accessibilityHidden(true)
            field()
                .themeFont(.body)
                .foregroundStyle(Theme.Colors.textPrimary)
                .tint(Theme.Colors.primary)
                .autocorrectionDisabled()
                .textInputAutocapitalization(.never)
                .accessibilityLabel(title)
                .padding(.horizontal, Theme.Spacing.l)
                .frame(minHeight: 52)
                .background(RoundedRectangle(cornerRadius: Theme.Radius.field, style: .continuous).fill(Theme.Colors.field))
        }
    }

    /// "…doesn’t store your PIN", unless the phone can keep it for Face ID (§5.14).
    private var privacyNote: String {
        guard let method = model.biometrics.method else { return Copy.Login.privacyNote(library: model.libraryName) }
        return Copy.Login.privacyNoteBiometrics(library: model.libraryName, method: method)
    }

    private func placeholder(_ text: String) -> Text {
        Text(text).foregroundStyle(Theme.Colors.textMuted)
    }

    private func submit() {
        guard form.canSubmit else { return }
        focus = nil
        Task { await form.submit(to: model) }
    }

    private func signInWithBiometrics() {
        focus = nil
        Task { await form.signInWithBiometrics(model) }
    }

    #if DEBUG
    /// Screenshot hooks (DEBUG): `-LSLoginCard <card> -LSLoginPIN <pin>` fill the form and
    /// `-LSLoginSubmit YES` signs in once, **in demo mode only** — never against a real library.
    private func applyDebugHooks() async {
        let defaults = UserDefaults.standard
        guard model.isDemo, form.cardNumber.isEmpty, let card = defaults.string(forKey: "LSLoginCard") else { return }
        form.cardNumber = card
        form.pin = defaults.string(forKey: "LSLoginPIN") ?? ""
        if defaults.bool(forKey: "LSLoginSubmit") {
            try? await Task.sleep(for: .milliseconds(400))
            await form.submit(to: model)
        }
    }
    #endif
}
