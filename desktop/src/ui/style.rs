//! Small styling helpers on top of the tokens.

use gpui_kit::{BoxShadow, Div, Hsla, ParentElement, Styled, Window, div, point, px};

use crate::theme::{self, Font, colors};

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
