import Foundation
import LibreSpydusCore
import Testing
@testable import LibreSpydus

extension AppModel.Dependencies {
    /// Isolated stores; every backend is a `FakeBackend` (recorded in `backends`).
    @MainActor
    static func testing(defaults: UserDefaults = UserDefaults(suiteName: "LibreSpydusTests-\(UUID().uuidString)")!,
                        sessions: InMemorySecretStore = InMemorySecretStore(),
                        cookies: InMemorySecretStore = InMemorySecretStore(),
                        directory: URL = TestData.temporaryDirectory(),
                        demo: Bool = false,
                        onBackend: @escaping @MainActor (FakeBackend) -> Void = { _ in }) -> AppModel.Dependencies {
        AppModel.Dependencies(
            defaults: defaults, sessions: sessions, verificationCookies: cookies, favouritesDirectory: directory,
            isDemo: demo,
            makeBackend: { origin in
                let backend = FakeBackend(origin: origin ?? "https://canning.spydus.com")
                onBackend(backend)
                return backend
            }
        )
    }
}

@MainActor
struct AppModelTests {
    @Test func startsInOnboardingWithoutALibrary() {
        let model = AppModel(dependencies: .testing())
        #expect(model.needsOnboarding)
        #expect(model.rootSheet == nil)
        #expect(!model.isSignedIn)
    }

    @Test func persistsTheSelectedLibrary() {
        let defaults = UserDefaults(suiteName: "LibreSpydusTests-\(UUID().uuidString)")!
        let first = AppModel(dependencies: .testing(defaults: defaults))
        first.selectLibrary(.canning)
        let second = AppModel(dependencies: .testing(defaults: defaults))
        #expect(second.library == .canning)
        #expect(second.backend.origin == "https://canning.spydus.com")
    }

    @Test func reappliesTheVerificationCookieWhenTheClientIsCreated() {
        let cookies = InMemorySecretStore(["https://canning.spydus.com": "aws-waf-token=abc"])
        var created: [FakeBackend] = []
        let model = AppModel(dependencies: .testing(cookies: cookies) { created.append($0) })
        model.selectLibrary(.canning)
        #expect(created.last?.verificationCookies == ["aws-waf-token=abc"])
        #expect(model.isVerified)
    }

    @Test func completeVerificationStoresAppliesAndBumpsTheGeneration() throws {
        let cookies = InMemorySecretStore()
        var created: [FakeBackend] = []
        let model = AppModel(dependencies: .testing(cookies: cookies) { created.append($0) })
        model.selectLibrary(.canning)
        model.report(SpydusError.HumanVerificationRequired(message: "human verification required"))
        #expect(model.rootSheet == .verification)

        let generation = model.verificationGeneration
        model.completeVerification(cookie: "aws-waf-token=xyz")
        #expect(model.rootSheet == nil)
        #expect(model.verificationGeneration == generation + 1)
        #expect(cookies.value(for: "https://canning.spydus.com") == "aws-waf-token=xyz")
        #expect(try #require(created.last).verificationCookies.last == "aws-waf-token=xyz")

        model.clearVerification()
        #expect(!model.isVerified)
        #expect(cookies.value(for: "https://canning.spydus.com") == nil)
    }

    @Test func stopsAskingAfterTwoVerificationsTheLibraryStillRejects() {
        let model = AppModel(dependencies: .testing())
        model.selectLibrary(.canning)
        let challenge = SpydusError.HumanVerificationRequired(message: "human verification required")
        model.completeVerification(cookie: "aws-waf-token=1")
        model.completeVerification(cookie: "aws-waf-token=2")
        let error = model.report(challenge)
        #expect(error?.kind == .verificationUnavailable)
        #expect(!model.verificationNeeded)
    }

    @Test func signInStoresTheSessionAndLoadsTheAccount() async throws {
        let sessions = InMemorySecretStore()
        let model = AppModel(dependencies: .testing(sessions: sessions))
        model.selectLibrary(.canning)
        try await model.signIn(username: "card", password: "ok")
        #expect(model.isSignedIn)
        #expect(model.accountName == "Fake Borrower")
        #expect(sessions.value(for: "https://canning.spydus.com") == "fake-session")

        await #expect(throws: SpydusError.self) { try await model.signIn(username: "card", password: "bad") }
    }

    @Test func notLoggedInClearsTheSessionAndAsksToSignIn() async throws {
        let sessions = InMemorySecretStore(["https://canning.spydus.com": "stale"])
        let model = AppModel(dependencies: .testing(sessions: sessions))
        model.selectLibrary(.canning)
        #expect(model.isSignedIn)
        let generation = model.accountGeneration
        let error = model.report(SpydusError.NotLoggedIn(message: "not logged in"))
        #expect(error?.message == "Please sign in again.")
        #expect(!model.isSignedIn)
        #expect(model.rootSheet == .login)
        #expect(model.accountGeneration > generation)
        #expect(sessions.value(for: "https://canning.spydus.com") == nil)
    }

    @Test func cancellationIsNotAnError() {
        let model = AppModel(dependencies: .testing())
        #expect(model.report(CancellationError()) == nil)
    }

    @Test func demoModeStartsSignedIn() {
        let model = AppModel(dependencies: .testing(demo: true))
        model.selectLibrary(.canning)
        #expect(model.isSignedIn)
    }

    @Test func changingLibraryResetsNavigationAndSwitchesFavourites() {
        let model = AppModel(dependencies: .testing())
        model.selectLibrary(.canning)
        model.favourites.add(BookRef(id: "1", title: "One"))
        model.push(.results(query: "q", sort: .newest, title: "Q"), on: .home)
        #expect(model.path(for: .home).count == 1)

        model.selectLibrary(origin: "https://hutt.spydus.co.nz")
        #expect(model.library?.name == "Hutt")
        #expect(model.library?.region == "NZ")
        #expect(model.path(for: .home).isEmpty)
        #expect(model.favourites.items.isEmpty)

        model.selectLibrary(.canning)
        #expect(model.favourites.items.map(\.id) == ["1"])
    }

    @Test func homeSearchHandsTheQueryToTheSearchTab() {
        let model = AppModel(dependencies: .testing())
        model.openSearch(query: "dune")
        #expect(model.selectedTab == .search)
        #expect(model.takePendingSearch() == "dune")
        #expect(model.takePendingSearch() == nil)
    }

    @Test func reselectingATabPopsToRoot() {
        let model = AppModel(dependencies: .testing())
        model.push(.detail(BookRef(id: "1", title: "One")), on: .favourites)
        #expect(model.selectedTab == .favourites)
        model.select(.favourites)
        #expect(model.path(for: .favourites).isEmpty)
    }
}
