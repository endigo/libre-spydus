import Foundation
import LibreSpydusCore

/// Small display helpers shared by every screen.
enum Formatting {
    /// "Haig, Matt, author" → "Matt Haig"; "Galligan, Gale, 1964- illustrator" → "Gale Galligan";
    /// "Disney Enterprises" → unchanged. Nil/blank → nil.
    static func authorName(_ raw: String?) -> String? {
        guard let raw = raw.nonEmpty else { return nil }
        let parts = raw.split(separator: ",").map { $0.trimmingCharacters(in: .whitespaces) }.filter { !$0.isEmpty }
        guard let surname = parts.first else { return nil }
        if parts.count >= 2, isGivenName(parts[1]) {
            return "\(parts[1]) \(surname)"
        }
        return surname
    }

    /// "By Matt Haig", or nil without an author.
    static func byline(_ rawAuthor: String?) -> String? {
        authorName(rawAuthor).map { Copy.Detail.byline(author: $0) }
    }

    /// "Test Borrower" → "Test"; "Borrower, Test" → "Test".
    static func firstName(_ accountName: String?) -> String? {
        guard let name = accountName.nonEmpty else { return nil }
        if let comma = name.firstIndex(of: ",") {
            let given = name[name.index(after: comma)...].trimmingCharacters(in: .whitespaces)
            return given.split(separator: " ").first.map(String.init) ?? String(name[..<comma])
        }
        return name.split(separator: " ").first.map(String.init)
    }

    /// Home greeting: "Hello Test!" signed in, else "Hello!".
    static func greeting(accountName: String?) -> String {
        firstName(accountName).map { Copy.Home.greeting(name: $0) } ?? Copy.Home.greetingSignedOut
    }

    /// "1 result" / "1,234 results".
    static func resultCount(_ total: Int64?) -> String? {
        guard let total else { return nil }
        return Copy.Search.results(count: Int(clamping: total))
    }

    /// "27 Oct"; "27 Oct 2027" when it isn't this year (docs/native-apps.md §7.1).
    static func shortDate(_ date: Date, now: Date = .now, calendar: Calendar = .current) -> String {
        let sameYear = calendar.component(.year, from: date) == calendar.component(.year, from: now)
        let style = Date.FormatStyle(calendar: calendar, timeZone: calendar.timeZone).day().month(.abbreviated)
        return date.formatted(sameYear ? style : style.year())
    }

    /// When the saved account was fetched, relative to `now`: "Updated just now",
    /// "Updated 12 min ago", "Updated today at 9:05 am", "Updated yesterday at 4:20 pm",
    /// "Updated Monday at 4:20 pm" (within a week), "Updated 3 Oct at 4:20 pm" (time and date in
    /// the device's locale; the year only when it isn't this year).
    static func updated(_ date: Date, now: Date = .now, calendar: Calendar = .current, locale: Locale = .current) -> String {
        let seconds = now.timeIntervalSince(date)
        if seconds < 60 { return Copy.YourBooks.updatedJustNow }
        if seconds < 60 * 60 { return Copy.YourBooks.updatedMinutesAgo(count: Int(seconds / 60)) }
        let style = Date.FormatStyle(locale: locale, calendar: calendar, timeZone: calendar.timeZone)
        let time = date.formatted(style.hour().minute())
        let days = calendar.dateComponents([.day], from: calendar.startOfDay(for: date),
                                           to: calendar.startOfDay(for: now)).day ?? Int.max
        switch days {
        case 0: return Copy.YourBooks.updatedToday(time: time)
        case 1: return Copy.YourBooks.updatedYesterday(time: time)
        case 2..<7: return Copy.YourBooks.updatedOn(day: date.formatted(style.weekday(.wide)), time: time)
        default:
            let sameYear = calendar.component(.year, from: date) == calendar.component(.year, from: now)
            let day = date.formatted(sameYear ? style.day().month(.abbreviated) : style.day().month(.abbreviated).year())
            return Copy.YourBooks.updatedOn(day: day, time: time)
        }
    }

    private static func isGivenName(_ part: String) -> Bool {
        guard let first = part.first, first.isLetter else { return false }
        let roles: Set<String> = [
            "author", "editor", "illustrator", "translator", "narrator", "compiler", "contributor",
            "performer", "director", "creator", "artist", "photographer", "composer", "writer",
        ]
        return !roles.contains(part.lowercased())
    }
}

extension String {
    /// The first letter capitalised, for a sentence that starts with a library's name, which may be
    /// `Copy.Common.yourLibrary` ("your library asks you…" → "Your library asks you…").
    var capitalisedFirst: String { prefix(1).uppercased() + dropFirst() }
}

extension Calendar {
    /// Library dates are Gregorian whatever the device calendar is (Buddhist, Japanese, …);
    /// in the device's time zone.
    static var library: Calendar {
        var calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = .current
        return calendar
    }
}

/// Due dates as the library writes them ("27 Oct 2026").
enum DueDate {
    /// The core's `parseDueDate` ("27 Oct 2026", "27/10/2026", "2026-10-27", …) as a local date.
    static func parse(_ text: String, calendar: Calendar = .library) -> Date? {
        guard let day = parseDueDate(text: text) else { return nil }
        return calendar.date(from: DateComponents(year: Int(day.year), month: Int(day.month), day: Int(day.day)))
    }

    /// The library's own format, "27 Oct 2026" (used by demo data).
    static func format(_ date: Date, calendar: Calendar = .library) -> String {
        let formatter = DateFormatter()
        formatter.locale = Locale(identifier: "en_AU_POSIX")
        formatter.calendar = calendar
        formatter.timeZone = calendar.timeZone
        formatter.dateFormat = "d MMM yyyy"
        return formatter.string(from: date)
    }
}

/// A loan's due state: "Due 27 Oct · 3 days left", urgent (≤ 3 days) or overdue.
struct DueStatus: Equatable, Sendable {
    var dueDate: Date
    /// Whole days from today; negative when overdue.
    var daysLeft: Int

    init(dueDate: Date, today: Date = .now, calendar: Calendar = .library) {
        self.dueDate = dueDate
        daysLeft = calendar.dateComponents([.day], from: calendar.startOfDay(for: today),
                                           to: calendar.startOfDay(for: dueDate)).day ?? 0
    }

    /// From `PatronItem.fields["Due"]`.
    init?(dueText: String?, today: Date = .now, calendar: Calendar = .library) {
        guard let text = dueText.nonEmpty, let date = DueDate.parse(text, calendar: calendar) else { return nil }
        self.init(dueDate: date, today: today, calendar: calendar)
    }

    var isOverdue: Bool { daysLeft < 0 }
    /// Danger colour: overdue or due within 3 days.
    var isUrgent: Bool { daysLeft <= 3 }

    /// "3 days left", "1 day left", "Due today", "2 days overdue".
    var daysText: String {
        switch daysLeft {
        case ..<0: Copy.Loans.daysOverdue(count: -daysLeft)
        case 0: Copy.Loans.dueToday
        default: Copy.Loans.daysLeft(count: daysLeft)
        }
    }

    /// "Due 27 Oct".
    var dueText: String { Copy.Loans.dueOn(date: Formatting.shortDate(dueDate)) }

    /// Borrowed card meta line: "Due 27 Oct · 3 days left".
    var metaLine: String { Copy.Loans.lineDays(date: Formatting.shortDate(dueDate), days: daysText) }
}
