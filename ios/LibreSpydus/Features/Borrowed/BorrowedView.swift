import SwiftUI

/// "Your Books" (I5): Loans | Reservations, cancel/edit, pull to refresh. Wave-1 stub.
struct BorrowedView: View {
    @Environment(AppModel.self) private var model

    var body: some View {
        PlaceholderScreen(title: "Your Books", systemImage: "books.vertical",
                          message: model.isSignedIn ? "Your loans and reservations will appear here."
                                                    : "Sign in to see your loans and reservations.") {
            if !model.isSignedIn {
                PrimaryPillButton(title: "Sign in") { model.loginNeeded = true }
            }
        }
        .toolbar(.hidden, for: .navigationBar)
    }
}
