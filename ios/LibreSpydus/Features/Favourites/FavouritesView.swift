import SwiftUI

/// The My Picks tab (docs/native-apps.md §5.11, §6): signed in, the Pick list on the library
/// account (`MyPicksView`); signed out, the books saved on this phone (`LocalFavouritesView`).
struct FavouritesView: View {
    @Environment(AppModel.self) private var model

    var body: some View {
        switch Content(model) {
        case .myPicks: MyPicksView()
        case .local: LocalFavouritesView()
        }
    }

    /// Which list the tab shows.
    enum Content: Equatable {
        case myPicks
        case local

        @MainActor
        init(_ model: AppModel) {
            self = model.isSignedIn ? .myPicks : .local
        }

        /// "My Picks" either way (§7.1); the subtitle says where the books are saved.
        var title: String { Copy.Picks.title }
        var subtitle: String {
            switch self {
            case .myPicks: Copy.Picks.subtitleAccount
            case .local: Copy.Picks.subtitlePhone
            }
        }
    }

    /// Two columns across the screen's gutters.
    static func columnWidth(for width: CGFloat) -> CGFloat {
        max(80, ((width - Theme.Spacing.l * 3) / 2).rounded(.down))
    }
}

/// My Picks while signed out (§5.3, §6): this library's books saved on this phone, newest first, as a
/// two-column cover grid. Tap → detail; remove from the context menu or in Edit mode. They're local
/// and per library, so switching library shows that library's list.
struct LocalFavouritesView: View {
    @Environment(AppModel.self) private var model
    /// `-LSFavouritesEditing YES` (DEBUG) opens in Edit mode, for screenshots.
    @State private var isEditing = Self.startsEditing

    var body: some View {
        let items = model.favourites.items
        GeometryReader { proxy in
            ScrollView {
                VStack(alignment: .leading, spacing: Theme.Spacing.l) {
                    header(count: items.count)
                    if let error = model.favourites.lastError {
                        Banner(message: error, tone: .error)
                    }
                    if items.isEmpty {
                        empty
                    } else {
                        grid(items, columnWidth: FavouritesView.columnWidth(for: proxy.size.width))
                    }
                }
                .padding(.horizontal, Theme.Spacing.l)
                .padding(.bottom, Theme.Spacing.xxl)
                .animation(.default, value: items)
                .animation(.default, value: isEditing)
            }
            .defaultScrollAnchor(BorrowedDebugHooks.initialScrollAnchor)
        }
        .screenBackground()
        .toolbar(.hidden, for: .navigationBar)
        .onChange(of: items.isEmpty) { _, isEmpty in
            if isEmpty { isEditing = false }
        }
        .onChange(of: model.favourites.origin) { _, _ in isEditing = false }
    }

    // MARK: Header

    private func header(count: Int) -> some View {
        HStack(alignment: .firstTextBaseline) {
            VStack(alignment: .leading, spacing: Theme.Spacing.xs) {
                Text(FavouritesView.Content.local.title)
                    .themeFont(.greeting)
                    .foregroundStyle(Theme.Colors.textPrimary)
                    .accessibilityAddTraits(.isHeader)
                Text(FavouritesView.Content.local.subtitle)
                    .themeFont(.caption)
                    .foregroundStyle(Theme.Colors.textSecondary)
            }
            Spacer()
            if count > 0 {
                Button(isEditing ? Copy.Common.done : Copy.Common.edit) { isEditing.toggle() }
                    .themeFont(.bodyStrong)
                    .foregroundStyle(Theme.Colors.primary)
                    .buttonStyle(.borderless)
                    .accessibilityHint(isEditing ? Copy.Picks.doneEditingHint : Copy.Picks.editHint)
            }
        }
        .padding(.top, Theme.Spacing.l)
    }

    // MARK: Grid

    private func grid(_ items: [BookRef], columnWidth: CGFloat) -> some View {
        let columns = Array(repeating: GridItem(.fixed(columnWidth), spacing: Theme.Spacing.l, alignment: .top), count: 2)
        return LazyVGrid(columns: columns, alignment: .leading, spacing: Theme.Spacing.xl) {
            ForEach(items) { ref in
                cell(ref, width: columnWidth)
            }
        }
    }

    private func cell(_ ref: BookRef, width: CGFloat) -> some View {
        NavigationLink(value: Route.detail(ref)) {
            ShelfCover(ref, caption: Formatting.authorName(ref.author), width: width, showsTitle: true)
        }
        .buttonStyle(PressScaleStyle())
        .disabled(isEditing)
        .overlay(alignment: .topTrailing) {
            if isEditing {
                Button {
                    remove(ref)
                } label: {
                    Image(systemName: "minus")
                        .font(.system(size: 15, weight: .bold))
                        .foregroundStyle(Theme.Colors.onPrimary)
                        .frame(width: 30, height: 30)
                        .background(Circle().fill(Theme.Colors.danger))
                        .overlay(Circle().strokeBorder(Theme.Colors.background, lineWidth: 2))
                        .frame(width: 44, height: 44)
                        .contentShape(Circle())
                }
                .buttonStyle(PressScaleStyle())
                .offset(x: 10, y: -10)
                .accessibilityLabel(Copy.Picks.removeTitle(title: ref.title))
                .transition(.scale.combined(with: .opacity))
            }
        }
        .contextMenu {
            Button(Copy.Common.open, systemImage: "book") { model.push(.detail(ref), on: .favourites) }
            Button(Copy.Picks.remove, systemImage: "bookmark.slash", role: .destructive) { remove(ref) }
        } preview: {
            CoverImage(url: ref.coverUrl, title: ref.title, author: ref.author, width: 200, shadow: false)
        }
        .accessibilityActions {
            Button(Copy.Picks.remove) { remove(ref) }
        }
    }

    private var empty: some View {
        EmptyStateView(systemImage: "bookmark", title: Copy.Picks.emptyTitle,
                       message: Copy.Picks.emptyPhoneMessage,
                       actionTitle: Copy.Picks.findABook) { model.openSearch() }
            .frame(minHeight: 420)
    }

    private func remove(_ ref: BookRef) {
        withAnimation { model.favourites.remove(id: ref.id) }
        AccessibilityNotification.Announcement(Copy.Picks.removedTitle(title: ref.title)).post()
    }

    private static var startsEditing: Bool {
        #if DEBUG
        UserDefaults.standard.bool(forKey: "LSFavouritesEditing")
        #else
        false
        #endif
    }
}
