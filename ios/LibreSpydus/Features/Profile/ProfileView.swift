import SwiftUI

/// Profile (I2): library card, account, sign in/out, change library, verification status,
/// version. Wave-1 stub.
struct ProfileView: View {
    @Environment(AppModel.self) private var model

    var body: some View {
        PlaceholderScreen(title: model.accountName ?? "Profile", systemImage: "person.crop.circle",
                          message: model.library.map { "\($0.name) · \($0.origin)" }) {
            VStack(spacing: Theme.Spacing.m) {
                if model.isSignedIn {
                    PrimaryPillButton(title: "Sign out") { model.signOut() }
                } else {
                    PrimaryPillButton(title: "Sign in") { model.loginNeeded = true }
                }
                Button("Change library") { model.libraryPickerPresented = true }
                    .foregroundStyle(Theme.Colors.primary)
            }
        }
        .toolbar(.hidden, for: .navigationBar)
    }
}
