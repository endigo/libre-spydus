import SwiftUI

/// Home (I3): greeting, search field, "New arrivals", "Popular genres", "Your borrowed books".
/// Wave-1 stub.
struct HomeView: View {
    @Environment(AppModel.self) private var model

    var body: some View {
        PlaceholderScreen(
            title: Formatting.greeting(accountName: model.accountName),
            systemImage: "house",
            message: "\(model.libraryName): new arrivals, genres and your loans will appear here."
        ) {
            SearchFieldButton { model.openSearch() }
                .padding(.top, Theme.Spacing.s)
        }
        .toolbar(.hidden, for: .navigationBar)
    }
}
