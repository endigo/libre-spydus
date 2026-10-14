//! A Featured collection (D3; docs/native-apps.md §5.10): one shelf per section. Placeholder until
//! D3 builds it.

use gpui_kit::{Context, IntoElement, Render, Window};
use librespydus_core::FeaturedLink;

use crate::ui::{Lucide, PlaceholderScreen};

pub struct CollectionScreen {
    link: FeaturedLink,
}

impl CollectionScreen {
    pub fn new(link: FeaturedLink, _window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self { link }
    }
}

impl Render for CollectionScreen {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        PlaceholderScreen::new("collection-screen", self.link.title.clone(), Lucide::Sparkles)
    }
}
