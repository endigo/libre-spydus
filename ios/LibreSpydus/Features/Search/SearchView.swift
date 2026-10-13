import SwiftUI

/// The Search tab: search field (focused when Home hands over), Relevance / Newest, results with
/// infinite scroll, and recent searches while the field is empty. Searches run on Return only.
struct SearchView: View {
    @Environment(AppModel.self) private var model
    @State private var text = ""
    /// The query whose results are shown (set on Return).
    @State private var submitted: String?
    @State private var sort: SearchSortOption = Self.initialSort
    @State private var attempt = 0
    @State private var results = SearchResultsModel()
    @State private var recents = RecentSearchStore()
    @FocusState private var fieldFocused: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            header
            if showsResults, let submitted {
                SearchResultsList(results: results, query: submitted) { attempt += 1 }
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                RecentSearchesView(recents: recents, libraryName: model.libraryName) { query in
                    text = query
                    submit()
                }
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        .screenBackground()
        .toolbar(.hidden, for: .navigationBar)
        .task(id: request) {
            guard let request else { return }
            await results.load(request, backend: model.backend) { model.report($0) }
        }
        .onChange(of: text) { _, newText in
            // Clearing the field ends the search: recent searches show until the next Return.
            if newText.trimmingCharacters(in: .whitespaces).isEmpty { submitted = nil }
        }
        .onChange(of: model.pendingSearch, initial: true) { _, pending in
            guard pending != nil else { return }
            takePendingSearch()
        }
    }

    private var header: some View {
        VStack(alignment: .leading, spacing: Theme.Spacing.m) {
            Text(Copy.Search.title)
                .themeFont(.title)
                .foregroundStyle(Theme.Colors.textPrimary)
                .accessibilityAddTraits(.isHeader)
            SearchField(text: $text, onSubmit: submit)
                .focused($fieldFocused)
            if showsResults {
                Picker(Copy.Search.sort, selection: $sort) {
                    ForEach(SearchSortOption.allCases) { option in
                        Text(option.label).tag(option)
                    }
                }
                .pickerStyle(.segmented)
                .accessibilityLabel(Copy.Search.sortResults)
            }
        }
        .padding(.horizontal, Theme.Spacing.l)
        .padding(.top, Theme.Spacing.s)
        .padding(.bottom, Theme.Spacing.m)
    }

    /// Results for the submitted query; recent searches once the field is cleared.
    private var showsResults: Bool { submitted != nil }

    private var request: SearchResultsModel.Request? {
        guard let submitted else { return nil }
        return SearchResultsModel.Request(query: submitted, sort: sort, origin: model.backend.origin,
                                          verification: model.verificationGeneration, attempt: attempt)
    }

    private func submit() {
        let query = text.split(whereSeparator: \.isWhitespace).joined(separator: " ")
        guard !query.isEmpty else { return }
        text = query
        recents.record(query)
        submitted = query
        attempt += 1
        fieldFocused = false
    }

    /// Home's search field: focus the field, or run the query it handed over.
    private func takePendingSearch() {
        guard let pending = model.takePendingSearch() else { return }
        let query = pending.trimmingCharacters(in: .whitespacesAndNewlines)
        if query.isEmpty {
            Task {
                // Focus only sticks once the tab switch has been laid out.
                try? await Task.sleep(for: .milliseconds(250))
                fieldFocused = true
            }
        } else {
            text = query
            submit()
        }
    }

    private static var initialSort: SearchSortOption {
        #if DEBUG
        // Evidence hook: `-LSSearchSort newest`.
        if let raw = UserDefaults.standard.string(forKey: "LSSearchSort"), let sort = SearchSortOption(rawValue: raw) {
            return sort
        }
        #endif
        return .relevance
    }
}

/// Recent searches (tap to run again, Clear), or a prompt when there are none.
private struct RecentSearchesView: View {
    let recents: RecentSearchStore
    let libraryName: String
    let run: (String) -> Void

    var body: some View {
        if recents.items.isEmpty {
            EmptyStateView(systemImage: "books.vertical",
                           title: Copy.Search.introTitle(library: libraryName),
                           message: Copy.Search.introMessage)
        } else {
            ScrollView {
                VStack(alignment: .leading, spacing: 0) {
                    SectionHeader(title: Copy.Search.recentSearches, actionTitle: Copy.Common.clear) { recents.clear() }
                        .padding(.bottom, Theme.Spacing.s)
                    ForEach(recents.items, id: \.self) { query in
                        Button {
                            run(query)
                        } label: {
                            HStack(spacing: Theme.Spacing.m) {
                                Image(systemName: "clock.arrow.circlepath")
                                    .foregroundStyle(Theme.Colors.textMuted)
                                    .accessibilityHidden(true)
                                Text(query)
                                    .themeFont(.body)
                                    .foregroundStyle(Theme.Colors.textPrimary)
                                    .lineLimit(2)
                                    .multilineTextAlignment(.leading)
                                Spacer(minLength: Theme.Spacing.s)
                                Image(systemName: "arrow.up.left")
                                    .font(.footnote)
                                    .foregroundStyle(Theme.Colors.textMuted)
                                    .accessibilityHidden(true)
                            }
                            .padding(.vertical, Theme.Spacing.m)
                            .contentShape(Rectangle())
                        }
                        .buttonStyle(.plain)
                        // The label is the query as shown, so Voice Control can match it.
                        .accessibilityHint(Copy.Search.recentSearchHint)
                        .contextMenu {
                            Button(Copy.Common.remove, systemImage: "trash", role: .destructive) { recents.remove(query) }
                        }
                        Divider().overlay(Theme.Colors.divider)
                    }
                }
                .padding(.horizontal, Theme.Spacing.l)
                .padding(.top, Theme.Spacing.s)
                .padding(.bottom, Theme.Spacing.xxl)
            }
            .scrollDismissesKeyboard(.immediately)
        }
    }
}
