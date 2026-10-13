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

    static let readyTitle = "Ready to collect"

    /// "<Title> · <Branch>", the branch as Your Books shows it for the row (`ReservationRow.pickup`:
    /// the pickup location, else the branch in Canning's status text, title-cased when shouted);
    /// just the title when the branch is unknown; "A reservation" without a title.
    static func readyBody(_ item: PatronItem) -> String {
        let title = item.title.nonEmpty ?? "A reservation"
        guard let branch = ReservationRow(item: item, kind: .ready).pickup else { return title }
        return "\(title) · \(branch)"
    }

    // MARK: Due-date reminders

    /// "Due Thursday 16 Oct" (English weekday and short month, Gregorian); "Due today" when the
    /// reminder shows on the due date, "Due tomorrow" the day before.
    static func dueTitle(due: SimpleDate, shownOn day: SimpleDate) -> String {
        let calendar = gregorian
        guard let dueDate = date(due, calendar: calendar), let shown = date(day, calendar: calendar) else {
            return "Due \(due.day)/\(due.month)"
        }
        switch calendar.dateComponents([.day], from: shown, to: dueDate).day {
        case 0: return "Due today"
        case 1: return "Due tomorrow"
        default:
            let formatter = DateFormatter()
            formatter.locale = Locale(identifier: "en_US_POSIX")
            formatter.calendar = calendar
            formatter.timeZone = calendar.timeZone
            formatter.dateFormat = "EEEE d MMM"
            return "Due \(formatter.string(from: dueDate))"
        }
    }

    /// One loan: its title; two: "<A> and <B>"; more: "<A> and N more". A loan without a title
    /// is "A loan".
    static func dueBody(_ items: [PatronItem]) -> String {
        let titles = items.map { $0.title.nonEmpty ?? "A loan" }
        switch titles.count {
        case 0: return ""
        case 1: return titles[0]
        case 2: return "\(titles[0]) and \(titles[1])"
        default: return "\(titles[0]) and \(titles.count - 1) more"
        }
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

    /// Where a tap on the notification `id` goes: Ready → Reservations (ready ones are listed
    /// first), Due → Loans.
    static func segment(forNotificationID id: String) -> BorrowedViewModel.Segment? {
        if id.hasPrefix("ready:") { return .reservations }
        if id.hasPrefix("due:") { return .loans }
        return nil
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
