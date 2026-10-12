import SwiftUI

/// Favourites (I5): saved books grid, tap → detail, remove. Wave-1 stub.
struct FavouritesView: View {
    @Environment(AppModel.self) private var model

    var body: some View {
        PlaceholderScreen(title: "Favourites", systemImage: "bookmark",
                          message: "\(model.favourites.items.count) saved on this device.")
            .toolbar(.hidden, for: .navigationBar)
    }
}
