//! Home (D3; docs/desktop-app.md §6): greeting, search field, New arrivals, Popular genres,
//! Featured, Your loans. Placeholder until D3 builds it.

use gpui_kit::{Context, IntoElement, Render, Window};

use crate::copy::Copy;
use crate::ui::{Lucide, PlaceholderScreen};

pub struct HomeScreen {}

impl HomeScreen {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {}
    }
}

impl Render for HomeScreen {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        PlaceholderScreen::new("home-screen", Copy::Tabs::home, Lucide::House)
    }
}
