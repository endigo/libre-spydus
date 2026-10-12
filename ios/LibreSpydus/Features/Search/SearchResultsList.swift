import SwiftUI

/// The results list both the Search tab and `SearchResultsView` show: count line, result rows
/// (tap → detail), infinite scroll with a footer spinner, and the loading / no-results / error
/// states.
struct SearchResultsList: View {
    let results: SearchResultsModel
    let query: String
    /// Re-runs the first page (bumps the request's `attempt`).
    let retry: () -> Void

    var body: some View {
        switch results.phase {
        case .idle, .loading:
            SearchSkeletonList()
        case .noResults:
            EmptyStateView(systemImage: "text.magnifyingglass", title: "No results for “\(query)”",
                           message: "Check the spelling, or try fewer or different words.")
        case .failed(let error):
            // ErrorStateView hides Retry for errors that retrying can't fix.
            ErrorStateView(error: error, retry: retry)
        case .loaded:
            loadedList
        }
    }

    private var loadedList: some View {
        ScrollViewReader { proxy in
            ScrollView {
                LazyVStack(alignment: .leading, spacing: Theme.Spacing.m) {
                    if let count = Formatting.resultCount(results.total) {
                        Text(count)
                            .themeFont(.caption)
                            .foregroundStyle(Theme.Colors.textSecondary)
                            .padding(.horizontal, Theme.Spacing.l)
                            .accessibilityAddTraits(.isHeader)
                    }
                    ForEach(Array(results.items.enumerated()), id: \.element.id) { index, item in
                        SearchResultRow(item: item)
                            .padding(.horizontal, Theme.Spacing.l)
                            .onAppear {
                                // Prefetch a few rows before the end.
                                if index >= results.items.count - 4 {
                                    Task { await results.loadMore() }
                                }
                            }
                    }
                    footer
                        .id(Self.footerID)
                }
                .padding(.bottom, Theme.Spacing.xxl)
            }
            .scrollDismissesKeyboard(.immediately)
            #if DEBUG
            .task {
                // Evidence hook: `-LSResultsScrollToEnd YES` shows the paging footer.
                guard UserDefaults.standard.bool(forKey: "LSResultsScrollToEnd") else { return }
                try? await Task.sleep(for: .milliseconds(600))
                withAnimation { proxy.scrollTo(Self.footerID, anchor: .center) }
            }
            #endif
        }
    }

    private static var footerID: String { "results.footer" }

    @ViewBuilder
    private var footer: some View {
        Group {
            if let error = results.loadMoreError {
                VStack(spacing: Theme.Spacing.s) {
                    Text(error.message)
                        .themeFont(.caption)
                        .foregroundStyle(Theme.Colors.textSecondary)
                        .multilineTextAlignment(.center)
                    if error.canRetry {
                        Button {
                            Task { await results.retryLoadMore() }
                        } label: {
                            Label("Retry", systemImage: "arrow.clockwise")
                                .themeFont(.bodyStrong)
                                .foregroundStyle(Theme.Colors.primary)
                        }
                        .buttonStyle(.borderless)
                        .accessibilityLabel("Retry loading more results")
                    }
                }
            } else if results.hasMore {
                ProgressView()
                    .controlSize(.regular)
                    .tint(Theme.Colors.textSecondary)
                    .accessibilityLabel("Loading more results")
                    .task(id: results.nextPageUrl) { await results.loadMore() }
            } else if !results.items.isEmpty {
                Text("End of results")
                    .themeFont(.caption)
                    .foregroundStyle(Theme.Colors.textMuted)
            }
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, Theme.Spacing.l)
        .padding(.horizontal, Theme.Spacing.l)
    }
}

/// A result: cover, title, author, year and format chips (digital formats tinted).
struct SearchResultRow: View {
    let item: SearchResultItem
    static let maxChips = 4

    var body: some View {
        NavigationLink(value: Route.detail(item.ref)) {
            BookRowCard(item.ref, meta: item.ref.year, chips: chips)
                .contentShape(RoundedRectangle(cornerRadius: Theme.Radius.card, style: .continuous))
        }
        .buttonStyle(PressScaleStyle())
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(accessibilityText)
        .accessibilityAddTraits(.isButton)
    }

    private var chips: [Chip] {
        var chips = item.formats.prefix(Self.maxChips).map { Chip(format: $0) }
        if item.formats.count > Self.maxChips {
            chips.append(Chip(text: "+\(item.formats.count - Self.maxChips)"))
        }
        return chips
    }

    private var accessibilityText: String {
        var parts = [item.ref.title]
        if let byline = Formatting.byline(item.ref.author) { parts.append(byline) }
        if let year = item.ref.year { parts.append(year) }
        let formats = item.formats.map(\.displayLabel)
        if !formats.isEmpty { parts.append(formats.joined(separator: ", ")) }
        return parts.joined(separator: ". ")
    }
}

/// Placeholder rows while the first page loads.
struct SearchSkeletonList: View {
    var rows = 5

    var body: some View {
        ScrollView {
            VStack(spacing: Theme.Spacing.m) {
                ForEach(0..<rows, id: \.self) { _ in
                    HStack(alignment: .top, spacing: Theme.Spacing.l) {
                        SkeletonBlock(width: Theme.Cover.smallWidth,
                                      height: Theme.Cover.height(forWidth: Theme.Cover.smallWidth),
                                      radius: Theme.Radius.cover)
                        VStack(alignment: .leading, spacing: Theme.Spacing.s) {
                            SkeletonBlock(height: 14)
                            SkeletonBlock(width: 140, height: 12)
                            SkeletonBlock(width: 90, height: 18, radius: 9)
                                .padding(.top, Theme.Spacing.xs)
                        }
                    }
                    .padding(Theme.Spacing.l)
                    .background(RoundedRectangle(cornerRadius: Theme.Radius.card, style: .continuous)
                        .fill(Theme.Colors.surface))
                }
            }
            .padding(.horizontal, Theme.Spacing.l)
            .padding(.top, Theme.Spacing.s)
        }
        .scrollDisabled(true)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("Loading results")
    }
}
