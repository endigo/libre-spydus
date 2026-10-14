//! Platform-specific bits (docs/desktop-app.md: AppKit-only code lives here, so the rest stays
//! cross-platform). GPUI already handles the menu bar, windows and key equivalents; what's left is
//! the macOS window chrome.

use gpui_kit::{Pixels, Point, TitlebarOptions, point, px};

use crate::copy::Copy;

/// The title bar: transparent on macOS, so the white sidebar runs to the window's top edge with
/// the traffic lights over it.
pub fn titlebar() -> TitlebarOptions {
    TitlebarOptions {
        title: Some(Copy::Profile::appName.into()),
        appears_transparent: cfg!(target_os = "macos"),
        traffic_light_position: traffic_lights(),
    }
}

fn traffic_lights() -> Option<Point<Pixels>> {
    if cfg!(target_os = "macos") { Some(point(px(16.), px(16.))) } else { None }
}
