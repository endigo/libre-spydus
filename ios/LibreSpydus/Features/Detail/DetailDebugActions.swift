#if DEBUG
import Foundation

/// Screenshot hooks for book detail (DEBUG only), like `-LSGalleryAnchor` for the gallery:
///
///     -LSRoute detail:<url> -LSDetailAction format:EBK,favourite
///
/// Applied once, to the first detail screen, after its record loads. Actions (comma-separated):
/// `format:<code>` selects a format, `favourite` saves the book (signed in: to My Picks, demo mode
/// only), `related` scrolls to the
/// carousels, `bottom` to the end, `copies` to an edition-level item's "Copies" list once it has
/// loaded, `open-related` opens the first related item, `reserve` opens the reserve sheet and
/// `confirm` also confirms it (demo mode only).
struct DetailDebugActions: Equatable, Sendable {
    var formatCode: String?
    var favourite = false
    var scrollToRelated = false
    var scrollToBottom = false
    var scrollToCopies = false
    var openRelated = false
    var reserve = false
    var confirm = false

    init?(arguments: [String]) {
        guard let index = arguments.firstIndex(of: "-LSDetailAction"), index + 1 < arguments.endIndex else { return nil }
        for action in arguments[index + 1].split(separator: ",").map({ $0.trimmingCharacters(in: .whitespaces) }) {
            switch action.lowercased() {
            case let format where format.hasPrefix("format:"): formatCode = String(action.dropFirst("format:".count))
            case "favourite", "favorite": favourite = true
            case "related": scrollToRelated = true
            case "bottom": scrollToBottom = true
            case "copies": scrollToCopies = true
            case "open-related": openRelated = true
            case "reserve": reserve = true
            case "confirm": reserve = true; confirm = true
            default: break
            }
        }
    }

    @MainActor private static var pending = DetailDebugActions(arguments: ProcessInfo.processInfo.arguments)

    /// The launch actions, handed out once.
    @MainActor static func take() -> DetailDebugActions? {
        defer { pending = nil }
        return pending
    }
}
#endif
