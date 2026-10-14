//! The copy-once offer after an interactive sign-in (docs/native-apps.md §5.11; iOS
//! `picksCopyOfferAlert`): "Add this computer’s picks to your library account?" — Add / Not now.
//! It waits until no other dialog is open (the sign-in dialog closing), shows once, and either
//! answer goes to `AppState::answer_picks_copy_offer`, which remembers it for the library.

use std::time::Duration;

use gpui_kit::component::WindowExt;
use gpui_kit::{App, Global, InteractiveElement, ParentElement, Styled, TestSupportExt, Window, div, px};

use crate::copy::Copy;
use crate::state::AppState;
use crate::theme::{self, Font, colors, spacing};
use crate::ui::{DialogButton, FontExt, dialog_title, on_click, sheet_dialog};

/// Whether the offer's dialog is up, or a check is waiting for another dialog to close.
#[derive(Default)]
struct OfferPresenter {
    open: bool,
    waiting: bool,
}

impl Global for OfferPresenter {}

/// Shows the offer whenever `AppState` has one, wherever the reader is (the shell calls this once
/// for its window).
pub fn watch_picks_copy_offer(window: &mut Window, cx: &mut App) {
    let state = AppState::global(cx);
    window.observe(&state, cx, |_, window, cx| present_if_needed(window, cx)).detach();
    present_if_needed(window, cx);
}

/// Opens the offer's dialog when one is pending and none is showing. Another dialog on screen (the
/// sign-in dialog closing): look again shortly.
pub fn present_if_needed(window: &mut Window, cx: &mut App) {
    let Some(count) = AppState::global(cx).read(cx).picks_copy_offer().map(|offer| offer.count) else { return };
    let presenter = cx.default_global::<OfferPresenter>();
    if presenter.open || presenter.waiting {
        return;
    }
    if window.has_active_dialog(cx) {
        cx.global_mut::<OfferPresenter>().waiting = true;
        window
            .spawn(cx, async move |cx| {
                cx.background_executor().timer(Duration::from_millis(400)).await;
                let _ = cx.update(|window, cx| {
                    cx.global_mut::<OfferPresenter>().waiting = false;
                    present_if_needed(window, cx);
                });
            })
            .detach();
        return;
    }
    cx.global_mut::<OfferPresenter>().open = true;
    open(count, window, cx);
}

fn answer(add: bool, window: &mut Window, cx: &mut App) {
    window.close_dialog(cx);
    cx.global_mut::<OfferPresenter>().open = false;
    AppState::global(cx).update(cx, |state, cx| state.answer_picks_copy_offer(add, cx)).detach();
}

fn open(count: usize, window: &mut Window, cx: &mut App) {
    window.open_dialog(cx, move |dialog, _, _| {
        sheet_dialog(dialog)
            .w(px(440.))
            .close_button(false)
            .keyboard(false)
            .overlay_closable(false)
            .title(dialog_title(Copy::Picks::offerTitle))
            .child(
                div()
                    .id("picks-offer")
                    .test_support()
                    .typography(Font::Body)
                    .text_color(theme::color(colors::ON_SHEET_SECONDARY))
                    .child(Copy::Picks::offerMessage(count as i64)),
            )
            .footer(
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .justify_end()
                    .gap(px(spacing::S))
                    .child(DialogButton::new(
                        "picks-offer-not-now",
                        Copy::Common::notNow,
                        on_click(|window, cx| answer(false, window, cx)),
                    ))
                    .child(
                        DialogButton::new(
                            "picks-offer-add",
                            Copy::Common::add,
                            on_click(|window, cx| answer(true, window, cx)),
                        )
                        .confirming(true),
                    ),
            )
    });
}
