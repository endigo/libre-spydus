import Foundation
import LibreSpydusCore

/// What the book page shows instead of "Reserve" when the patron already has the edition on screen
/// (docs/native-apps.md §6 "Already yours"): "On loan · Due 14 Oct", "Ready to collect · Riverton
/// Library", "Reserved · Riverton Library". The due date and pickup branch are read the way Your
/// Books reads them (`DueStatus`, `ReservationRow`). Tapping it opens Your Books on `segment`.
struct HeldStatus: Equatable, Sendable {
    enum Tone: Equatable, Sendable {
        case loan
        case overdue
        case ready
        case reserved
    }

    /// One line: "On loan · Due 14 Oct".
    var text: String
    /// VoiceOver: "On loan, due 14 October".
    var accessibilityLabel: String
    var tone: Tone
    var systemImage: String
    var segment: BorrowedViewModel.Segment

    init(_ holding: AccountHolding, today: Date = .now, calendar: Calendar = .libraryDates) {
        switch holding.kind {
        case .loan:
            self.init(loan: holding.item, today: today, calendar: calendar)
        case .reservationReady:
            let pickup = ReservationRow(item: holding.item, kind: .ready).pickup
            self.init(text: ["Ready to collect", pickup].compactMap(\.self).joined(separator: " · "),
                      accessibilityLabel: pickup.map { "Ready to collect at \($0)" } ?? "Ready to collect",
                      tone: .ready, systemImage: "checkmark.circle.fill", segment: .reservations)
        case .reservationPending:
            let row = ReservationRow(item: holding.item, kind: .pending)
            let detail: (text: String, spoken: String)? = if let pickup = row.pickup {
                (pickup, "pick up at \(pickup)")
            } else if let rank = row.rank {
                Int(rank) != nil ? ("#\(rank) in queue", "number \(rank) in the queue") : ("Rank \(rank)", "rank \(rank)")
            } else {
                nil
            }
            self.init(text: ["Reserved", detail?.text].compactMap(\.self).joined(separator: " · "),
                      accessibilityLabel: ["Reserved", detail?.spoken].compactMap(\.self).joined(separator: ", "),
                      tone: .reserved, systemImage: "clock.fill", segment: .reservations)
        }
    }

    init(text: String, accessibilityLabel: String, tone: Tone, systemImage: String, segment: BorrowedViewModel.Segment) {
        self.text = text
        self.accessibilityLabel = accessibilityLabel
        self.tone = tone
        self.systemImage = systemImage
        self.segment = segment
    }

    /// "On loan · Due 14 Oct", "On loan · Overdue"; a due date the core can't read is shown as the
    /// library wrote it.
    private init(loan item: PatronItem, today: Date, calendar: Calendar) {
        let dueText = item.fields["Due"].nonEmpty
        if let due = DueStatus(dueText: dueText, today: today, calendar: calendar) {
            let spokenDate = due.dueDate.formatted(.dateTime.day().month(.wide))
            if due.isOverdue {
                self.init(text: "On loan · Overdue", accessibilityLabel: "On loan, overdue, was due \(spokenDate)",
                          tone: .overdue, systemImage: "exclamationmark.circle.fill", segment: .loans)
            } else {
                self.init(text: "On loan · \(due.dueText)", accessibilityLabel: "On loan, due \(spokenDate)",
                          tone: .loan, systemImage: "book.closed.fill", segment: .loans)
            }
        } else {
            self.init(text: ["On loan", dueText.map { "Due \($0)" }].compactMap(\.self).joined(separator: " · "),
                      accessibilityLabel: ["On loan", dueText.map { "due \($0)" }].compactMap(\.self).joined(separator: ", "),
                      tone: .loan, systemImage: "book.closed.fill", segment: .loans)
        }
    }
}
