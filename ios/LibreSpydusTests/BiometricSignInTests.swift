import Foundation
import LibreSpydusCore
import Testing
@testable import LibreSpydus

/// Face ID's vault without the Keychain or the system check: details in memory, and each check
/// passes unless `failure` says otherwise.
final class FakeBiometricVault: BiometricVault, @unchecked Sendable {
    private let lock = NSLock()
    private var _details: [String: SignInDetails]
    private var _available: Biometry?
    private var _failure: BiometricFailure?
    private var _checks = 0

    init(details: [String: SignInDetails] = [:], available: Biometry? = .faceID) {
        _details = details
        _available = available
    }

    var details: [String: SignInDetails] { lock.withLock { _details } }
    var checks: Int { lock.withLock { _checks } }
    /// What the next checks do: nil passes.
    var failure: BiometricFailure? {
        get { lock.withLock { _failure } }
        set { lock.withLock { _failure = newValue } }
    }
    var available: Biometry? {
        get { lock.withLock { _available } }
        set { lock.withLock { _available = newValue } }
    }

    func biometry() -> Biometry? { available }
    func username(origin: String) -> String? { lock.withLock { _details[origin]?.username } }

    func save(_ details: SignInDetails, origin: String, reason: String) async throws {
        try check()
        lock.withLock { _details[origin] = details }
    }

    func read(origin: String, reason: String) async throws -> SignInDetails {
        guard let saved = lock.withLock({ _details[origin] }) else { throw BiometricFailure.missing }
        try check()
        return saved
    }

    func delete(origin: String) { lock.withLock { _ = _details.removeValue(forKey: origin) } }
    func deleteAll() { lock.withLock { _details.removeAll() } }

    private func check() throws {
        try lock.withLock {
            _checks += 1
            if let _failure { throw _failure }
        }
    }
}

@MainActor
private final class Flag {
    var isOn = false
}

@MainActor
struct BiometricSignInTests {
    private static let origin = "https://canning.spydus.com"
    private static let kept = SignInDetails(username: "card", pin: "ok")

    /// A model at Canning with `vault`; signed in when `session` is given.
    private func makeModel(vault: FakeBiometricVault = FakeBiometricVault(), session: String? = nil,
                           defaults: UserDefaults = PicksTestData.defaults(), active: @escaping @MainActor () -> Bool = { true },
                           onBackend: @escaping @MainActor (FakeBackend) -> Void = { _ in }) -> AppModel {
        var dependencies = AppModel.Dependencies.testing(
            defaults: defaults,
            sessions: InMemorySecretStore(session.map { [Self.origin: $0] } ?? [:]),
            onBackend: onBackend)
        dependencies.biometricVault = vault
        dependencies.isAppActive = active
        let model = AppModel(dependencies: dependencies)
        model.selectLibrary(.canning)
        return model
    }

    private func sessionEnded(_ model: AppModel) -> UserFacingError? {
        model.report(SpydusError.NotLoggedIn(message: "not logged in"))
    }

    // MARK: The offer

    @Test func aTypedSignInOffersToKeepTheDetails() async throws {
        let model = makeModel()
        try await model.signIn(username: "card", password: "ok")
        let offer = try #require(model.biometrics.offer)
        #expect(offer.asks)
        #expect(offer.method == "Face ID")
        #expect(offer.details == Self.kept)
        #expect(Copy.Biometrics.offerTitle(method: offer.method) == "Sign in with Face ID next time?")
    }

    @Test func aRestoredSessionOrNoFaceIDOffersNothing() async throws {
        let restored = makeModel(session: "token")
        #expect(restored.isSignedIn)
        #expect(restored.biometrics.offer == nil)

        let noFaceID = makeModel(vault: FakeBiometricVault(available: nil))
        try await noFaceID.signIn(username: "card", password: "ok")
        #expect(noFaceID.biometrics.offer == nil)
        #expect(noFaceID.biometrics.method == nil)
    }

    @Test func useKeepsTheDetailsAfterTheCheck() async throws {
        let vault = FakeBiometricVault()
        let model = makeModel(vault: vault)
        try await model.signIn(username: "card", password: "ok")
        await model.answerBiometricOffer(use: true)
        #expect(vault.details[Self.origin] == Self.kept)
        #expect(vault.checks == 1)
        #expect(model.biometrics.hasDetails)
        #expect(model.biometrics.offer == nil)
    }

    @Test func aCancelledCheckKeepsNothingAndAsksAgainNextTime() async throws {
        let vault = FakeBiometricVault()
        vault.failure = .declined
        let model = makeModel(vault: vault)
        try await model.signIn(username: "card", password: "ok")
        await model.answerBiometricOffer(use: true)
        #expect(vault.details.isEmpty)
        #expect(!model.biometrics.hasDetails)

        try await model.signIn(username: "card", password: "ok")
        #expect(model.biometrics.offer != nil)
    }

    @Test func notNowIsRememberedUntilSignOut() async throws {
        let defaults = PicksTestData.defaults()
        let model = makeModel(defaults: defaults)
        try await model.signIn(username: "card", password: "ok")
        await model.answerBiometricOffer(use: false)
        try await model.signIn(username: "card", password: "ok")
        #expect(model.biometrics.offer == nil)
        // Remembered across launches.
        let relaunched = makeModel(defaults: defaults)
        try await relaunched.signIn(username: "card", password: "ok")
        #expect(relaunched.biometrics.offer == nil)

        relaunched.signOut()
        try await relaunched.signIn(username: "card", password: "ok")
        #expect(relaunched.biometrics.offer != nil)
    }

    @Test func myPicksOfferStaysForAfterThisOne() async throws {
        let model = makeModel()
        model.favourites.add(BookRef(id: "work:1", title: "First", url: nil))
        try await model.signIn(username: "card", password: "ok")
        #expect(model.biometrics.offer != nil)
        #expect(model.picksCopyOffer != nil)
        await model.answerBiometricOffer(use: false)
        #expect(model.picksCopyOffer != nil)
    }

    @Test func aTypedSignInWithAnotherUsernameDeletesTheDetails() async throws {
        let vault = FakeBiometricVault(details: [Self.origin: Self.kept])
        let model = makeModel(vault: vault)
        // The same username (case and spaces aside) keeps them, and there's nothing to offer.
        try await model.signIn(username: " CARD ", password: "ok")
        #expect(vault.details[Self.origin] == Self.kept)
        #expect(model.biometrics.offer == nil)

        try await model.signIn(username: "someone-else", password: "ok")
        #expect(vault.details.isEmpty)
        #expect(!model.biometrics.hasDetails)
        #expect(model.biometrics.offer?.details.username == "someone-else")
    }

    // MARK: Signing back in

    @Test func anEndedSessionSignsBackInWithTheCheck() async throws {
        let vault = FakeBiometricVault(details: [Self.origin: Self.kept])
        let model = makeModel(vault: vault, session: "stale")
        let generation = model.accountGeneration
        _ = sessionEnded(model)
        // The session stays (and no sheet) while the check runs.
        #expect(model.isSignedIn)
        #expect(model.rootSheet == nil)
        #expect(model.biometrics.state == .running)
        // More ended sessions meanwhile are the same sign-in.
        _ = sessionEnded(model)

        #expect(await eventually { model.biometrics.state == .idle })
        #expect(model.session == "fake-session")
        #expect(model.rootSheet == nil)
        #expect(model.accountGeneration > generation)
        #expect(vault.checks == 1)
        #expect(model.biometrics.offer == nil)
        #expect(model.accountName == "Fake Borrower")
    }

    @Test func aCancelledCheckSignsOutAndShowsTheForm() async throws {
        let vault = FakeBiometricVault(details: [Self.origin: Self.kept])
        vault.failure = .declined
        let model = makeModel(vault: vault, session: "stale")
        _ = sessionEnded(model)
        #expect(await eventually { model.biometrics.state == .idle })
        #expect(!model.isSignedIn)
        #expect(model.rootSheet == .login)
        // Kept, but the sheet doesn't ask again by itself; it does the next time it opens.
        #expect(model.biometrics.hasDetails)
        #expect(!model.biometrics.takeAutomaticCheck())
        #expect(model.biometrics.takeAutomaticCheck())
        #expect(model.biometrics.takeFormError() == nil)
    }

    @Test func aChangedPinDeletesTheDetailsAndSaysSo() async throws {
        let vault = FakeBiometricVault(details: [Self.origin: SignInDetails(username: "card", pin: "old")])
        let model = makeModel(vault: vault, session: "stale")
        _ = sessionEnded(model)
        #expect(await eventually { model.biometrics.state == .idle })
        #expect(!model.isSignedIn)
        #expect(model.rootSheet == .login)
        #expect(vault.details.isEmpty)
        #expect(!model.biometrics.hasDetails)
        #expect(model.biometrics.takeFormError()?.message == "Card number or PIN not recognised.")
    }

    @Test func anotherErrorKeepsTheDetails() async throws {
        let vault = FakeBiometricVault(details: [Self.origin: Self.kept])
        let model = makeModel(vault: vault, session: "stale", onBackend: { $0.loginError = .Network(message: "offline") })
        _ = sessionEnded(model)
        #expect(await eventually { model.biometrics.state == .idle })
        #expect(!model.isSignedIn)
        #expect(model.rootSheet == .login)
        #expect(vault.details[Self.origin] == Self.kept)
        #expect(model.biometrics.takeFormError()?.message == Copy.Errors.network(library: "City of Canning"))
    }

    @Test func unreadableDetailsAreGone() async throws {
        let vault = FakeBiometricVault(details: [Self.origin: Self.kept])
        vault.failure = .missing
        let model = makeModel(vault: vault, session: "stale")
        _ = sessionEnded(model)
        #expect(await eventually { model.biometrics.state == .idle })
        #expect(model.rootSheet == .login)
        #expect(!model.biometrics.hasDetails)
    }

    @Test func withoutDetailsAnEndedSessionAsksAsBefore() {
        let model = makeModel(session: "stale")
        #expect(sessionEnded(model)?.message == "Please sign in again.")
        #expect(!model.isSignedIn)
        #expect(model.rootSheet == .login)
    }

    @Test func inTheBackgroundItWaitsForTheForeground() async throws {
        let vault = FakeBiometricVault(details: [Self.origin: Self.kept])
        let foreground = Flag()
        let model = makeModel(vault: vault, session: "stale", active: { foreground.isOn })
        _ = sessionEnded(model)
        #expect(model.biometrics.state == .waiting)
        #expect(model.isSignedIn)
        #expect(model.rootSheet == nil)
        #expect(vault.checks == 0)

        foreground.isOn = true
        model.appBecameActive()
        #expect(await eventually { model.biometrics.state == .idle })
        #expect(model.session == "fake-session")
        #expect(vault.checks == 1)
    }

    @Test func aTypedSignInEndsOneThatWaitedForTheForeground() async throws {
        let vault = FakeBiometricVault(details: [Self.origin: Self.kept])
        let model = makeModel(vault: vault, session: "stale", active: { false })
        _ = sessionEnded(model)
        #expect(model.biometrics.state == .waiting)
        try await model.signIn(username: "card", password: "ok")
        #expect(model.biometrics.state == .idle)
        model.appBecameActive()
        #expect(model.biometrics.state == .idle)
        #expect(vault.checks == 0)
    }

    @Test func theSheetsButtonSignsInAndShowsTheLibrarysError() async throws {
        let vault = FakeBiometricVault(details: [Self.origin: Self.kept])
        let model = makeModel(vault: vault)
        #expect(model.biometrics.takeAutomaticCheck())
        let form = LoginFormModel()
        await form.signInWithBiometrics(model)
        #expect(model.session == "fake-session")
        #expect(form.error == nil)

        let other = FakeBiometricVault(details: [Self.origin: SignInDetails(username: "card", pin: "old")])
        let rejected = makeModel(vault: other)
        await form.signInWithBiometrics(rejected)
        #expect(form.error?.message == "Card number or PIN not recognised.")
        #expect(!rejected.biometrics.isAvailable)
    }

    // MARK: Profile, sign-out, reset

    @Test func profilesSwitchTurnsOnThroughTheSheetAndOffAtOnce() async throws {
        let vault = FakeBiometricVault()
        let model = makeModel(vault: vault, session: "token")
        model.setBiometricSignIn(true)
        #expect(model.rootSheet == .login)
        #expect(model.biometrics.isTurningOn)
        #expect(!model.biometrics.takeAutomaticCheck())

        // Closing the sheet leaves it off.
        model.dismiss(.login)
        #expect(!model.biometrics.isTurningOn)

        model.setBiometricSignIn(true)
        try await model.signIn(username: "card", password: "ok")
        // No question: the check runs once the sheet has gone (`biometricOfferAlert`).
        #expect(model.biometrics.offer?.asks == false)
        #expect(!model.biometrics.isTurningOn)
        await model.answerBiometricOffer(use: true)
        #expect(model.biometrics.hasDetails)
        #expect(vault.details[Self.origin] == Self.kept)
        #expect(vault.checks == 1)

        model.setBiometricSignIn(false)
        #expect(vault.details.isEmpty)
        #expect(!model.biometrics.hasDetails)
    }

    @Test func signOutForgetsTheDetails() {
        let vault = FakeBiometricVault(details: [Self.origin: Self.kept])
        let model = makeModel(vault: vault, session: "token")
        #expect(model.biometrics.hasDetails)
        model.signOut()
        #expect(vault.details.isEmpty)
        #expect(!model.biometrics.hasDetails)
    }

    @Test func resetForgetsEveryLibrarysDetailsAndAnswers() {
        let defaults = PicksTestData.defaults()
        defaults.set([Self.origin], forKey: "\(BiometricSignIn.namespace).offerDeclined")
        defaults.set([Self.origin], forKey: "\(BiometricSignIn.demoNamespace).offerDeclined")
        let vault = FakeBiometricVault(details: [Self.origin: Self.kept, "https://other.spydus.com": Self.kept])
        var dependencies = AppModel.Dependencies.testing(defaults: defaults)
        dependencies.biometricVault = vault
        dependencies.resetPersistentState()
        #expect(vault.details.isEmpty)
        #expect(defaults.object(forKey: "\(BiometricSignIn.namespace).offerDeclined") == nil)
        #expect(defaults.object(forKey: "\(BiometricSignIn.demoNamespace).offerDeclined") == nil)
    }
}
