import Foundation
import LibreSpydusCore
import Observation
import UIKit

/// App-wide state (docs/native-apps.md §5.2), injected with `.environment(model)`;
/// read it in a view with `@Environment(AppModel.self) private var model`.
@MainActor
@Observable
final class AppModel {
    // MARK: Library

    /// The selected library, persisted. Nil → onboarding (library picker).
    private(set) var library: LibraryInfo?
    /// One backend per selected origin, rebuilt when the library changes.
    private(set) var backend: any LibraryBackend

    // MARK: Account

    /// Session token for the current origin (Keychain; in memory in demo mode).
    private(set) var session: String?
    /// The signed-in account (name, loans, reservations), saved on the device per origin and
    /// refreshed in the background (§5.2a). Screens show `account.snapshot`.
    let account: AccountStore
    /// The borrower's name from the account snapshot.
    var accountName: String? { account.snapshot?.accountName }
    var isSignedIn: Bool { session != nil }

    // MARK: Library card (§5.9)

    /// The library card number for the selected library: the one entered on the card sheet, else
    /// the account's own. Nil: the card sheet asks for it.
    var cardNumber: String? { Self.cardNumber(saved: savedCardNumber, account: accountCardNumber) }
    /// Entered or edited on the card sheet; Keychain per origin (in memory in demo mode). Deleted on
    /// sign-out, kept when the session expires.
    private(set) var savedCardNumber: String?
    /// The card number the library account itself lists: the core reads it from the account's
    /// Personal details page into the snapshot.
    var accountCardNumber: String? { account.snapshot?.cardNumber }
    /// When the card expires ("12 Mar 2027", as the Personal details page writes it), while the card
    /// shown is the account's own rather than a different number entered on the card.
    var cardExpires: String? {
        guard let expires = account.snapshot?.cardExpires?.trimmingCharacters(in: .whitespacesAndNewlines),
              !expires.isEmpty, cardNumber == Self.cardNumber(saved: nil, account: accountCardNumber) else { return nil }
        return expires
    }

    // MARK: Local data

    let favourites: FavouritesStore
    /// My Picks (§5.11): the Pick list on the library account, which the My Picks tab shows (and the
    /// bookmark saves to) while signed in.
    let picks: PicksStore
    /// After an interactive sign-in: offer once to add this library's local favourites to My Picks
    /// (`picksCopyOfferAlert`). Nil when there's nothing to offer.
    private(set) var picksCopyOffer: PicksCopyOffer?
    /// "Ready to collect" alerts and due-date reminders (§5.8): the Profile switches and the
    /// processing of every fetched snapshot.
    let alerts: AlertCenter
    /// Sign back in with Face ID (§5.14): the kept details, the offer, a sign-in with the check.
    let biometrics: BiometricSignIn

    // MARK: Navigation

    var selectedTab: AppTab = .home
    /// Each tab's `NavigationStack` path.
    var paths: [AppTab: [Route]] = [:]
    /// Query typed on Home, picked up by the Search tab (`takePendingSearch()`).
    var pendingSearch: String?
    /// The Your Books segment to show (a book page's "On loan" / "Reserved" pill), picked up by
    /// that tab (`takePendingBorrowedSegment()`).
    private(set) var pendingBorrowedSegment: BorrowedViewModel.Segment?

    // MARK: Root sheets (RootView presents `rootSheet`)

    var loginNeeded = false
    var verificationNeeded = false
    var libraryPickerPresented = false
    /// A screen's own sheet is up (Your Books' edit sheet). Two sheets can't be up at once, so a
    /// root sheet asked for meanwhile (a background refresh found the session expired) waits
    /// until it has gone.
    var isPresentingFeatureSheet = false
    #if DEBUG
    /// `-LSRoute gallery`: the DesignSystem components gallery.
    var componentsGalleryPresented = false
    #endif

    // MARK: Reload triggers

    /// Bumped when verification completes or is cleared: screens re-run their loads.
    private(set) var verificationGeneration = 0
    /// Bumped on sign-in, sign-out, session loss and library change.
    private(set) var accountGeneration = 0
    /// The current origin's verification cookie (`aws-waf-token=…`), if verified on this device.
    private(set) var verificationCookie: String?

    let isDemo: Bool

    @ObservationIgnored private let dependencies: Dependencies
    /// Bumped whenever `backend` is replaced, so late results for an old library are dropped.
    @ObservationIgnored private var backendGeneration = 0
    @ObservationIgnored private var verificationCompletions: [String: Int] = [:]
    /// Each library's card barcode symbology once read, so the card shows at once after the first open.
    @ObservationIgnored private var cardSymbologies: [String: BarcodeSymbology] = [:]

    init(dependencies: Dependencies) {
        self.dependencies = dependencies
        isDemo = dependencies.isDemo
        let stored = Self.loadLibrary(from: dependencies.defaults)
        library = stored
        backend = dependencies.makeBackend(stored?.origin)
        favourites = FavouritesStore(directory: dependencies.favouritesDirectory)
        // Demo mode never saves the account, unless a DEBUG launch flag asks to (screenshots).
        let savesAccount = !dependencies.isDemo || dependencies.persistsDemoAccount
        account = AccountStore(cache: savesAccount ? dependencies.accountCacheDirectory.map(AccountSnapshotCache.init) : nil)
        // Demo mode keeps its own switches and keys, so it never changes the real ones.
        let alerts = AlertCenter(defaults: dependencies.defaults, system: dependencies.alertSystem,
                                 namespace: dependencies.isDemo ? AlertCenter.demoNamespace : AlertCenter.namespace)
        self.alerts = alerts
        // Demo mode keeps its own "offered" flags, so it never changes the real ones.
        picks = PicksStore(defaults: dependencies.defaults,
                           namespace: dependencies.isDemo ? PicksStore.demoNamespace : PicksStore.namespace)
        // Demo mode keeps its own "Not now" answers too.
        biometrics = BiometricSignIn(vault: dependencies.biometricVault, defaults: dependencies.defaults,
                                     namespace: dependencies.isDemo ? BiometricSignIn.demoNamespace : BiometricSignIn.namespace)
        account.report = { [weak self] error in self?.report(error) }
        picks.report = { [weak self] error in self?.report(error) }
        account.didFetch = { snapshot, origin in await alerts.process(snapshot, origin: origin) }
        installSecrets()
    }

    // MARK: Derived

    /// The library's display name (`Copy.Common.yourLibrary` before one is chosen).
    var libraryName: String { library?.name ?? Copy.Common.yourLibrary }
    var isVerified: Bool { verificationCookie != nil }
    var needsOnboarding: Bool { library == nil }

    /// `.task(id: model.reloadKey)`: re-runs a screen's load on library change, verification
    /// and sign-in/out.
    var reloadKey: ReloadKey {
        ReloadKey(origin: backend.origin, verification: verificationGeneration, account: accountGeneration)
    }

    struct ReloadKey: Hashable, Sendable {
        var origin: String
        var verification: Int
        var account: Int
    }

    /// The sheet RootView shows; verification wins over login over the library picker.
    var rootSheet: RootSheet? {
        guard library != nil, !isPresentingFeatureSheet else { return nil }
        if verificationNeeded { return .verification }
        if loginNeeded { return .login }
        if libraryPickerPresented { return .libraryPicker }
        return nil
    }

    func dismiss(_ sheet: RootSheet) {
        switch sheet {
        case .verification: verificationNeeded = false
        case .login:
            loginNeeded = false
            biometrics.sheetClosed()
        case .libraryPicker: libraryPickerPresented = false
        }
    }

    // MARK: Library selection

    func selectLibrary(_ info: LibraryInfo) {
        libraryPickerPresented = false
        let changed = library?.origin != info.origin
        library = info
        Self.saveLibrary(info, to: dependencies.defaults)
        if changed { install(dependencies.makeBackend(info.origin)) }
    }

    /// Selects a library by origin alone (`-LSLibraryOrigin`); name and region are guessed from
    /// the host until `refreshLibraryInfo()` finds the directory entry.
    func selectLibrary(origin: String) {
        let newBackend = dependencies.makeBackend(origin)
        guard library?.origin != newBackend.origin else { return }
        let info = LibraryInfo.placeholder(origin: newBackend.origin)
        libraryPickerPresented = false
        library = info
        Self.saveLibrary(info, to: dependencies.defaults)
        install(newBackend)
    }

    /// Replaces a placeholder `library` with its Civica directory entry (one public GET).
    func refreshLibraryInfo() async {
        guard let current = library, current.isPlaceholder else { return }
        guard let libraries = try? await listLibraries(includeTest: true),
              let match = libraries.first(where: { $0.origin == current.origin }),
              library?.origin == match.origin else { return }
        library = match
        Self.saveLibrary(match, to: dependencies.defaults)
    }

    // MARK: Account

    /// Logs in and stores the session for this origin. Throws `SpydusError` (show it with
    /// `UserFacingError(error, libraryName:)`; `report(_:)` for verification). `typed`: the sign-in
    /// form's details, which may be kept for Face ID and are followed by the offers; a sign-in with
    /// the check (§5.14) offers nothing.
    func signIn(username: String, password: String, typed: Bool = true) async throws {
        let generation = backendGeneration
        let backend = self.backend
        let token = try await backend.login(username: username, password: password)
        guard generation == backendGeneration else { throw CancellationError() }
        session = token
        dependencies.sessions.setValue(token, for: backend.origin)
        loginNeeded = false
        accountGeneration += 1
        // A sign-in with Face ID is the account on screen (a typed one with another username deletes
        // its details), so its saved data stays until the fresh one.
        account.use(origin: backend.origin, session: token, backend: backend, sameAccount: !typed)
        picks.use(origin: backend.origin, session: token, backend: backend)
        if typed {
            biometrics.didSignInByTyping(SignInDetails(username: username, pin: password), origin: backend.origin)
            offerPicksCopy()
        }
        rescheduleMorningCheck()
        await account.refresh(force: true)
    }

    /// The reader answered the copy-once offer: remembered for this library either way. Add saves
    /// the library's local favourites to My Picks, oldest first, and keeps them on the device.
    func answerPicksCopyOffer(add: Bool) async {
        guard let offer = picksCopyOffer else { return }
        picksCopyOffer = nil
        picks.markOffered(origin: offer.origin)
        guard add, isSignedIn, offer.origin == backend.origin, favourites.origin == offer.origin else { return }
        await picks.copy(Array(favourites.items.reversed()))
    }

    /// After an interactive sign-in (never a restored session): this library's local favourites,
    /// if any, can go to My Picks, once.
    private func offerPicksCopy() {
        let origin = backend.origin
        guard favourites.origin == origin, !favourites.items.isEmpty, !picks.wasOffered(origin: origin) else { return }
        picksCopyOffer = PicksCopyOffer(origin: origin, count: favourites.items.count)
    }

    /// A hold was placed (or the account otherwise changed outside Your Books): the saved account
    /// is fetched again, so Your Books and Home's shelf show it.
    func accountDidChange() {
        account.libraryDidChange()
    }

    func signOut() {
        // The card number and Face ID's details go with the account; an expired session (`report`)
        // keeps them.
        dependencies.cardNumbers.setValue(nil, for: backend.origin)
        savedCardNumber = nil
        biometrics.forget(origin: backend.origin)
        clearSession()
    }

    /// Saves the number entered on the card sheet for this library (trimmed); a blank one is
    /// deleted. It wins over the account's own number.
    func setCardNumber(_ number: String?) {
        let trimmed = number?.trimmingCharacters(in: .whitespacesAndNewlines)
        let value = trimmed?.isEmpty == false ? trimmed : nil
        dependencies.cardNumbers.setValue(value, for: backend.origin)
        savedCardNumber = value
    }

    /// The selected library's card barcode symbology if it has been read before, else nil.
    var knownCardSymbology: BarcodeSymbology? { cardSymbologies[backend.origin] }

    /// Asks the library which symbology its card scanners read (one public GET, cached in the
    /// core) and remembers it for `knownCardSymbology`.
    func loadCardSymbology() async -> BarcodeSymbology {
        let backend = self.backend
        let symbology = await backend.cardBarcodeSymbology()
        cardSymbologies[backend.origin] = symbology
        return symbology
    }

    /// `saved` (entered on the card) wins over `account` (the library's own record); blank is none.
    nonisolated static func cardNumber(saved: String?, account: String?) -> String? {
        let candidates = [saved, account].compactMap { $0?.trimmingCharacters(in: .whitespacesAndNewlines) }
        return candidates.first { !$0.isEmpty }
    }

    #if DEBUG
    /// Profile › Developer › Expire session (§5.14): an invalid session in place of the real one,
    /// then a refresh, so the library (or `DemoBackend`) answers `NotLoggedIn`.
    func expireSessionForDebugging() async {
        guard session != nil else { return }
        let expired = "expired-session"
        session = expired
        dependencies.sessions.setValue(expired, for: backend.origin)
        account.use(origin: backend.origin, session: expired, backend: backend, sameAccount: true)
        picks.use(origin: backend.origin, session: expired, backend: backend)
        await account.refresh(force: true)
    }
    #endif

    /// Fetches the account snapshot (name, loans, reservations) now, or only when it's out of
    /// date unless `force`; a lost session clears itself through `report`. Returns false only
    /// when the request failed; a library page without a name is a success with
    /// `accountName == nil`.
    @discardableResult
    func refreshAccount(force: Bool = true) async -> Bool {
        await account.refresh(force: force)
        return account.lastRefreshError == nil
    }

    // MARK: Errors

    /// Routes a failed call (docs/native-apps.md §5.7) and returns what to show, or nil for a
    /// cancellation. `HumanVerificationRequired` presents the verification sheet; `NotLoggedIn`
    /// signs back in with Face ID when the library has details kept (§5.14), else clears the
    /// session and presents login.
    @discardableResult
    func report(_ error: any Error) -> UserFacingError? {
        if error is CancellationError { return nil }
        if let urlError = error as? URLError, urlError.code == .cancelled { return nil }
        let userError = UserFacingError(error, libraryName: libraryName)
        switch error as? SpydusError {
        case .HumanVerificationRequired:
            // README "Human verification": after two completed verifications that the library
            // still rejects, stop asking and point to the website instead.
            if verificationCompletions[backend.origin, default: 0] >= 2 {
                return .verificationUnavailable(libraryName: libraryName)
            }
            verificationNeeded = true
        case .NotLoggedIn:
            if signBackInWithBiometrics() { break }
            clearSession()
            loginNeeded = true
        default:
            break
        }
        return userError
    }

    // MARK: Verification (AWS WAF)

    /// The user finished the check in the verification web view: store and apply
    /// `"aws-waf-token=<value>"` for this origin, then let screens retry.
    func completeVerification(cookie: String) {
        let origin = backend.origin
        dependencies.verificationCookies.setValue(cookie, for: origin)
        verificationCookie = cookie
        backend.setVerificationCookie(cookie)
        verificationCompletions[origin, default: 0] += 1
        verificationNeeded = false
        verificationGeneration += 1
        // A refresh the challenge stopped tries again.
        Task { await account.refresh() }
    }

    func clearVerification() {
        let origin = backend.origin
        dependencies.verificationCookies.setValue(nil, for: origin)
        verificationCookie = nil
        backend.setVerificationCookie(nil)
        verificationCompletions[origin] = nil
        verificationGeneration += 1
    }

    /// User-Agent for the verification web view: the WAF token may be tied to it, so it is the
    /// core's own (`userAgent()`).
    static let webViewUserAgent = LibreSpydusCore.userAgent()

    // MARK: Navigation

    func path(for tab: AppTab) -> [Route] {
        paths[tab] ?? []
    }

    func setPath(_ path: [Route], for tab: AppTab) {
        paths[tab] = path
    }

    /// Pushes `route` on `tab`'s stack (default: the current tab) and shows that tab.
    func push(_ route: Route, on tab: AppTab? = nil) {
        let tab = tab ?? selectedTab
        paths[tab, default: []].append(route)
        selectedTab = tab
    }

    /// Tab bar tap: switch tabs, or pop to root when the tab is already selected.
    func select(_ tab: AppTab) {
        if tab == selectedTab {
            paths[tab] = []
        } else {
            selectedTab = tab
        }
    }

    /// Home's search field: switch to the Search tab, which focuses and runs `query`.
    func openSearch(query: String? = nil) {
        pendingSearch = query ?? ""
        paths[.search] = []
        selectedTab = .search
    }

    /// For the Search tab: the query Home handed over (nil if none), cleared once taken.
    func takePendingSearch() -> String? {
        defer { pendingSearch = nil }
        return pendingSearch
    }

    /// A book page's "On loan" / "Reserved" pill: Your Books, back at its list, on `segment`.
    func openYourBooks(on segment: BorrowedViewModel.Segment) {
        pendingBorrowedSegment = segment
        paths[.borrowed] = []
        selectedTab = .borrowed
    }

    /// For Your Books: the segment a book page asked for (nil if none), cleared once taken.
    func takePendingBorrowedSegment() -> BorrowedViewModel.Segment? {
        defer { pendingBorrowedSegment = nil }
        return pendingBorrowedSegment
    }

    // MARK: Private

    private func install(_ newBackend: any LibraryBackend) {
        backendGeneration += 1
        backend = newBackend
        paths = [:]
        pendingSearch = nil
        pendingBorrowedSegment = nil
        picksCopyOffer = nil
        loginNeeded = false
        verificationNeeded = false
        installSecrets()
        accountGeneration += 1
        rescheduleMorningCheck()
        // RootView's `.task(id: backend.origin)` calls refreshAccount() for the new origin.
    }

    /// Loads this origin's verification cookie and session and re-applies the cookie to the
    /// new client (§5.2).
    private func installSecrets() {
        let origin = backend.origin
        verificationCookie = dependencies.verificationCookies.value(for: origin)
        if let verificationCookie { backend.setVerificationCookie(verificationCookie) }
        if library == nil {
            session = nil
            savedCardNumber = nil
        } else if isDemo {
            // Demo mode starts signed in as the fixture's "Test Borrower", with its card.
            session = dependencies.sessions.value(for: origin) ?? DemoBackend.session
            // Its card number comes with the demo account's snapshot, like a real account's.
            savedCardNumber = dependencies.cardNumbers.value(for: origin)
        } else {
            session = dependencies.sessions.value(for: origin)
            savedCardNumber = dependencies.cardNumbers.value(for: origin)
        }
        account.use(origin: library == nil ? nil : origin, session: session, backend: backend)
        picks.use(origin: library == nil ? nil : origin, session: session, backend: backend)
        favourites.load(origin: library == nil ? nil : origin)
        biometrics.use(origin: library == nil ? nil : origin)
    }

    /// Whether the app is in the foreground, where the Face ID check can run.
    func isAppActive() -> Bool {
        dependencies.isAppActive()
    }

    /// Signs out of this library without forgetting its card number or Face ID's details (an
    /// ended session; `signOut` forgets them).
    func clearSession() {
        dependencies.sessions.setValue(nil, for: backend.origin)
        session = nil
        account.clear(origin: backend.origin)
        picks.use(origin: nil, session: nil, backend: backend)
        picksCopyOffer = nil
        alerts.forget(origin: backend.origin)
        accountGeneration += 1
        rescheduleMorningCheck()
    }

    nonisolated private static let libraryKey = "LibreSpydus.selectedLibrary"

    private static func loadLibrary(from defaults: UserDefaults) -> LibraryInfo? {
        guard let data = defaults.data(forKey: libraryKey),
              let stored = try? JSONDecoder().decode(StoredLibrary.self, from: data) else { return nil }
        return stored.info
    }

    private static func saveLibrary(_ info: LibraryInfo, to defaults: UserDefaults) {
        if let data = try? JSONEncoder().encode(StoredLibrary(info)) {
            defaults.set(data, forKey: libraryKey)
        }
    }
}

// MARK: - Dependencies

extension AppModel {
    /// Everything AppModel persists to or creates; tests pass in-memory stores and temp dirs.
    struct Dependencies {
        var defaults: UserDefaults
        var sessions: any SecretStore
        var verificationCookies: any SecretStore
        var favouritesDirectory: URL
        var isDemo: Bool
        /// Where the account is saved per origin (`AccountSnapshotCache`); nil: in memory only.
        var accountCacheDirectory: URL?
        /// DEBUG `-LSPersistDemoAccount`: demo mode saves its account too (otherwise it never does).
        var persistsDemoAccount = false
        /// Where the core saves pages (§5.13; `CoreBackend.savedPagesDirectory`), so `-LSReset`
        /// can delete them; nil in tests.
        var savedPagesDirectory: URL?
        var makeBackend: @MainActor (_ origin: String?) -> any LibraryBackend
        /// The notification centre, background task scheduler and clock the alerts use; nothing
        /// reaches the system unless set (tests).
        var alertSystem: AlertSystem = .inert
        /// Library card numbers entered on the card sheet, per origin (§5.9).
        var cardNumbers: any SecretStore = InMemorySecretStore()
        /// Face ID's sign-in details per origin (§5.14); none by default (tests pass a fake).
        var biometricVault: any BiometricVault = NoBiometricVault()
        /// Whether the app is in the foreground (the Face ID check can't run in the background).
        var isAppActive: @MainActor () -> Bool = { true }

        /// The app's real stores. Demo mode keeps its session and card number in memory (never the
        /// Keychain), and its account too unless `persistDemoAccount` (then in a folder of its own).
        /// `demoNewReady` (DEBUG `-LSDemoNewReady`): the demo account has one more reservation ready
        /// from its second refresh on. `alertTime` (DEBUG `-LSAlertTime`) replaces 07:45. `demoKeychain`
        /// (DEBUG `-LSDemoKeychain`): demo mode keeps Face ID's details in the Keychain, under its own
        /// service, instead of memory.
        static func live(demo: Bool, persistDemoAccount: Bool = false, demoRefreshFails: Bool = false,
                         demoNewReady: Bool = false, demoKeychain: Bool = false,
                         alertTime: DateComponents = AlertContent.alertTime) -> Dependencies {
            Dependencies(
                defaults: .standard,
                sessions: demo ? InMemorySecretStore() : KeychainSecretStore(service: "com.librespydus.session"),
                verificationCookies: KeychainSecretStore(service: "com.librespydus.verification"),
                favouritesDirectory: FavouritesStore.defaultDirectory,
                isDemo: demo,
                accountCacheDirectory: demo
                    ? AccountSnapshotCache.defaultDirectory.appending(path: "Demo", directoryHint: .isDirectory)
                    : AccountSnapshotCache.defaultDirectory,
                persistsDemoAccount: demo && persistDemoAccount,
                savedPagesDirectory: CoreBackend.savedPagesDirectory,
                makeBackend: { origin -> any LibraryBackend in
                    demo
                        ? DemoBackend.live(origin: origin, failsAccountSnapshot: demoRefreshFails, addsReadyOnNextRefresh: demoNewReady)
                        : CoreBackend(origin: origin)
                },
                alertSystem: .live(alertTime: alertTime),
                cardNumbers: demo ? InMemorySecretStore() : KeychainSecretStore(service: "com.librespydus.card"),
                biometricVault: demo && !demoKeychain ? MemoryBiometricVault() as any BiometricVault
                    : KeychainBiometricVault(service: demo ? "com.librespydus.signin.demo" : "com.librespydus.signin"),
                isAppActive: { UIApplication.shared.applicationState == .active }
            )
        }

        /// `-LSReset`: forget the library, every session, verification, card number, favourite,
        /// recent search, saved account, saved page, alert setting, My Picks copy offer and Face ID
        /// sign-in.
        func resetPersistentState() {
            defaults.removeObject(forKey: AppModel.libraryKey)
            defaults.removeObject(forKey: "LibreSpydus.recentSearches") // RecentSearchStore.defaultsKey
            sessions.removeAll()
            verificationCookies.removeAll()
            cardNumbers.removeAll()
            FavouritesStore.deleteAll(in: favouritesDirectory)
            if let accountCacheDirectory { AccountSnapshotCache.deleteAll(in: accountCacheDirectory) }
            if let savedPagesDirectory { try? FileManager.default.removeItem(at: savedPagesDirectory) }
            AlertCenter.deleteAll(in: defaults)
            PicksStore.deleteAll(in: defaults)
            BiometricSignIn.deleteAll(vault: biometricVault, defaults: defaults)
        }
    }
}

// MARK: - LibraryInfo helpers

extension LibraryInfo {
    /// Canning, the core's default library: its entry in Civica's directory (logo included), so
    /// the app shows it correctly before or without a directory fetch.
    static let canning = LibraryInfo(
        // copy-exempt: Canning's entry in Civica's directory (data, as the directory gives it)
        id: "24fdee8d-ae0f-41b3-b97e-c96059bd06f4", name: "City of Canning",
        origin: "https://canning.spydus.com",
        logoUrl: "https://canning.spydus.com/api/maintenance/1.0/imagebrowser/image?blobName=d226b0c4-06d6-44b7-ba98-e42f06b9fd15.png",
        country: "Australia", region: "AU", isTest: false // copy-exempt: directory data (Canning's country)
    )

    /// A stand-in built from the origin alone, until the directory entry is known.
    static func placeholder(origin: String) -> LibraryInfo {
        if origin == canning.origin { return canning }
        let host = URL(string: origin)?.host() ?? origin
        let label = host.split(separator: ".").first.map(String.init) ?? host
        let region: String
        let country: String?
        switch true {
        case host.hasSuffix(".uk"): (region, country) = ("UK", Copy.Onboarding.regionUK)
        case host.hasSuffix(".nz"): (region, country) = ("NZ", Copy.Onboarding.regionNZ)
        case host.hasSuffix(".sg"): (region, country) = ("SG", Copy.Onboarding.regionSG)
        case host.hasSuffix(".au"): (region, country) = ("AU", Copy.Onboarding.regionAU)
        default: (region, country) = ("OTHER", nil)
        }
        return LibraryInfo(id: "placeholder:\(origin)", name: label.prefix(1).uppercased() + label.dropFirst(),
                           origin: origin, logoUrl: nil, country: country, region: region, isTest: false)
    }

    var isPlaceholder: Bool { id.hasPrefix("placeholder:") }
}

/// `LibraryInfo` as persisted in UserDefaults (UniFFI records aren't Codable).
private struct StoredLibrary: Codable {
    var id: String
    var name: String
    var origin: String
    var logoUrl: String?
    var country: String?
    var region: String
    var isTest: Bool

    init(_ info: LibraryInfo) {
        id = info.id
        name = info.name
        origin = info.origin
        logoUrl = info.logoUrl
        country = info.country
        region = info.region
        isTest = info.isTest
    }

    var info: LibraryInfo {
        LibraryInfo(id: id, name: name, origin: origin, logoUrl: logoUrl, country: country, region: region, isTest: isTest)
    }
}
