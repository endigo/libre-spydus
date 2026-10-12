import SwiftUI

/// Human-verification sheet (I2): the library page in a web view for the user to complete;
/// `aws-waf-token` → `AppModel.completeVerification`. Wave-1 stub.
struct VerificationView: View {
    @Environment(AppModel.self) private var model
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        PlaceholderScreen(title: "Verify you're human", systemImage: "checkmark.shield",
                          message: "\(model.libraryName) asks you to confirm you're human.") {
            Button("Close") { dismiss() }
                .foregroundStyle(Theme.Colors.primary)
        }
    }
}
