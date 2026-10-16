import SwiftUI

/// How the app lays itself out (docs/native-apps.md §6.1). `.compact` is the phone layout: an
/// iPhone, and an iPad window that's narrow (Slide Over, a narrow Split View). `.wide` is a
/// regular-width iPad window: a sidebar instead of the tab bar, each page in a centred column, card
/// and cover grids, and the book page in two columns when there's room.
enum AppLayout: Sendable, Equatable {
    case compact
    case wide

    init(_ sizeClass: UserInterfaceSizeClass?) {
        self = sizeClass == .regular ? .wide : .compact
    }
}

extension EnvironmentValues {
    /// Set by `RootView` from the window's horizontal size class.
    @Entry var appLayout: AppLayout = .compact
}

/// The wide layout's measurements (docs/native-apps.md §6.1), the desktop app's (docs/desktop-app.md §6).
enum PageLayout {
    /// A page's column is centred and at most this wide.
    static let contentMaxWidth: CGFloat = 1040
    /// The sidebar's width.
    static let sidebarWidth: CGFloat = 260
    /// The narrowest a book card gets in a card grid (Your Books, search results, My Picks).
    static let cardMinWidth: CGFloat = 300
    /// The most columns a card grid has.
    static let cardMaxColumns = 3
    /// Between cards in a card grid.
    static let cardSpacing = Theme.Spacing.m
    /// A cover in a cover grid (the book page's other books, My Picks on the device) is at least
    /// this wide.
    static let coverMinWidth: CGFloat = 120
    /// The most covers in a row of a cover grid.
    static let coverMaxColumns = 6
    /// Between covers in a cover grid.
    static let coverSpacing = Theme.Spacing.l
    /// The book page's left column (cover, title, Reserve and the white sheet) beside the other
    /// books.
    static let detailLeftWidth: CGFloat = 440
    /// The book page goes two-column when its page is at least this wide.
    static let detailTwoColumnMinWidth: CGFloat = 820

    /// How many `cardMinWidth` cards fit across `width`, 1 to `cardMaxColumns`.
    static func cardColumns(width: CGFloat) -> Int {
        let fits = Int(((width + cardSpacing) / (cardMinWidth + cardSpacing)).rounded(.down))
        return min(max(fits, 1), cardMaxColumns)
    }

    /// How many covers fit across `width` (at least two, at most `coverMaxColumns`), and each
    /// cover's width, filling the row.
    static func coverGrid(width: CGFloat) -> (columns: Int, coverWidth: CGFloat) {
        let fits = Int(((width + coverSpacing) / (coverMinWidth + coverSpacing)).rounded(.down))
        let columns = min(max(fits, 2), coverMaxColumns)
        let cover = ((width - coverSpacing * CGFloat(columns - 1)) / CGFloat(columns)).rounded(.down)
        return (columns, max(cover, 1))
    }

    /// `columns` equal grid columns, `spacing` apart.
    static func gridItems(_ columns: Int, spacing: CGFloat) -> [GridItem] {
        Array(repeating: GridItem(.flexible(), spacing: spacing, alignment: .top), count: columns)
    }
}

extension View {
    /// The wide layout's page column: at most `PageLayout.contentMaxWidth` wide, centred. The
    /// phone layout fills the screen as before.
    func pageColumn() -> some View {
        modifier(PageColumn())
    }
}

private struct PageColumn: ViewModifier {
    @Environment(\.appLayout) private var layout

    func body(content: Content) -> some View {
        if layout == .wide {
            content
                .frame(maxWidth: PageLayout.contentMaxWidth)
                .frame(maxWidth: .infinity)
        } else {
            content
        }
    }
}

/// Cards in rows of `columns`, each row as tall as its tallest card: the wide layout's card grids
/// (Your Books, search results, My Picks on the account; docs/native-apps.md §6.1). Rows are built
/// as they scroll into view. `card` gets each item's index in `items`.
struct CardRows<Item: Identifiable, Card: View>: View {
    let items: [Item]
    let columns: Int
    var spacing: CGFloat = PageLayout.cardSpacing
    @ViewBuilder let card: (_ index: Int, _ item: Item) -> Card

    var body: some View {
        let columns = max(columns, 1)
        LazyVStack(alignment: .leading, spacing: spacing) {
            ForEach(Array(stride(from: 0, to: items.count, by: columns)), id: \.self) { start in
                let end = min(start + columns, items.count)
                HStack(alignment: .top, spacing: spacing) {
                    ForEach(Array(items[start..<end].enumerated()), id: \.element.id) { offset, item in
                        card(start + offset, item)
                            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
                    }
                    // A short last row keeps its cards the width of the others.
                    ForEach(end..<(start + columns), id: \.self) { _ in
                        Color.clear
                            .frame(maxWidth: .infinity, maxHeight: 0)
                            .accessibilityHidden(true)
                    }
                }
                // Each card is asked its own height first; the row takes the tallest.
                .fixedSize(horizontal: false, vertical: true)
            }
        }
    }
}

extension View {
    /// Keeps `width` at this view's width (the wide layout's grids count their columns from it).
    func measuringWidth(_ width: Binding<CGFloat>) -> some View {
        onGeometryChange(for: CGFloat.self) { $0.size.width } action: { width.wrappedValue = $0 }
    }

    /// Keeps `height` at how far the window's controls (an iPad window that doesn't fill the
    /// screen has close, minimise and resize at its top leading corner) reach down into this view;
    /// 0 without them (§6.1).
    func measuringWindowControls(_ height: Binding<CGFloat>) -> some View {
        onGeometryChange(for: CGFloat.self) { proxy in
            if #available(iOS 26, *) {
                return proxy.containerCornerInsets.topLeading.height
            }
            return 0
        } action: { height.wrappedValue = $0 }
    }
}
