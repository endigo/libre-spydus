import Foundation

/// Debug launch hooks (docs/native-apps.md §5.6), read from launch arguments in DEBUG builds only:
///
///     xcrun simctl launch <udid> com.librespydus \
///       -LSLibraryOrigin https://canning.spydus.com -LSDemo YES -LSRoute borrowed
///
/// - `-LSLibraryOrigin <origin>`: select a library, skip the picker.
/// - `-LSRoute <route>`: `home`, `search:<query>`, `results:<query>`, `detail:<url>`, `borrowed`,
///   `favourites`, `profile`, `libraries`, `login`, `verify`, `gallery` (DesignSystem gallery).
/// - `-LSDemo YES`: demo mode, already signed in as "Test Borrower".
/// - `-LSReset YES`: clear the persisted library, sessions, verification and favourites first.
/// - `-LSPersistDemoAccount YES` (with `-LSDemo`): save the demo account on the device like a real
///   one (in its own folder), to show a cold launch with saved data. Off by default: demo mode
///   keeps the account in memory.
/// - `-LSDemoRefreshFails YES` (with `-LSDemo`): every account refresh fails, to show the
///   "Couldn't refresh" state.
/// - `-LSMorningCheckNow YES`: run the morning check (docs/native-apps.md §5.8) once the launch
///   refresh has settled, instead of waiting for 07:45.
/// - `-LSDemoNewReady YES` (with `-LSDemo`): one more demo reservation is ready from the account's
///   second refresh on, so a "Ready to collect" alert can be shown.
/// - `-LSAlertTime HH:mm`: due reminders show (and the morning check is scheduled) at this time
///   instead of 07:45, to see a reminder arrive.
/// - `-LSDemoAlert ready|due|due-several` (with `-LSDemo`): a few seconds after launch, show one
///   alert from the demo account, to tap it (`AppModel.postDemoAlert`).
struct LaunchOptions: Equatable, Sendable {
    /// `-LSDemoAlert`: which demo alert to show.
    enum DemoAlert: String, Equatable, Sendable {
        /// "Ready to collect" for the first ready reservation.
        case ready
        /// The first due-date reminder naming one loan.
        case due
        /// A due-date reminder naming every loan due from today on.
        case dueSeveral = "due-several"
    }

    enum LaunchRoute: Equatable, Sendable {
        case home
        case search(String)
        case results(String)
        case detail(String)
        case borrowed
        case favourites
        case profile
        case libraries
        case login
        case verify
        case gallery

        /// Parses `name` or `name:<argument>`; nil for an unknown route or a missing argument.
        init?(_ text: String) {
            let name: Substring
            let argument: String?
            if let colon = text.firstIndex(of: ":") {
                name = text[..<colon]
                argument = String(text[text.index(after: colon)...]).trimmingCharacters(in: .whitespaces)
            } else {
                name = Substring(text)
                argument = nil
            }
            let nonEmptyArgument = argument.flatMap { $0.isEmpty ? nil : $0 }
            switch name.lowercased() {
            case "home": self = .home
            case "search": self = .search(argument ?? "")
            case "results":
                guard let query = nonEmptyArgument else { return nil }
                self = .results(query)
            case "detail":
                guard let url = nonEmptyArgument else { return nil }
                self = .detail(url)
            case "borrowed": self = .borrowed
            case "favourites", "favorites": self = .favourites
            case "profile": self = .profile
            case "libraries": self = .libraries
            case "login": self = .login
            case "verify": self = .verify
            case "gallery": self = .gallery
            default: return nil
            }
        }
    }

    var libraryOrigin: String?
    var route: LaunchRoute?
    var demo = false
    var reset = false
    var persistDemoAccount = false
    var demoRefreshFails = false
    var morningCheckNow = false
    var demoNewReady = false
    /// `-LSAlertTime HH:mm` as hour and minute; nil: 07:45.
    var alertTime: DateComponents?
    var demoAlert: DemoAlert?

    /// This process's options; always empty in release builds.
    static var current: LaunchOptions {
        #if DEBUG
        parse(ProcessInfo.processInfo.arguments)
        #else
        LaunchOptions()
        #endif
    }

    /// Parses `-LSKey value` pairs (the same form `UserDefaults` reads); unknown keys are ignored.
    static func parse(_ arguments: [String]) -> LaunchOptions {
        var options = LaunchOptions()
        var index = arguments.startIndex
        while index < arguments.endIndex {
            let key = arguments[index]
            let next = arguments.index(after: index)
            guard key.hasPrefix("-LS"), next < arguments.endIndex else {
                index = next
                continue
            }
            let value = arguments[next]
            switch key {
            case "-LSLibraryOrigin":
                let origin = value.trimmingCharacters(in: .whitespaces)
                options.libraryOrigin = origin.isEmpty ? nil : origin
            case "-LSRoute": options.route = LaunchRoute(value)
            case "-LSDemo": options.demo = isYes(value)
            case "-LSReset": options.reset = isYes(value)
            case "-LSPersistDemoAccount": options.persistDemoAccount = isYes(value)
            case "-LSDemoRefreshFails": options.demoRefreshFails = isYes(value)
            case "-LSMorningCheckNow": options.morningCheckNow = isYes(value)
            case "-LSDemoNewReady": options.demoNewReady = isYes(value)
            case "-LSAlertTime": options.alertTime = time(value)
            case "-LSDemoAlert": options.demoAlert = DemoAlert(rawValue: value.lowercased())
            default:
                index = next
                continue
            }
            index = arguments.index(after: next)
        }
        return options
    }

    private static func isYes(_ value: String) -> Bool {
        ["yes", "true", "1"].contains(value.lowercased())
    }

    /// "7:45" / "14:05" → hour and minute; nil when it isn't a time of day.
    private static func time(_ value: String) -> DateComponents? {
        let parts = value.trimmingCharacters(in: .whitespaces).split(separator: ":")
        guard parts.count == 2, let hour = Int(parts[0]), let minute = Int(parts[1]),
              (0..<24).contains(hour), (0..<60).contains(minute) else { return nil }
        return DateComponents(hour: hour, minute: minute)
    }
}

#if DEBUG
extension LaunchOptions {
    /// Applies the library and route hooks to a freshly created model (`-LSReset` and `-LSDemo`
    /// are handled before the model exists, in `LibreSpydusApp.init`).
    @MainActor
    func apply(to model: AppModel) {
        if let libraryOrigin {
            model.selectLibrary(origin: libraryOrigin)
            Task { await model.refreshLibraryInfo() }
        }
        switch route {
        case nil: break
        case .home: model.selectedTab = .home
        case .search(let query): model.openSearch(query: query)
        case .results(let query):
            model.setPath([.results(query: query, sort: .relevance, title: query)], for: .search)
            model.selectedTab = .search
        case .detail(let url):
            model.setPath([.detail(BookRef(url: url))], for: .home)
            model.selectedTab = .home
        case .borrowed: model.selectedTab = .borrowed
        case .favourites: model.selectedTab = .favourites
        case .profile: model.selectedTab = .profile
        case .libraries: model.libraryPickerPresented = true
        case .login: model.loginNeeded = true
        case .verify: model.verificationNeeded = true
        case .gallery: model.componentsGalleryPresented = true
        }
        if morningCheckNow {
            Task {
                // After the launch refresh (this joins it), so the check fetches the account again.
                await model.refreshAccount()
                await model.runMorningCheck()
            }
        }
        if let demoAlert, model.isDemo {
            Task {
                await model.refreshAccount()
                await model.postDemoAlert(demoAlert)
            }
        }
    }
}
#endif
