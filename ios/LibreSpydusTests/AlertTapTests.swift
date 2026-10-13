import Foundation
import LibreSpydusCore
import Testing
@testable import LibreSpydus

/// Tapping an alert opens the book page over Your Books (docs/native-apps.md §5.8 Tap): what each
/// notification carries, how a tap is read back, and where `AppModel.openAlert` goes.
@MainActor
struct AlertTapTests {
    private let origin = AlertFixtures.origin
    private let hash = AlertFixtures.hash
    private let other = "https://hutt.spydus.co.nz"

    /// The demo account's loans (each due on a different day) and reservations: real-looking items
    /// with bib IRNs and URLs, like a saved snapshot.
    private func snapshot() throws -> AccountSnapshot {
        let fixtures = try DemoFixtures.bundled()
        return AccountSnapshot(version: DemoBackend.snapshotVersion, accountName: "Borrower, Test",
                               loans: PatronList(total: Int64(fixtures.loans.count), items: fixtures.loans),
                               reservations: Reservations(ready: PatronList(total: Int64(fixtures.ready.count), items: fixtures.ready),
                                                          pending: PatronList(total: Int64(fixtures.pending.count), items: fixtures.pending)),
                               fetchedAtMs: 0)
    }

    private func reminder(_ loans: [PatronItem]) -> DueReminder {
        DueReminder(key: "due:2026-10-20", due: AlertFixtures.day(2026, 10, 20), remindOn: AlertFixtures.day(2026, 10, 18), items: loans)
    }

    // MARK: Payload

    @Test func aReadyAlertCarriesItsKindLibraryAndReservationKey() throws {
        let midnight = try #require(try snapshot().reservations.ready.items.first)
        let notification = AlertContent.readyNotification(midnight, origin: origin)
        #expect(notification.id == "ready:\(hash):19000001")
        #expect(notification.payload == AlertPayload(kind: .ready, origin: origin, keys: ["19000001"]))
        let userInfo = notification.payload.userInfo
        #expect(userInfo["kind"] as? String == "ready")
        #expect(userInfo["origin"] as? String == origin)
        #expect(userInfo["keys"] as? [String] == ["19000001"])
    }

    @Test func aDueReminderCarriesEveryLoanItNames() throws {
        let loans = try snapshot().loans.items
        let one = AlertContent.dueNotification(reminder([loans[1]]), origin: origin)
        #expect(one.payload == AlertPayload(kind: .due, origin: origin, keys: ["18000002"]))
        #expect(one.payload.userInfo["kind"] as? String == "due")
        let several = AlertContent.dueNotification(reminder(Array(loans[1...3])), origin: origin)
        #expect(several.payload == AlertPayload(kind: .due, origin: origin, keys: ["18000002", "18000003", "18000004"]))
        #expect(several.payload.userInfo["keys"] as? [String] == ["18000002", "18000003", "18000004"])
        #expect(several.body == "Whistler and 2 more")
    }

    @Test func thePayloadReadsBackFromUserInfo() {
        let payload = AlertPayload(kind: .due, origin: origin, keys: ["18000002", "bib:19243435"])
        // As the notification centre hands it back: [AnyHashable: Any].
        let stored: [AnyHashable: Any] = payload.userInfo.reduce(into: [:]) { $0[AnyHashable($1.key)] = $1.value }
        #expect(AlertPayload(userInfo: stored) == payload)
        #expect(AlertPayload(userInfo: [:]) == nil)
        #expect(AlertPayload(userInfo: ["kind": "overdue", "origin": origin]) == nil)
        #expect(AlertPayload(userInfo: ["kind": "ready", "origin": origin]) == AlertPayload(kind: .ready, origin: origin, keys: []))
    }

    // MARK: Reading a tap

    @Test func aTapIsReadFromThePayload() {
        let payload = AlertPayload(kind: .ready, origin: origin, keys: ["19000001"])
        let tap = AlertTap(notificationID: "ready:\(hash):19000001", userInfo: payload.userInfo)
        #expect(tap == AlertTap(kind: .ready, originHash: hash, keys: ["19000001"]))
        #expect(tap?.segment == .reservations)
        // The payload wins over the ID.
        let due = AlertTap(notificationID: "due:\(hash):2026-10-20",
                           userInfo: AlertPayload(kind: .due, origin: other, keys: ["1", "2"]).userInfo)
        #expect(due == AlertTap(kind: .due, originHash: AlertContent.originHash(other), keys: ["1", "2"]))
        #expect(due?.segment == .loans)
    }

    @Test func anOlderBuildsNotificationIsReadFromItsID() {
        // Ready: the key is in the ID, colons and all.
        #expect(AlertTap(notificationID: "ready:\(hash):19000001", userInfo: [:])
                == AlertTap(kind: .ready, originHash: hash, keys: ["19000001"]))
        #expect(AlertTap(notificationID: "ready:\(hash):title:Hollow bones: a novel", userInfo: [:])
                == AlertTap(kind: .ready, originHash: hash, keys: ["title:Hollow bones: a novel"]))
        // Due: the ID names a date, not its loans.
        #expect(AlertTap(notificationID: "due:\(hash):2026-10-20", userInfo: [:])
                == AlertTap(kind: .due, originHash: hash, keys: []))
        #expect(AlertTap(notificationID: "something-else", userInfo: [:]) == nil)
        #expect(AlertTap(notificationID: "ready:\(hash)", userInfo: [:]) == nil)
        #expect(AlertTap(notificationID: "overdue:\(hash):1", userInfo: [:]) == nil)
    }

    // MARK: Which book

    @Test func theBookIsTheNamedRowsOwnRef() throws {
        let snapshot = try snapshot()
        let ready = AlertTap(kind: .ready, originHash: hash, keys: ["19000001"])
        let midnight = ReservationSections(snapshot.reservations).ready[0]
        #expect(AlertContent.book(for: ready, in: snapshot) == midnight.ref)
        #expect(midnight.ref.url == patronItemRecordUrl(item: midnight.item))
        // A pending reservation (alerted, then back in the queue) is still in Your Books.
        #expect(AlertContent.book(for: AlertTap(kind: .ready, originHash: hash, keys: ["19000003"]), in: snapshot)?.title == "Hollow bones")
        let whistler = try #require(LoanRow.rows(from: snapshot.loans.items).first { $0.item.irn == "18000002" })
        #expect(AlertContent.book(for: AlertTap(kind: .due, originHash: hash, keys: ["18000002"]), in: snapshot) == whistler.ref)
    }

    @Test func noBookForSeveralLoansAnUnknownKeyOrNone() throws {
        let snapshot = try snapshot()
        #expect(AlertContent.book(for: AlertTap(kind: .due, originHash: hash, keys: ["18000002", "18000003"]), in: snapshot) == nil)
        #expect(AlertContent.book(for: AlertTap(kind: .ready, originHash: hash, keys: ["19999999"]), in: snapshot) == nil)
        #expect(AlertContent.book(for: AlertTap(kind: .due, originHash: hash, keys: []), in: snapshot) == nil)
        #expect(AlertContent.book(for: AlertTap(kind: .ready, originHash: hash, keys: [""]), in: snapshot) == nil)
        // A loan's key doesn't open it from a ready alert, nor a reservation's from a due reminder.
        #expect(AlertContent.book(for: AlertTap(kind: .ready, originHash: hash, keys: ["18000002"]), in: snapshot) == nil)
        #expect(AlertContent.book(for: AlertTap(kind: .due, originHash: hash, keys: ["19000001"]), in: snapshot) == nil)
    }

    // MARK: Routing

    private func makeModel(snapshot: AccountSnapshot) -> AppModel {
        let cache = TestData.temporaryDirectory()
        try? AccountSnapshotCache(directory: cache).save(snapshot, origin: origin)
        let model = AppModel(dependencies: .testing(sessions: InMemorySecretStore([origin: "session"]), accountCache: cache))
        model.selectLibrary(.canning)
        return model
    }

    @Test func aReadyAlertOpensTheBookOverReservations() throws {
        let snapshot = try snapshot()
        let model = makeModel(snapshot: snapshot)
        model.selectedTab = .home
        model.push(.detail(BookRef(url: "https://canning.spydus.com/other")), on: .home)
        model.openAlert(AlertTap(AlertContent.readyNotification(snapshot.reservations.ready.items[0], origin: origin).payload))
        #expect(model.selectedTab == .borrowed)
        #expect(model.pendingBorrowedSegment == .reservations)
        #expect(model.path(for: .borrowed) == [.detail(ReservationSections(snapshot.reservations).ready[0].ref)])
        // The other tab keeps its stack.
        #expect(model.path(for: .home).count == 1)
    }

    @Test func aOneLoanReminderOpensTheBookOverLoans() throws {
        let snapshot = try snapshot()
        let model = makeModel(snapshot: snapshot)
        // Whatever Your Books showed before goes: Back lands on the list.
        model.push(.detail(BookRef(url: "https://canning.spydus.com/other")), on: .borrowed)
        let whistler = snapshot.loans.items[1]
        model.openAlert(AlertTap(AlertContent.dueNotification(reminder([whistler]), origin: origin).payload))
        #expect(model.selectedTab == .borrowed)
        #expect(model.pendingBorrowedSegment == .loans)
        #expect(model.path(for: .borrowed) == [.detail(LoanRow(item: whistler).ref)])
    }

    @Test func fallsBackToTheList() throws {
        let snapshot = try snapshot()
        let model = makeModel(snapshot: snapshot)
        let cases: [(AlertTap, BorrowedViewModel.Segment)] = [
            // Several loans.
            (AlertTap(AlertPayload(kind: .due, origin: origin, keys: ["18000002", "18000003"])), .loans),
            // Collected or returned: no longer in the saved account.
            (AlertTap(AlertPayload(kind: .ready, origin: origin, keys: ["19999999"])), .reservations),
            (AlertTap(AlertPayload(kind: .due, origin: origin, keys: ["18999999"])), .loans),
            // Another library's alert, though the key matches.
            (AlertTap(AlertPayload(kind: .ready, origin: other, keys: ["19000001"])), .reservations),
            // An older build's due reminder: no keys.
            (try #require(AlertTap(notificationID: "due:\(hash):2026-10-20", userInfo: [:])), .loans),
        ]
        for (tap, segment) in cases {
            model.push(.detail(BookRef(url: "https://canning.spydus.com/other")), on: .borrowed)
            model.openAlert(tap)
            #expect(model.selectedTab == .borrowed)
            #expect(model.path(for: .borrowed).isEmpty, "\(tap)")
            #expect(model.takePendingBorrowedSegment() == segment)
        }
    }

    @Test func anOlderBuildsReadyAlertStillOpensItsBook() throws {
        let snapshot = try snapshot()
        let model = makeModel(snapshot: snapshot)
        model.openAlert(try #require(AlertTap(notificationID: "ready:\(hash):19000001", userInfo: [:])))
        #expect(model.path(for: .borrowed) == [.detail(ReservationSections(snapshot.reservations).ready[0].ref)])
    }

    @Test func signedOutItOpensTheList() throws {
        let model = makeModel(snapshot: try snapshot())
        model.signOut()
        model.openAlert(AlertTap(AlertPayload(kind: .ready, origin: origin, keys: ["19000001"])))
        #expect(model.selectedTab == .borrowed)
        #expect(model.path(for: .borrowed).isEmpty)
    }

    /// A tap that launches the app: the router is installed after the model, whose `init` has
    /// already read the saved account from disk, so the first tap finds the book without a fetch.
    @Test func aColdLaunchReadsTheSavedAccountBeforeRouting() throws {
        let snapshot = try snapshot()
        let defaults = UserDefaults(suiteName: "LibreSpydusTests-\(UUID().uuidString)")!
        let sessions = InMemorySecretStore([origin: "session"])
        let cache = TestData.temporaryDirectory()
        try AccountSnapshotCache(directory: cache).save(snapshot, origin: origin)
        AppModel(dependencies: .testing(defaults: defaults, sessions: sessions, accountCache: cache)).selectLibrary(.canning)

        // The relaunch: nothing fetched (FakeBackend would bring an empty account).
        var backends: [FakeBackend] = []
        let launched = AppModel(dependencies: .testing(defaults: defaults, sessions: sessions, accountCache: cache) {
            backends.append($0)
        })
        #expect(launched.account.snapshot == snapshot)
        launched.openAlert(AlertTap(AlertPayload(kind: .due, origin: origin, keys: ["18000003"])))
        #expect(launched.selectedTab == .borrowed)
        #expect(launched.path(for: .borrowed) == [.detail(LoanRow(item: snapshot.loans.items[2]).ref)])
        #expect(backends.count == 1)
        #expect(!launched.account.isRefreshing)
    }
}
