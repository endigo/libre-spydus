#if DEBUG
import SwiftUI

/// Every DesignSystem component on one screen (`-LSRoute gallery`, DEBUG only).
/// `-LSGalleryAnchor <rows|chips|buttons|sheet|states|type>` scrolls to a section (screenshots).
struct ComponentsGallery: View {
    @Environment(\.dismiss) private var dismiss
    @State private var query = "harry potter"
    @State private var bookmarked = true

    private let sample = BookRef(
        id: "brn:467086", title: "The long night", author: "White, Christian",
        coverUrl: "https://secure.syndetics.com/index.aspx?isbn=9781399197809/lc.gif&client=citycanning&upc=&oclc=&issn=",
        year: "2026",
        formats: [FormatRef(code: "BK", label: "Book"), FormatRef(code: "EBK", label: "eBook")]
    )
    private let noCover = BookRef(id: "title:gallery", title: "Thinking about water", author: "Bhatnagar, Nirat")

    var body: some View {
        NavigationStack {
            ScrollViewReader { proxy in
                ScrollView {
                    VStack(alignment: .leading, spacing: Theme.Spacing.xl) {
                        group("SearchField / SearchFieldButton") {
                            SearchField(text: $query)
                            SearchFieldButton {}
                        }
                        group("SectionHeader · ShelfCover · CoverImage") {
                            SectionHeader(title: "New arrivals") {}
                            ScrollView(.horizontal, showsIndicators: false) {
                                HStack(alignment: .top, spacing: Theme.Spacing.l) {
                                    ShelfCover(sample, caption: "3 days left")
                                    ShelfCover(noCover, caption: "Due today", captionColor: Theme.Colors.danger)
                                    ShelfCover(BookRef(id: "a", title: "More than words", author: "Santopolo, Jill"),
                                               showsTitle: true)
                                }
                                .padding(.bottom, Theme.Spacing.m)
                            }
                        }
                        group("GenreTile") {
                            HStack(spacing: Theme.Spacing.s) {
                                GenreTile(title: "Biographies", systemImage: "person.fill", color: Theme.Colors.genre[0]) {}
                                GenreTile(title: "Fiction", systemImage: "wand.and.stars", color: Theme.Colors.genre[1]) {}
                                GenreTile(title: "Science", systemImage: "atom", color: Theme.Colors.genre[2]) {}
                                GenreTile(title: "History", systemImage: "building.columns", color: Theme.Colors.genre[3]) {}
                            }
                        }
                        group("BookRowCard", id: "rows") {
                            BookRowCard(sample, meta: "Due 14 Oct · 2 days left", metaColor: Theme.Colors.danger)
                            BookRowCard(noCover, meta: "Ready to collect · Riverton", chips: sample.formats.map { Chip(format: $0) },
                                        actions: [.init(title: "Cancel", perform: {}), .init(title: "Edit", perform: {})],
                                        highlighted: true)
                        }
                        group("Chip · Banner", id: "chips") {
                            FlowRow {
                                Chip(text: "Book")
                                Chip(format: FormatRef(code: "EBK", label: "eBook"))
                                Chip(text: "Selected", tone: .selected)
                                Chip(text: "Available", tone: .success)
                                Chip(text: "Reserved", tone: .warning)
                                Chip(text: "Overdue", tone: .danger)
                            }
                            Banner(message: "Reservation placed", tone: .success)
                            Banner(message: "You already have a reservation for this title.", tone: .error)
                        }
                        group("PrimaryPillButton · CircleIconButton", id: "buttons") {
                            HStack(spacing: Theme.Spacing.l) {
                                PrimaryPillButton(title: "Reserve") {}
                                CircleIconButton(systemImage: bookmarked ? "bookmark.fill" : "bookmark",
                                                 accessibilityLabel: "Favourite") { bookmarked.toggle() }
                            }
                            HStack(spacing: Theme.Spacing.l) {
                                PrimaryPillButton(title: "Signing in", isLoading: true) {}
                                PrimaryPillButton(title: "Disabled") {}.disabled(true)
                            }
                        }
                        group("Detail sheet surface", id: "sheet") {
                            VStack(alignment: .leading, spacing: Theme.Spacing.m) {
                                SectionHeader(title: "Similar books", onSheet: true) {}
                                Text("Synopsis text on the white sheet.")
                                    .themeFont(.body)
                                    .foregroundStyle(Theme.Colors.onSheetSecondary)
                                FlowRow { Chip(text: "Fiction", tone: .onSheet); Chip(text: "Thriller", tone: .onSheet) }
                            }
                            .padding(Theme.Spacing.xl)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .sheetSurface()
                        }
                        group("LoadingView · EmptyStateView · ErrorStateView", id: "states") {
                            LoadingView(message: "Loading…").frame(height: 120)
                            EmptyStateView(systemImage: "magnifyingglass", title: "No results for “zzzz”",
                                           message: "Try another title or author.").frame(height: 200)
                            ErrorStateView(error: UserFacingError(kind: .network, message: "Can't reach Canning. Check your connection.",
                                                                  detail: "network error: connect timed out after 10s"),
                                           retry: {})
                                .frame(height: 320)
                        }
                        group("Type", id: "type") {
                            ForEach(ThemeFont.allCases, id: \.self) { token in
                                Text(String(describing: token)).themeFont(token).foregroundStyle(Theme.Colors.textPrimary)
                            }
                        }
                    }
                    .padding(Theme.Spacing.l)
                }
                .task {
                    guard let anchor = UserDefaults.standard.string(forKey: "LSGalleryAnchor") else { return }
                    try? await Task.sleep(for: .milliseconds(300))
                    proxy.scrollTo(anchor, anchor: .top)
                }
            }
            .screenBackground()
            .navigationTitle("Components")
            .navigationBarTitleDisplayMode(.inline)
            .navyNavigationBar()
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Done") { dismiss() }
                }
            }
            .safeAreaInset(edge: .bottom, spacing: 0) {
                LSTabBar(selection: .home) { _ in }
            }
        }
    }

    private func group(_ title: String, id: String? = nil, @ViewBuilder content: () -> some View) -> some View {
        VStack(alignment: .leading, spacing: Theme.Spacing.m) {
            Text(title.uppercased())
                .themeFont(.caption)
                .foregroundStyle(Theme.Colors.textMuted)
            content()
        }
        .id(id ?? title)
    }
}
#endif
