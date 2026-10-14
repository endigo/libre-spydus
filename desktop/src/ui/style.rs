//! Small styling helpers on top of the tokens.

use gpui_kit::{BoxShadow, Div, Hsla, ParentElement, Styled, div, point, px};

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

/// A small caps-style label over a gallery group (debug screens).
pub fn caption_label(text: impl Into<gpui_kit::SharedString>) -> Div {
    div().typography(Font::Caption).text_color(theme::color(colors::TEXT_MUTED)).child(text.into())
}
