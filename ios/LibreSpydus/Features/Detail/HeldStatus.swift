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
            self.init(text: pickup.map { Copy.Detail.readyToCollectAt(branch: $0) } ?? Copy.Detail.readyToCollect,
                      accessibilityLabel: pickup.map { Copy.Detail.readyToCollectAtLabel(branch: $0) } ?? Copy.Detail.readyToCollect,
                      tone: .ready, systemImage: "checkmark.circle.fill", segment: .reservations)
        case .reservationPending:
            let row = ReservationRow(item: holding.item, kind: .pending)
            let line: (text: String, spoken: String) = if let pickup = row.pickup {
                (Copy.Detail.reservedAt(branch: pickup), Copy.Detail.reservedAtLabel(branch: pickup))
            } else if let rank = row.rank, Int(rank) != nil {
                (Copy.Detail.reservedQueue(place: rank), Copy.Detail.reservedQueueLabel(place: rank))
            } else if let rank = row.rank {
                (Copy.Detail.reservedQueueText(place: rank), Copy.Detail.reservedQueueTextLabel(place: rank))
            } else {
                (Copy.Detail.reserved, Copy.Detail.reserved)
            }
            self.init(text: line.text, accessibilityLabel: line.spoken,
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
                self.init(text: Copy.Detail.onLoanOverdue, accessibilityLabel: Copy.Detail.onLoanOverdueLabel(date: spokenDate),
                          tone: .overdue, systemImage: "exclamationmark.circle.fill", segment: .loans)
            } else {
                self.init(text: Copy.Detail.onLoanDue(date: Formatting.shortDate(due.dueDate)),
                          accessibilityLabel: Copy.Detail.onLoanDueLabel(date: spokenDate),
                          tone: .loan, systemImage: "book.closed.fill", segment: .loans)
            }
        } else {
            self.init(text: dueText.map { Copy.Detail.onLoanDue(date: $0) } ?? Copy.Detail.onLoan,
                      accessibilityLabel: dueText.map { Copy.Detail.onLoanDueLabel(date: $0) } ?? Copy.Detail.onLoan,
                      tone: .loan, systemImage: "book.closed.fill", segment: .loans)
        }
    }
}
