#if DEBUG
import Foundation

/// Evidence hooks for the card sheet (DEBUG), run once by Home after launch:
/// - `-LSDemoCardNumber <number>` (demo mode only): the demo card number instead of
///   `DEMO0012345`; `none` deletes it, so the sheet asks for one.
/// - `-LSMyCard open` opens the card sheet; `-LSMyCard edit` opens it on the entry form. Only while
///   signed in, like the Home button.
@MainActor
enum MyCardDebugHooks {
    enum Open { case card, edit }

    /// Applies the hooks; what to open, if anything.
    static func apply(_ model: AppModel, defaults: UserDefaults = .standard) -> Open? {
        if model.isDemo, let number = defaults.string(forKey: "LSDemoCardNumber") {
            model.setCardNumber(number == "none" ? nil : number)
        }
        guard model.isSignedIn else { return nil }
        switch defaults.string(forKey: "LSMyCard") {
        case "open": return .card
        case "edit": return .edit
        default: return nil
        }
    }
}
#endif
