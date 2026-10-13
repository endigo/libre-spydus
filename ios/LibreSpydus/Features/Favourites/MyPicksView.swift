import SwiftUI

/// Favourites while signed in (docs/native-apps.md §5.11): "My Picks", the Pick list on the library
/// account (`AppModel.picks`), in rows exactly like search results (cover, title, author, year,
/// format chips; none for an edition-level item). Tap → book page. Swipe left to remove (also in
/// the long-press menu and as a VoiceOver action): the row leaves at once and comes back if the
/// library refuses. Pull to refresh; pages past the first load as the list scrolls.
struct MyPicksView: View {
    @Environment(AppModel.self) private var model

    var body: some View {
        let picks = model.picks
        List {
            header
                .plainRow(EdgeInsets(top: Theme.Spacing.l, leading: Theme.Spacing.l, bottom: Theme.Spacing.m,
                                     trailing: Theme.Spacing.l))
            content(picks)
        }
        .listStyle(.plain)
        .scrollContentBackground(.hidden)
        .environment(\.defaultMinListRowHeight, 0)
        .refreshable { await picks.refresh() }
        .animation(.default, value: picks.items.map(\.id))
        .screenBackground()
        .toolbar(.hidden, for: .navigationBar)
        // Launch, sign-in, verification, and coming back to the tab after a save elsewhere.
        .task(id: LoadTrigger(reload: model.reloadKey, onScreen: model.selectedTab == .favourites, stale: picks.isStale)) {
            await picks.loadIfNeeded()
        }
    }

    private struct LoadTrigger: Hashable {
        var reload: AppModel.ReloadKey
        var onScreen: Bool
        var stale: Bool
    }

    private static let rowInsets = EdgeInsets(top: Theme.Spacing.m / 2, leading: Theme.Spacing.l,
                                              bottom: Theme.Spacing.m / 2, trailing: Theme.Spacing.l)

    private var header: some View {
        VStack(alignment: .leading, spacing: Theme.Spacing.xs) {
            Text(PicksText.title)
                .themeFont(.greeting)
                .foregroundStyle(Theme.Colors.textPrimary)
                .accessibilityAddTraits(.isHeader)
            Text(PicksText.subtitle)
                .themeFont(.caption)
                .foregroundStyle(Theme.Colors.textSecondary)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    @ViewBuilder
    private func content(_ picks: PicksStore) -> some View {
        switch picks.phase {
        case .idle, .loading:
            SearchSkeletonList()
                .frame(height: 560, alignment: .top)
                .padding(.horizontal, -Theme.Spacing.l)
                .plainRow(Self.rowInsets)
        case .empty:
            EmptyStateView(systemImage: "bookmark", title: PicksText.emptyTitle, message: PicksText.emptyMessage)
                .frame(minHeight: 420)
                .plainRow(Self.rowInsets)
        case .failed(let error):
            // ErrorStateView hides Retry for errors that retrying can't fix.
            ErrorStateView(error: error) { Task { await picks.refresh() } }
                .frame(minHeight: 420)
                .plainRow(Self.rowInsets)
        case .loaded:
            ForEach(Array(picks.items.enumerated()), id: \.element.id) { index, item in
                row(item)
                    .onAppear {
                        // Prefetch a few rows before the end.
                        if index >= picks.items.count - 4 { Task { await picks.loadMore() } }
                    }
            }
            footer(picks)
        }
    }

    private func row(_ item: SearchResultItem) -> some View {
        // The search results row's card. A button rather than a navigation link, which a list
        // would give a disclosure chevron.
        Button {
            model.push(.detail(item.ref), on: .favourites)
        } label: {
            SearchResultCard(item: item)
        }
        .buttonStyle(PressScaleStyle())
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(SearchResultCard.accessibilityText(item))
        .accessibilityAddTraits(.isButton)
        .plainRow(Self.rowInsets)
        .swipeActions(edge: .trailing, allowsFullSwipe: true) {
            Button(role: .destructive) {
                remove(item)
            } label: {
                Label("Remove", systemImage: "bookmark.slash")
            }
            .tint(Theme.Colors.danger)
        }
        .contextMenu {
            Button("Open", systemImage: "book") { model.push(.detail(item.ref), on: .favourites) }
            Button(PicksText.remove, systemImage: "bookmark.slash", role: .destructive) { remove(item) }
        } preview: {
            CoverImage(url: item.ref.coverUrl, title: item.ref.title, author: item.ref.author, width: 200, shadow: false)
        }
        .accessibilityAction(named: PicksText.remove) { remove(item) }
    }

    @ViewBuilder
    private func footer(_ picks: PicksStore) -> some View {
        if let error = picks.loadMoreError {
            VStack(spacing: Theme.Spacing.s) {
                Text(error.message)
                    .themeFont(.caption)
                    .foregroundStyle(Theme.Colors.textSecondary)
                    .multilineTextAlignment(.center)
                if error.canRetry {
                    Button {
                        Task { await picks.retryLoadMore() }
                    } label: {
                        Label("Retry", systemImage: "arrow.clockwise")
                            .themeFont(.bodyStrong)
                            .foregroundStyle(Theme.Colors.primary)
                    }
                    .buttonStyle(.borderless)
                    .accessibilityLabel("Retry loading more picks")
                }
            }
            .frame(maxWidth: .infinity)
            .padding(.vertical, Theme.Spacing.l)
            .plainRow(Self.rowInsets)
        } else if picks.nextPageUrl != nil {
            ProgressView()
                .controlSize(.regular)
                .tint(Theme.Colors.textSecondary)
                .frame(maxWidth: .infinity)
                .padding(.vertical, Theme.Spacing.l)
                .accessibilityLabel("Loading more picks")
                .task(id: picks.nextPageUrl) { await picks.loadMore() }
                .plainRow(Self.rowInsets)
        }
    }

    private func remove(_ item: SearchResultItem) {
        Task { await model.picks.remove(item) }
    }
}

private extension View {
    /// A list row that draws only its content: no background, separator or default insets.
    func plainRow(_ insets: EdgeInsets) -> some View {
        listRowInsets(insets)
            .listRowBackground(Color.clear)
            .listRowSeparator(.hidden)
    }
}
