//! A library's directory logo on a white rounded tile, or a monogram on a genre colour while it
//! loads and when there's none (most directory entries have no logo). iOS `LibraryLogo.swift`,
//! without its light-logo-on-navy check.

use gpui_kit::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, ObjectFit, ParentElement, RenderOnce, SharedString,
    Styled, StyledImage, Window, div, img, linear_color_stop, linear_gradient, px,
};
use librespydus_core::LibraryInfo;

use super::directory::monogram;
use crate::theme;

#[derive(IntoElement)]
pub struct LibraryLogo {
    id: ElementId,
    name: String,
    logo_url: Option<String>,
    size: f32,
}

impl LibraryLogo {
    pub fn new(id: impl Into<ElementId>, name: impl Into<String>, logo_url: Option<String>) -> Self {
        Self { id: id.into(), name: name.into(), logo_url, size: 44. }
    }

    pub fn library(id: impl Into<ElementId>, library: &LibraryInfo) -> Self {
        Self::new(id, library.name.clone(), library.logo_url.clone())
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
}

fn monogram_tile(name: &str, size: f32) -> AnyElement {
    let tint = theme::genre_color(name);
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(linear_gradient(135., linear_color_stop(tint.opacity(0.8), 0.), linear_color_stop(tint, 1.)))
        .text_size(px(size * 0.38))
        .font_weight(gpui_kit::FontWeight::BOLD)
        .text_color(theme::color(0xFFFFFF))
        .child(monogram(name))
        .into_any_element()
}

impl RenderOnce for LibraryLogo {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let size = self.size;
        let frame = div()
            .flex_none()
            .size(px(size))
            .rounded(px(size * 0.26))
            .overflow_hidden()
            .border_1()
            .border_color(theme::color_alpha(0xFFFFFF, 0.08));
        match self.logo_url.filter(|url| url.starts_with("http")) {
            Some(url) => {
                let (loading, fallback) = (self.name.clone(), self.name);
                frame.bg(theme::color(0xFFFFFF)).child(
                    img(SharedString::from(url))
                        .id(self.id)
                        .size_full()
                        .p(px(size * 0.1))
                        .object_fit(ObjectFit::Contain)
                        .with_loading(move || monogram_tile(&loading, size))
                        .with_fallback(move || monogram_tile(&fallback, size)),
                )
            }
            None => frame.child(monogram_tile(&self.name, size)),
        }
    }
}
