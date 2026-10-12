import Foundation
import LibreSpydusCore

// What the Borrowed cards show, built from the core's `PatronItem`s (docs/native-apps.md §4 data
// notes). Pure (no I/O), so the sorting and wording are unit-tested in BorrowedTests.

extension PatronItem {
    /// The href of the first action whose label contains `keyword` (case-insensitive): the
    /// labels vary by library ("Cancel reservation", "Edit", "Edit reservation").
    func actionHref(matching keyword: String) -> String? {
        (actions.first { $0.label.range(of: keyword, options: .caseInsensitive) != nil }?.href).nonEmpty
    }

    /// `/cancel/i`: the reservation's cancel link (`cancelReservation(url:)`).
    var cancelHref: String? { actionHref(matching: "cancel") }
    /// `/edit/i`: the reservation's edit link (`editOptions(url:)` / `editReservation(url:)`).
    var editHref: String? { actionHref(matching: "edit") }
}

extension Calendar {
    /// For the library's own dates ("27 Oct 2026", "2026-11-01"): Gregorian in the device's time
    /// zone, whatever calendar the device uses (a Buddhist-calendar device would otherwise read
    /// 2026 as year 2026 BE and send it back wrong).
    static var libraryDates: Calendar {
        var calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = .current
        calendar.locale = .current
        return calendar
    }
}

// MARK: - Loans

/// One loan card: "Due 27 Oct · 3 days left", danger colour when due within 3 days or overdue.
struct LoanRow: Identifiable, Equatable {
    /// The loan's IRN (made unique within its list by `rows(from:)`).
    var id: String
    let item: PatronItem
    let ref: BookRef
    /// Nil when there's no "Due" field or the core can't read it as a date.
    let due: DueStatus?
    /// `fields["Due"]` as the library wrote it.
    let dueText: String?

    init(item: PatronItem, today: Date = .now, calendar: Calendar = .libraryDates) {
        self.item = item
        ref = BookRef(item)
        id = item.irn.nonEmpty ?? ref.id
        dueText = item.fields["Due"].nonEmpty
        due = DueStatus(dueText: dueText, today: today, calendar: calendar)
    }

    /// The meta line; an unreadable date is shown as the library wrote it.
    var meta: String? {
        if let due { return Self.dueLine(due) }
        guard let dueText else { return nil }
        return dueText.range(of: "due", options: .caseInsensitive) == nil ? "Due \(dueText)" : dueText
    }

    var isUrgent: Bool { due?.isUrgent ?? false }

    /// "Due 27 Oct · 3 days left", "Due tomorrow · 13 Oct", "Due today · 12 Oct",
    /// "Due 10 Oct · 2 days overdue".
    static func dueLine(_ due: DueStatus) -> String {
        let date = Formatting.shortDate(due.dueDate)
        switch due.daysLeft {
        case ..<(-1): return "Due \(date) · \(-due.daysLeft) days overdue"
        case -1: return "Due \(date) · 1 day overdue"
        case 0: return "Due today · \(date)"
        case 1: return "Due tomorrow · \(date)"
        default: return "Due \(date) · \(due.daysLeft) days left"
        }
    }

    /// Loans soonest-due first (overdue at the top); loans without a readable date go last,
    /// in the library's order.
    static func rows(from items: [PatronItem], today: Date = .now, calendar: Calendar = .libraryDates) -> [LoanRow] {
        var ids = UniqueIDs()
        let rows = items.enumerated().map { index, item in
            var row = LoanRow(item: item, today: today, calendar: calendar)
            row.id = ids.claim(row.id)
            return (index, row)
        }
        return rows.sorted { lhs, rhs in
            switch (lhs.1.due?.dueDate, rhs.1.due?.dueDate) {
            case let (left?, right?) where left != right: left < right
            case (.some, nil): true
            case (nil, .some): false
            default: lhs.0 < rhs.0
            }
        }.map(\.1)
    }
}

// MARK: - Reservations

/// One reservation card, ready for pickup or pending.
struct ReservationRow: Identifiable, Equatable {
    enum Kind: Equatable { case ready, pending }

    /// The reservation's IRN, else its cancel/edit link (unique within `ReservationSections`).
    var id: String
    let kind: Kind
    let item: PatronItem
    let ref: BookRef
    /// The library's status, e.g. "In transit"; nil when it only names the pickup branch (see
    /// `StatusText`).
    let status: String?
    /// `fields["Rank"]`: place in the queue.
    let rank: String?
    /// `fields["Placed"]` as the library wrote it.
    let placed: String?
    /// `fields["Pickup location"]`, or the branch named in the status text; title-cased when the
    /// library shouts it ("RIVERTON LIBRARY").
    let pickup: String?
    /// Ready reservations: the last day to collect, as the library wrote it ("17 Oct 2026").
    let pickupBy: String?
    let cancelHref: String?
    let editHref: String?

    init(item: PatronItem, kind: Kind) {
        self.item = item
        self.kind = kind
        ref = BookRef(item)
        id = item.irn.nonEmpty ?? item.cancelHref ?? item.editHref ?? ref.id
        let statusText = StatusText(item.fields["Status"])
        status = statusText.status
        rank = item.fields["Rank"].nonEmpty
        placed = item.fields["Placed"].nonEmpty
        pickup = Self.displayName(item.fields["Pickup location"]) ?? Self.displayName(statusText.pickup)
        pickupBy = statusText.pickupBy
        cancelHref = item.cancelHref
        editHref = item.editHref
    }

    var isReady: Bool { kind == .ready }

    /// The status chip: "Ready for pickup", or the library's own pending status.
    var statusLabel: String {
        isReady ? "Ready for pickup" : (status ?? "Pending")
    }

    /// Ready: "Pick up at Riverton Library". Pending: "#4 in queue · Placed 2 Oct" and the
    /// pickup branch on a second line.
    var meta: String? {
        let pickupLine = pickup.map { "Pick up at \($0)" }
        if isReady {
            guard let pickupBy else { return pickupLine }
            let by = "by \(DueDate.parse(pickupBy, calendar: .libraryDates).map(Formatting.shortDate) ?? pickupBy)"
            return pickupLine.map { "\($0) \(by)" } ?? "Pick up \(by)"
        }
        var first: [String] = []
        if let rank { first.append(Int(rank) != nil ? "#\(rank) in queue" : "Rank \(rank)") }
        if let placed {
            first.append("Placed \(DueDate.parse(placed, calendar: .libraryDates).map(Formatting.shortDate) ?? placed)")
        }
        let lines = [first.isEmpty ? nil : first.joined(separator: " · "), pickupLine].compactMap(\.self)
        return lines.isEmpty ? nil : lines.joined(separator: "\n")
    }

    /// "RIVERTON LIBRARY" → "Riverton Library"; mixed case is kept.
    static func displayName(_ raw: String?) -> String? {
        guard let name = raw.nonEmpty else { return nil }
        guard name == name.uppercased(), name.contains(where: \.isLetter) else { return name }
        return name.capitalized(with: Locale(identifier: "en_AU"))
    }
}

/// A reservation's `Status` field. Some libraries (Canning) put the pickup branch there instead of
/// a status: "Requested pickup location: RIVERTON LIBRARY" while pending, "Please pickup from:
/// RIVERTON LIBRARY by 17 Oct 2026" once ready. Those become a branch (and collect-by date) with no
/// status; anything else is kept as the status.
struct StatusText: Equatable {
    var status: String?
    var pickup: String?
    var pickupBy: String?

    init(_ raw: String?) {
        guard let text = raw.nonEmpty else { return }
        if let match = text.firstMatch(of: /(?i)^requested\s+pick\s?-?up\s+location\s*:\s*(.+)$/) {
            pickup = Optional(String(match.1)).nonEmpty
        } else if let match = text.firstMatch(of: /(?i)^please\s+pick\s?-?up\s+from\s*:\s*(.+?)(?:\s+by\s+(.+))?$/) {
            pickup = Optional(String(match.1)).nonEmpty
            pickupBy = match.2.map(String.init).nonEmpty
        } else {
            status = text
        }
    }

    init(status: String? = nil, pickup: String? = nil, pickupBy: String? = nil) {
        self.status = status
        self.pickup = pickup
        self.pickupBy = pickupBy
    }
}

/// Reservations split for display: ready for pickup first, then pending.
struct ReservationSections: Equatable {
    var ready: [ReservationRow]
    var pending: [ReservationRow]

    init(ready: [ReservationRow] = [], pending: [ReservationRow] = []) {
        self.ready = ready
        self.pending = pending
    }

    /// From the core's two lists. A row the library lists on both pages is shown once, as ready.
    init(_ reservations: Reservations) {
        var ids = UniqueIDs()
        func row(_ item: PatronItem, _ kind: ReservationRow.Kind) -> ReservationRow {
            var row = ReservationRow(item: item, kind: kind)
            row.id = ids.claim(row.id)
            return row
        }
        ready = reservations.ready.items.map { row($0, .ready) }
        let readyIrns = Set(reservations.ready.items.compactMap { $0.irn.nonEmpty })
        pending = reservations.pending.items
            .filter { item in item.irn.nonEmpty.map { !readyIrns.contains($0) } ?? true }
            .map { row($0, .pending) }
    }

    var count: Int { ready.count + pending.count }
    var isEmpty: Bool { count == 0 }

    func row(id: String) -> ReservationRow? {
        (ready + pending).first { $0.id == id }
    }

    /// Without the row `id` (a cancelled reservation, before the list is reloaded).
    func removing(id: String) -> ReservationSections {
        ReservationSections(ready: ready.filter { $0.id != id }, pending: pending.filter { $0.id != id })
    }
}

/// Keeps row ids unique within one list: a repeated key gets `#2`, `#3`, ….
private struct UniqueIDs {
    private var used: Set<String> = []

    mutating func claim(_ id: String) -> String {
        var unique = id
        var suffix = 1
        while used.contains(unique) {
            suffix += 1
            unique = "\(id)#\(suffix)"
        }
        used.insert(unique)
        return unique
    }
}
