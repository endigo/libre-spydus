//! Profile (D2; docs/desktop-app.md §6): a settings-style page. Placeholder until D2 builds it.

use gpui_kit::{Context, IntoElement, Render, Window};

use crate::copy::Copy;
use crate::ui::{Lucide, PlaceholderScreen};

pub struct ProfileScreen {}

impl ProfileScreen {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {}
    }
}

impl Render for ProfileScreen {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        PlaceholderScreen::new("profile-screen", Copy::Profile::title, Lucide::CircleUserRound)
    }
}
