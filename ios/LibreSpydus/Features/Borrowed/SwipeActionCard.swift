import SwiftUI
import UIKit

/// A Your Books card that slides left to uncover its actions as rounded tiles beside it, shaped
/// like the card (SwiftUI's `.swipeActions` only works in a `List` and draws flat full-height
/// buttons): a reservation's Edit and Cancel, a loan's Renew. `openID` keeps one card open at a
/// time: starting a swipe on another card closes the open one, and BorrowedView closes it on a
/// scroll, a tap elsewhere or a tab change. Swiping only opens the card; an action runs when its
/// tile is tapped.
///
/// The drag is a UIKit pan that only begins when the finger moves sideways, so a vertical drag
/// that starts on a card always scrolls the list. It needs iOS 18 (`UIGestureRecognizerRepresentable`):
/// a SwiftUI `DragGesture` on a card stops the list scrolling, so iOS 17 has no swipe and offers
/// the same actions in the long-press menu (`SwipeReveal.hint` says which).
struct SwipeActionCard<Action: SwipeTileAction, Content: View>: View {
    let id: String
    /// Empty: no swipe at all (no links, or a change in flight).
    let actions: [Action]
    @Binding var openID: String?
    let perform: (Action) -> Void
    @ViewBuilder let content: Content

    /// The card's offset while a finger is on it; nil while it rests open or closed.
    @State private var dragOffset: CGFloat?
    @State private var dragStart: CGFloat = 0

    private var isOpen: Bool { openID == id && !actions.isEmpty }
    private var openWidth: CGFloat { SwipeReveal.openWidth(tiles: actions.count) }
    private var offset: CGFloat { dragOffset ?? (isOpen ? -openWidth : 0) }

    var body: some View {
        content
            .offset(x: offset)
            .background(alignment: .trailing) {
                // Only while uncovered: a closed card has no hidden buttons behind it.
                if offset < 0 { tiles.transition(.opacity) }
            }
            .animation(SwipeReveal.animation, value: isOpen)
            .swipeGesture(isEnabled: !actions.isEmpty, isOpen: isOpen, onChanged: dragChanged, onEnded: dragEnded)
    }

    private var tiles: some View {
        let progress = SwipeReveal.progress(offset: offset, openWidth: openWidth)
        return HStack(spacing: SwipeReveal.spacing) {
            ForEach(actions) { action in
                Button { perform(action) } label: { tile(action) }
                    .buttonStyle(PressScaleStyle())
            }
        }
        .scaleEffect(0.8 + 0.2 * progress, anchor: .trailing)
        .opacity(Double(progress))
        .allowsHitTesting(isOpen && dragOffset == nil)
        // VoiceOver reaches the same actions on the card itself.
        .accessibilityHidden(true)
    }

    private func tile(_ action: Action) -> some View {
        VStack(spacing: Theme.Spacing.xs) {
            Image(systemName: action.systemImage)
                .font(.system(size: 20, weight: .semibold))
            Text(action.title)
                .themeFont(.bodyStrong)
                .lineLimit(1)
                .minimumScaleFactor(0.7)
        }
        .foregroundStyle(Theme.Colors.onAccent)
        .padding(.horizontal, Theme.Spacing.xs)
        .frame(width: SwipeReveal.tileWidth)
        .frame(maxHeight: .infinity)
        .background(RoundedRectangle(cornerRadius: Theme.Radius.card, style: .continuous).fill(action.tint))
        .contentShape(RoundedRectangle(cornerRadius: Theme.Radius.card, style: .continuous))
    }

    private func dragChanged(_ translation: CGFloat) {
        if dragOffset == nil {
            dragStart = offset
            if let openID, openID != id {
                withAnimation(SwipeReveal.animation) { self.openID = nil }
            }
        }
        dragOffset = SwipeReveal.offset(start: dragStart, translation: translation, openWidth: openWidth)
    }

    private func dragEnded(_ translation: CGFloat, velocity: CGFloat) {
        guard dragOffset != nil else { return }
        let current = SwipeReveal.offset(start: dragStart, translation: translation, openWidth: openWidth)
        let open = !actions.isEmpty && SwipeReveal.settlesOpen(offset: current, velocity: velocity, openWidth: openWidth)
        withAnimation(SwipeReveal.animation) {
            dragOffset = nil
            if open {
                openID = id
            } else if openID == id {
                openID = nil
            }
        }
    }
}

private extension View {
    @ViewBuilder
    func swipeGesture(isEnabled: Bool, isOpen: Bool, onChanged: @escaping (CGFloat) -> Void,
                      onEnded: @escaping (_ translation: CGFloat, _ velocity: CGFloat) -> Void) -> some View {
        if #available(iOS 18, *) {
            gesture(HorizontalPan(isEnabled: isEnabled, isOpen: isOpen, onChanged: onChanged, onEnded: onEnded))
        } else {
            self
        }
    }
}

/// A pan that decides at its start: sideways (`SwipeReveal.isSwipe`) begins the swipe, anything
/// else fails at once and leaves the touch to the scroll view.
@available(iOS 18, *)
private struct HorizontalPan: UIGestureRecognizerRepresentable {
    var isEnabled: Bool
    var isOpen: Bool
    var onChanged: (CGFloat) -> Void
    var onEnded: (_ translation: CGFloat, _ velocity: CGFloat) -> Void

    func makeCoordinator(converter: CoordinateSpaceConverter) -> Coordinator { Coordinator() }

    func makeUIGestureRecognizer(context: Context) -> UIPanGestureRecognizer {
        let pan = UIPanGestureRecognizer()
        pan.maximumNumberOfTouches = 1
        pan.delegate = context.coordinator
        return pan
    }

    func updateUIGestureRecognizer(_ recognizer: UIPanGestureRecognizer, context: Context) {
        context.coordinator.isEnabled = isEnabled
        context.coordinator.isOpen = isOpen
    }

    func handleUIGestureRecognizerAction(_ recognizer: UIPanGestureRecognizer, context: Context) {
        let translation = recognizer.translation(in: recognizer.view).x
        switch recognizer.state {
        case .began, .changed: onChanged(translation)
        case .ended: onEnded(translation, recognizer.velocity(in: recognizer.view).x)
        case .cancelled, .failed: onEnded(translation, 0)
        default: break
        }
    }

    @MainActor
    final class Coordinator: NSObject, UIGestureRecognizerDelegate {
        var isEnabled = false
        var isOpen = false

        func gestureRecognizerShouldBegin(_ recognizer: UIGestureRecognizer) -> Bool {
            guard isEnabled, let pan = recognizer as? UIPanGestureRecognizer else { return false }
            let velocity = pan.velocity(in: pan.view)
            let translation = pan.translation(in: pan.view)
            // The velocity says where the finger is heading; a near-still start falls back on how
            // far it has moved.
            let motion = abs(velocity.x) + abs(velocity.y) > 1 ? velocity : translation
            return SwipeReveal.isSwipe(CGSize(width: motion.x, height: motion.y), isOpen: isOpen)
        }
    }
}
