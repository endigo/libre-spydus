import Foundation
import LibreSpydusCore
import Testing
@testable import LibreSpydus

/// A notification centre that records: immediate notifications are "delivered", calendar ones
/// "pending".
actor FakeNotifications: NotificationScheduling {
    private(set) var status: AlertPermission
    /// What the permission prompt answers while undecided.
    private let grants: Bool
    private(set) var permissionRequests = 0
    private(set) var added: [AlertNotification] = []
    private(set) var pending: [String: AlertNotification] = [:]
    private(set) var delivered: [String] = []

    init(status: AlertPermission = .notDetermined, grants: Bool = true, pending: [String] = [], delivered: [String] = []) {
        self.status = status
        self.grants = grants
        self.pending = Dictionary(uniqueKeysWithValues: pending.map {
            ($0, AlertNotification(id: $0, title: "old", body: "old", at: DateComponents(hour: 7, minute: 45),
                                   payload: AlertPayload(kind: $0.hasPrefix("due:") ? .due : .ready, origin: "old", keys: [])))
        })
        self.delivered = delivered
    }

    func permission() -> AlertPermission { status }

    func requestPermission() -> Bool {
        permissionRequests += 1
        if status == .notDetermined { status = grants ? .allowed : .denied }
        return status == .allowed
    }

    func setStatus(_ status: AlertPermission) { self.status = status }

    func add(_ notification: AlertNotification) {
        added.append(notification)
        if notification.at == nil {
            delivered.append(notification.id)
        } else {
            pending[notification.id] = notification
        }
    }

    func pendingIDs() -> [String] { pending.keys.sorted() }
    func deliveredIDs() -> [String] { delivered }

    func remove(pending ids: [String], delivered deliveredIDs: [String]) {
        for id in ids { pending[id] = nil }
        delivered.removeAll { deliveredIDs.contains($0) }
    }

    func clearAdded() { added = [] }
}

final class FakeMorningCheck: MorningCheckScheduling, @unchecked Sendable {
    enum Event: Equatable {
        case submit(Date)
        case cancel
    }

    private let lock = NSLock()
    private var _events: [Event] = []
    var events: [Event] { lock.withLock { _events } }
    var last: Event? { events.last }

    func submit(earliest: Date) { lock.withLock { _events.append(.submit(earliest)) } }
    func cancel() { lock.withLock { _events.append(.cancel) } }
}

@MainActor
struct AlertCenterTests {
    private let origin = AlertFixtures.origin
    private let hash = AlertFixtures.hash
    private let perth = AlertFixtures.calendar()

    private func makeCenter(_ notifications: FakeNotifications = FakeNotifications(),
                            morningCheck: FakeMorningCheck = FakeMorningCheck(),
                            now: Date = AlertFixtures.date(2026, 10, 13, 10, 0),
                            defaults: UserDefaults = UserDefaults(suiteName: "LibreSpydusTests-\(UUID().uuidString)")!) -> AlertCenter {
        let calendar = perth
        return AlertCenter(defaults: defaults, system: AlertSystem(notifications: notifications, morningCheck: morningCheck,
                                                                   now: { now }, calendar: { calendar }))
    }

    private let midnight = AlertFixtures.ready("19000001", "The midnight library")
    private let hollow = AlertFixtures.ready("19000003", "Hollow bones",
                                             fields: ["Status": "Please pickup from: RIVERTON LIBRARY by 20 Oct 2026"])

    // MARK: Ready

    @Test func turningReadyOnRemembersWhatIsWaitingWithoutANotification() async {
        let notifications = FakeNotifications()
        let center = makeCenter(notifications)
        await center.setReady(true, origin: origin, saved: AlertFixtures.snapshot(ready: [midnight]))
        #expect(center.readyEnabled)
        #expect(await notifications.permissionRequests == 1)
        #expect(center.alertedKeys(originHash: hash) == ["19000001"])
        #expect(await notifications.added.isEmpty)
        // The same list again: still nothing.
        await center.process(AlertFixtures.snapshot(ready: [midnight]), origin: origin)
        #expect(await notifications.added.isEmpty)
    }

    @Test func processPostsOneNotificationPerNewlyReadyReservationAndSavesTheKeys() async {
        let notifications = FakeNotifications()
        let center = makeCenter(notifications)
        await center.setReady(true, origin: origin, saved: AlertFixtures.snapshot(ready: [midnight]))
        let second = AlertFixtures.ready("19000009", nil, fields: [:])
        await center.process(AlertFixtures.snapshot(ready: [midnight, hollow, second]), origin: origin)
        #expect(await notifications.added == [
            AlertNotification(id: "ready:\(hash):19000003", title: "Ready to collect", body: "Hollow bones · Riverton Library",
                              payload: AlertPayload(kind: .ready, origin: origin, keys: ["19000003"])),
            AlertNotification(id: "ready:\(hash):19000009", title: "Ready to collect", body: "A reservation",
                              payload: AlertPayload(kind: .ready, origin: origin, keys: ["19000009"])),
        ])
        #expect(center.alertedKeys(originHash: hash) == ["19000001", "19000003", "19000009"])

        // Collected, then ready again later: alerted again (the keys are replaced, not merged).
        await notifications.clearAdded()
        await center.process(AlertFixtures.snapshot(ready: [hollow]), origin: origin)
        #expect(await notifications.added.isEmpty)
        #expect(center.alertedKeys(originHash: hash) == ["19000003"])
        await center.process(AlertFixtures.snapshot(ready: [midnight, hollow]), origin: origin)
        #expect(await notifications.added.map(\.id) == ["ready:\(hash):19000001"])
    }

    @Test func aLibraryWithNoSavedKeysIsSeededNotAnnounced() async {
        let notifications = FakeNotifications(status: .allowed)
        let center = makeCenter(notifications)
        // Switched on without a saved account: nothing is recorded yet.
        await center.setReady(true, origin: origin, saved: nil)
        #expect(center.alertedKeys(originHash: hash) == nil)
        await center.process(AlertFixtures.snapshot(ready: [midnight]), origin: origin)
        #expect(await notifications.added.isEmpty)
        #expect(center.alertedKeys(originHash: hash) == ["19000001"])
        await center.process(AlertFixtures.snapshot(ready: [midnight, hollow]), origin: origin)
        #expect(await notifications.added.map(\.id) == ["ready:\(hash):19000003"])
        // A saved empty list is a record: what comes ready next is announced.
        await notifications.clearAdded()
        await center.process(AlertFixtures.snapshot(), origin: origin)
        #expect(center.alertedKeys(originHash: hash) == [])
        await center.process(AlertFixtures.snapshot(ready: [midnight]), origin: origin)
        #expect(await notifications.added.map(\.id) == ["ready:\(hash):19000001"])
    }

    @Test func keysAreKeptPerLibrary() async {
        let notifications = FakeNotifications()
        let center = makeCenter(notifications)
        await center.setReady(true, origin: origin, saved: AlertFixtures.snapshot(ready: [midnight]))
        let other = "https://hutt.spydus.co.nz"
        let otherHash = AlertContent.originHash(other)
        // Another library seen for the first time: seeded.
        await center.process(AlertFixtures.snapshot(ready: [midnight]), origin: other)
        #expect(await notifications.added.isEmpty)
        await center.process(AlertFixtures.snapshot(ready: [midnight, hollow]), origin: other)
        #expect(await notifications.added.map(\.id) == ["ready:\(otherHash):19000003"])
        #expect(center.alertedKeys(originHash: hash) == ["19000001"])
        #expect(center.alertedKeys(originHash: otherHash) == ["19000001", "19000003"])
    }

    @Test func withBothSwitchesOffNothingIsPostedOrScheduled() async {
        let notifications = FakeNotifications(status: .allowed)
        let center = makeCenter(notifications)
        #expect(!center.readyEnabled && !center.dueEnabled)
        await center.process(AlertFixtures.snapshot(ready: [midnight, hollow],
                                                    loans: [AlertFixtures.loan("1", "Whistler", due: "20 Oct 2026")]),
                             origin: origin)
        #expect(await notifications.added.isEmpty)
        #expect(center.alertedKeys(originHash: hash) == nil)
        #expect(await notifications.permissionRequests == 0)
    }

    @Test func turningReadyOffForgetsEveryLibrarysKeys() async {
        let defaults = UserDefaults(suiteName: "LibreSpydusTests-\(UUID().uuidString)")!
        let notifications = FakeNotifications()
        let center = makeCenter(notifications, defaults: defaults)
        await center.setReady(true, origin: origin, saved: AlertFixtures.snapshot(ready: [midnight]))
        await center.process(AlertFixtures.snapshot(ready: [hollow]), origin: "https://hutt.spydus.co.nz")
        #expect(defaults.bool(forKey: "alerts.readyEnabled"))
        await center.setReady(false, origin: origin, saved: nil)
        #expect(!center.readyEnabled)
        #expect(!defaults.bool(forKey: "alerts.readyEnabled"))
        #expect(defaults.dictionaryRepresentation().keys.filter { $0.hasPrefix("alerts.readyKeys.") }.isEmpty)
        // Off: a new ready reservation says nothing.
        await notifications.clearAdded()
        await center.process(AlertFixtures.snapshot(ready: [midnight, hollow]), origin: origin)
        #expect(await notifications.added.isEmpty)
        // The switches survive a relaunch.
        await center.setReady(true, origin: origin, saved: nil)
        #expect(makeCenter(defaults: defaults).readyEnabled)
    }

    // MARK: Permission

    @Test func refusedPermissionLeavesTheSwitchOffAndSaysSo() async {
        let defaults = UserDefaults(suiteName: "LibreSpydusTests-\(UUID().uuidString)")!
        let notifications = FakeNotifications(grants: false)
        let center = makeCenter(notifications, defaults: defaults)
        await center.setReady(true, origin: origin, saved: AlertFixtures.snapshot(ready: [midnight]))
        #expect(!center.readyEnabled)
        #expect(center.permissionDenied)
        #expect(center.asking == nil)
        #expect(center.alertedKeys(originHash: hash) == nil)
        #expect(!defaults.bool(forKey: "alerts.readyEnabled"))
        await center.setDue(true, origin: origin, saved: nil)
        #expect(!center.dueEnabled)
        // Allowed in Settings meanwhile: the line goes; on again works.
        await notifications.setStatus(.allowed)
        await center.refreshPermission()
        #expect(!center.permissionDenied)
        await center.setDue(true, origin: origin, saved: nil)
        #expect(center.dueEnabled)
        // Turned off in Settings while a switch is on: the line shows.
        await notifications.setStatus(.denied)
        await center.refreshPermission()
        #expect(center.permissionDenied)
    }

    @Test func aDeniedPermissionWithBothSwitchesOffShowsNothing() async {
        let center = makeCenter(FakeNotifications(status: .denied))
        await center.refreshPermission()
        #expect(!center.permissionDenied)
    }

    // MARK: Due

    @Test func dueRemindersReplaceThisLibrarysPendingOnesAndSkipAPassed0745() async {
        let other = "due:\(AlertContent.originHash("https://hutt.spydus.co.nz")):2026-10-21"
        let notifications = FakeNotifications(status: .allowed,
                                              pending: ["due:\(hash):2026-10-30", other, "ready:\(hash):19000001"])
        // Tuesday 13 Oct 2026, 10:00 in Perth.
        let center = makeCenter(notifications, now: AlertFixtures.date(2026, 10, 13, 10, 0))
        let snapshot = AlertFixtures.snapshot(loans: [
            AlertFixtures.loan("1", "The long night", due: "11 Oct 2026"),     // overdue: no reminder
            AlertFixtures.loan("2", "Whistler", due: "15 Oct 2026"),           // remind today 07:45: passed
            AlertFixtures.loan("3", "The Love Library", due: "20 Oct 2026"),   // remind Sun 18 Oct
            AlertFixtures.loan("4", "Everything to the sea", due: "25 Oct 2026"),
            AlertFixtures.loan("5", "The secret thread", due: "25 Oct 2026"),
        ])
        await center.setDue(true, origin: origin, saved: nil)
        await center.process(snapshot, origin: origin)
        let gregorian = Calendar(identifier: .gregorian)
        #expect(await notifications.added == [
            AlertNotification(id: "due:\(hash):2026-10-20", title: "Due Tuesday 20 Oct", body: "The Love Library",
                              at: DateComponents(calendar: gregorian, year: 2026, month: 10, day: 18, hour: 7, minute: 45),
                              payload: AlertPayload(kind: .due, origin: origin, keys: ["3"])),
            AlertNotification(id: "due:\(hash):2026-10-25", title: "Due Sunday 25 Oct",
                              body: "Everything to the sea and The secret thread",
                              at: DateComponents(calendar: gregorian, year: 2026, month: 10, day: 23, hour: 7, minute: 45),
                              payload: AlertPayload(kind: .due, origin: origin, keys: ["4", "5"])),
        ])
        // The old reminder (a renewed loan) is gone; another library's and the ready ones stay.
        #expect(await notifications.pendingIDs() == [
            "due:\(hash):2026-10-20", "due:\(hash):2026-10-25", other, "ready:\(hash):19000001",
        ].sorted())
    }

    @Test func aReminderForTodayIsScheduledWhile0745IsAhead() async {
        let notifications = FakeNotifications(status: .allowed)
        let center = makeCenter(notifications, now: AlertFixtures.date(2026, 10, 13, 7, 0))
        await center.setDue(true, origin: origin, saved: nil)
        await center.process(AlertFixtures.snapshot(loans: [
            AlertFixtures.loan("1", "Whistler", due: "15 Oct 2026"),
            AlertFixtures.loan("2", "Hollow bones", due: "13 Oct 2026"),
            AlertFixtures.loan("3", "The Love Library", due: "14 Oct 2026"),
        ]), origin: origin)
        #expect(await notifications.added.map(\.title) == ["Due today", "Due tomorrow", "Due Thursday 15 Oct"])
        #expect(await notifications.added.map { $0.at?.day } == [13, 13, 13])
    }

    @Test func turningDueOnSchedulesFromTheSavedAccountAndOffRemovesThem() async {
        let notifications = FakeNotifications(status: .allowed, pending: ["ready:\(hash):19000001"])
        let center = makeCenter(notifications)
        let saved = AlertFixtures.snapshot(loans: [AlertFixtures.loan("3", "The Love Library", due: "20 Oct 2026")])
        await center.setDue(true, origin: origin, saved: saved)
        #expect(await notifications.pendingIDs() == ["due:\(hash):2026-10-20", "ready:\(hash):19000001"])
        await center.setDue(false, origin: origin, saved: saved)
        #expect(!center.dueEnabled)
        #expect(await notifications.pendingIDs() == ["ready:\(hash):19000001"])
        // Off: a refresh schedules nothing.
        await notifications.clearAdded()
        await center.process(saved, origin: origin)
        #expect(await notifications.added.isEmpty)
    }

    // MARK: Morning check

    @Test func theMorningCheckIsScheduledOnlyWhileSignedInWithASwitchOn() async {
        let morningCheck = FakeMorningCheck()
        let center = makeCenter(FakeNotifications(status: .allowed), morningCheck: morningCheck,
                                now: AlertFixtures.date(2026, 10, 13, 10, 0))
        center.scheduleMorningCheck(signedIn: true)
        #expect(morningCheck.last == .cancel)
        await center.setReady(true, origin: origin, saved: nil)
        center.scheduleMorningCheck(signedIn: true)
        #expect(morningCheck.last == .submit(AlertFixtures.date(2026, 10, 14, 7, 45)))
        center.scheduleMorningCheck(signedIn: false)
        #expect(morningCheck.last == .cancel)
        await center.setReady(false, origin: origin, saved: nil)
        await center.setDue(true, origin: origin, saved: nil)
        center.scheduleMorningCheck(signedIn: true)
        #expect(morningCheck.last == .submit(AlertFixtures.date(2026, 10, 14, 7, 45)))
    }

    // MARK: Sign-out

    @Test func forgettingALibraryDeletesItsKeysAndNotificationsOnly() async {
        let otherHash = AlertContent.originHash("https://hutt.spydus.co.nz")
        let notifications = FakeNotifications(status: .allowed,
                                              pending: ["due:\(hash):2026-10-20", "due:\(otherHash):2026-10-20"],
                                              delivered: ["ready:\(hash):19000001", "ready:\(otherHash):19000001"])
        let center = makeCenter(notifications)
        await center.setReady(true, origin: origin, saved: AlertFixtures.snapshot(ready: [midnight]))
        await center.process(AlertFixtures.snapshot(ready: [midnight]), origin: "https://hutt.spydus.co.nz")
        await center.forget(origin: origin).value
        #expect(center.alertedKeys(originHash: hash) == nil)
        #expect(center.alertedKeys(originHash: otherHash) == ["19000001"])
        #expect(await notifications.pendingIDs() == ["due:\(otherHash):2026-10-20"])
        #expect(await notifications.deliveredIDs().filter { $0.contains(hash) }.isEmpty)
        #expect(await notifications.deliveredIDs().contains("ready:\(otherHash):19000001"))
        #expect(center.readyEnabled)
    }

    @Test func demoModeKeepsItsOwnSwitchesAndResetClearsBoth() async {
        let defaults = UserDefaults(suiteName: "LibreSpydusTests-\(UUID().uuidString)")!
        let calendar = perth
        let system = AlertSystem(notifications: FakeNotifications(), morningCheck: FakeMorningCheck(), calendar: { calendar })
        let demo = AlertCenter(defaults: defaults, system: system, namespace: AlertCenter.demoNamespace)
        await demo.setReady(true, origin: origin, saved: AlertFixtures.snapshot(ready: [midnight]))
        #expect(!AlertCenter(defaults: defaults, system: system).readyEnabled)
        #expect(AlertCenter(defaults: defaults, system: system).alertedKeys(originHash: hash) == nil)
        #expect(defaults.stringArray(forKey: "alerts.demo.readyKeys.\(hash)") == ["19000001"])
        AlertCenter.deleteAll(in: defaults)
        #expect(!AlertCenter(defaults: defaults, system: system, namespace: AlertCenter.demoNamespace).readyEnabled)
        #expect(defaults.dictionaryRepresentation().keys.filter { $0.hasPrefix("alerts.") }.isEmpty)
    }
}

// MARK: - The app: AppModel, the demo account and the morning check

@MainActor
struct AlertAppModelTests {
    private func makeModel(notifications: FakeNotifications, morningCheck: FakeMorningCheck,
                           addsReady: Bool = true) -> AppModel {
        let calendar = AlertFixtures.calendar()
        let dependencies = AppModel.Dependencies(
            defaults: UserDefaults(suiteName: "LibreSpydusTests-\(UUID().uuidString)")!,
            sessions: InMemorySecretStore(), verificationCookies: InMemorySecretStore(),
            favouritesDirectory: TestData.temporaryDirectory(), isDemo: true, accountCacheDirectory: nil,
            makeBackend: { origin in
                let fixtures = try! DemoFixtures.bundled()
                return DemoBackend(publicBackend: FakeBackend(origin: origin ?? AlertFixtures.origin), fixtures: fixtures,
                                   mutationDelay: .zero, snapshotDelay: .zero, addsReadyOnNextRefresh: addsReady)
            },
            alertSystem: AlertSystem(notifications: notifications, morningCheck: morningCheck, calendar: { calendar })
        )
        let model = AppModel(dependencies: dependencies)
        model.selectLibrary(.canning)
        return model
    }

    @Test func demoMorningCheckAlertsTheNewlyReadyReservation() async {
        let notifications = FakeNotifications()
        let morningCheck = FakeMorningCheck()
        let model = makeModel(notifications: notifications, morningCheck: morningCheck)
        #expect(model.isSignedIn)
        await model.refreshAccount()
        await model.setReadyAlerts(true)
        #expect(model.alerts.alertedKeys(originHash: AlertFixtures.hash) == ["19000001"])
        if case .submit = morningCheck.last {} else { Issue.record("morning check not scheduled: \(morningCheck.events)") }

        await model.runMorningCheck()
        #expect(await notifications.added == [AlertNotification(
            id: "ready:\(AlertFixtures.hash):\(DemoFixtures.newReadyIrn)", title: "Ready to collect",
            body: "Hollow bones · Riverton Library",
            payload: AlertPayload(kind: .ready, origin: AlertFixtures.origin, keys: [DemoFixtures.newReadyIrn]))])
        #expect(model.account.snapshot?.reservations.ready.items.count == 2)
        if case .submit = morningCheck.last {} else { Issue.record("next morning check not scheduled") }
        // A foreground refresh afterwards: nothing new.
        await model.refreshAccount()
        #expect(await notifications.added.count == 1)
    }

    @Test func everyFetchedSnapshotIsProcessed() async {
        let notifications = FakeNotifications()
        let model = makeModel(notifications: notifications, morningCheck: FakeMorningCheck(), addsReady: false)
        // Ready on before anything was loaded: the first refresh remembers what's waiting.
        await model.setReadyAlerts(true)
        await model.setDueAlerts(true)
        await model.refreshAccount()
        #expect(model.alerts.alertedKeys(originHash: AlertFixtures.hash) == ["19000001"])
        let added = await notifications.added
        #expect(added.filter { $0.id.hasPrefix("ready:") }.isEmpty)
        // Demo loans are due in −2, 2, 5, 12 and 19 days; whichever reminders' 07:45 is ahead are scheduled.
        let due = added.filter { $0.id.hasPrefix("due:") }
        #expect(due.count >= 3)
        #expect(due.allSatisfy { $0.at?.hour == 7 && $0.at?.minute == 45 })
    }

    @Test func signingOutForgetsTheKeysAndCancelsTheMorningCheck() async {
        let notifications = FakeNotifications()
        let morningCheck = FakeMorningCheck()
        let model = makeModel(notifications: notifications, morningCheck: morningCheck)
        await model.refreshAccount()
        await model.setReadyAlerts(true)
        #expect(model.alerts.alertedKeys(originHash: AlertFixtures.hash) != nil)
        model.signOut()
        #expect(model.alerts.alertedKeys(originHash: AlertFixtures.hash) == nil)
        #expect(morningCheck.last == .cancel)
        // Signed out: the morning check does nothing but keep it cancelled.
        await model.runMorningCheck()
        #expect(await notifications.added.isEmpty)
        #expect(morningCheck.last == .cancel)

        // Signed in again: what is waiting (now with "Hollow bones" too) isn't announced as new.
        try? await model.signIn(username: "demo", password: "demo")
        #expect(model.isSignedIn)
        if case .submit = morningCheck.last {} else { Issue.record("morning check not scheduled after sign-in") }
        #expect(model.account.snapshot?.reservations.ready.items.count == 2)
        #expect(model.alerts.alertedKeys(originHash: AlertFixtures.hash) == ["19000001", "19000003"])
        await model.runMorningCheck()
        #expect(await notifications.added.isEmpty)
    }

    @Test func waitAtMostGivesUpAtTheLimitOrOnCancellation() async {
        #expect(await waitAtMost(.seconds(5)) { 42 } == 42)
        let clock = ContinuousClock()
        var start = clock.now
        let slow = await waitAtMost(.milliseconds(100)) { () -> Int in
            try? await Task.sleep(for: .seconds(2))
            return 1
        }
        #expect(slow == nil)
        #expect(clock.now - start < .seconds(1))
        start = clock.now
        let waiter = Task { @MainActor in
            await waitAtMost(.seconds(30)) { () -> Int in
                try? await Task.sleep(for: .seconds(2))
                return 1
            }
        }
        try? await Task.sleep(for: .milliseconds(50))
        waiter.cancel()
        #expect(await waiter.value == nil)
        #expect(clock.now - start < .seconds(1))
    }

    @Test func launchHooksParse() {
        let options = LaunchOptions.parse(["app", "-LSMorningCheckNow", "YES", "-LSDemoNewReady", "true",
                                           "-LSAlertTime", "14:05"])
        #expect(options.morningCheckNow)
        #expect(options.demoNewReady)
        #expect(options.alertTime == DateComponents(hour: 14, minute: 5))
        #expect(LaunchOptions.parse(["-LSAlertTime", "25:00"]).alertTime == nil)
        #expect(LaunchOptions.parse(["-LSAlertTime", "7.45"]).alertTime == nil)
        #expect(!LaunchOptions.parse([]).morningCheckNow)
        #expect(LaunchOptions.parse(["-LSDemoAlert", "due-several"]).demoAlert == .dueSeveral)
        #expect(LaunchOptions.parse(["-LSDemoAlert", "Ready"]).demoAlert == .ready)
        #expect(LaunchOptions.parse(["-LSDemoAlert", "overdue"]).demoAlert == nil)
    }

    @Test func demoNewReadyArrivesOnTheSecondRefresh() async throws {
        let demo = DemoBackend(publicBackend: FakeBackend(), fixtures: try DemoFixtures.bundled(), snapshotDelay: .zero,
                               addsReadyOnNextRefresh: true)
        let first = try await demo.accountSnapshot(session: DemoBackend.session)
        #expect(first.reservations.ready.items.map(\.irn) == ["19000001"])
        let second = try await demo.accountSnapshot(session: DemoBackend.session)
        #expect(second.reservations.ready.items.map(\.irn) == ["19000001", "19000003"])
        #expect(second.reservations.ready.items[1].fields["Status"]?.hasPrefix("Please pickup from: RIVERTON LIBRARY by ") == true)
        #expect(!second.reservations.pending.items.contains { $0.irn == "19000003" })
        let third = try await demo.accountSnapshot(session: DemoBackend.session)
        #expect(third.reservations.ready.items.count == 2)
    }
}
