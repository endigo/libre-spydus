//! The book page (D4; docs/desktop-app.md §6): the navy panel and the white sheet. Placeholder
//! until D4 builds it.

use gpui_kit::{Context, IntoElement, Render, Window};

use crate::copy::Copy;
use crate::state::BookRef;
use crate::ui::{Lucide, PlaceholderScreen};

pub struct DetailScreen {
    book: BookRef,
}

impl DetailScreen {
    pub fn new(book: BookRef, _window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self { book }
    }

    pub fn book(&self) -> &BookRef {
        &self.book
    }
}

impl Render for DetailScreen {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let title = if self.book.title.is_empty() { Copy::Detail::details.to_owned() } else { self.book.title.clone() };
        let screen = PlaceholderScreen::new("detail-screen", title, Lucide::BookOpenText);
        match &self.book.url {
            Some(url) => screen.detail(url.clone()),
            None => screen,
        }
    }
}
