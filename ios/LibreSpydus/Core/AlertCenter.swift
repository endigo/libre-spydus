import Foundation
import LibreSpydusCore
import Observation

/// "Ready to collect" alerts and due-date reminders (docs/native-apps.md §5.8): local notifications
/// raised from the account snapshots the app already loads. `AppModel` owns one.
///
/// - **Switches** (Profile → Notifications): `readyEnabled`, `dueEnabled`, app-wide in UserDefaults,
///   both off until turned on. Turning one on asks for permission; refused, it stays off and
///   `permissionDenied` makes Profile say so.
/// - **`process(_:origin:)`** runs after every fetched snapshot (`AccountStore.didFetch`: foreground
///   refreshes and the morning check alike). Ready: one notification per reservation that is newly
///   ready (`readyAlerts`), with the alerted keys saved per library; a library with no saved keys
///   (signed in again, or switched on without a saved account) is only seeded. Due: this library's pending
///   reminders are replaced by calendar triggers at 07:45 on each `remindOn` (`dueReminders`, 2 days
///   ahead), skipping one whose 07:45 has passed.
/// - **Alerted keys** live under `alerts.readyKeys.<sha256(origin)>`; sign-out and session expiry
///   delete that library's keys and notifications (`forget(origin:)`).
///
/// UserDefaults keys: `alerts.readyEnabled`, `alerts.dueEnabled`, `alerts.readyKeys.<hash>`. Demo
/// mode uses `alerts.demo.` instead of `alerts.`, so it never changes the real settings.
@MainActor
@Observable
final class AlertCenter {
    nonisolated static let namespace = "alerts."
    nonisolated static let demoNamespace = "alerts.demo."

    /// A notification's kind; its raw value is the one in the notification's payload.
    enum Kind: String, Equatable, Sendable {
        case ready
        case due
    }

    /// "Ready to collect": alert when a reservation is ready to collect.
    private(set) var readyEnabled: Bool
    /// "Due-date reminders": 2 days before loans are due, at 7:45.
    private(set) var dueEnabled: Bool
    /// Notifications are off for the app in Settings while the user wants alerts: Profile shows
    /// "Notifications are turned off for LibreSpydus." with Open Settings.
    private(set) var permissionDenied = false
    /// The switch whose permission request is on screen (it shows as on meanwhile).
    private(set) var asking: Kind?

    var anyEnabled: Bool { readyEnabled || dueEnabled }

    @ObservationIgnored private let defaults: UserDefaults
    @ObservationIgnored let system: AlertSystem
    @ObservationIgnored private let readyEnabledKey: String
    @ObservationIgnored private let dueEnabledKey: String
    /// Followed by sha256(origin).
    @ObservationIgnored private let readyKeysPrefix: String

    init(defaults: UserDefaults, system: AlertSystem, namespace: String = AlertCenter.namespace) {
        self.defaults = defaults
        self.system = system
        readyEnabledKey = namespace + "readyEnabled"
        dueEnabledKey = namespace + "dueEnabled"
        readyKeysPrefix = namespace + "readyKeys."
        readyEnabled = defaults.bool(forKey: readyEnabledKey)
        dueEnabled = defaults.bool(forKey: dueEnabledKey)
    }

    // MARK: Switches

    /// The "Ready to collect" switch. On: asks for permission (refused → stays off) and remembers
    /// what is already waiting in `saved` without a notification (`readyAlerts(saved, [])`); with
    /// no saved account the first fetched one is remembered instead. Off: forgets every library's
    /// alerted keys.
    func setReady(_ on: Bool, origin: String?, saved: AccountSnapshot?) async {
        if on {
            guard !readyEnabled, await allow(.ready) else { return }
            if let origin {
                let hash = AlertContent.originHash(origin)
                if let saved {
                    setAlertedKeys(readyAlerts(snapshot: saved, alertedKeys: []).alertedKeys, originHash: hash)
                } else {
                    defaults.removeObject(forKey: readyKeysPrefix + hash)
                }
            }
            readyEnabled = true
        } else {
            readyEnabled = false
            for key in defaults.dictionaryRepresentation().keys where key.hasPrefix(readyKeysPrefix) {
                defaults.removeObject(forKey: key)
            }
        }
        defaults.set(readyEnabled, forKey: readyEnabledKey)
    }

    /// The "Due-date reminders" switch. On: asks for permission (refused → stays off) and schedules
    /// the reminders for `saved` straight away. Off: removes every pending reminder.
    func setDue(_ on: Bool, origin: String?, saved: AccountSnapshot?) async {
        if on {
            guard !dueEnabled, await allow(.due) else { return }
            dueEnabled = true
            defaults.set(true, forKey: dueEnabledKey)
            if let origin, let saved { await scheduleDueReminders(for: saved, origin: origin) }
        } else {
            dueEnabled = false
            defaults.set(false, forKey: dueEnabledKey)
            let pending = await system.notifications.pendingIDs().filter { $0.hasPrefix("due:") }
            await system.notifications.remove(pending: pending, delivered: [])
        }
    }

    /// Profile appeared or the app came back: notifications allowed in Settings meanwhile hide the
    /// "turned off" line; turned off there while a switch is on shows it.
    func refreshPermission() async {
        let permission = await system.notifications.permission()
        permissionDenied = permission == .denied && (permissionDenied || anyEnabled)
    }

    // MARK: Processing

    /// A snapshot of `origin`'s account was just fetched: alert what's newly ready and reschedule
    /// the due reminders, as the switches say.
    func process(_ snapshot: AccountSnapshot, origin: String) async {
        let hash = AlertContent.originHash(origin)
        if readyEnabled {
            let saved = alertedKeys(originHash: hash)
            let result = readyAlerts(snapshot: snapshot, alertedKeys: saved ?? [])
            // Saved before posting (nothing else runs on the main actor meanwhile), so an overlapping
            // run can't post the same reservation twice. The keys replace the saved ones.
            setAlertedKeys(result.alertedKeys, originHash: hash)
            // No saved keys (signed in again after a sign-out or an expired session, or switched on
            // without a saved account): what is waiting now isn't new, so it's only remembered.
            let newItems = saved == nil ? [] : result.newItems
            if saved == nil { AlertLog.print("ready: remembered \(result.alertedKeys.count) waiting, nothing announced") }
            for item in newItems {
                await system.notifications.add(AlertContent.readyNotification(item, origin: origin))
            }
        }
        if dueEnabled {
            await scheduleDueReminders(for: snapshot, origin: origin)
        }
    }

    /// Replaces the library's pending `due:` notifications with one per `dueReminders` entry, at
    /// 07:45 on its `remindOn`; one whose 07:45 has already passed is left out.
    private func scheduleDueReminders(for snapshot: AccountSnapshot, origin: String) async {
        let calendar = system.calendar()
        let now = system.now()
        let reminders = dueReminders(snapshot: snapshot, today: AlertContent.simpleDate(now, calendar: calendar),
                                     daysBefore: AlertContent.dueDaysBefore)
        let prefix = "due:\(AlertContent.originHash(origin)):"
        let old = await system.notifications.pendingIDs().filter { $0.hasPrefix(prefix) }
        await system.notifications.remove(pending: old, delivered: [])
        for reminder in reminders {
            guard let at = AlertContent.alertDate(on: reminder.remindOn, calendar: calendar, time: system.alertTime),
                  at > now else { continue }
            await system.notifications.add(AlertContent.dueNotification(reminder, origin: origin, time: system.alertTime))
        }
    }

    // MARK: Morning check

    /// Submits the morning check for the next 07:45 while signed in with a switch on; cancels it
    /// otherwise.
    func scheduleMorningCheck(signedIn: Bool) {
        if signedIn, anyEnabled {
            system.morningCheck.submit(earliest: AlertContent.nextAlertDate(after: system.now(), calendar: system.calendar(),
                                                                            time: system.alertTime))
        } else {
            system.morningCheck.cancel()
        }
    }

    // MARK: Sign-out

    /// Signed out of `origin`, or its session expired: its alerted keys go at once, its pending and
    /// delivered notifications (ready and due) in the returned task. Other libraries keep theirs.
    @discardableResult
    func forget(origin: String) -> Task<Void, Never> {
        let hash = AlertContent.originHash(origin)
        defaults.removeObject(forKey: readyKeysPrefix + hash)
        let notifications = system.notifications
        return Task {
            let pending = await notifications.pendingIDs().filter { AlertContent.belongs($0, toOriginHash: hash) }
            let delivered = await notifications.deliveredIDs().filter { AlertContent.belongs($0, toOriginHash: hash) }
            await notifications.remove(pending: pending, delivered: delivered)
        }
    }

    /// `-LSReset`: both switches off and every library's alerted keys gone, demo mode's too.
    nonisolated static func deleteAll(in defaults: UserDefaults) {
        for key in defaults.dictionaryRepresentation().keys where key.hasPrefix(namespace) {
            defaults.removeObject(forKey: key)
        }
    }

    // MARK: Alerted keys

    /// The ready reservations already alerted for the library `originHash` (`patronItemKey`s); nil
    /// when none are recorded (not the same as an empty list).
    func alertedKeys(originHash: String) -> [String]? {
        defaults.stringArray(forKey: readyKeysPrefix + originHash)
    }

    private func setAlertedKeys(_ keys: [String], originHash: String) {
        defaults.set(keys, forKey: readyKeysPrefix + originHash)
    }

    // MARK: Private

    /// Asks for permission with `kind`'s switch showing on; refused → `permissionDenied`.
    private func allow(_ kind: Kind) async -> Bool {
        asking = kind
        defer { asking = nil }
        let allowed = await system.notifications.requestPermission()
        permissionDenied = !allowed
        return allowed
    }
}
