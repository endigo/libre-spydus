import SwiftUI

/// Search tab (I3): search field (focused when Home hands over a query), Relevance/Newest,
/// results with infinite scroll. Wave-1 stub.
struct SearchView: View {
    @Environment(AppModel.self) private var model
    @State private var query = ""

    var body: some View {
        PlaceholderScreen(title: "Search", systemImage: "magnifyingglass",
                          message: "Search \(model.libraryName)'s catalogue.") {
            SearchField(text: $query) {
                let text = query.trimmingCharacters(in: .whitespaces)
                guard !text.isEmpty else { return }
                model.push(.results(query: text, sort: .relevance, title: text), on: .search)
            }
        }
        .toolbar(.hidden, for: .navigationBar)
        .onAppear {
            if let pending = model.takePendingSearch(), !pending.isEmpty { query = pending }
        }
    }
}
