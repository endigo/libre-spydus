//! Onboarding and account dialogs (D2): the full-window library picker, and the library picker
//! and sign-in dialogs the shell opens for `AppState::request_library_picker` /
//! `request_sign_in`. Placeholders until D2 builds them.

use gpui_kit::component::WindowExt;
use gpui_kit::{App, Context, IntoElement, ParentElement, Render, Styled, Window, div, px};

use crate::copy::Copy;
use crate::theme::{self, Font, colors};
use crate::ui::{FontExt, Lucide, PlaceholderScreen, dialog_title, sheet_dialog};

/// The whole window while no library is selected.
pub struct LibraryPickerScreen {}

impl LibraryPickerScreen {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {}
    }
}

impl Render for LibraryPickerScreen {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        PlaceholderScreen::new("library-picker-screen", Copy::Onboarding::title, Lucide::LibraryBig)
    }
}

/// The library picker after onboarding (Profile's Change library, the sidebar header).
pub fn open_library_picker(window: &mut Window, cx: &mut App) {
    open_placeholder_dialog(Copy::Onboarding::changeTitle, window, cx);
}

/// Sign in (the sidebar footer, `AppState::request_sign_in`, an expired session).
pub fn open_sign_in(window: &mut Window, cx: &mut App) {
    open_placeholder_dialog(Copy::Login::title, window, cx);
}

pub(crate) fn open_placeholder_dialog(title: &'static str, window: &mut Window, cx: &mut App) {
    window.open_dialog(cx, move |dialog, _, _| {
        sheet_dialog(dialog).w(px(420.)).title(dialog_title(title)).child(
            div().typography(Font::Caption).text_color(theme::color(colors::ON_SHEET_SECONDARY)).child("coming soon"), // copy-exempt: debug placeholder until D2's dialog replaces it
        )
    });
}
