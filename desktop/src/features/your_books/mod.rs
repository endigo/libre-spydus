//! Your Books (D5; docs/desktop-app.md §6): Loans | Reservations. Placeholder until D5 builds it.

use gpui_kit::{Context, IntoElement, Render, Window};

use crate::copy::Copy;
use crate::ui::{Lucide, PlaceholderScreen};

pub struct YourBooksScreen {}

impl YourBooksScreen {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {}
    }
}

impl Render for YourBooksScreen {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        PlaceholderScreen::new("your-books-screen", Copy::YourBooks::title, Lucide::LibraryBig)
    }
}
