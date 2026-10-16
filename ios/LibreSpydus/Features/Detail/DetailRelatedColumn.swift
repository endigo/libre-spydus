import SwiftUI

/// The book page's right column in the wide layout (docs/native-apps.md §6.1): the other books
/// (similar, by the author, in the series, other editions) as cover grids on navy, in place of the
/// sheet's carousels. While the book loads, two untitled grids of blank covers hold the space, so
/// the page opens in the two columns it most likely ends up with.
struct DetailRelatedColumn: View {
    let model: BookDetailModel
    /// The column's width: the grids fill it.
    let width: CGFloat
    /// Starts the related lists' loads (once the record is in).
    let startRelated: () -> Void

    var body: some View {
        let grid = PageLayout.coverGrid(width: width)
        LazyVStack(alignment: .leading, spacing: Theme.Spacing.xl) {
            if model.record.value == nil {
                ForEach(0..<2, id: \.self) { _ in
                    VStack(alignment: .leading, spacing: Theme.Spacing.m) {
                        SkeletonBlock(width: 160, height: 18)
                        covers(grid) { _ in placeholder(grid.coverWidth) }
                    }
                }
                .accessibilityHidden(true)
            } else {
                ForEach(model.visiblePanels) { panel in
                    section(panel, grid: grid)
                }
            }
        }
        .task(id: model.record.value?.related) {
            if model.hasRelatedPanels { startRelated() }
        }
    }

    /// True while this column has something to show: the book is loading, or it has other books
    /// that are loading or loaded.
    static func isShown(for model: BookDetailModel) -> Bool {
        switch model.record {
        case .idle, .loading: true
        case .failed: false
        case .loaded: !model.visiblePanels.isEmpty
        }
    }

    private func section(_ panel: RelatedPanel, grid: (columns: Int, coverWidth: CGFloat)) -> some View {
        let state = model.related[panel] ?? .loading
        return VStack(alignment: .leading, spacing: Theme.Spacing.m) {
            SectionHeader(title: panel.title)
            Group {
                if let refs = state.value {
                    LazyVGrid(columns: columns(grid), alignment: .leading, spacing: Theme.Spacing.xl) {
                        ForEach(refs) { ref in
                            NavigationLink(value: Route.detail(ref)) {
                                ShelfCover(title: DetailText.displayTitle(ref.title), author: ref.author,
                                           coverUrl: ref.coverUrl, caption: Formatting.authorName(ref.author) ?? ref.year,
                                           width: grid.coverWidth, showsTitle: true)
                            }
                            .buttonStyle(PressScaleStyle())
                        }
                    }
                } else {
                    covers(grid) { _ in placeholder(grid.coverWidth) }
                        .accessibilityHidden(true)
                }
            }
            .accessibilityElement(children: .contain)
            .accessibilityLabel(state.isLoading ? Copy.Detail.loadingPanel(panel: panel.title.lowercased()) : panel.title)
        }
    }

    /// One row of `grid`'s width.
    private func covers(_ grid: (columns: Int, coverWidth: CGFloat),
                        @ViewBuilder cover: @escaping (Int) -> some View) -> some View {
        LazyVGrid(columns: columns(grid), alignment: .leading, spacing: Theme.Spacing.xl) {
            ForEach(0..<grid.columns, id: \.self) { index in cover(index) }
        }
    }

    private func columns(_ grid: (columns: Int, coverWidth: CGFloat)) -> [GridItem] {
        Array(repeating: GridItem(.fixed(grid.coverWidth), spacing: PageLayout.coverSpacing, alignment: .top),
              count: grid.columns)
    }

    private func placeholder(_ width: CGFloat) -> some View {
        RoundedRectangle(cornerRadius: Theme.Radius.cover, style: .continuous)
            .fill(Theme.Colors.surface)
            .frame(width: width, height: Theme.Cover.height(forWidth: width))
    }
}
