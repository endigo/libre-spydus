import SwiftUI

/// Sign-in sheet (I2): library card number + PIN → `AppModel.signIn`. Wave-1 stub.
struct LoginView: View {
    @Environment(AppModel.self) private var model
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        PlaceholderScreen(title: "Sign in to \(model.libraryName)", systemImage: "person.badge.key",
                          message: model.isDemo ? "Demo mode: card number “demo”, PIN “demo”." : nil) {
            Button("Close") { dismiss() }
                .foregroundStyle(Theme.Colors.primary)
        }
    }
}
