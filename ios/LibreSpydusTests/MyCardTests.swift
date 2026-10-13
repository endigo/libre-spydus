import Foundation
import LibreSpydusCore
import Testing
@testable import LibreSpydus

/// "My card" (docs/native-apps.md §5.9): where the card number is kept, the card sheet's state and
/// the symbology it draws with. No network: every backend is a fake.
@MainActor
struct MyCardTests {
    static let canning = LibraryInfo.canning.origin
    static let cumberland = LibraryInfo.placeholder(origin: "https://cumberlandlibraries.spydus.com")

    func makeModel(cards: InMemorySecretStore = InMemorySecretStore(), sessions: InMemorySecretStore = InMemorySecretStore(),
               demo: Bool = false, symbology: BarcodeSymbology = .code128) -> AppModel {
        var dependencies = AppModel.Dependencies.testing(sessions: sessions, demo: demo)
        dependencies.cardNumbers = cards
        dependencies.makeBackend = { origin in
            SymbologyBackend(origin: origin ?? Self.canning, symbology: symbology)
        }
        let model = AppModel(dependencies: dependencies)
        model.selectLibrary(.canning)
        return model
    }

    func signedIn(cards: InMemorySecretStore = InMemorySecretStore(), symbology: BarcodeSymbology = .code128) async throws -> AppModel {
        let model = makeModel(cards: cards, symbology: symbology)
        try await model.signIn(username: "jdoe", password: "ok")
        return model
    }

    // MARK: Card number storage

    @Test func signingInDoesNotSaveTheSignInID() async throws {
        // The sign-in ID may be a username, not the card's number (coordinator amendment to §5.9).
        let cards = InMemorySecretStore()
        let model = try await signedIn(cards: cards)
        #expect(model.isSignedIn)
        #expect(model.cardNumber == nil)
        #expect(cards.value(for: Self.canning) == nil)
    }

    @Test func enteringSavesTheNumberTrimmedPerLibrary() async throws {
        let cards = InMemorySecretStore()
        let model = try await signedIn(cards: cards)
        model.setCardNumber("  CL114D0001234 \n")
        #expect(model.cardNumber == "CL114D0001234")
        #expect(cards.value(for: Self.canning) == "CL114D0001234")

        model.selectLibrary(Self.cumberland)
        #expect(model.cardNumber == nil)
        model.setCardNumber("C0042")
        #expect(cards.value(for: Self.cumberland.origin) == "C0042")

        model.selectLibrary(.canning)
        #expect(model.cardNumber == "CL114D0001234")
    }

    @Test func theSavedNumberIsThereOnTheNextLaunch() async throws {
        let cards = InMemorySecretStore()
        let first = try await signedIn(cards: cards)
        first.setCardNumber("CL114D0001234")
        let second = makeModel(cards: cards)
        #expect(second.cardNumber == "CL114D0001234")
    }

    @Test func editReplacesTheNumber() async throws {
        let cards = InMemorySecretStore()
        let model = try await signedIn(cards: cards)
        model.setCardNumber("CL114D0001234")
        model.setCardNumber("CL114D0009999")
        #expect(model.cardNumber == "CL114D0009999")
        #expect(cards.value(for: Self.canning) == "CL114D0009999")
        model.setCardNumber("   ")
        #expect(model.cardNumber == nil)
        #expect(cards.value(for: Self.canning) == nil)
    }

    @Test func signOutDeletesThisLibrarysNumber() async throws {
        let cards = InMemorySecretStore([Self.cumberland.origin: "C0042"])
        let model = try await signedIn(cards: cards)
        model.setCardNumber("CL114D0001234")
        model.signOut()
        #expect(!model.isSignedIn)
        #expect(model.cardNumber == nil)
        #expect(cards.value(for: Self.canning) == nil)
        // Another library's number stays with it.
        #expect(cards.value(for: Self.cumberland.origin) == "C0042")
    }

    @Test func anExpiredSessionKeepsTheNumber() async throws {
        let cards = InMemorySecretStore()
        let model = try await signedIn(cards: cards)
        model.setCardNumber("CL114D0001234")
        model.report(SpydusError.NotLoggedIn(message: "not logged in"))
        #expect(!model.isSignedIn)
        #expect(model.rootSheet == .login)
        #expect(model.cardNumber == "CL114D0001234")
        #expect(cards.value(for: Self.canning) == "CL114D0001234")
    }

    @Test func theSavedNumberWinsOverTheAccountsOwn() {
        #expect(AppModel.cardNumber(saved: "CL114D0009999", account: "CL114D0001234") == "CL114D0009999")
        #expect(AppModel.cardNumber(saved: nil, account: " CL114D0001234 ") == "CL114D0001234")
        #expect(AppModel.cardNumber(saved: "  ", account: "CL114D0001234") == "CL114D0001234")
        #expect(AppModel.cardNumber(saved: nil, account: nil) == nil)
        // The account snapshot doesn't carry a card number yet.
        #expect(makeModel().accountCardNumber == nil)
    }

    @Test func demoModeHasACardNumberInMemory() {
        let cards = InMemorySecretStore()
        let model = makeModel(cards: cards, demo: true)
        #expect(model.isSignedIn)
        #expect(model.cardNumber == DemoBackend.cardNumber)
        #expect(DemoBackend.cardNumber == "DEMO0012345")
        model.setCardNumber("DEMO0099999")
        #expect(model.cardNumber == "DEMO0099999")
        // The app's demo stores are in memory; the real ones are the Keychain's.
        #expect(AppModel.Dependencies.live(demo: true).cardNumbers is InMemorySecretStore)
        #expect((AppModel.Dependencies.live(demo: false).cardNumbers as? KeychainSecretStore)?.service == "com.librespydus.card")
    }

    @Test func resetForgetsEveryCardNumber() {
        let cards = InMemorySecretStore([Self.canning: "CL114D0001234", Self.cumberland.origin: "C0042"])
        var dependencies = AppModel.Dependencies.testing()
        dependencies.cardNumbers = cards
        dependencies.resetPersistentState()
        #expect(cards.value(for: Self.canning) == nil)
        #expect(cards.value(for: Self.cumberland.origin) == nil)
    }

    // MARK: Symbology

    @Test func theSymbologyComesFromTheBackend() async throws {
        let model = try await signedIn(symbology: .code39)
        #expect(model.knownCardSymbology == nil)
        let card = MyCardModel(cardNumber: "cl114d0001234", knownSymbology: model.knownCardSymbology)
        #expect(card.barcode(for: "cl114d0001234") == .loading)

        await card.loadSymbology(model)
        #expect(card.symbology == .code39)
        #expect(model.knownCardSymbology == .code39)
        guard case .bars(let barcode) = card.barcode(for: "cl114d0001234") else {
            Issue.record("expected bars")
            return
        }
        #expect(barcode.symbology == .code39)
        #expect(barcode.text == "CL114D0001234")
        // Opened again: the remembered symbology shows at once, no spinner.
        let reopened = MyCardModel(cardNumber: "cl114d0001234", knownSymbology: model.knownCardSymbology)
        #expect(reopened.symbology == .code39)
    }

    @Test func theSymbologyIsRememberedPerLibrary() async throws {
        let model = try await signedIn(symbology: .codabar)
        _ = await model.loadCardSymbology()
        #expect(model.knownCardSymbology == .codabar)
        model.selectLibrary(Self.cumberland)
        #expect(model.knownCardSymbology == nil)
        model.selectLibrary(.canning)
        #expect(model.knownCardSymbology == .codabar)
    }

    @Test func backendsPassTheSymbologyThrough() async throws {
        // Demo mode asks the library itself (a public GET), like the real app.
        let demo = DemoBackend(publicBackend: SymbologyBackend(origin: Self.canning, symbology: .code39),
                               fixtures: try DemoFixtures.bundled())
        #expect(await demo.cardBarcodeSymbology() == .code39)
        // A backend that doesn't ask: Code 128, the core's default.
        #expect(await FakeBackend().cardBarcodeSymbology() == .code128)
    }

    @Test func barcodeAreaStates() {
        #expect(MyCardModel.barcode(for: "DEMO0012345", symbology: nil) == .loading)
        // Nothing can encode it, so there's no need to wait for the symbology.
        #expect(MyCardModel.barcode(for: "CL114Ä", symbology: nil) == .unencodable("CL114Ä"))
        #expect(MyCardModel.barcode(for: "CL114Ä", symbology: .code128) == .unencodable("CL114Ä"))
        guard case .bars(let code128) = MyCardModel.barcode(for: "DEMO0012345", symbology: .code128) else {
            Issue.record("expected bars")
            return
        }
        #expect(code128.symbology == .code128)
        #expect(code128.text == "DEMO0012345")
        // Code 39 has no "@": the number falls back to Code 128.
        guard case .bars(let fallback) = MyCardModel.barcode(for: "CL114@1", symbology: .code39) else {
            Issue.record("expected bars")
            return
        }
        #expect(fallback.symbology == .code128)
    }

    // MARK: Card sheet

    @Test func opensOnTheCardOrTheEntryForm() {
        let withNumber = MyCardModel(cardNumber: "DEMO0012345", knownSymbology: .code128)
        #expect(withNumber.mode == .card)
        let without = MyCardModel(cardNumber: nil, knownSymbology: nil)
        #expect(without.mode == .entry)
        #expect(without.draft.isEmpty)
        #expect(!without.isEditingExisting)
        let editing = MyCardModel(cardNumber: "DEMO0012345", knownSymbology: nil, editing: true)
        #expect(editing.mode == .entry)
        #expect(editing.draft == "DEMO0012345")
        #expect(editing.isEditingExisting)
    }

    @Test func saveIsDisabledWhileBlank() async throws {
        let model = try await signedIn()
        let card = MyCardModel(cardNumber: nil, knownSymbology: nil)
        #expect(!card.canSave)
        card.draft = "  \n"
        #expect(!card.canSave)
        #expect(!card.save(to: model))
        #expect(card.mode == .entry)
        #expect(model.cardNumber == nil)

        card.draft = " CL114D0001234 "
        #expect(card.canSave)
        #expect(card.save(to: model))
        #expect(card.mode == .card)
        #expect(model.cardNumber == "CL114D0001234")
    }

    @Test func editFillsTheFormAndCancelGoesBack() async throws {
        let model = try await signedIn()
        model.setCardNumber("CL114D0001234")
        let card = MyCardModel(cardNumber: model.cardNumber, knownSymbology: .code128)
        card.edit(current: model.cardNumber)
        #expect(card.mode == .entry)
        #expect(card.draft == "CL114D0001234")
        card.draft = "CL114D0009999"
        card.cancelEditing()
        #expect(card.mode == .card)
        #expect(model.cardNumber == "CL114D0001234")

        card.edit(current: model.cardNumber)
        card.draft = "CL114D0009999"
        card.save(to: model)
        #expect(model.cardNumber == "CL114D0009999")
        #expect(card.mode == .card)

        // Without a number there's no card to go back to.
        let first = MyCardModel(cardNumber: nil, knownSymbology: nil)
        first.cancelEditing()
        #expect(first.mode == .entry)
    }

    // MARK: Brightness

    @Test func brightnessGoesToFullAndBack() {
        var level: CGFloat = 0.4
        var writes: [CGFloat] = []
        let boost = BrightnessBoost(read: { level }, write: { level = $0; writes.append($0) })
        boost.restore()
        #expect(writes.isEmpty)
        boost.raise()
        #expect(level == 1)
        boost.raise()
        #expect(boost.saved == 0.4)
        boost.restore()
        #expect(level == 0.4)
        #expect(!boost.isRaised)
        boost.restore()
        #expect(writes == [1, 0.4])
    }
}

/// `FakeBackend` (canned public reads, sign-in with password "ok") reporting `symbology`.
final class SymbologyBackend: LibraryBackend, @unchecked Sendable {
    private let fake: FakeBackend
    let symbology: BarcodeSymbology
    var origin: String { fake.origin }
    var isDemo: Bool { false }

    init(origin: String, symbology: BarcodeSymbology) {
        fake = FakeBackend(origin: origin)
        self.symbology = symbology
    }

    func cardBarcodeSymbology() async -> BarcodeSymbology { symbology }

    func setVerificationCookie(_ cookie: String?) { fake.setVerificationCookie(cookie) }
    func search(query: String) async throws -> SearchResults { try await fake.search(query: query) }
    func searchSorted(query: String, sort: SearchSortOption) async throws -> SearchResults {
        try await fake.searchSorted(query: query, sort: sort)
    }
    func page(url: String) async throws -> SearchResults { try await fake.page(url: url) }
    func availability(url: String) async throws -> Availability { try await fake.availability(url: url) }
    func record(url: String) async throws -> RecordDetail { try await fake.record(url: url) }
    func related(url: String) async throws -> RelatedList { try await fake.related(url: url) }
    func login(username: String, password: String) async throws -> String {
        try await fake.login(username: username, password: password)
    }
    func account(session: String) async throws -> AccountLinks { try await fake.account(session: session) }
    func loans(session: String) async throws -> PatronList { try await fake.loans(session: session) }
    func reservations(session: String) async throws -> Reservations { try await fake.reservations(session: session) }
    func accountSnapshot(session: String) async throws -> AccountSnapshot { try await fake.accountSnapshot(session: session) }
    func reserveOptions(url: String, session: String) async throws -> ReserveOptions {
        try await fake.reserveOptions(url: url, session: session)
    }
    func placeReservation(url: String, session: String, xdays: String?, pickup: String?) async throws -> ReserveResult {
        try await fake.placeReservation(url: url, session: session, xdays: xdays, pickup: pickup)
    }
    func cancelReservation(url: String, session: String) async throws -> ReserveResult {
        try await fake.cancelReservation(url: url, session: session)
    }
    func editOptions(url: String, session: String) async throws -> EditOptions {
        try await fake.editOptions(url: url, session: session)
    }
    func editReservation(url: String, session: String, overrides: EditOverrides) async throws -> ReserveResult {
        try await fake.editReservation(url: url, session: session, overrides: overrides)
    }
}
