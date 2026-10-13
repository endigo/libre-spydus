import SwiftUI

/// A results list for `Route.results` (Home "View All", genre tiles, `-LSRoute results:`): the
/// Search tab's list without the field, titled in the navigation bar, with a sort menu.
struct SearchResultsView: View {
    let query: String
    let sort: SearchSortOption
    let title: String

    @Environment(AppModel.self) private var model
    @State private var results = SearchResultsModel()
    @State private var activeSort: SearchSortOption
    @State private var attempt = 0

    init(query: String, sort: SearchSortOption, title: String) {
        self.query = query
        self.sort = sort
        self.title = title
        _activeSort = State(initialValue: sort)
    }

    var body: some View {
        SearchResultsList(results: results, query: query) { attempt += 1 }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .screenBackground()
            .navigationTitle(title)
            .navigationBarTitleDisplayMode(.inline)
            .navyNavigationBar()
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Menu {
                        Picker(Copy.Search.sort, selection: $activeSort) {
                            ForEach(SearchSortOption.allCases) { option in
                                Text(option.label).tag(option)
                            }
                        }
                    } label: {
                        Image(systemName: "arrow.up.arrow.down")
                    }
                    .accessibilityLabel(Copy.Search.sortResults)
                    .accessibilityValue(activeSort.label)
                }
            }
            .task(id: request) {
                await results.load(request, backend: model.backend) { model.report($0) }
            }
    }

    private var request: SearchResultsModel.Request {
        SearchResultsModel.Request(query: query, sort: activeSort, origin: model.backend.origin,
                                   verification: model.verificationGeneration, attempt: attempt)
    }
}
