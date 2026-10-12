import SwiftUI

/// Favourites (docs/native-apps.md §6): this library's saved books, newest first, as a two-column
/// cover grid. Tap → detail; remove from the context menu or in Edit mode. Favourites are local
/// and per library, so switching library shows that library's list.
struct FavouritesView: View {
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
                        grid(items, columnWidth: Self.columnWidth(for: proxy.size.width))
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
                Text("Favourites")
                    .themeFont(.greeting)
                    .foregroundStyle(Theme.Colors.textPrimary)
                    .accessibilityAddTraits(.isHeader)
                Text(count > 0 ? "\(count) saved for \(model.libraryName)" : "Saved on this device for \(model.libraryName)")
                    .themeFont(.caption)
                    .foregroundStyle(Theme.Colors.textSecondary)
            }
            Spacer()
            if count > 0 {
                Button(isEditing ? "Done" : "Edit") { isEditing.toggle() }
                    .themeFont(.bodyStrong)
                    .foregroundStyle(Theme.Colors.primary)
                    .buttonStyle(.borderless)
                    .accessibilityLabel(isEditing ? "Done editing favourites" : "Edit favourites")
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
                .accessibilityLabel("Remove \(ref.title) from Favourites")
                .transition(.scale.combined(with: .opacity))
            }
        }
        .contextMenu {
            Button("Open", systemImage: "book") { model.push(.detail(ref), on: .favourites) }
            Button("Remove from Favourites", systemImage: "bookmark.slash", role: .destructive) { remove(ref) }
        } preview: {
            CoverImage(url: ref.coverUrl, title: ref.title, author: ref.author, width: 200, shadow: false)
        }
        .accessibilityActions {
            Button("Remove from Favourites") { remove(ref) }
        }
    }

    private var empty: some View {
        EmptyStateView(systemImage: "bookmark", title: "No favourites yet",
                       message: "Tap the bookmark on any book to save it here.",
                       actionTitle: "Find a book") { model.openSearch() }
            .frame(minHeight: 420)
    }

    private func remove(_ ref: BookRef) {
        withAnimation { model.favourites.remove(id: ref.id) }
        AccessibilityNotification.Announcement("Removed \(ref.title) from Favourites").post()
    }

    private static var startsEditing: Bool {
        #if DEBUG
        UserDefaults.standard.bool(forKey: "LSFavouritesEditing")
        #else
        false
        #endif
    }

    /// Two columns across the screen's gutters.
    static func columnWidth(for width: CGFloat) -> CGFloat {
        max(80, ((width - Theme.Spacing.l * 3) / 2).rounded(.down))
    }
}
