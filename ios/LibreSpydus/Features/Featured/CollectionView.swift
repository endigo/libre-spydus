import LibreSpydusCore
import SwiftUI

/// A Featured collection for `Route.collection` (§5.10), titled with the link: one shelf per
/// section (title, "View All" → its results list, the first page's covers). Shelves load as they
/// come into view, three at a time; an empty one is hidden, a failed one offers Retry in its row.
/// A collection without sections (eLibrary: links to outside services) offers its web page.
struct CollectionView: View {
    @Environment(AppModel.self) private var app
    @State private var model: CollectionModel
    @State private var attempt = 0
    @State private var browserPage: BrowserPage?

    init(link: FeaturedLink) {
        _model = State(initialValue: CollectionModel(link: link))
    }

    var body: some View {
        ScrollViewReader { proxy in
            content
                #if DEBUG
                .task(id: model.phase) { await applyDebugHooks(proxy) }
                #endif
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .screenBackground()
        .navigationTitle(model.title)
        .navigationBarTitleDisplayMode(.inline)
        .navyNavigationBar()
        .task(id: request) {
            await model.load(request, backend: app.backend) { app.report($0) }
        }
        .inAppBrowser($browserPage)
    }

    private var request: CollectionModel.Request {
        CollectionModel.Request(origin: app.backend.origin, verification: app.verificationGeneration, attempt: attempt)
    }

    @ViewBuilder
    private var content: some View {
        switch model.phase {
        case .loading:
            CollectionSkeleton()
        case .failed(let error):
            ErrorStateView(error: error) { attempt += 1 }
        case .noSections:
            EmptyStateView(systemImage: "globe", title: model.title,
                           message: Copy.Featured.websiteOnly,
                           actionTitle: Copy.Common.openWebsite) { openWebsite() }
        case .shelves where model.isEmpty:
            EmptyStateView(systemImage: FeaturedIcon.systemImage(for: model.link.icon), title: Copy.Featured.emptyTitle,
                           message: Copy.Featured.emptyMessage,
                           actionTitle: Copy.Common.openWebsite) { openWebsite() }
        case .shelves:
            shelves
        }
    }

    private var shelves: some View {
        ScrollView {
            LazyVStack(alignment: .leading, spacing: Theme.Spacing.xl) {
                ForEach(model.visibleShelves) { shelf in
                    CollectionShelfRow(shelf: shelf) {
                        app.push(shelf.route)
                    } retry: {
                        model.retryShelf(shelf.index)
                    }
                    .onAppear { model.shelfAppeared(shelf.index) }
                    .onDisappear { model.shelfDisappeared(shelf.index) }
                }
            }
            .padding(.top, Theme.Spacing.l)
            .padding(.bottom, Theme.Spacing.xxl)
        }
        .scrollIndicators(.hidden)
    }

    private func openWebsite() {
        guard let url = InAppBrowser.webURL(model.link.url) else { return }
        browserPage = BrowserPage(url: url)
    }

    #if DEBUG
    /// Evidence hooks, applied once: `-LSCollectionOpen <section>` opens that section's "View All";
    /// `-LSCollectionItem "<section>|<words>"` scrolls to the section and opens its first item whose
    /// title has those words; `-LSCollectionScrollTo <section>` scrolls to it;
    /// `-LSCollectionOpenWebsite YES` taps "Open website" on a collection without sections.
    private func applyDebugHooks(_ proxy: ScrollViewProxy) async {
        let defaults = UserDefaults.standard
        if model.phase == .noSections, defaults.bool(forKey: "LSCollectionOpenWebsite"), !CollectionDebugHooks.applied {
            CollectionDebugHooks.applied = true
            try? await Task.sleep(for: .milliseconds(800))
            openWebsite()
        }
        guard model.phase == .shelves, !CollectionDebugHooks.applied else { return }
        func section(_ title: String) -> CollectionModel.Shelf? {
            model.shelves.first { $0.section.title.caseInsensitiveCompare(title) == .orderedSame }
        }
        if let title = defaults.string(forKey: "LSCollectionOpen"), let shelf = section(title) {
            CollectionDebugHooks.applied = true
            app.push(shelf.route)
        } else if let hook = defaults.string(forKey: "LSCollectionItem") {
            let parts = hook.split(separator: "|", maxSplits: 1).map(String.init)
            guard let shelf = section(parts[0]) else { return }
            CollectionDebugHooks.applied = true
            try? await Task.sleep(for: .milliseconds(500))
            withAnimation { proxy.scrollTo(shelf.id, anchor: .top) }
            let words = parts.count > 1 ? parts[1].lowercased() : ""
            for _ in 0..<80 {
                if case .loaded(let refs) = model.shelves[shelf.index].state {
                    if let ref = refs.first(where: { words.isEmpty || $0.title.lowercased().contains(words) }) {
                        try? await Task.sleep(for: .milliseconds(600)) // covers
                        app.push(.detail(ref))
                    }
                    return
                }
                try? await Task.sleep(for: .milliseconds(250))
            }
        } else if let title = defaults.string(forKey: "LSCollectionScrollTo"), let shelf = section(title) {
            CollectionDebugHooks.applied = true
            try? await Task.sleep(for: .milliseconds(500))
            withAnimation { proxy.scrollTo(shelf.id, anchor: .top) }
        }
    }
    #endif
}

#if DEBUG
@MainActor
private enum CollectionDebugHooks {
    static var applied = false
}
#endif

/// One section: its title with "View All", then its covers (skeleton while loading, Retry on failure).
private struct CollectionShelfRow: View {
    let shelf: CollectionModel.Shelf
    let viewAll: () -> Void
    let retry: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: Theme.Spacing.m) {
            SectionHeader(title: shelf.section.title, action: viewAll)
                .padding(.horizontal, Theme.Spacing.l)
            switch shelf.state {
            case .idle, .queued, .loading:
                ShelfSkeleton()
            case .failed(let error):
                SectionErrorCard(error: error, retry: retry)
            case .loaded(let refs):
                CoverShelf(items: refs.map { ShelfEntry(ref: $0, caption: DetailText.displayTitle($0.title)) })
            }
        }
    }
}

/// The collection page loading: a few section headers over cover skeletons.
private struct CollectionSkeleton: View {
    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: Theme.Spacing.xl) {
                ForEach(0..<3, id: \.self) { _ in
                    VStack(alignment: .leading, spacing: Theme.Spacing.m) {
                        SkeletonBlock(width: 160, height: 16)
                            .padding(.horizontal, Theme.Spacing.l)
                        ShelfSkeleton()
                    }
                }
            }
            .padding(.top, Theme.Spacing.l)
        }
        .scrollDisabled(true)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(Copy.Common.loading)
    }
}
