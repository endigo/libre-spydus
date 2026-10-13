import LibreSpydusCore
import SwiftUI

/// Home: library logo (→ Profile), greeting with "My card" while signed in (→ the card sheet,
/// §5.9), search field (→ Search tab), "New arrivals",
/// "Popular genres" and "Your borrowed books". New arrivals loads with its own skeleton and error;
/// the loans come from the saved account, shown at once and fetched again in the background when
/// out of date. Pull to refresh; reloads on library, verification and sign-in changes.
struct HomeView: View {
    @Environment(AppModel.self) private var model
    @State private var home = HomeViewModel()
    @State private var showingCard = false
    /// The card sheet opens on its entry form (DEBUG `-LSMyCard edit`).
    @State private var cardStartsEditing = false
    #if DEBUG
    @State private var appliedDebugHook = false
    #endif

    var body: some View {
        ScrollViewReader { proxy in
            ScrollView {
                VStack(alignment: .leading, spacing: Theme.Spacing.xl) {
                    header
                    SearchFieldButton { model.openSearch() }
                        .padding(.horizontal, Theme.Spacing.l)
                    arrivalsSection
                    genresSection
                    loansSection
                        .id(Self.loansID)
                }
                .padding(.top, Theme.Spacing.s)
                .padding(.bottom, Theme.Spacing.xxl)
            }
            .scrollIndicators(.hidden)
            #if DEBUG
            .task { await applyDebugHooks(proxy) }
            #endif
        }
        .screenBackground()
        .overlay(alignment: .top) {
            // Covers scrolled up pass under a navy status bar, not behind the clock.
            Theme.Colors.background
                .ignoresSafeArea(edges: .top)
                .frame(height: 0)
        }
        .refreshable { await home.refresh(model) }
        .task(id: model.reloadKey) { await home.loadIfNeeded(model) }
        .onChange(of: model.selectedTab) { _, tab in
            // Back on Home: refresh the account in the background if it's out of date.
            guard tab == .home, model.isSignedIn else { return }
            Task { await model.refreshAccount(force: false) }
        }
        .toolbar(.hidden, for: .navigationBar)
        .onChange(of: showingCard) { _, showing in
            // Root sheets (login after an expired session) wait until the card has gone.
            if showing { model.isPresentingFeatureSheet = true }
        }
        .sheet(isPresented: $showingCard, onDismiss: {
            model.isPresentingFeatureSheet = false
            cardStartsEditing = false
        }) {
            MyCardSheet(cardNumber: model.cardNumber, knownSymbology: model.knownCardSymbology,
                        editing: cardStartsEditing)
                .environment(model)
        }
    }

    private static let loansID = "home.loans"

    // MARK: Header

    private var header: some View {
        VStack(alignment: .leading, spacing: Theme.Spacing.l) {
            Button {
                model.select(.profile)
            } label: {
                LibraryLogo(name: model.libraryName, logoUrl: model.library?.logoUrl, size: 44)
            }
            .buttonStyle(PressScaleStyle())
            .accessibilityLabel("\(model.libraryName) profile")
            .accessibilityHint("Opens Profile")
            HStack(alignment: .center, spacing: Theme.Spacing.m) {
                VStack(alignment: .leading, spacing: Theme.Spacing.xs) {
                    Text(HomeGreeting.title(accountName: model.accountName))
                        .themeFont(.greeting)
                        .foregroundStyle(Theme.Colors.textPrimary)
                        .accessibilityAddTraits(.isHeader)
                    Text(model.libraryName)
                        .themeFont(.body)
                        .foregroundStyle(Theme.Colors.textSecondary)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                if model.isSignedIn {
                    MyCardButton { showingCard = true }
                }
            }
        }
        .padding(.horizontal, Theme.Spacing.l)
    }

    // MARK: New arrivals

    private var arrivalsSection: some View {
        VStack(alignment: .leading, spacing: Theme.Spacing.m) {
            SectionHeader(title: home.arrivalsShelf?.title ?? "New arrivals",
                          action: home.arrivalsRoute.map { route in { model.push(route, on: .home) } })
                .padding(.horizontal, Theme.Spacing.l)
            switch home.arrivals {
            case .idle, .loading:
                ShelfSkeleton()
            case .failed(let error):
                SectionErrorCard(error: error) { Task { await home.retryArrivals(model) } }
            case .loaded(let books) where books.isEmpty:
                SectionMessageCard(systemImage: "sparkles", title: "Nothing new right now",
                                   message: "Check back soon for the latest additions.")
            case .loaded(let books):
                CoverShelf(items: books.map { ShelfEntry(ref: $0, caption: $0.title) })
            }
        }
    }

    // MARK: Genres

    private var genresSection: some View {
        VStack(alignment: .leading, spacing: Theme.Spacing.m) {
            SectionHeader(title: "Popular genres")
                .padding(.horizontal, Theme.Spacing.l)
            ScrollView(.horizontal) {
                HStack(alignment: .top, spacing: Theme.Spacing.s) {
                    ForEach(Array(home.genres.enumerated()), id: \.element.key) { index, genre in
                        GenreTile(title: genre.label, systemImage: GenreStyle.systemImage(for: genre.key),
                                  color: Theme.Colors.genre[index % Theme.Colors.genre.count]) {
                            model.push(HomeViewModel.route(for: genre), on: .home)
                        }
                        .accessibilityHint("Shows \(genre.label) books")
                    }
                }
                .padding(.horizontal, Theme.Spacing.l)
            }
            .scrollIndicators(.hidden)
            .scrollClipDisabled()
        }
    }

    // MARK: Your borrowed books

    private var loansSection: some View {
        VStack(alignment: .leading, spacing: Theme.Spacing.m) {
            SectionHeader(title: "Your borrowed books",
                          action: model.isSignedIn ? { model.select(.borrowed) } : nil)
                .padding(.horizontal, Theme.Spacing.l)
            if !model.isSignedIn {
                SignInCard(libraryName: model.libraryName) { model.loginNeeded = true }
            } else {
                switch home.loans(model) {
                case .idle, .loading:
                    ShelfSkeleton()
                case .failed(let error):
                    SectionErrorCard(error: error) { Task { await home.retryLoans(model) } }
                case .loaded(let loans) where loans.isEmpty:
                    SectionMessageCard(systemImage: "books.vertical", title: "No loans right now",
                                       message: "Books you borrow will show up here with their due dates.")
                case .loaded(let loans):
                    CoverShelf(items: loans.map { loan in
                        ShelfEntry(ref: loan.ref, caption: loan.caption, captionColor: color(for: loan.tone),
                                   accessibilityCaption: loan.caption)
                    })
                }
            }
        }
    }

    private func color(for tone: LoanShelfItem.Tone) -> Color {
        switch tone {
        case .normal: Theme.Colors.textSecondary
        case .soon: Theme.Colors.warning
        case .overdue: Theme.Colors.danger
        }
    }

    #if DEBUG
    /// Evidence hooks: `-LSHomeOpen arrivals` / `-LSHomeOpen genre:<key>` opens that list as if
    /// tapped; `-LSHomeScrollTo loans` scrolls down to "Your borrowed books"; `-LSMyCard open|edit`
    /// opens the card sheet (on its entry form), after `-LSDemoCardNumber` (`MyCardDebugHooks`).
    private func applyDebugHooks(_ proxy: ScrollViewProxy) async {
        guard !appliedDebugHook else { return }
        appliedDebugHook = true
        let defaults = UserDefaults.standard
        if let hook = defaults.string(forKey: "LSHomeOpen") {
            if hook == "arrivals", let route = home.arrivalsRoute {
                model.push(route, on: .home)
            } else if hook.hasPrefix("genre:"), let genre = home.genres.first(where: { $0.key == hook.dropFirst(6) }) {
                model.push(HomeViewModel.route(for: genre), on: .home)
            }
        }
        if let open = MyCardDebugHooks.apply(model) {
            cardStartsEditing = open == .edit
            showingCard = true
        }
        if defaults.string(forKey: "LSHomeScrollTo") == "loans" {
            try? await Task.sleep(for: .milliseconds(1500))
            withAnimation { proxy.scrollTo(Self.loansID, anchor: .center) }
        }
    }
    #endif
}

/// One cover on a Home shelf.
private struct ShelfEntry: Identifiable {
    let ref: BookRef
    var caption: String?
    var captionColor: Color = Theme.Colors.textSecondary
    /// Read after the title and author (loans: "3 days left").
    var accessibilityCaption: String?

    var id: String { ref.id }
}

/// A horizontal row of covers; tap → book detail.
private struct CoverShelf: View {
    let items: [ShelfEntry]

    var body: some View {
        ScrollView(.horizontal) {
            LazyHStack(alignment: .top, spacing: Theme.Spacing.m) {
                ForEach(items) { entry in
                    NavigationLink(value: Route.detail(entry.ref)) {
                        ShelfCover(entry.ref, caption: entry.caption, captionColor: entry.captionColor)
                    }
                    .buttonStyle(PressScaleStyle())
                    .accessibilityElement(children: .ignore)
                    .accessibilityLabel(label(for: entry))
                    .accessibilityAddTraits(.isButton)
                }
            }
            .padding(.horizontal, Theme.Spacing.l)
        }
        .scrollIndicators(.hidden)
        .scrollClipDisabled()
    }

    private func label(for entry: ShelfEntry) -> String {
        [entry.ref.title, Formatting.byline(entry.ref.author), entry.accessibilityCaption]
            .compactMap { $0 }
            .joined(separator: ", ")
    }
}
