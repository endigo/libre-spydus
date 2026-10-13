import SwiftUI

/// A results list for `Route.resultsPage` (§5.10): the Search tab's list started from a URL
/// (`page(url)`, then `nextPageUrl`) instead of a query — a Featured saved search ("Coming Soon")
/// or a collection section's "View All". Titled in the navigation bar, without a sort menu.
struct ResultsPageView: View {
    let url: String
    let title: String

    @Environment(AppModel.self) private var model
    @State private var results = SearchResultsModel()
    @State private var attempt = 0

    var body: some View {
        SearchResultsList(results: results, query: title, retry: { attempt += 1 },
                          noResultsTitle: Copy.Search.listEmptyTitle(title: title))
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .screenBackground()
            .navigationTitle(title)
            .navigationBarTitleDisplayMode(.inline)
            .navyNavigationBar()
            .task(id: request) {
                await results.load(request, backend: model.backend) { model.report($0) }
            }
    }

    private var request: SearchResultsModel.Request {
        SearchResultsModel.Request(query: title, sort: .relevance, url: url, origin: model.backend.origin,
                                   verification: model.verificationGeneration, attempt: attempt)
    }
}
