import CryptoKit
import Foundation
import LibreSpydusCore

/// What the alerts say, their identifiers and when they show (docs/native-apps.md §5.8). Pure (no
/// I/O), so the wording and the timing are unit-tested in AlertContentTests; Android (A7) says the
/// same.
enum AlertContent {
    /// The morning alert time, local: due reminders show then and the morning check runs from then.
    static let alertTime = DateComponents(hour: 7, minute: 45)
    /// Due reminders come this many days before the due date.
    static let dueDaysBefore: UInt32 = 2

    // MARK: Ready to collect

    static let readyTitle = Copy.Alerts.readyTitle

    /// "<Title> · <Branch>", the branch as Your Books shows it for the row (`ReservationRow.pickup`:
    /// the pickup location, else the branch in Canning's status text, title-cased when shouted);
    /// just the title when the branch is unknown; "A reservation" without a title.
    static func readyBody(_ item: PatronItem) -> String {
        let title = item.title.nonEmpty ?? Copy.Alerts.untitledReservation
        guard let branch = ReservationRow(item: item, kind: .ready).pickup else { return title }
        return Copy.Alerts.readyBody(title: title, branch: branch)
    }

    // MARK: Due-date reminders

    /// "Due Thursday 16 Oct" (English weekday and short month, Gregorian); "Due today" when the
    /// reminder shows on the due date, "Due tomorrow" the day before.
    static func dueTitle(due: SimpleDate, shownOn day: SimpleDate) -> String {
        let calendar = gregorian
        let formatter = DateFormatter()
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.calendar = calendar
        formatter.timeZone = calendar.timeZone
        guard let dueDate = date(due, calendar: calendar), let shown = date(day, calendar: calendar) else {
            // Not a calendar date: its day and month, in the same "16 Oct" style.
            let month = formatter.shortMonthSymbols[(Int(due.month) + 11) % 12]
            return Copy.Loans.dueOn(date: "\(due.day) \(month)")
        }
        switch calendar.dateComponents([.day], from: shown, to: dueDate).day {
        case 0: return Copy.Loans.dueToday
        case 1: return Copy.Loans.dueTomorrow
        default:
            formatter.dateFormat = "EEEE d MMM"
            return Copy.Loans.dueOn(date: formatter.string(from: dueDate))
        }
    }

    /// One loan: its title; two: "<A> and <B>"; more: "<A> and N more". A loan without a title
    /// is "A loan".
    static func dueBody(_ items: [PatronItem]) -> String {
        let titles = items.map { $0.title.nonEmpty ?? Copy.Alerts.untitledLoan }
        switch titles.count {
        case 0: return ""
        case 1: return titles[0]
        case 2: return Copy.Alerts.dueTwo(first: titles[0], second: titles[1])
        default: return Copy.Alerts.dueMore(count: titles.count - 1, first: titles[0])
        }
    }

    // MARK: Notifications

    /// The "Ready to collect" notification for `item`, a newly ready reservation of `origin`'s
    /// account; shown at once.
    static func readyNotification(_ item: PatronItem, origin: String) -> AlertNotification {
        let key = patronItemKey(item: item)
        return AlertNotification(id: readyID(originHash: originHash(origin), key: key), title: readyTitle,
                                 body: readyBody(item), payload: AlertPayload(kind: .ready, origin: origin, keys: [key]))
    }

    /// The due-date reminder for `reminder`'s loans of `origin`'s account, at `time` on its
    /// `remindOn`. It names every loan due that day.
    static func dueNotification(_ reminder: DueReminder, origin: String, time: DateComponents = alertTime) -> AlertNotification {
        AlertNotification(id: dueID(originHash: originHash(origin), due: reminder.due),
                          title: dueTitle(due: reminder.due, shownOn: reminder.remindOn), body: dueBody(reminder.items),
                          at: triggerComponents(on: reminder.remindOn, time: time),
                          payload: AlertPayload(kind: .due, origin: origin, keys: reminder.items.map { patronItemKey(item: $0) }))
    }

    // MARK: Identifiers

    /// sha256(origin) in hex, as the account cache names its files: says nothing about the library.
    static func originHash(_ origin: String) -> String {
        SHA256.hash(data: Data(origin.utf8)).map { String(format: "%02x", $0) }.joined()
    }

    /// `ready:<originHash>:<patronItemKey>`.
    static func readyID(originHash: String, key: String) -> String {
        "ready:\(originHash):\(key)"
    }

    /// `due:<originHash>:<YYYY-MM-DD>`.
    static func dueID(originHash: String, due: SimpleDate) -> String {
        "due:\(originHash):\(String(format: "%04d-%02d-%02d", due.year, due.month, due.day))"
    }

    /// Every notification of one library, ready and due.
    static func belongs(_ id: String, toOriginHash hash: String) -> Bool {
        id.hasPrefix("ready:\(hash):") || id.hasPrefix("due:\(hash):")
    }

    // MARK: Tap

    /// The book page a tap on an alert opens over its Your Books list: the one reservation or loan
    /// it names, found by `patronItemKey` among the rows Your Books shows for `snapshot` and opened
    /// with that row's own `BookRef`, as tapping the row does. Nil (just the list) when it names
    /// several loans or none, or the item is no longer in the account (collected, returned).
    static func book(for tap: AlertTap, in snapshot: AccountSnapshot) -> BookRef? {
        guard tap.keys.count == 1, let key = tap.keys.first, !key.isEmpty else { return nil }
        switch tap.kind {
        case .ready:
            let sections = ReservationSections(snapshot.reservations)
            return (sections.ready + sections.pending).first { patronItemKey(item: $0.item) == key }?.ref
        case .due:
            return LoanRow.rows(from: snapshot.loans.items).first { patronItemKey(item: $0.item) == key }?.ref
        }
    }

    // MARK: Time

    /// The next `time` (07:45) strictly after `now` in `calendar`'s time zone: today's while it's
    /// still ahead, else tomorrow's. `Calendar` keeps it at 07:45 local across a DST change.
    static func nextAlertDate(after now: Date, calendar: Calendar, time: DateComponents = alertTime) -> Date {
        calendar.nextDate(after: now, matching: DateComponents(hour: time.hour ?? 7, minute: time.minute ?? 45),
                          matchingPolicy: .nextTime)
            ?? now.addingTimeInterval(24 * 60 * 60)
    }

    /// The local date and time a reminder for `day` shows: a calendar trigger's components,
    /// Gregorian whatever the device's calendar, in the device's time zone.
    static func triggerComponents(on day: SimpleDate, time: DateComponents = alertTime) -> DateComponents {
        DateComponents(calendar: Calendar(identifier: .gregorian), year: Int(day.year), month: Int(day.month),
                       day: Int(day.day), hour: time.hour ?? 7, minute: time.minute ?? 45)
    }

    /// `triggerComponents(on:)` as a moment in `calendar`'s time zone.
    static func alertDate(on day: SimpleDate, calendar: Calendar, time: DateComponents = alertTime) -> Date? {
        var components = triggerComponents(on: day, time: time)
        components.calendar = nil
        return calendar.date(from: components)
    }

    /// `date`'s local day in `calendar` (Gregorian, the device's time zone).
    static func simpleDate(_ date: Date, calendar: Calendar) -> SimpleDate {
        let parts = calendar.dateComponents([.year, .month, .day], from: date)
        return SimpleDate(year: Int32(parts.year ?? 1970), month: UInt8(parts.month ?? 1), day: UInt8(parts.day ?? 1))
    }

    // MARK: Private

    /// Gregorian in UTC: day arithmetic on calendar dates, free of DST.
    private static var gregorian: Calendar {
        var calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = TimeZone(identifier: "UTC") ?? .gmt
        return calendar
    }

    private static func date(_ day: SimpleDate, calendar: Calendar) -> Date? {
        calendar.date(from: DateComponents(year: Int(day.year), month: Int(day.month), day: Int(day.day)))
    }
}
