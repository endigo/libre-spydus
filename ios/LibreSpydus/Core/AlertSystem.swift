import BackgroundTasks
import Foundation
import UIKit
import UserNotifications

/// A local notification as the alerts post it: `UNNotificationRequest` without the framework, so
/// tests can record what would be shown.
struct AlertNotification: Equatable, Sendable {
    var id: String
    var title: String
    var body: String
    /// When it shows: a local date and time (a calendar trigger), or nil for at once.
    var at: DateComponents?
    /// What a tap opens (§5.8 Tap), kept in the notification's `userInfo`.
    var payload: AlertPayload
}

/// What a notification names, so a tap can open it (docs/native-apps.md §5.8 Tap): its kind, its
/// library and the `patronItemKey`s of the reservation or loans it is about. Kept in `userInfo` as
/// `kind` ("ready" / "due"), `origin` and `keys`.
struct AlertPayload: Equatable, Sendable {
    var kind: AlertCenter.Kind
    var origin: String
    var keys: [String]

    private static let kindKey = "kind"
    private static let originKey = "origin"
    private static let keysKey = "keys"

    var userInfo: [String: Any] {
        [Self.kindKey: kind.rawValue, Self.originKey: origin, Self.keysKey: keys]
    }
}

extension AlertPayload {
    /// Read back from a notification's `userInfo`; nil without one (posted by an older build).
    init?(userInfo: [AnyHashable: Any]) {
        guard let kind = (userInfo[Self.kindKey] as? String).flatMap(AlertCenter.Kind.init(rawValue:)),
              let origin = userInfo[Self.originKey] as? String else { return nil }
        self.init(kind: kind, origin: origin, keys: userInfo[Self.keysKey] as? [String] ?? [])
    }
}

/// A tapped notification, as `AppModel.openAlert` routes it (§5.8 Tap).
struct AlertTap: Equatable, Sendable {
    var kind: AlertCenter.Kind
    /// The notification's library, as `AlertContent.originHash(origin)`.
    var originHash: String
    /// The `patronItemKey`s it names; empty when they aren't known.
    var keys: [String]

    /// The list of Your Books it opens: Reservations for a ready alert, Loans for a due reminder.
    var segment: BorrowedViewModel.Segment {
        switch kind {
        case .ready: .reservations
        case .due: .loans
        }
    }
}

extension AlertTap {
    init(_ payload: AlertPayload) {
        self.init(kind: payload.kind, originHash: AlertContent.originHash(payload.origin), keys: payload.keys)
    }

    /// Reads a tapped notification: its payload, else (posted by an older build, without one) its
    /// ID: `ready:<originHash>:<key>` names its reservation, `due:<originHash>:<date>` no loan. Nil
    /// for an ID that isn't an alert's.
    init?(notificationID id: String, userInfo: [AnyHashable: Any]) {
        if let payload = AlertPayload(userInfo: userInfo) {
            self.init(payload)
            return
        }
        // The key itself may contain ":" (`bib:<IRN>`, `title:<title>`).
        let parts = id.split(separator: ":", maxSplits: 2, omittingEmptySubsequences: false).map(String.init)
        guard parts.count == 3, let kind = AlertCenter.Kind(rawValue: parts[0]), !parts[1].isEmpty else { return nil }
        self.init(kind: kind, originHash: parts[1], keys: kind == .ready ? [parts[2]] : [])
    }
}

/// Whether the app may show notifications (Settings → LibreSpydus → Notifications).
enum AlertPermission: Equatable, Sendable {
    case notDetermined
    case denied
    case allowed
}

/// What the alerts need from `UNUserNotificationCenter`. `SystemNotifications` is the real one;
/// tests use a recording fake, so they never touch the notification centre.
protocol NotificationScheduling: Sendable {
    func permission() async -> AlertPermission
    /// Asks once (`[.alert, .sound]`); later calls answer without asking. True when allowed.
    func requestPermission() async -> Bool
    func add(_ notification: AlertNotification) async
    func pendingIDs() async -> [String]
    func deliveredIDs() async -> [String]
    func remove(pending: [String], delivered: [String]) async
}

/// Submits or cancels the morning check (`BGTaskScheduler`); tests record the calls.
protocol MorningCheckScheduling: Sendable {
    func submit(earliest: Date)
    func cancel()
}

/// Everything `AlertCenter` reaches outside the app: the notification centre, the background task
/// scheduler and the clock.
struct AlertSystem: Sendable {
    var notifications: any NotificationScheduling
    var morningCheck: any MorningCheckScheduling
    var now: @Sendable () -> Date = { .now }
    /// Gregorian in the device's time zone, asked each time (the time zone can change).
    var calendar: @Sendable () -> Calendar = { .library }
    /// 07:45, or the DEBUG `-LSAlertTime` override.
    var alertTime = AlertContent.alertTime

    /// The real notification centre and background task scheduler.
    static func live(alertTime: DateComponents = AlertContent.alertTime) -> AlertSystem {
        AlertSystem(notifications: SystemNotifications(), morningCheck: SystemMorningCheckScheduler(), alertTime: alertTime)
    }

    /// Nothing reaches the system: the default for tests that don't look at alerts.
    static var inert: AlertSystem {
        AlertSystem(notifications: InertNotifications(), morningCheck: InertMorningCheckScheduler())
    }
}

// MARK: - Live

/// `UNUserNotificationCenter.current()`.
struct SystemNotifications: NotificationScheduling {
    private var center: UNUserNotificationCenter { .current() }

    func permission() async -> AlertPermission {
        switch await center.notificationSettings().authorizationStatus {
        case .notDetermined: .notDetermined
        case .denied: .denied
        default: .allowed // authorized, provisional, ephemeral
        }
    }

    func requestPermission() async -> Bool {
        (try? await center.requestAuthorization(options: [.alert, .sound])) ?? false
    }

    func add(_ notification: AlertNotification) async {
        let content = UNMutableNotificationContent()
        content.title = notification.title
        content.body = notification.body
        content.sound = .default
        // Ready alerts and due reminders stack separately in Notification Centre.
        content.threadIdentifier = notification.id.hasPrefix("due:") ? "due" : "ready"
        content.userInfo = notification.payload.userInfo
        let trigger = notification.at.map { UNCalendarNotificationTrigger(dateMatching: $0, repeats: false) }
        do {
            try await center.add(UNNotificationRequest(identifier: notification.id, content: content, trigger: trigger))
            AlertLog.print("posted \(notification.id)\(notification.at.map { " for \(Self.describe($0))" } ?? ""): \(notification.title) — \(notification.body)")
        } catch {
            AlertLog.print("couldn't post \(notification.id): \(error.localizedDescription)")
        }
    }

    func pendingIDs() async -> [String] {
        await center.pendingNotificationRequests().map(\.identifier)
    }

    func deliveredIDs() async -> [String] {
        await center.deliveredNotifications().map(\.request.identifier)
    }

    func remove(pending: [String], delivered: [String]) async {
        if !pending.isEmpty { center.removePendingNotificationRequests(withIdentifiers: pending) }
        if !delivered.isEmpty { center.removeDeliveredNotifications(withIdentifiers: delivered) }
    }

    private static func describe(_ components: DateComponents) -> String {
        String(format: "%04d-%02d-%02d %02d:%02d", components.year ?? 0, components.month ?? 0, components.day ?? 0,
               components.hour ?? 0, components.minute ?? 0)
    }
}

/// `BGTaskScheduler.shared` with the app-refresh task `com.librespydus.morning-check` (Info.plist
/// `BGTaskSchedulerPermittedIdentifiers`; handled by the app's `.backgroundTask(.appRefresh(…))`).
///
/// A submit before that handler is registered raises an Objective-C exception ("No launch handler
/// registered"), and SwiftUI registers it while the app finishes launching, after `App.init`. So
/// once `holdUntilLaunched()` has been called (in `App.init`), requests wait until the app has
/// finished launching; only the latest is kept.
struct SystemMorningCheckScheduler: MorningCheckScheduling {
    static let identifier = "com.librespydus.morning-check"
    private static let gate = LaunchGate()

    /// From `App.init`: hold submits and cancels until the app has finished launching.
    static func holdUntilLaunched() {
        gate.holdUntilLaunched()
    }

    func submit(earliest: Date) {
        Self.gate.run {
            let request = BGAppRefreshTaskRequest(identifier: Self.identifier)
            request.earliestBeginDate = earliest
            do {
                // Replaces the request already submitted, if any.
                try BGTaskScheduler.shared.submit(request)
                AlertLog.print("morning check scheduled from \(earliest.formatted(date: .abbreviated, time: .shortened))")
            } catch {
                // The simulator has no background tasks (`unavailable`); a device may refuse when
                // Background App Refresh is off. The foreground refreshes still alert.
                AlertLog.print("morning check not scheduled: \(error.localizedDescription)")
            }
        }
    }

    func cancel() {
        Self.gate.run {
            BGTaskScheduler.shared.cancel(taskRequestWithIdentifier: Self.identifier)
        }
    }
}

/// Runs actions at once, or, while held, keeps the latest until the app has finished launching
/// (`UIApplication.didFinishLaunchingNotification`, then one more turn of the main queue).
private final class LaunchGate: @unchecked Sendable {
    private let lock = NSLock()
    private var isHeld = false
    private var waiting: (@Sendable () -> Void)?
    private var observer: (any NSObjectProtocol)?

    func holdUntilLaunched() {
        lock.withLock {
            guard observer == nil else { return }
            isHeld = true
            observer = NotificationCenter.default.addObserver(forName: UIApplication.didFinishLaunchingNotification,
                                                              object: nil, queue: .main) { [weak self] _ in
                DispatchQueue.main.async { self?.release() }
            }
        }
    }

    func run(_ action: @escaping @Sendable () -> Void) {
        let runNow = lock.withLock {
            if isHeld { waiting = action }
            return !isHeld
        }
        if runNow { action() }
    }

    private func release() {
        let action = lock.withLock {
            isHeld = false
            defer { waiting = nil }
            return waiting
        }
        action?()
    }
}

/// Shows alerts as banners while the app is open (`willPresent`) and, when one is tapped, opens
/// what it names (`AlertTap`, routed by `AppModel.openAlert`). Installed as the notification centre's
/// delegate at launch, once the model has loaded the saved account, so a tap that launches the app
/// is routed too. The centre holds its delegate weakly: keep it.
final class AlertNotificationRouter: NSObject, UNUserNotificationCenterDelegate, Sendable {
    private let open: @MainActor @Sendable (AlertTap) -> Void

    private init(open: @escaping @MainActor @Sendable (AlertTap) -> Void) {
        self.open = open
    }

    /// Makes the router the notification centre's delegate.
    static func install(open: @escaping @MainActor @Sendable (AlertTap) -> Void) -> AlertNotificationRouter {
        let router = AlertNotificationRouter(open: open)
        UNUserNotificationCenter.current().delegate = router
        return router
    }

    // The completion-handler forms, not the `async` ones: Swift calls an `async` delegate method's
    // completion handler off the main thread, and UIKit aborts on that when a tap opens the app.

    func userNotificationCenter(_ center: UNUserNotificationCenter, willPresent notification: UNNotification,
                                withCompletionHandler completionHandler: @escaping (UNNotificationPresentationOptions) -> Void) {
        completionHandler([.banner, .list, .sound])
    }

    func userNotificationCenter(_ center: UNUserNotificationCenter, didReceive response: UNNotificationResponse,
                                withCompletionHandler completionHandler: @escaping () -> Void) {
        let request = response.notification.request
        let tap = response.actionIdentifier == UNNotificationDefaultActionIdentifier
            ? AlertTap(notificationID: request.identifier, userInfo: request.content.userInfo) : nil
        AlertLog.print("tapped \(request.identifier): \(tap.map { "\($0.kind) \($0.keys)" } ?? "not an alert")")
        let open = self.open
        let done = UncheckedSendable(completionHandler)
        DispatchQueue.main.async {
            MainActor.assumeIsolated {
                if let tap { open(tap) }
            }
            done.value()
        }
    }
}

/// Carries a value the compiler can't check across to the main queue (UIKit's completion handler).
private struct UncheckedSendable<Value>: @unchecked Sendable {
    let value: Value

    init(_ value: Value) {
        self.value = value
    }
}

/// `🔔 alerts …` console lines in DEBUG builds, like the timing log: what was posted or scheduled
/// and what the morning check did.
enum AlertLog {
    static func print(_ message: @autoclosure () -> String) {
        #if DEBUG
        Swift.print("🔔 alerts \(message())")
        #endif
    }
}

// MARK: - Inert

private struct InertNotifications: NotificationScheduling {
    func permission() async -> AlertPermission { .notDetermined }
    func requestPermission() async -> Bool { false }
    func add(_ notification: AlertNotification) async {}
    func pendingIDs() async -> [String] { [] }
    func deliveredIDs() async -> [String] { [] }
    func remove(pending: [String], delivered: [String]) async {}
}

private struct InertMorningCheckScheduler: MorningCheckScheduling {
    func submit(earliest: Date) {}
    func cancel() {}
}
