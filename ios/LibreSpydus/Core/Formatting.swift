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
        authorName(rawAuthor).map { "By \($0)" }
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
        firstName(accountName).map { "Hello \($0)!" } ?? "Hello!"
    }

    /// "1 result" / "1,234 results".
    static func resultCount(_ total: Int64?) -> String? {
        guard let total else { return nil }
        let number = total.formatted(.number.locale(Locale.current))
        return total == 1 ? "1 result" : "\(number) results"
    }

    /// "27 Oct".
    static func shortDate(_ date: Date) -> String {
        date.formatted(.dateTime.day().month(.abbreviated))
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

/// Due dates as the library writes them ("27 Oct 2026").
enum DueDate {
    /// The core's `parseDueDate` ("27 Oct 2026", "27/10/2026", "2026-10-27", …) as a local date.
    static func parse(_ text: String, calendar: Calendar = .current) -> Date? {
        guard let day = parseDueDate(text: text) else { return nil }
        return calendar.date(from: DateComponents(year: Int(day.year), month: Int(day.month), day: Int(day.day)))
    }

    /// The library's own format, "27 Oct 2026" (used by demo data).
    static func format(_ date: Date, calendar: Calendar = .current) -> String {
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

    init(dueDate: Date, today: Date = .now, calendar: Calendar = .current) {
        self.dueDate = dueDate
        daysLeft = calendar.dateComponents([.day], from: calendar.startOfDay(for: today),
                                           to: calendar.startOfDay(for: dueDate)).day ?? 0
    }

    /// From `PatronItem.fields["Due"]`.
    init?(dueText: String?, today: Date = .now, calendar: Calendar = .current) {
        guard let text = dueText.nonEmpty, let date = DueDate.parse(text, calendar: calendar) else { return nil }
        self.init(dueDate: date, today: today, calendar: calendar)
    }

    var isOverdue: Bool { daysLeft < 0 }
    /// Danger colour: overdue or due within 3 days.
    var isUrgent: Bool { daysLeft <= 3 }

    /// "3 days left", "1 day left", "Due today", "2 days overdue".
    var daysText: String {
        switch daysLeft {
        case ..<(-1): "\(-daysLeft) days overdue"
        case -1: "1 day overdue"
        case 0: "Due today"
        case 1: "1 day left"
        default: "\(daysLeft) days left"
        }
    }

    /// "Due 27 Oct".
    var dueText: String { "Due \(Formatting.shortDate(dueDate))" }

    /// Borrowed card meta line: "Due 27 Oct · 3 days left".
    var metaLine: String { "\(dueText) · \(daysText)" }
}
