import Foundation
import LibreSpydusCore
import Observation

/// After a typed sign-in: keep the details behind the check? (`biometricOfferAlert`.) The PIN
/// stays in memory only until the answer.
struct BiometricOffer: Equatable, Identifiable {
    let id = UUID()
    let origin: String
    /// `{method}`: "Face ID".
    let method: String
    let details: SignInDetails
    /// False when Profile's switch asked for it: the check runs without the question, once the
    /// sign-in sheet (and iOS's "Save Password?") has gone.
    var asks = true
}

/// Sign back in with Face ID (docs/native-apps.md §5.14) for the selected library: whether the
/// check is available and the library has details kept, the offer after a typed sign-in, and a
/// sign-in with the check under way. `AppModel` runs the sign-ins (the extension below).
@MainActor
@Observable
final class BiometricSignIn {
    enum State: Equatable {
        case idle
        /// The session ended while the app was in the background: starts once it's back.
        case waiting
        case running
    }

    /// The check this phone can run (read again when the app comes to the foreground).
    private(set) var biometry: Biometry?
    /// The username of the selected library's kept details; nil: none kept.
    private(set) var username: String?
    /// "Sign in with Face ID next time?" after a typed sign-in.
    private(set) var offer: BiometricOffer?
    private(set) var state: State = .idle
    /// Profile's switch was turned on: the next typed sign-in keeps the details without asking.
    private(set) var isTurningOn = false
    /// The sign-in form's error after a sign-in with the check that didn't work (taken once).
    private(set) var formError: UserFacingError?
    /// The sign-in sheet doesn't start the check by itself the next time it opens: one was just
    /// cancelled or failed.
    private(set) var skipsAutomaticCheck = false

    @ObservationIgnored private let vault: any BiometricVault
    @ObservationIgnored private let defaults: UserDefaults
    @ObservationIgnored private let declinedKey: String
    @ObservationIgnored private(set) var origin: String?

    nonisolated static let namespace = "LibreSpydus.biometrics"
    /// Demo mode's own "Not now" answers, so it never changes the real ones.
    nonisolated static let demoNamespace = "LibreSpydus.demo.biometrics"

    init(vault: any BiometricVault, defaults: UserDefaults, namespace: String) {
        self.vault = vault
        self.defaults = defaults
        declinedKey = "\(namespace).offerDeclined"
        biometry = vault.biometry()
    }

    /// `{method}` while the check is available.
    var method: String? { biometry?.name }
    var hasDetails: Bool { username != nil }
    /// The selected library can sign in with the check now.
    var isAvailable: Bool { biometry != nil && hasDetails }
    /// Profile's switch: on while details are kept, or while turning it on is under way.
    var isSwitchedOn: Bool { hasDetails || isTurningOn || offer?.asks == false }

    /// The selected library (nil: none); anything under way for the previous one is dropped.
    func use(origin: String?) {
        self.origin = origin
        username = origin.flatMap(vault.username(origin:))
        offer = nil
        state = .idle
        isTurningOn = false
        formError = nil
        skipsAutomaticCheck = false
    }

    /// The app came to the foreground: a face or finger may have been enrolled or removed.
    func refreshAvailability() {
        biometry = vault.biometry()
        username = origin.flatMap(vault.username(origin:))
    }

    // MARK: Turning on

    /// A typed sign-in to `origin` succeeded: details kept for another username go, then the
    /// details are kept without asking (Profile's switch) or offered, unless the reader said Not now.
    func didSignInByTyping(_ details: SignInDetails, origin: String) {
        guard origin == self.origin else { return }
        formError = nil
        skipsAutomaticCheck = false
        // Signed in: a sign-in with the check that waited for the foreground isn't needed.
        if state == .waiting { state = .idle }
        if let kept = username, !Self.sameUsername(kept, details.username) {
            vault.delete(origin: origin)
            username = nil
        }
        let turningOn = isTurningOn
        isTurningOn = false
        guard let biometry, !hasDetails else { return }
        if turningOn || !wasDeclined(origin: origin) {
            offer = BiometricOffer(origin: origin, method: biometry.name, details: details, asks: !turningOn)
        }
    }

    /// The reader answered the offer. Use → the check, then the details are kept (a cancelled or
    /// failed check keeps nothing and isn't remembered); Not now → remembered for this library.
    func answerOffer(use: Bool, reason: String) async {
        guard let offer else { return }
        self.offer = nil
        if use {
            await keep(offer.details, origin: offer.origin, reason: reason)
        } else {
            setDeclined(true, origin: offer.origin)
        }
    }

    /// Profile's switch turned on: the sign-in sheet opens and its next typed sign-in keeps the
    /// details. Off: they're deleted at once.
    func setOn(_ on: Bool) {
        guard let origin else { return }
        if on {
            isTurningOn = true
        } else {
            isTurningOn = false
            vault.delete(origin: origin)
            username = nil
        }
    }

    /// The sign-in sheet closed: turning on from Profile ends with it.
    func sheetClosed() {
        isTurningOn = false
    }

    private func keep(_ details: SignInDetails, origin: String, reason: String) async {
        do {
            try await vault.save(details, origin: origin, reason: reason)
            if origin == self.origin { username = details.username }
        } catch {
            // Cancelled or failed: nothing kept; the switch on Profile still says so.
        }
    }

    // MARK: Signing in

    /// Asks for the check and hands back the selected library's details. Throws
    /// `BiometricFailure` (`.missing`: they were unreadable and are gone).
    func read(reason: String) async throws -> SignInDetails {
        guard let origin else { throw BiometricFailure.missing }
        do {
            return try await vault.read(origin: origin, reason: reason)
        } catch BiometricFailure.missing {
            if origin == self.origin { username = nil }
            throw BiometricFailure.missing
        }
    }

    func begin() { state = .running }
    func wait() { state = .waiting }
    func finish() { state = .idle }

    /// The library turned the kept details down (the PIN changed): they go.
    func deleteDetails() {
        guard let origin else { return }
        vault.delete(origin: origin)
        username = nil
    }

    /// A sign-in with the check didn't work: the form shows `error` (if any) and the sheet waits
    /// for the reader instead of asking again.
    func failed(showing error: UserFacingError?) {
        formError = error
        skipsAutomaticCheck = true
    }

    /// For the sign-in form when it opens: the error to show, once.
    func takeFormError() -> UserFacingError? {
        defer { formError = nil }
        return formError
    }

    /// For the sign-in sheet when it opens: whether to start the check at once (once per opening).
    func takeAutomaticCheck() -> Bool {
        defer { skipsAutomaticCheck = false }
        return isAvailable && !skipsAutomaticCheck && !isTurningOn && state == .idle
    }

    // MARK: Sign out

    /// Signed out of `origin`: its details and its Not now go.
    func forget(origin: String) {
        vault.delete(origin: origin)
        setDeclined(false, origin: origin)
        if origin == self.origin {
            username = nil
            offer = nil
            isTurningOn = false
        }
    }

    /// `-LSReset`: every library's details and answers, real and demo.
    nonisolated static func deleteAll(vault: any BiometricVault, defaults: UserDefaults) {
        vault.deleteAll()
        for namespace in [namespace, demoNamespace] {
            defaults.removeObject(forKey: "\(namespace).offerDeclined")
        }
    }

    // MARK: Private

    private func wasDeclined(origin: String) -> Bool {
        declinedOrigins.contains(origin)
    }

    private func setDeclined(_ declined: Bool, origin: String) {
        var origins = declinedOrigins
        if declined { origins.insert(origin) } else { origins.remove(origin) }
        defaults.set(origins.sorted(), forKey: declinedKey)
    }

    private var declinedOrigins: Set<String> {
        Set(defaults.stringArray(forKey: declinedKey) ?? [])
    }

    nonisolated static func sameUsername(_ a: String, _ b: String) -> Bool {
        a.trimmingCharacters(in: .whitespacesAndNewlines)
            .caseInsensitiveCompare(b.trimmingCharacters(in: .whitespacesAndNewlines)) == .orderedSame
    }
}

// MARK: - AppModel

extension AppModel {
    /// How a sign-in with the check ended.
    enum BiometricSignInResult {
        case signedIn
        /// Cancelled, "Enter PIN", not recognised, unavailable, or the details were unreadable.
        case declined
        /// The library answered with an error (the PIN changed: the details are gone).
        case failed(any Error)
        /// The library changed meanwhile.
        case stale
    }

    /// `NotLoggedIn` (from `report`): sign back in with the check when this library has details,
    /// keeping the session and the saved account on screen until it ends. False when there's
    /// nothing to sign in with: the caller signs out and asks as before.
    func signBackInWithBiometrics() -> Bool {
        guard biometrics.isAvailable else { return false }
        // Already running; one that is waiting starts now if the app is in the foreground after all.
        guard biometrics.state != .running else { return true }
        guard isAppActive() else {
            biometrics.wait()
            return true
        }
        biometrics.begin()
        Task { await runBiometricSignIn() }
        return true
    }

    /// The app is in the foreground again (RootView): a sign-in that waited for it starts.
    func appBecameActive() {
        biometrics.refreshAvailability()
        guard biometrics.state == .waiting else { return }
        guard biometrics.isAvailable else {
            biometrics.finish()
            signOutAfterCheck(error: nil)
            return
        }
        biometrics.begin()
        Task { await runBiometricSignIn() }
    }

    /// The check, then `signIn` with the kept details exactly as from the form, without the offers.
    /// The sign-in sheet's "Sign in with Face ID" button and the automatic sign-in both use it.
    func signInWithBiometrics() async -> BiometricSignInResult {
        let origin = backend.origin
        let details: SignInDetails
        do {
            details = try await biometrics.read(reason: Copy.Biometrics.reason(library: libraryName))
        } catch {
            return backend.origin == origin ? .declined : .stale
        }
        guard backend.origin == origin else { return .stale }
        do {
            try await signIn(username: details.username, password: details.pin, typed: false)
            return .signedIn
        } catch {
            guard backend.origin == origin, !(error is CancellationError) else { return .stale }
            if case .CredentialsRejected = error as? SpydusError { biometrics.deleteDetails() }
            return .failed(error)
        }
    }

    /// Profile's "Sign in with Face ID" switch.
    func setBiometricSignIn(_ on: Bool) {
        biometrics.setOn(on)
        if on { loginNeeded = true }
    }

    /// The reader answered "Sign in with Face ID next time?".
    func answerBiometricOffer(use: Bool) async {
        await biometrics.answerOffer(use: use, reason: Copy.Biometrics.reason(library: libraryName))
    }

    private func runBiometricSignIn() async {
        let origin = backend.origin
        let result = await signInWithBiometrics()
        guard backend.origin == origin else { return }
        biometrics.finish()
        switch result {
        case .signedIn, .stale:
            break
        case .declined:
            signOutAfterCheck(error: nil)
        case .failed(let error):
            signOutAfterCheck(error: error)
        }
    }

    /// The sign-in with the check didn't work: signed out as before, and the sign-in sheet shows
    /// the form (with the library's error) instead of asking for the check again. A verification
    /// challenge opens its sheet first (`report`), the form after it.
    private func signOutAfterCheck(error: (any Error)?) {
        clearSession()
        var shown: UserFacingError?
        if let error, !Self.isSessionEnded(error) {
            shown = report(error)
        }
        biometrics.failed(showing: shown?.kind == .verificationRequired ? nil : shown)
        loginNeeded = true
    }

    private static func isSessionEnded(_ error: any Error) -> Bool {
        if case .NotLoggedIn = error as? SpydusError { true } else { false }
    }
}
