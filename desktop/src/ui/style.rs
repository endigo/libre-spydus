//! Small styling helpers on top of the tokens.

use gpui_kit::{
    BoxShadow, Div, Hsla, InteractiveElement, IntoElement, ParentElement, Role, SharedString,
    StatefulInteractiveElement, Styled, TestSupportExt, Window, div, point, px,
};

use crate::theme::{self, Font, colors, spacing};

pub trait FontExt: Styled + Sized {
    /// A type token: its size and weight.
    fn typography(self, font: Font) -> Self {
        self.text_size(font.size()).font_weight(font.weight())
    }
}

impl<T: Styled> FontExt for T {}

/// A token shadow: (colour, opacity, blur radius, y).
pub fn shadow((hex, opacity, blur, y): (u32, f32, f32, f32)) -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: theme::color_alpha(hex, opacity),
        offset: point(px(0.), px(y)),
        blur_radius: px(blur),
        spread_radius: px(0.),
        inset: false,
    }]
}

/// A soft glow under a coloured control (pills, the bookmark button).
pub fn glow(color: Hsla) -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: color.opacity(0.35),
        offset: point(px(0.), px(4.)),
        blur_radius: px(8.),
        spread_radius: px(0.),
        inset: false,
    }]
}

/// A screen's centred content column, at most `CONTENT_MAX_WIDTH` wide (docs/desktop-app.md §6).
pub fn content_column() -> Div {
    div().w_full().max_w(px(theme::CONTENT_MAX_WIDTH)).mx_auto().px(px(theme::spacing::XL))
}

// Every page's frame (docs/desktop-app.md §6), so titles and content line up from page to page: a
// page starts `PAGE_TOP` below the toolbar, `PAGE_GAP` separates its header from what follows, and
// `PAGE_BOTTOM` ends it. A page whose header stays put while the rest scrolls splits the gap: half
// under the header (`page_header`), half over the scrolling part (`page_body`).

/// Between the toolbar and a page's first line.
pub const PAGE_TOP: f32 = spacing::S;
/// Between a page's header (its title row) and what follows; also between Home's sections.
pub const PAGE_GAP: f32 = spacing::XL;
/// Under a page's last line.
pub const PAGE_BOTTOM: f32 = spacing::XXL;

/// A page that scrolls as a whole: its header, then its sections `PAGE_GAP` apart.
pub fn page_column() -> Div {
    content_column().pt(px(PAGE_TOP)).pb(px(PAGE_BOTTOM)).flex().flex_col().gap(px(PAGE_GAP))
}

/// The header of a page whose header stays put.
pub fn page_header() -> Div {
    content_column().flex_none().pt(px(PAGE_TOP)).pb(px(PAGE_GAP / 2.))
}

/// The scrolling part under a `page_header`.
pub fn page_body() -> Div {
    content_column().pt(px(PAGE_GAP / 2.)).pb(px(PAGE_BOTTOM))
}

/// A section's title at the top of its page: Your Books, Search, My Picks, Profile (Home's is its
/// greeting).
pub fn page_title(id: &'static str, title: &'static str) -> impl IntoElement {
    div()
        .id(id)
        .test_support()
        .role(Role::Heading)
        .aria_label(SharedString::from(title))
        .typography(Font::Greeting)
        .text_color(theme::color(colors::TEXT_PRIMARY))
        .child(title)
}

/// The narrowest a book card (`BookRowCard`) gets in a card grid.
pub const CARD_MIN_WIDTH: f32 = 300.;
/// The most columns a card grid has.
pub const CARD_MAX_COLUMNS: u16 = 3;

/// How many card columns fit in the content column at this window's width (Your Books, Search
/// results).
pub fn card_columns(window: &Window) -> u16 {
    card_columns_for(f32::from(window.viewport_size().width))
}

/// `card_columns` for a window `viewport_width` wide: the content column beside the sidebar, at
/// most `CONTENT_MAX_WIDTH`, holds as many `CARD_MIN_WIDTH` columns as fit, 1 to `CARD_MAX_COLUMNS`.
pub fn card_columns_for(viewport_width: f32) -> u16 {
    let content =
        (viewport_width - crate::shell::sidebar::WIDTH).min(theme::CONTENT_MAX_WIDTH) - 2. * theme::spacing::XL;
    let gap = theme::spacing::M;
    let fits = ((content + gap) / (CARD_MIN_WIDTH + gap)).floor();
    (fits.max(1.) as u16).min(CARD_MAX_COLUMNS)
}

/// A grid of book cards, `columns` wide; each row is as tall as its tallest card.
pub fn card_grid(columns: u16) -> Div {
    div().w_full().grid().grid_cols(columns).gap(px(theme::spacing::M))
}

/// A small caps-style label over a gallery group (debug screens).
pub fn caption_label(text: impl Into<gpui_kit::SharedString>) -> Div {
    div().typography(Font::Caption).text_color(theme::color(colors::TEXT_MUTED)).child(text.into())
}

#[cfg(test)]
mod card_grid_tests {
    use super::card_columns_for;

    #[test]
    fn columns_follow_the_window_width() {
        assert_eq!(card_columns_for(1280.), 3, "the default window");
        assert_eq!(card_columns_for(2560.), 3, "never more than three");
        assert_eq!(card_columns_for(960.), 2, "the smallest window");
        assert_eq!(card_columns_for(500.), 1, "never fewer than one");
    }
}
