import SwiftUI

/// The app shell: five tab stacks under the custom tab bar, or beside the sidebar in the wide
/// layout (docs/native-apps.md §6.1), plus the root sheets (library picker, login, verification)
/// driven by `AppModel` flags.
struct RootView: View {
    @Environment(AppModel.self) private var model
    @Environment(\.scenePhase) private var scenePhase
    @Environment(\.horizontalSizeClass) private var sizeClass
    /// Tabs are built on first visit and then kept alive, so each keeps its stack and state.
    @State private var visitedTabs: Set<AppTab> = []
    /// How far the window's controls reach into the app, in a window that doesn't fill the iPad's
    /// screen (§6.1): what's in that corner (a tab screen's title, the sidebar's library) starts
    /// below them.
    @State private var windowControlsHeight: CGFloat = 0

    var body: some View {
        @Bindable var model = model
        let layout = AppLayout(sizeClass)
        Group {
            switch layout {
            case .compact:
                tabs(topClearance: windowControlsHeight)
                    .safeAreaInset(edge: .bottom, spacing: 0) {
                        LSTabBar(selection: model.selectedTab) { model.select($0) }
                    }
            case .wide:
                HStack(spacing: 0) {
                    LSSidebar(selection: model.selectedTab) { model.select($0) }
                        .safeAreaPadding(.top, windowControlsHeight)
                    // Shelves draw past their scroll views' edges, but never over the sidebar (the
                    // mask reaches under the status bar, so its backdrop stays).
                    tabs(topClearance: 0).mask { Rectangle().ignoresSafeArea() }
                }
            }
        }
        .measuringWindowControls($windowControlsHeight)
        .environment(\.appLayout, layout)
        .ignoresSafeArea(.keyboard, edges: .bottom)
        .onChange(of: model.selectedTab, initial: true) { _, tab in
            visitedTabs.insert(tab)
        }
        .sheet(item: rootSheet) { sheet in
            Group {
                switch sheet {
                case .libraryPicker: LibraryPickerView()
                case .login: LoginView()
                case .verification: VerificationView()
                }
            }
            .environment(model)
        }
        .fullScreenCover(isPresented: onboarding) {
            LibraryPickerView()
                .environment(model)
        }
        #if DEBUG
        .fullScreenCover(isPresented: $model.componentsGalleryPresented) {
            ComponentsGallery()
                .environment(model)
        }
        #endif
        .task(id: model.backend.origin) {
            // Launch and library change: the saved account is on screen; fetch a fresh one.
            await model.refreshAccount()
        }
        .task(id: model.reloadKey) {
            // Signed in (launch, sign-in, library change, verification): which books are in My Picks.
            await model.picks.loadIfNeeded()
        }
        .biometricOfferAlert(model)
        .picksCopyOfferAlert(model)
        .onChange(of: scenePhase) { _, phase in
            guard phase == .active else { return }
            // Back in the app: a Face ID sign-in that waited for it starts (§5.14), and the account
            // is refreshed if it's more than a couple of minutes old.
            model.appBecameActive()
            Task { await model.refreshAccount(force: false) }
        }
    }

    /// The tab stacks, with the status bar's backdrop and My Picks' confirmations over them. The
    /// tab screens start `topClearance` lower (below the window's controls).
    private func tabs(topClearance: CGFloat) -> some View {
        ZStack {
            Theme.Colors.background.ignoresSafeArea()
            ForEach(AppTab.allCases) { tab in
                if visitedTabs.contains(tab) || tab == model.selectedTab {
                    let isSelected = tab == model.selectedTab
                    TabStack(tab: tab, topClearance: topClearance)
                        .opacity(isSelected ? 1 : 0)
                        .allowsHitTesting(isSelected)
                        .accessibilityHidden(!isSelected)
                }
            }
        }
        .overlay(alignment: .top) {
            // Scrolled content never runs under the clock: the status bar gets a navy backdrop
            // (the tab screens hide their navigation bars, so nothing else covers it).
            GeometryReader { proxy in
                // The reader sits inside the safe area; its insets say how tall the bar is.
                Theme.Colors.background
                    .frame(maxWidth: .infinity)
                    .frame(height: proxy.safeAreaInsets.top)
                    .offset(y: -proxy.safeAreaInsets.top)
            }
            .allowsHitTesting(false)
            .accessibilityHidden(true)
        }
        // My Picks' confirmations, above the tab bar on any screen (§5.11).
        .overlay(alignment: .bottom) { PicksNoticeOverlay() }
    }

    private var rootSheet: Binding<RootSheet?> {
        Binding(
            get: { model.rootSheet },
            set: { newValue in
                if newValue == nil, let shown = model.rootSheet { model.dismiss(shown) }
            }
        )
    }

    /// First launch: the picker can't be dismissed until a library is chosen.
    private var onboarding: Binding<Bool> {
        Binding(get: { model.needsOnboarding }, set: { _ in })
    }
}

/// One tab's `NavigationStack`, handling the shared routes.
private struct TabStack: View {
    let tab: AppTab
    /// The window's controls' height, when they sit over this stack's top leading corner.
    let topClearance: CGFloat
    @Environment(AppModel.self) private var model

    var body: some View {
        NavigationStack(path: path) {
            root
                .tabBarClearance()
                // The tab screens hide the navigation bar, so they start below the window's
                // controls themselves (the screens pushed on them have bars, which already do),
                // and what scrolls up passes under navy there.
                .safeAreaPadding(.top, topClearance)
                .overlay(alignment: .top) {
                    Theme.Colors.background
                        .frame(height: topClearance)
                        .allowsHitTesting(false)
                        .accessibilityHidden(true)
                }
                .navigationDestination(for: Route.self) { route in
                    Group {
                        switch route {
                        case .detail(let ref):
                            BookDetailView(ref: ref)
                        case let .results(query, sort, title):
                            SearchResultsView(query: query, sort: sort, title: title)
                        case .collection(let link):
                            CollectionView(link: link)
                        case let .resultsPage(url, title):
                            ResultsPageView(url: url, title: title)
                        }
                    }
                    .tabBarClearance()
                }
        }
    }

    @ViewBuilder
    private var root: some View {
        switch tab {
        case .home: HomeView()
        case .borrowed: BorrowedView()
        case .search: SearchView()
        case .favourites: FavouritesView()
        case .profile: ProfileView()
        }
    }

    private var path: Binding<[Route]> {
        Binding(get: { model.path(for: tab) }, set: { model.setPath($0, for: tab) })
    }
}

