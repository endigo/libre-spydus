//! My card (D2; docs/native-apps.md §5.9): the library card as a barcode, in a white dialog.
//! Placeholder until D2 builds it.

use gpui_kit::{App, Window};

use crate::copy::Copy;

pub fn open_my_card(window: &mut Window, cx: &mut App) {
    crate::features::onboarding::open_placeholder_dialog(Copy::MyCard::title, window, cx);
}
