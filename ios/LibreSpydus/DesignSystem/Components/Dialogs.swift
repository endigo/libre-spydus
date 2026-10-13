import SwiftUI

extension View {
    /// Presents `content`'s dialogs (alerts, confirmation dialogs) with the neutral text colour as
    /// their tint instead of the app's coral: in a dialog, coral reads as a destructive action
    /// (docs/native-apps.md §7 "Dialogs"). A `.destructive` button still shows the system red, and
    /// the confirming action is the bold one (`.keyboardShortcut(.defaultAction)`). The host is
    /// this view's background, the same size as the view, so a dialog that anchors to its view
    /// (an iOS 27 popover) still anchors here.
    func dialogs<Content: View>(@ViewBuilder _ content: () -> Content) -> some View {
        background { content().tint(Theme.Colors.textPrimary) }
    }
}
