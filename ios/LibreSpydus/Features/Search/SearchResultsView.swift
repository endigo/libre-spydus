import SwiftUI

/// A results list for `Route.results` (I3): Home "View All", genre tiles, submitted searches.
/// Wave-1 stub.
struct SearchResultsView: View {
    let query: String
    let sort: SearchSortOption
    let title: String

    var body: some View {
        PlaceholderScreen(title: title, systemImage: "list.bullet.rectangle",
                          message: "Results for “\(query)”, sorted by \(sort.label.lowercased()).")
            .navigationTitle(title)
            .navigationBarTitleDisplayMode(.inline)
    }
}
