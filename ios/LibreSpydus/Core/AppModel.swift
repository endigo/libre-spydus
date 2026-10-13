import Foundation
import LibreSpydusCore
import Observation

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
    /// The card number the library account itself lists (its Personal details page). The account
    /// snapshot doesn't carry it yet, so nil; read it from `account.snapshot` here once it does.
    var accountCardNumber: String? { nil }

    // MARK: Local data

    let favourites: FavouritesStore
    /// "Ready to collect" alerts and due-date reminders (§5.8): the Profile switches and the
    /// processing of every fetched snapshot.
    let alerts: AlertCenter

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
        account.report = { [weak self] error in self?.report(error) }
        account.didFetch = { snapshot, origin in await alerts.process(snapshot, origin: origin) }
        installSecrets()
    }

    // MARK: Derived

    /// The library's display name ("your library" before one is chosen).
    var libraryName: String { library?.name ?? "your library" }
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
        case .login: loginNeeded = false
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
    /// `UserFacingError(error, libraryName:)`; `report(_:)` for verification).
    func signIn(username: String, password: String) async throws {
        let generation = backendGeneration
        let backend = self.backend
        let token = try await backend.login(username: username, password: password)
        guard generation == backendGeneration else { throw CancellationError() }
        session = token
        dependencies.sessions.setValue(token, for: backend.origin)
        loginNeeded = false
        accountGeneration += 1
        account.use(origin: backend.origin, session: token, backend: backend)
        rescheduleMorningCheck()
        await account.refresh(force: true)
    }

    /// A hold was placed (or the account otherwise changed outside Your Books): the saved account
    /// is fetched again, so Your Books and Home's shelf show it.
    func accountDidChange() {
        account.libraryDidChange()
    }

    func signOut() {
        // The card number goes with the account; an expired session (`report`) keeps it.
        dependencies.cardNumbers.setValue(nil, for: backend.origin)
        savedCardNumber = nil
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
    /// clears the session and presents login.
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
            savedCardNumber = dependencies.cardNumbers.value(for: origin) ?? DemoBackend.cardNumber
        } else {
            session = dependencies.sessions.value(for: origin)
            savedCardNumber = dependencies.cardNumbers.value(for: origin)
        }
        account.use(origin: library == nil ? nil : origin, session: session, backend: backend)
        favourites.load(origin: library == nil ? nil : origin)
    }

    private func clearSession() {
        dependencies.sessions.setValue(nil, for: backend.origin)
        session = nil
        account.clear(origin: backend.origin)
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
        var makeBackend: @MainActor (_ origin: String?) -> any LibraryBackend
        /// The notification centre, background task scheduler and clock the alerts use; nothing
        /// reaches the system unless set (tests).
        var alertSystem: AlertSystem = .inert
        /// Library card numbers entered on the card sheet, per origin (§5.9).
        var cardNumbers: any SecretStore = InMemorySecretStore()

        /// The app's real stores. Demo mode keeps its session and card number in memory (never the
        /// Keychain), and its account too unless `persistDemoAccount` (then in a folder of its own).
        /// `demoNewReady` (DEBUG `-LSDemoNewReady`): the demo account has one more reservation ready
        /// from its second refresh on. `alertTime` (DEBUG `-LSAlertTime`) replaces 07:45.
        static func live(demo: Bool, persistDemoAccount: Bool = false, demoRefreshFails: Bool = false,
                         demoNewReady: Bool = false, alertTime: DateComponents = AlertContent.alertTime) -> Dependencies {
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
                makeBackend: { origin -> any LibraryBackend in
                    demo
                        ? DemoBackend.live(origin: origin, failsAccountSnapshot: demoRefreshFails, addsReadyOnNextRefresh: demoNewReady)
                        : CoreBackend(origin: origin)
                },
                alertSystem: .live(alertTime: alertTime),
                cardNumbers: demo ? InMemorySecretStore() : KeychainSecretStore(service: "com.librespydus.card")
            )
        }

        /// `-LSReset`: forget the library, every session, verification, card number, favourite,
        /// recent search, saved account and alert setting.
        func resetPersistentState() {
            defaults.removeObject(forKey: AppModel.libraryKey)
            defaults.removeObject(forKey: "LibreSpydus.recentSearches") // RecentSearchStore.defaultsKey
            sessions.removeAll()
            verificationCookies.removeAll()
            cardNumbers.removeAll()
            FavouritesStore.deleteAll(in: favouritesDirectory)
            if let accountCacheDirectory { AccountSnapshotCache.deleteAll(in: accountCacheDirectory) }
            AlertCenter.deleteAll(in: defaults)
        }
    }
}

// MARK: - LibraryInfo helpers

extension LibraryInfo {
    /// Canning, the core's default library: its entry in Civica's directory (logo included), so
    /// the app shows it correctly before or without a directory fetch.
    static let canning = LibraryInfo(
        id: "24fdee8d-ae0f-41b3-b97e-c96059bd06f4", name: "City of Canning",
        origin: "https://canning.spydus.com",
        logoUrl: "https://canning.spydus.com/api/maintenance/1.0/imagebrowser/image?blobName=d226b0c4-06d6-44b7-ba98-e42f06b9fd15.png",
        country: "Australia", region: "AU", isTest: false
    )

    /// A stand-in built from the origin alone, until the directory entry is known.
    static func placeholder(origin: String) -> LibraryInfo {
        if origin == canning.origin { return canning }
        let host = URL(string: origin)?.host() ?? origin
        let label = host.split(separator: ".").first.map(String.init) ?? host
        let region: String
        let country: String?
        switch true {
        case host.hasSuffix(".uk"): (region, country) = ("UK", "United Kingdom")
        case host.hasSuffix(".nz"): (region, country) = ("NZ", "New Zealand")
        case host.hasSuffix(".sg"): (region, country) = ("SG", "Singapore")
        case host.hasSuffix(".au"): (region, country) = ("AU", "Australia")
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
