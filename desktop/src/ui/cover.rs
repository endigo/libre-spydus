//! A book cover (iOS `CoverImage.swift`): `img(url)` over a placeholder generated from the title
//! (title and author on a genre colour), so the placeholder shows while it loads, when it fails,
//! and through Syndetics' transparent 1×1 "no cover" image. Rounded corners and a soft shadow.

use gpui_kit::{
    App, ElementId, InteractiveElement, IntoElement, ObjectFit, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, StyledImage, Window, div, img, linear_color_stop, linear_gradient, px,
};

use crate::copy::Copy;
use crate::state::formatting::author_name;
use crate::theme::{self, colors, cover, radius};
use crate::ui::style::shadow;

#[derive(IntoElement)]
pub struct CoverImage {
    id: ElementId,
    url: Option<String>,
    title: String,
    author: Option<String>,
    width: f32,
    shadow: bool,
}

impl CoverImage {
    pub fn new(id: impl Into<ElementId>, url: Option<String>, title: impl Into<String>) -> Self {
        Self { id: id.into(), url, title: title.into(), author: None, width: cover::SHELF_WIDTH, shadow: true }
    }

    pub fn author(mut self, author: Option<String>) -> Self {
        self.author = author;
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn shadow(mut self, shadow: bool) -> Self {
        self.shadow = shadow;
        self
    }
}

impl RenderOnce for CoverImage {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let width = self.width;
        let height = cover::height(width);
        let label: SharedString = Copy::A11y::coverOf(&self.title).into();
        let mut frame = div()
            .id(self.id.clone())
            .aria_label(label)
            .relative()
            .flex_none()
            .w(px(width))
            .h(px(height))
            .rounded(px(radius::COVER))
            .overflow_hidden()
            .border_1()
            .border_color(theme::color_alpha(0xFFFFFF, 0.06))
            .bg(theme::color(colors::SURFACE_ELEVATED))
            .child(CoverPlaceholder { title: self.title.clone(), author: self.author.clone(), width });
        if self.shadow {
            frame = frame.shadow(shadow(theme::shadow::CARD));
        }
        if let Some(url) = self.url.filter(|url| url.starts_with("http")) {
            frame = frame.child(
                img(SharedString::from(url))
                    .id(SharedString::from(format!("{}-img", self.id)))
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .object_fit(ObjectFit::Cover),
            );
        }
        frame
    }
}

/// The generated cover: the title (and author) on a token colour picked from the title.
#[derive(IntoElement)]
pub struct CoverPlaceholder {
    pub title: String,
    pub author: Option<String>,
    pub width: f32,
}

impl RenderOnce for CoverPlaceholder {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let width = self.width;
        let base = theme::genre_color(&self.title);
        let title_size = (width * 0.13).max(9.);
        let author_size = (width * 0.075).max(6.);
        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .bg(linear_gradient(180., linear_color_stop(base.opacity(0.85), 0.), linear_color_stop(base, 1.)))
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .h_full()
                    .w(px((width * 0.06).max(3.)))
                    .bg(theme::color_alpha(0x000000, 0.14)),
            )
            .child(
                div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .justify_between()
                    .pt(px(width * 0.1))
                    .pb(px(width * 0.1))
                    .pl(px(width * 0.14))
                    .pr(px(width * 0.08))
                    .child(
                        div()
                            .text_size(px(title_size))
                            .font_weight(gpui_kit::FontWeight::BOLD)
                            // copy-exempt: a font family name
                            .font_family("Georgia")
                            .line_height(px(title_size * 1.15))
                            .text_color(theme::color(0xFFFFFF))
                            .line_clamp(5)
                            .text_ellipsis()
                            .child(self.title.clone()),
                    )
                    .children(author_name(self.author.as_deref()).map(|name| {
                        div()
                            .text_size(px(author_size))
                            .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                            .text_color(theme::color_alpha(0xFFFFFF, 0.85))
                            .line_clamp(2)
                            .text_ellipsis()
                            .child(name.to_uppercase())
                    })),
            )
    }
}
