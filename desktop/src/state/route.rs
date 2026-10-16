//! The sidebar's sections and the routes pushed on their stacks (docs/desktop-app.md §5.5; iOS
//! `Route.swift`).

use librespydus_core::{FeaturedLink, SearchSort};

use crate::copy::Copy;
use crate::state::book_ref::BookRef;

/// The five sections, in the sidebar's order (the phones' tabs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Section {
    #[default]
    Home,
    YourBooks,
    Search,
    MyPicks,
    Profile,
}

impl Section {
    pub const ALL: [Section; 5] = [Self::Home, Self::YourBooks, Self::Search, Self::MyPicks, Self::Profile];

    pub fn title(self) -> &'static str {
        match self {
            Self::Home => Copy::Tabs::home,
            Self::YourBooks => Copy::Tabs::yourBooks,
            Self::Search => Copy::Tabs::search,
            Self::MyPicks => Copy::Tabs::myPicks,
            Self::Profile => Copy::Tabs::profile,
        }
    }

    /// The element id of its sidebar item (`sidebar-home`, …), for tests and accessibility.
    pub fn id(self) -> &'static str {
        match self {
            Self::Home => "sidebar-home",
            Self::YourBooks => "sidebar-your-books",
            Self::Search => "sidebar-search",
            Self::MyPicks => "sidebar-my-picks",
            Self::Profile => "sidebar-profile",
        }
    }

    /// ⌘1–⌘5.
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|section| *section == self).unwrap_or_default()
    }
}

/// Pushed on a section's stack; every section's stack shows each of them.
#[derive(Debug, Clone, PartialEq)]
pub enum Route {
    /// Book detail (`DetailScreen`).
    Detail(BookRef),
    /// A results list (`ResultsScreen`), e.g. Home's View All or a genre tile.
    Results { query: String, sort: SearchSort, title: String },
    /// A Featured collection (`CollectionScreen`, §5.10).
    Collection(FeaturedLink),
    /// A results list that starts from a URL: a Featured saved search or a collection section's
    /// View All. No sort toggle.
    ResultsPage { url: String, title: String },
}

/// Where a results list comes from (`ResultsScreen::new`).
#[derive(Debug, Clone, PartialEq)]
pub enum ResultsSource {
    Query { query: String, sort: SearchSort, title: String },
    Page { url: String, title: String },
}

impl ResultsSource {
    pub fn title(&self) -> &str {
        match self {
            Self::Query { title, .. } | Self::Page { title, .. } => title,
        }
    }
}

/// Your Books' two lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum YourBooksSegment {
    #[default]
    Loans,
    Reservations,
}

impl YourBooksSegment {
    pub fn title(self) -> &'static str {
        match self {
            Self::Loans => Copy::YourBooks::loans,
            Self::Reservations => Copy::YourBooks::reservations,
        }
    }
}

/// The label of a search order (Relevance / Newest).
pub fn sort_label(sort: SearchSort) -> &'static str {
    match sort {
        SearchSort::Relevance => Copy::Search::relevance,
        SearchSort::Newest => Copy::Search::newest,
    }
}
