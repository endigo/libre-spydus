//! Design tokens (docs/desktop-app.md §7.1), transcribed by hand from `design/tokens.json` (the
//! source of truth; the test below checks every value still agrees, like iOS
//! `ThemeTokensTests`). Navy screens, a white sidebar (the phones' tab bar), white dialogs and the
//! white sheet on the book page.

use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::{App, Hsla, Pixels, Rgba, px, rgb, rgba};

/// Colours as `0xRRGGBB`.
pub mod colors {
    pub const BACKGROUND: u32 = 0x2B2D42;
    pub const SURFACE: u32 = 0x35374E;
    pub const SURFACE_ELEVATED: u32 = 0x3E405A;
    pub const FIELD: u32 = 0x3A3C55;
    pub const DIVIDER: u32 = 0x44465F;
    pub const TEXT_PRIMARY: u32 = 0xFFFFFF;
    pub const TEXT_SECONDARY: u32 = 0xA4A7C1;
    pub const TEXT_MUTED: u32 = 0x7D809B;
    pub const PRIMARY: u32 = 0xF2665E;
    pub const PRIMARY_GRADIENT_START: u32 = 0xF7806F;
    pub const PRIMARY_GRADIENT_END: u32 = 0xEF5A5C;
    pub const ON_PRIMARY: u32 = 0xFFFFFF;
    pub const ACCENT: u32 = 0x4B5BD6;
    pub const ON_ACCENT: u32 = 0xFFFFFF;
    pub const SHEET: u32 = 0xFFFFFF;
    pub const ON_SHEET: u32 = 0x24263A;
    pub const ON_SHEET_SECONDARY: u32 = 0x5F627A;
    pub const CHIP_ON_SHEET: u32 = 0xEEF0F7;
    pub const TAB_BAR: u32 = 0xFFFFFF;
    pub const TAB_ACTIVE: u32 = 0x2B2D42;
    pub const TAB_INACTIVE: u32 = 0xB4B6C6;
    pub const SUCCESS: u32 = 0x3CC48A;
    pub const WARNING: u32 = 0xF5A524;
    pub const DANGER: u32 = 0xF2665E;
    pub const DIGITAL: u32 = 0x2EC4B6;
    pub const GENRE: [u32; 8] = [0xF26B5B, 0x9B59D0, 0x4F6BE3, 0xF29F4B, 0x2EC4B6, 0xE85D9B, 0x5DB85C, 0x7A6FF0];

    /// Every named colour with its `tokens.json` key (the test walks this).
    pub const NAMED: [(&str, u32); 25] = [
        ("background", BACKGROUND),
        ("surface", SURFACE),
        ("surfaceElevated", SURFACE_ELEVATED),
        ("field", FIELD),
        ("divider", DIVIDER),
        ("textPrimary", TEXT_PRIMARY),
        ("textSecondary", TEXT_SECONDARY),
        ("textMuted", TEXT_MUTED),
        ("primary", PRIMARY),
        ("primaryGradientStart", PRIMARY_GRADIENT_START),
        ("primaryGradientEnd", PRIMARY_GRADIENT_END),
        ("onPrimary", ON_PRIMARY),
        ("accent", ACCENT),
        ("onAccent", ON_ACCENT),
        ("sheet", SHEET),
        ("onSheet", ON_SHEET),
        ("onSheetSecondary", ON_SHEET_SECONDARY),
        ("chipOnSheet", CHIP_ON_SHEET),
        ("tabBar", TAB_BAR),
        ("tabActive", TAB_ACTIVE),
        ("tabInactive", TAB_INACTIVE),
        ("success", SUCCESS),
        ("warning", WARNING),
        ("danger", DANGER),
        ("digital", DIGITAL),
    ];
}

pub mod radius {
    pub const CARD: f32 = 16.;
    pub const COVER: f32 = 10.;
    pub const FIELD: f32 = 14.;
    pub const TILE: f32 = 18.;
    pub const SHEET: f32 = 28.;
    pub const PILL: f32 = 999.;
}

pub mod spacing {
    pub const XS: f32 = 4.;
    pub const S: f32 = 8.;
    pub const M: f32 = 12.;
    pub const L: f32 = 16.;
    pub const XL: f32 = 24.;
    pub const XXL: f32 = 32.;
}

pub mod cover {
    /// Width / height.
    pub const ASPECT: f32 = 0.68;
    pub const SHELF_WIDTH: f32 = 120.;
    pub const SMALL_WIDTH: f32 = 72.;
    pub const DETAIL_WIDTH: f32 = 140.;

    pub fn height(width: f32) -> f32 {
        (width / ASPECT).round()
    }
}

/// The phones' tab bar height (the sidebar's row height follows its type, not this).
pub const TAB_BAR_HEIGHT: f32 = 64.;

/// A content column is centred and at most this wide (docs/desktop-app.md §6).
pub const CONTENT_MAX_WIDTH: f32 = 1040.;

/// Type tokens: size and weight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Font {
    Greeting,
    Title,
    Section,
    Body,
    BodyStrong,
    Caption,
    Tab,
}

impl Font {
    pub const ALL: [Font; 7] =
        [Self::Greeting, Self::Title, Self::Section, Self::Body, Self::BodyStrong, Self::Caption, Self::Tab];

    pub fn key(self) -> &'static str {
        match self {
            Self::Greeting => "greeting",
            Self::Title => "title",
            Self::Section => "section",
            Self::Body => "body",
            Self::BodyStrong => "bodyStrong",
            Self::Caption => "caption",
            Self::Tab => "tab",
        }
    }

    pub fn size_value(self) -> f32 {
        match self {
            Self::Greeting => 28.,
            Self::Title => 22.,
            Self::Section => 18.,
            Self::Body | Self::BodyStrong => 15.,
            Self::Caption => 12.,
            Self::Tab => 11.,
        }
    }

    pub fn size(self) -> Pixels {
        px(self.size_value())
    }

    /// "regular", "medium" or "semibold", as tokens.json writes it.
    pub fn weight_name(self) -> &'static str {
        match self {
            Self::Greeting | Self::Title | Self::Section | Self::BodyStrong => "semibold",
            Self::Body | Self::Caption => "regular",
            Self::Tab => "medium",
        }
    }

    pub fn weight(self) -> gpui_kit::FontWeight {
        match self.weight_name() {
            "semibold" => gpui_kit::FontWeight::SEMIBOLD,
            "medium" => gpui_kit::FontWeight::MEDIUM,
            _ => gpui_kit::FontWeight::NORMAL,
        }
    }
}

/// Shadows: colour, opacity, blur radius, y offset.
pub mod shadow {
    pub const CARD: (u32, f32, f32, f32) = (0x000000, 0.25, 12., 6.);
    pub const TAB_CENTER: (u32, f32, f32, f32) = (0x2B2D42, 0.18, 10., 4.);
}

/// A token colour.
pub fn color(hex: u32) -> Hsla {
    rgb(hex).into()
}

/// A token colour at `opacity`.
pub fn color_alpha(hex: u32, opacity: f32) -> Hsla {
    let alpha = (opacity.clamp(0., 1.) * 255.).round() as u32;
    rgba((hex << 8) | alpha).into()
}

pub fn rgba_of(hex: u32) -> Rgba {
    rgb(hex)
}

/// A genre colour picked stably from `key` (FNV-1a, as iOS `Theme.Colors.genre(for:)`).
pub fn genre_color(key: &str) -> Hsla {
    color(colors::GENRE[(fnv1a(key) % colors::GENRE.len() as u64) as usize])
}

pub fn fnv1a(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
    }
    hash
}

/// Applies the tokens to gpui-kit's theme (dark mode, then our colours).
pub fn apply(cx: &mut App) {
    Theme::change(ThemeMode::Dark, None, cx);
    Theme::update(cx, |theme| {
        let c = &mut theme.colors;
        c.background = color(colors::BACKGROUND);
        c.foreground = color(colors::TEXT_PRIMARY);
        c.muted = color(colors::SURFACE);
        c.muted_foreground = color(colors::TEXT_SECONDARY);
        c.primary = color(colors::PRIMARY);
        c.primary_hover = color(colors::PRIMARY_GRADIENT_START);
        c.primary_active = color(colors::PRIMARY_GRADIENT_END);
        c.primary_foreground = color(colors::ON_PRIMARY);
        c.secondary = color(colors::SURFACE_ELEVATED);
        c.secondary_hover = color(colors::SURFACE);
        c.secondary_active = color(colors::SURFACE);
        c.secondary_foreground = color(colors::TEXT_PRIMARY);
        c.accent = color(colors::SURFACE_ELEVATED);
        c.accent_foreground = color(colors::TEXT_PRIMARY);
        c.border = color(colors::DIVIDER);
        c.input = color(colors::FIELD);
        c.ring = color(colors::ACCENT);
        c.link = color(colors::TEXT_PRIMARY);
        c.danger = color(colors::DANGER);
        c.success = color(colors::SUCCESS);
        c.warning = color(colors::WARNING);
        c.skeleton = color(colors::SURFACE_ELEVATED);
        // The phones' tab bar: white, navy when active, grey otherwise.
        c.sidebar = color(colors::TAB_BAR);
        c.sidebar_foreground = color(colors::TAB_ACTIVE);
        c.sidebar_border = color_alpha(colors::TAB_INACTIVE, 0.4);
        c.sidebar_accent = color_alpha(colors::TAB_ACTIVE, 0.08);
        c.sidebar_accent_foreground = color(colors::TAB_ACTIVE);
        c.sidebar_primary = color(colors::TAB_ACTIVE);
        c.sidebar_primary_foreground = color(colors::TAB_BAR);
        // Dialogs are white (ui::sheet_dialog), like the phones' system dialogs. Menus and
        // dropdowns are dark popovers: gpui-kit draws their items in `foreground`.
        c.popover = color(colors::SURFACE);
        c.popover_foreground = color(colors::TEXT_PRIMARY);
        // Switches as on iOS: a white thumb on a grey track when off (a switch's own colour when on),
        // visible on the dark cards and on white dialogs alike.
        c.switch = color_alpha(colors::TEXT_MUTED, 0.6);
        c.switch_thumb = color(colors::TEXT_PRIMARY);
        c.overlay = color_alpha(0x000000, 0.45);
        theme.radius = px(radius::FIELD);
        theme.radius_lg = px(radius::CARD);
        theme.font_size = Font::Body.size();
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn tokens() -> Value {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../design/tokens.json");
        serde_json::from_str(&std::fs::read_to_string(path).expect("design/tokens.json")).unwrap()
    }

    fn hex(value: &Value) -> u32 {
        u32::from_str_radix(value.as_str().unwrap().trim_start_matches('#'), 16).unwrap()
    }

    fn number(value: &Value) -> f32 {
        value.as_f64().unwrap() as f32
    }

    #[test]
    fn every_colour_matches_tokens_json() {
        let tokens = tokens();
        let json = tokens["color"].as_object().unwrap();
        for (key, value) in colors::NAMED {
            assert_eq!(hex(&json[key]), value, "color.{key}");
        }
        let genre: Vec<u32> = json["genre"].as_array().unwrap().iter().map(hex).collect();
        assert_eq!(genre, colors::GENRE);
        // Nothing in tokens.json is left out.
        assert_eq!(json.len(), colors::NAMED.len() + 1);
    }

    #[test]
    fn radii_spacing_type_and_covers_match() {
        let tokens = tokens();
        let r = &tokens["radius"];
        for (key, value) in [
            ("card", radius::CARD),
            ("cover", radius::COVER),
            ("field", radius::FIELD),
            ("tile", radius::TILE),
            ("sheet", radius::SHEET),
            ("pill", radius::PILL),
        ] {
            assert_eq!(number(&r[key]), value, "radius.{key}");
        }
        assert_eq!(r.as_object().unwrap().len(), 6);
        let s = &tokens["spacing"];
        for (key, value) in [
            ("xs", spacing::XS),
            ("s", spacing::S),
            ("m", spacing::M),
            ("l", spacing::L),
            ("xl", spacing::XL),
            ("xxl", spacing::XXL),
        ] {
            assert_eq!(number(&s[key]), value, "spacing.{key}");
        }
        assert_eq!(s.as_object().unwrap().len(), 6);
        let t = &tokens["type"];
        for font in Font::ALL {
            assert_eq!(number(&t[font.key()]["size"]), font.size_value(), "type.{}.size", font.key());
            assert_eq!(t[font.key()]["weight"].as_str().unwrap(), font.weight_name(), "type.{}.weight", font.key());
        }
        assert_eq!(t.as_object().unwrap().len(), Font::ALL.len());
        let c = &tokens["cover"];
        assert_eq!(number(&c["aspect"]), cover::ASPECT);
        assert_eq!(number(&c["shelfWidth"]), cover::SHELF_WIDTH);
        assert_eq!(number(&c["smallWidth"]), cover::SMALL_WIDTH);
        assert_eq!(number(&c["detailWidth"]), cover::DETAIL_WIDTH);
        assert_eq!(number(&tokens["tabBar"]["height"]), TAB_BAR_HEIGHT);
        for (key, (colour, opacity, blur, y)) in [("card", shadow::CARD), ("tabCenter", shadow::TAB_CENTER)] {
            let json = &tokens["shadow"][key];
            assert_eq!(hex(&json["color"]), colour, "shadow.{key}.color");
            assert_eq!(number(&json["opacity"]), opacity, "shadow.{key}.opacity");
            assert_eq!(number(&json["radius"]), blur, "shadow.{key}.radius");
            assert_eq!(number(&json["y"]), y, "shadow.{key}.y");
        }
    }

    #[test]
    fn genre_colours_are_stable() {
        assert_eq!(fnv1a(""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(genre_color("Fiction"), genre_color("Fiction"));
    }
}
