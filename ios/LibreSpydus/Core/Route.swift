import Foundation
import LibreSpydusCore

/// The five tabs (docs/native-apps.md §5.5), all ordinary tab-bar items.
enum AppTab: String, CaseIterable, Hashable, Identifiable, Sendable {
    case home
    case borrowed
    case search
    case favourites
    case profile

    var id: String { rawValue }

    var title: String {
        switch self {
        case .home: Copy.Tabs.home
        case .borrowed: Copy.Tabs.yourBooks
        case .search: Copy.Tabs.search
        case .favourites: Copy.Tabs.myPicks
        case .profile: Copy.Tabs.profile
        }
    }

    var systemImage: String {
        switch self {
        case .home: "house"
        case .borrowed: "books.vertical"
        case .search: "magnifyingglass"
        case .favourites: "bookmark"
        case .profile: "person.crop.circle"
        }
    }

    var selectedSystemImage: String {
        switch self {
        case .home: "house.fill"
        case .borrowed: "books.vertical.fill"
        case .search: "magnifyingglass"
        case .favourites: "bookmark.fill"
        case .profile: "person.crop.circle.fill"
        }
    }
}

/// Pushed onto a tab's `NavigationStack`; every tab's stack handles both
/// (`NavigationLink(value: Route.detail(ref))`, or `AppModel.push(_:)`).
enum Route: Hashable, Sendable {
    /// Book detail (`BookDetailView(ref:)`).
    case detail(BookRef)
    /// A results list (`SearchResultsView(query:sort:title:)`), e.g. Home "View All" or a genre tile.
    case results(query: String, sort: SearchSortOption, title: String)
    /// A Featured collection (`CollectionView(link:)`, §5.10): one shelf per section.
    case collection(FeaturedLink)
    /// A results list that starts from a URL (`ResultsPageView(url:title:)`): a Featured saved
    /// search or a collection section's "View All". No sort toggle.
    case resultsPage(url: String, title: String)
}

/// Root sheets presented by `RootView`, derived from `AppModel` flags.
enum RootSheet: String, Identifiable, Sendable {
    case libraryPicker
    case login
    case verification

    var id: String { rawValue }
}
