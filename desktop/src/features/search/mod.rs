//! Search and results lists (D3; docs/desktop-app.md §6). Placeholders until D3 builds them.

use gpui_kit::{Context, IntoElement, Render, Window};

use crate::copy::Copy;
use crate::state::ResultsSource;
use crate::ui::{Lucide, PlaceholderScreen};

/// The Search section's root: the field, sort, recent searches and results.
pub struct SearchScreen {}

impl SearchScreen {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {}
    }
}

impl Render for SearchScreen {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        PlaceholderScreen::new("search-screen", Copy::Search::title, Lucide::Search)
    }
}

/// A results list from a query (`Route::Results`) or a URL (`Route::ResultsPage`).
pub struct ResultsScreen {
    source: ResultsSource,
}

impl ResultsScreen {
    pub fn new(source: ResultsSource, _window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self { source }
    }

    pub fn source(&self) -> &ResultsSource {
        &self.source
    }
}

impl Render for ResultsScreen {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        PlaceholderScreen::new("results-screen", self.source.title().to_owned(), Lucide::Search)
    }
}
