import SwiftUI

/// The app shell: five tab stacks under the custom tab bar, plus the root sheets
/// (library picker, login, verification) driven by `AppModel` flags.
struct RootView: View {
    @Environment(AppModel.self) private var model
    @Environment(\.scenePhase) private var scenePhase
    /// Tabs are built on first visit and then kept alive, so each keeps its stack and state.
    @State private var visitedTabs: Set<AppTab> = []

    var body: some View {
        @Bindable var model = model
        ZStack {
            Theme.Colors.background.ignoresSafeArea()
            ForEach(AppTab.allCases) { tab in
                if visitedTabs.contains(tab) || tab == model.selectedTab {
                    let isSelected = tab == model.selectedTab
                    TabStack(tab: tab)
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
        .safeAreaInset(edge: .bottom, spacing: 0) {
            LSTabBar(selection: model.selectedTab) { model.select($0) }
        }
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
        .picksCopyOfferAlert()
        .onChange(of: scenePhase) { _, phase in
            // Back in the app: refresh the account if it's more than a couple of minutes old.
            if phase == .active { Task { await model.refreshAccount(force: false) } }
        }
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
    @Environment(AppModel.self) private var model

    var body: some View {
        NavigationStack(path: path) {
            root
                .tabBarClearance()
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
