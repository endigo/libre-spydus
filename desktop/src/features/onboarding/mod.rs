//! Onboarding and the account dialogs (D2; docs/native-apps.md §6 "Library picker", "Login"): the
//! full-window library picker while no library is selected, and the Change library and Sign in
//! dialogs the shell opens for `AppState::request_library_picker` / `request_sign_in`.
//! Human verification isn't here: desktop v1 has no web view (docs/desktop-app.md §5.3).

pub mod directory;
pub mod logo;
pub mod picker;
pub mod sheet;
pub mod sign_in;
pub mod store;

use gpui_kit::component::WindowExt;
use gpui_kit::{
    App, AppContext, Context, Entity, InteractiveElement, IntoElement, ParentElement, Render, Role, SharedString,
    StatefulInteractiveElement, Styled, TestSupportExt, Window, div, px,
};

pub use self::logo::LibraryLogo;
pub use self::picker::{LibraryPicker, PickerMode};
pub use self::store::{DirectoryFetch, DirectoryStore, set_directory_fetch};

use crate::copy::Copy;
use crate::theme::{self, Font, colors, spacing};
use crate::ui::{FontExt, Lucide, dialog_title, icon, sheet_dialog};

/// The whole window while no library is selected: "Choose your library".
pub struct LibraryPickerScreen {
    picker: Entity<LibraryPicker>,
}

impl LibraryPickerScreen {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let picker = cx.new(|cx| LibraryPicker::new(PickerMode::Onboarding, window, cx));
        let query = picker.read(cx).query().clone();
        query.update(cx, |query, cx| query.focus(window, cx));
        Self { picker }
    }
}

impl Render for LibraryPickerScreen {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().id("library-picker-screen").test_support().size_full().flex().justify_center().child(
            div()
                .w_full()
                .max_w(px(760.))
                .h_full()
                .flex()
                .flex_col()
                .px(px(spacing::XL))
                .pt(px(spacing::XXL + spacing::XL))
                .pb(px(spacing::L))
                .gap(px(spacing::L))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(spacing::S))
                        .child(icon(Lucide::LibraryBig, px(32.), theme::color(colors::PRIMARY)))
                        .child(
                            div()
                                .id("onboarding-title")
                                .test_support()
                                .role(Role::Heading)
                                .aria_label(SharedString::from(Copy::Onboarding::title))
                                .typography(Font::Greeting)
                                .text_color(theme::color(colors::TEXT_PRIMARY))
                                .child(Copy::Onboarding::title),
                        )
                        .child(
                            div()
                                .typography(Font::Body)
                                .text_color(theme::color(colors::TEXT_SECONDARY))
                                .child(Copy::Onboarding::intro),
                        ),
                )
                .child(div().flex_1().min_h_0().w_full().child(self.picker.clone())),
        )
    }
}

/// The library picker after onboarding (Profile's Change library, the sidebar header).
pub fn open_library_picker(window: &mut Window, cx: &mut App) {
    let picker = cx.new(|cx| LibraryPicker::new(PickerMode::Dialog, window, cx));
    let shown = picker.clone();
    window.open_dialog(cx, move |dialog, window, _| {
        // As tall as fits, up to a comfortable list.
        let height = (window.viewport_size().height - px(240.)).clamp(px(280.), px(560.));
        sheet_dialog(dialog)
            .w(px(600.))
            .close_button(false)
            // Enter in the search field reaches the dialog too: never close on it.
            .on_ok(|_, _, _| false)
            .title(sheet::dialog_header(dialog_title(Copy::Onboarding::changeTitle), "change-library-close"))
            .child(div().id("change-library").test_support().w_full().h(height).child(shown.clone()))
    });
    let query = picker.read(cx).query().clone();
    query.update(cx, |query, cx| query.focus(window, cx));
}

/// `LIBRESPYDUS_EVIDENCE=<feature>:<step>` (cargo feature `evidence`, screenshots only): the step
/// asked for `feature`, if any. D2's steps:
/// - `onboarding:filtered` / `onboarding:filtered-uk` (the picker searched for "zealand" /
///   "kingdom"), `onboarding:directory-error`,
///   `onboarding:directory-empty`, `onboarding:directory-loading` (a directory that fails, comes
///   back empty, or never answers; nothing reaches the network);
/// - `onboarding:sign-in-error` (demo mode: a wrong PIN submitted after 0.5 s),
///   `onboarding:sign-in-busy` (demo mode: demo / demo submitted after 3 s);
/// - `my_card:barcode-failed` (the library's barcode type "didn't load");
/// - `profile:sign-out-confirm` (the sign-out confirmation, 2 s after Profile opens).
#[cfg(feature = "evidence")]
pub(crate) fn evidence_step(feature: &str) -> Option<String> {
    let value = std::env::var("LIBRESPYDUS_EVIDENCE").ok()?;
    let (prefix, step) = value.split_once(':')?;
    (prefix == feature).then(|| step.to_owned())
}

/// Sign in (the sidebar footer, `AppState::request_sign_in`, an expired session).
pub fn open_sign_in(window: &mut Window, cx: &mut App) {
    sign_in::open(window, cx);
}
