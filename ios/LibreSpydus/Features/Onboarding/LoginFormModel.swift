import Foundation
import Observation

/// The sign-in form: card number + PIN → `AppModel.signIn`, errors per docs/native-apps.md §5.7.
@MainActor
@Observable
final class LoginFormModel {
    var cardNumber = ""
    var pin = ""
    private(set) var isSigningIn = false
    /// Shown inline under the fields. A verification challenge isn't shown here: `report` swaps
    /// this sheet for the verification sheet.
    private(set) var error: UserFacingError?

    var canSubmit: Bool {
        !trimmedCardNumber.isEmpty && !pin.isEmpty && !isSigningIn
    }

    private var trimmedCardNumber: String {
        cardNumber.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    /// The sheet opened: the error a sign-in with Face ID left (§5.14), if any.
    func appear(_ model: AppModel) {
        if let shown = model.biometrics.takeFormError() { error = shown }
    }

    /// Signs in; true on success (the sheet closes because `AppModel` clears `loginNeeded`).
    @discardableResult
    func submit(to model: AppModel) async -> Bool {
        guard canSubmit else { return false }
        isSigningIn = true
        error = nil
        defer { isSigningIn = false }
        do {
            try await model.signIn(username: trimmedCardNumber, password: pin)
            pin = ""
            return true
        } catch {
            let shown = model.report(error)
            self.error = shown?.kind == .verificationRequired ? nil : shown
            return false
        }
    }

    /// "Sign in with Face ID": the check, then a sign-in with the kept details. Cancelled → the
    /// form stays as it is; the library's error shows like a typed sign-in's.
    func signInWithBiometrics(_ model: AppModel) async {
        guard !isSigningIn else { return }
        isSigningIn = true
        error = nil
        defer { isSigningIn = false }
        if case .failed(let failure) = await model.signInWithBiometrics() {
            let shown = model.report(failure)
            error = shown?.kind == .verificationRequired ? nil : shown
        }
    }

    func clearError() {
        error = nil
    }
}
