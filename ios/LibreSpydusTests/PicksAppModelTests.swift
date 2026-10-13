import Foundation
import LibreSpydusCore
import Testing
@testable import LibreSpydus

/// My Picks through `AppModel` (§5.11): the store follows sign-in, sign-out and library changes,
/// Favourites switches lists, and the copy-once offer after an interactive sign-in.
@MainActor
struct PicksAppModelTests {
    static let hutt = LibraryInfo(id: "hutt", name: "Hutt City", origin: "https://hutt.spydus.co.nz", logoUrl: nil,
                                  country: "New Zealand", region: "NZ", isTest: false)

    /// A model whose backends are `PicksFake`s, one per origin (`fakes`).
    @MainActor
    final class Harness {
        let defaults = PicksTestData.defaults()
        let sessions: InMemorySecretStore
        let directory = TestData.temporaryDirectory()
        var fakes: [String: PicksFake] = [:]
        var irns: [String: [String]]

        init(sessions: [String: String] = [:], irns: [String: [String]] = [:]) {
            self.sessions = InMemorySecretStore(sessions)
            self.irns = irns
        }

        func makeModel() -> AppModel {
            AppModel(dependencies: AppModel.Dependencies(
                defaults: defaults, sessions: sessions, verificationCookies: InMemorySecretStore(),
                favouritesDirectory: directory, isDemo: false, accountCacheDirectory: nil,
                makeBackend: { [unowned self] origin in
                    let origin = origin ?? LibraryInfo.canning.origin
                    if let fake = fakes[origin] { return fake }
                    let fake = PicksFake(origin: origin, irns: irns[origin] ?? [])
                    fakes[origin] = fake
                    return fake
                }
            ))
        }
    }

    private static func favourite(_ id: String, _ title: String, url: String? = nil) -> BookRef {
        BookRef(id: id, title: title, url: url)
    }

    // MARK: Following the account

    @Test func signInPointsThePicksAtTheAccountAndSignOutClearsThem() async throws {
        let harness = Harness(irns: [LibraryInfo.canning.origin: ["1", "2"]])
        let model = harness.makeModel()
        model.selectLibrary(.canning)
        #expect(!model.picks.isActive)
        #expect(FavouritesView.Content(model) == .local)

        try await model.signIn(username: "card", password: "ok")
        #expect(model.picks.isActive)
        #expect(FavouritesView.Content(model) == .myPicks)
        await model.picks.loadIfNeeded()
        await model.picks.settle()
        #expect(model.picks.irns == ["1", "2"])

        model.signOut()
        #expect(!model.picks.isActive)
        #expect(model.picks.irns.isEmpty)
        #expect(model.picks.items.isEmpty)
        #expect(FavouritesView.Content(model) == .local)
    }

    @Test func aRestoredSessionLoadsThePicksAndALibraryChangeClearsThem() async {
        let harness = Harness(sessions: [LibraryInfo.canning.origin: PicksFake.session],
                              irns: [LibraryInfo.canning.origin: ["5"]])
        let model = harness.makeModel()
        model.selectLibrary(.canning)
        #expect(model.isSignedIn)
        await model.picks.loadIfNeeded()
        #expect(model.picks.contains("5"))

        model.selectLibrary(Self.hutt)
        #expect(!model.isSignedIn)
        #expect(!model.picks.isActive)
        #expect(model.picks.irns.isEmpty)
        #expect(model.picks.phase == .idle)
    }

    @Test func anExpiredSessionWhileSavingGoesToSignIn() async {
        let harness = Harness(sessions: [LibraryInfo.canning.origin: PicksFake.session])
        let model = harness.makeModel()
        model.selectLibrary(.canning)
        await model.picks.loadIfNeeded()
        let fake = harness.fakes[LibraryInfo.canning.origin]!
        await fake.failChanges(with: .NotLoggedIn(message: "not logged in"))

        #expect(await model.picks.setPicked(true, irn: "8") == false)
        #expect(!model.isSignedIn)
        #expect(model.loginNeeded)
        #expect(model.rootSheet == .login)
        #expect(!model.picks.isActive)
        #expect(!model.picks.contains("8"))
        // The login sheet says it; no "Couldn't update My Picks." on top.
        #expect(model.picks.notice == nil)
    }

    // MARK: Copy-once offer

    @Test func interactiveSignInWithLocalFavouritesOffersTheCopyOnce() async throws {
        let harness = Harness()
        let model = harness.makeModel()
        model.selectLibrary(.canning)
        model.favourites.add(Self.favourite("work:1", "First"), now: Date(timeIntervalSince1970: 1))
        model.favourites.add(Self.favourite("bib:2", "Second"), now: Date(timeIntervalSince1970: 2))

        try await model.signIn(username: "card", password: "ok")
        #expect(model.picksCopyOffer == PicksCopyOffer(origin: LibraryInfo.canning.origin, count: 2))

        await model.answerPicksCopyOffer(add: false)
        #expect(model.picksCopyOffer == nil)
        #expect(model.picks.wasOffered(origin: LibraryInfo.canning.origin))
        let fake = harness.fakes[LibraryInfo.canning.origin]!
        #expect(await fake.calls.filter { $0.hasPrefix("save:") }.isEmpty)

        // Never asked twice for this library, even after signing out and in again.
        model.signOut()
        try await model.signIn(username: "card", password: "ok")
        #expect(model.picksCopyOffer == nil)

        // A model started again (the flag is saved).
        let again = harness.makeModel()
        again.signOut()
        try await again.signIn(username: "card", password: "ok")
        #expect(again.picksCopyOffer == nil)
    }

    @Test func noOfferForARestoredSessionOrWithoutLocalFavourites() async throws {
        let harness = Harness(sessions: [LibraryInfo.canning.origin: PicksFake.session])
        let model = harness.makeModel()
        model.selectLibrary(.canning)
        model.favourites.add(Self.favourite("work:1", "First"))
        #expect(model.isSignedIn)
        #expect(model.picksCopyOffer == nil)

        // Signed in at another library without local favourites: nothing to offer.
        model.selectLibrary(Self.hutt)
        try await model.signIn(username: "card", password: "ok")
        #expect(model.picksCopyOffer == nil)
        #expect(!model.picks.wasOffered(origin: Self.hutt.origin))
    }

    @Test func theOfferIsPerLibrary() async throws {
        let harness = Harness()
        let model = harness.makeModel()
        model.selectLibrary(.canning)
        model.favourites.add(Self.favourite("work:1", "First"))
        try await model.signIn(username: "card", password: "ok")
        await model.answerPicksCopyOffer(add: false)

        model.selectLibrary(Self.hutt)
        model.favourites.add(Self.favourite("work:7", "Kiwi"))
        try await model.signIn(username: "card", password: "ok")
        #expect(model.picksCopyOffer == PicksCopyOffer(origin: Self.hutt.origin, count: 1))

        // Switching library drops an unanswered offer.
        model.selectLibrary(.canning)
        #expect(model.picksCopyOffer == nil)
    }

    @Test func addCopiesOldestFirstAndKeepsTheLocalList() async throws {
        let harness = Harness(irns: [LibraryInfo.canning.origin: ["9"]])
        let model = harness.makeModel()
        model.selectLibrary(.canning)
        let relatedURL = "https://canning.spydus.com/related"
        model.favourites.add(Self.favourite("work:1", "Oldest"), now: Date(timeIntervalSince1970: 1))
        model.favourites.add(Self.favourite("brn:30", "Related", url: relatedURL), now: Date(timeIntervalSince1970: 2))
        model.favourites.add(Self.favourite("bib:2", "Newest"), now: Date(timeIntervalSince1970: 3))
        let fake = harness.fakes[LibraryInfo.canning.origin]!
        await fake.setRecord(PicksTestData.record(bibIrn: "3"), for: relatedURL)

        try await model.signIn(username: "card", password: "ok")
        #expect(model.picksCopyOffer?.count == 3)
        await model.answerPicksCopyOffer(add: true)

        #expect(await fake.calls.filter { $0.hasPrefix("save:") } == ["save:1", "save:3", "save:2"])
        #expect(await fake.irns == ["9", "1", "3", "2"])
        #expect(model.picks.notice?.message == "Added 3 books to My Picks")
        #expect(model.picks.notice?.tone == .success)
        #expect(model.picks.irns == ["9", "1", "3", "2"])
        #expect(model.favourites.items.map(\.id) == ["bib:2", "brn:30", "work:1"])
        #expect(model.picks.wasOffered(origin: LibraryInfo.canning.origin))
    }

    @Test func resetForgetsTheOffers() async throws {
        let harness = Harness()
        let model = harness.makeModel()
        model.selectLibrary(.canning)
        model.favourites.add(Self.favourite("work:1", "First"))
        try await model.signIn(username: "card", password: "ok")
        await model.answerPicksCopyOffer(add: false)
        #expect(model.picks.wasOffered(origin: LibraryInfo.canning.origin))

        AppModel.Dependencies(defaults: harness.defaults, sessions: harness.sessions,
                              verificationCookies: InMemorySecretStore(), favouritesDirectory: harness.directory,
                              isDemo: false, makeBackend: { _ in FakeBackend() }).resetPersistentState()
        #expect(!model.picks.wasOffered(origin: LibraryInfo.canning.origin))
    }
}
