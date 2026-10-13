import SwiftUI

/// An action behind a Your Books card's swipe (`SwipeActionCard`), also in its long-press menu and
/// VoiceOver actions.
protocol SwipeTileAction: Identifiable, Hashable where ID == String {
    /// The swipe tile's label.
    var title: String { get }
    /// The long-press menu and VoiceOver.
    var menuTitle: String { get }
    var systemImage: String { get }
    var role: ButtonRole? { get }
    var tint: Color { get }
}

/// What a reservation card offers behind its swipe, in its long-press menu and as VoiceOver
/// actions: Edit (the sheet) and Cancel (confirmed first), each only when the library gave its link.
enum ReservationAction: String, CaseIterable, SwipeTileAction {
    case edit
    case cancel

    var id: String { rawValue }

    /// The swipe tile's label.
    var title: String {
        switch self {
        case .edit: Copy.Common.edit
        case .cancel: Copy.Common.cancel
        }
    }

    /// The long-press menu and VoiceOver.
    var menuTitle: String {
        switch self {
        case .edit: Copy.YourBooks.editReservation
        case .cancel: Copy.YourBooks.cancelReservation
        }
    }

    var systemImage: String {
        switch self {
        case .edit: "pencil"
        case .cancel: "xmark"
        }
    }

    var role: ButtonRole? { self == .cancel ? .destructive : nil }

    var tint: Color { self == .cancel ? Theme.Colors.danger : Theme.Colors.accent }

    /// Edit, then Cancel at the outer edge like iOS's destructive swipe actions. None while the
    /// row has a cancel or edit in flight.
    static func available(for row: ReservationRow, busy: Bool = false) -> [ReservationAction] {
        guard !busy else { return [] }
        return allCases.filter { action in
            switch action {
            case .edit: row.editHref != nil
            case .cancel: row.cancelHref != nil
            }
        }
    }
}

extension ReservationSections {
    /// Whether any reservation can be edited or cancelled (the swipe hint under the list).
    var hasActions: Bool {
        (ready + pending).contains { !ReservationAction.available(for: $0).isEmpty }
    }
}

/// Swipe-to-reveal geometry for a reservation card: how far it slides, how it resists past the
/// ends, and where it settles. Pure, so BorrowedSwipeTests checks it.
enum SwipeReveal {
    static let tileWidth: CGFloat = 76
    static let spacing: CGFloat = Theme.Spacing.s
    /// A release faster than this (points/s) settles in its direction, however short the swipe.
    static let flickVelocity: CGFloat = 400
    static let animation: Animation = .spring(response: 0.32, dampingFraction: 0.86)

    /// How far the card slides to show `count` tiles, each with the gap before it.
    static func openWidth(tiles count: Int) -> CGFloat {
        CGFloat(count) * (tileWidth + spacing)
    }

    /// The card's offset after a drag of `translation` from `start` (0 closed, −openWidth open):
    /// free in between, resisting past either end like a scroll view's edge, so a long swipe
    /// slows down instead of running away.
    static func offset(start: CGFloat, translation: CGFloat, openWidth: CGFloat) -> CGFloat {
        let raw = start + translation
        if raw > 0 { return rubberBand(raw, dimension: openWidth) }
        if raw < -openWidth { return -openWidth - rubberBand(-openWidth - raw, dimension: openWidth) }
        return raw
    }

    /// Whether the card rests open once the finger lifts: a flick settles in its direction,
    /// otherwise it opens past half way. However long or fast the swipe, it only ever opens:
    /// an action runs only when its tile is tapped.
    static func settlesOpen(offset: CGFloat, velocity: CGFloat, openWidth: CGFloat) -> Bool {
        guard openWidth > 0 else { return false }
        if velocity < -flickVelocity { return true }
        if velocity > flickVelocity { return false }
        return offset < -openWidth / 2
    }

    /// 0 closed … 1 open: the tiles fade and grow in with it.
    static func progress(offset: CGFloat, openWidth: CGFloat) -> CGFloat {
        guard openWidth > 0 else { return 0 }
        return min(max(-offset / openWidth, 0), 1)
    }

    /// Whether a drag moving by `motion` (its translation or velocity) is a swipe rather than a
    /// scroll: more sideways than vertical, and to the left unless the card is open (a closed
    /// card has nothing on its leading side).
    static func isSwipe(_ motion: CGSize, isOpen: Bool) -> Bool {
        abs(motion.width) > abs(motion.height) && (isOpen || motion.width < 0)
    }

    /// The swipe needs iOS 18 (see `SwipeActionCard`).
    static var isSupported: Bool {
        if #available(iOS 18, *) { true } else { false }
    }

    /// The hint under a list (the reservations' Edit / Cancel, the loans' Renew), for where the
    /// actions are on this device.
    static func hint(for segment: BorrowedViewModel.Segment = .reservations,
                     swipes: Bool = isSupported) -> (text: String, systemImage: String) {
        switch (segment, swipes) {
        case (.reservations, true): (Copy.YourBooks.swipeHint, "hand.draw")
        case (.reservations, false): (Copy.YourBooks.touchAndHoldHint, "hand.tap")
        case (.loans, true): (Copy.YourBooks.loanSwipeHint, "hand.draw")
        case (.loans, false): (Copy.YourBooks.loanTouchAndHoldHint, "hand.tap")
        }
    }

    /// UIScrollView's edge resistance: `distance` past the end moves the card ever less, never
    /// as far as `dimension`.
    private static func rubberBand(_ distance: CGFloat, dimension: CGFloat) -> CGFloat {
        (1 - 1 / (distance * 0.55 / dimension + 1)) * dimension
    }
}
