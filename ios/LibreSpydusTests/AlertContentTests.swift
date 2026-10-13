import Foundation
import LibreSpydusCore
import Testing
@testable import LibreSpydus

/// Fixtures shared by the alert tests (no network, no notification centre).
enum AlertFixtures {
    static let origin = "https://canning.spydus.com"
    static var hash: String { AlertContent.originHash(origin) }

    static func ready(_ irn: String?, _ title: String?,
                      fields: [String: String] = ["Status": "Awaiting pickup", "Pickup location": "RIVERTON LIBRARY"]) -> PatronItem {
        PatronItem(irn: irn, bibIrn: nil, title: title, bibUrl: nil, author: nil, year: nil, coverUrls: [], isbn: nil,
                   fields: fields, actions: [])
    }

    static func loan(_ irn: String, _ title: String?, due: String) -> PatronItem {
        PatronItem(irn: irn, bibIrn: nil, title: title, bibUrl: nil, author: nil, year: nil, coverUrls: [], isbn: nil,
                   fields: ["Due": due], actions: [])
    }

    static func snapshot(ready: [PatronItem] = [], loans: [PatronItem] = []) -> AccountSnapshot {
        AccountSnapshot(version: DemoBackend.snapshotVersion, accountName: "Borrower, Test",
                        loans: PatronList(total: Int64(loans.count), items: loans),
                        reservations: Reservations(ready: PatronList(total: Int64(ready.count), items: ready),
                                                   pending: PatronList(total: 0, items: [])),
                        fetchedAtMs: 0)
    }

    static func calendar(_ zone: String = "Australia/Perth") -> Calendar {
        var calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = TimeZone(identifier: zone)!
        return calendar
    }

    static func date(_ year: Int, _ month: Int, _ day: Int, _ hour: Int = 0, _ minute: Int = 0,
                     in calendar: Calendar = calendar()) -> Date {
        calendar.date(from: DateComponents(year: year, month: month, day: day, hour: hour, minute: minute))!
    }

    static func day(_ year: Int32, _ month: UInt8, _ day: UInt8) -> SimpleDate {
        SimpleDate(year: year, month: month, day: day)
    }
}

struct AlertContentTests {
    // MARK: Ready

    @Test func readyBodyIsTitleAndTheBranchYourBooksShows() {
        // The pickup location, title-cased like the card.
        #expect(AlertContent.readyBody(AlertFixtures.ready("1", "The midnight library")) == "The midnight library · Riverton Library")
        // Canning's status text: "Please pickup from: <BRANCH> by <date>".
        let canning = AlertFixtures.ready("2", "Hollow bones",
                                          fields: ["Status": "Please pickup from: RIVERTON LIBRARY by 20 Oct 2026"])
        #expect(AlertContent.readyBody(canning) == "Hollow bones · Riverton Library")
        #expect(AlertContent.readyBody(canning) == "Hollow bones · \(ReservationRow(item: canning, kind: .ready).pickup!)")
    }

    @Test func readyBodyWithoutABranchOrTitle() {
        let noBranch = AlertFixtures.ready("1", "Whistler", fields: ["Status": "Awaiting pickup"])
        #expect(AlertContent.readyBody(noBranch) == "Whistler")
        #expect(AlertContent.readyBody(AlertFixtures.ready("2", "  ")) == "A reservation · Riverton Library")
        #expect(AlertContent.readyBody(AlertFixtures.ready("3", nil, fields: [:])) == "A reservation")
        #expect(AlertContent.readyTitle == "Ready to collect")
    }

    // MARK: Due

    @Test func dueTitleSaysTodayTomorrowOrTheWeekday() {
        let day = AlertFixtures.day
        #expect(AlertContent.dueTitle(due: day(2026, 10, 15), shownOn: day(2026, 10, 15)) == "Due today")
        #expect(AlertContent.dueTitle(due: day(2026, 10, 15), shownOn: day(2026, 10, 14)) == "Due tomorrow")
        #expect(AlertContent.dueTitle(due: day(2026, 10, 15), shownOn: day(2026, 10, 13)) == "Due Thursday 15 Oct")
        #expect(AlertContent.dueTitle(due: day(2026, 11, 1), shownOn: day(2026, 10, 30)) == "Due Sunday 1 Nov")
        // Across a year end and a DST change (Sydney, 4 Oct 2026) it's still whole days.
        #expect(AlertContent.dueTitle(due: day(2027, 1, 1), shownOn: day(2026, 12, 31)) == "Due tomorrow")
        #expect(AlertContent.dueTitle(due: day(2026, 10, 5), shownOn: day(2026, 10, 3)) == "Due Monday 5 Oct")
        #expect(AlertContent.dueTitle(due: day(2026, 9, 16), shownOn: day(2026, 9, 14)) == "Due Wednesday 16 Sep")
    }

    @Test func dueBodyNamesOneTwoOrTheFirstAndHowManyMore() {
        let loans = ["Whistler", "The Love Library", "Everything to the sea", "The secret thread"]
            .enumerated().map { AlertFixtures.loan("\($0.offset)", $0.element, due: "15 Oct 2026") }
        #expect(AlertContent.dueBody(Array(loans.prefix(1))) == "Whistler")
        #expect(AlertContent.dueBody(Array(loans.prefix(2))) == "Whistler and The Love Library")
        #expect(AlertContent.dueBody(Array(loans.prefix(3))) == "Whistler and 2 more")
        #expect(AlertContent.dueBody(loans) == "Whistler and 3 more")
        #expect(AlertContent.dueBody([AlertFixtures.loan("9", nil, due: "15 Oct 2026")]) == "A loan")
    }

    // MARK: Identifiers and taps

    @Test func identifiersCarryTheOriginHashLikeTheAccountCache() {
        let hash = AlertContent.originHash(AlertFixtures.origin)
        let cacheFile = AccountSnapshotCache(directory: URL(filePath: "/tmp")).fileURL(for: AlertFixtures.origin)
        #expect(cacheFile.lastPathComponent == "\(hash).json")
        #expect(hash.count == 64)
        #expect(AlertContent.readyID(originHash: hash, key: "19000001") == "ready:\(hash):19000001")
        #expect(AlertContent.dueID(originHash: hash, due: AlertFixtures.day(2026, 3, 5)) == "due:\(hash):2026-03-05")
        #expect(AlertContent.belongs("due:\(hash):2026-03-05", toOriginHash: hash))
        #expect(!AlertContent.belongs("due:\(AlertContent.originHash("https://other.spydus.com")):2026-03-05", toOriginHash: hash))
    }

    @Test func aTapOpensTheRightYourBooksList() {
        // More in AlertTapTests.
        #expect(AlertTap(notificationID: "ready:abc:19000001", userInfo: [:])?.segment == .reservations)
        #expect(AlertTap(notificationID: "due:abc:2026-10-15", userInfo: [:])?.segment == .loans)
        #expect(AlertTap(notificationID: "something-else", userInfo: [:]) == nil)
    }

    // MARK: Time

    @Test func nextAlertIsTodayBefore0745AndTomorrowFromThen() {
        let perth = AlertFixtures.calendar()
        let date = { AlertFixtures.date($0, $1, $2, $3, $4, in: perth) }
        #expect(AlertContent.nextAlertDate(after: date(2026, 10, 13, 6, 30), calendar: perth) == date(2026, 10, 13, 7, 45))
        #expect(AlertContent.nextAlertDate(after: date(2026, 10, 13, 7, 45), calendar: perth) == date(2026, 10, 14, 7, 45))
        #expect(AlertContent.nextAlertDate(after: date(2026, 10, 13, 11, 32), calendar: perth) == date(2026, 10, 14, 7, 45))
        #expect(AlertContent.nextAlertDate(after: date(2026, 12, 31, 23, 0), calendar: perth) == date(2027, 1, 1, 7, 45))
        // `-LSAlertTime 14:05`.
        #expect(AlertContent.nextAlertDate(after: date(2026, 10, 13, 11, 32), calendar: perth,
                                           time: DateComponents(hour: 14, minute: 5)) == date(2026, 10, 13, 14, 5))
    }

    @Test func nextAlertStays0745LocalAcrossDaylightSaving() {
        // Sydney springs forward at 02:00 on 4 Oct 2026: 23:00 → 07:45 is 7 h 45 min, not 8 h 45.
        let sydney = AlertFixtures.calendar("Australia/Sydney")
        let before = AlertFixtures.date(2026, 10, 3, 23, 0, in: sydney)
        let next = AlertContent.nextAlertDate(after: before, calendar: sydney)
        #expect(sydney.dateComponents([.year, .month, .day, .hour, .minute], from: next)
                == DateComponents(year: 2026, month: 10, day: 4, hour: 7, minute: 45))
        #expect(next.timeIntervalSince(before) == 7.75 * 3600)
        // New York falls back at 02:00 on 1 Nov 2026: 23:00 → 07:45 is 9 h 45 min.
        let newYork = AlertFixtures.calendar("America/New_York")
        let evening = AlertFixtures.date(2026, 10, 31, 23, 0, in: newYork)
        let morning = AlertContent.nextAlertDate(after: evening, calendar: newYork)
        #expect(newYork.dateComponents([.hour, .minute], from: morning) == DateComponents(hour: 7, minute: 45))
        #expect(morning.timeIntervalSince(evening) == 9.75 * 3600)
    }

    @Test func remindersTriggerAt0745OnTheirDayInGregorian() {
        let components = AlertContent.triggerComponents(on: AlertFixtures.day(2026, 10, 18))
        #expect(components.year == 2026 && components.month == 10 && components.day == 18)
        #expect(components.hour == 7 && components.minute == 45)
        #expect(components.calendar?.identifier == .gregorian)
        #expect(components.timeZone == nil)
        let perth = AlertFixtures.calendar()
        #expect(AlertContent.alertDate(on: AlertFixtures.day(2026, 10, 18), calendar: perth)
                == AlertFixtures.date(2026, 10, 18, 7, 45, in: perth))
        #expect(AlertContent.simpleDate(AlertFixtures.date(2026, 10, 13, 23, 59, in: perth), calendar: perth)
                == AlertFixtures.day(2026, 10, 13))
    }
}
