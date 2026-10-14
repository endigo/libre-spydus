//! My Picks (D5; docs/desktop-app.md §6): a cover grid, on the account when signed in, on this
//! computer when not. Placeholder until D5 builds it.

use gpui_kit::{Context, IntoElement, Render, Window};

use crate::copy::Copy;
use crate::ui::{Lucide, PlaceholderScreen};

pub struct MyPicksScreen {}

impl MyPicksScreen {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {}
    }
}

impl Render for MyPicksScreen {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        PlaceholderScreen::new("my-picks-screen", Copy::Picks::title, Lucide::BookMarked)
    }
}
