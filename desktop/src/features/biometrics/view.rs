//! Touch ID's screens: the offer after a typed sign-in (iOS `biometricOfferAlert`), and Profile's
//! switch under the Account card.

use std::time::Duration;

use gpui_kit::component::WindowExt;
use gpui_kit::component::switch::Switch;
use gpui_kit::{
    AnyElement, App, Global, InteractiveElement, IntoElement, ParentElement, Role, SharedString,
    StatefulInteractiveElement, Styled, TestSupportExt, Window, div, px,
};

use super::Offer;
use crate::copy::Copy;
use crate::state::AppState;
use crate::theme::{self, Font, colors, radius, spacing};
use crate::ui::{DialogButton, FontExt, Lucide, dialog_title, icon, on_click, sheet_dialog};

/// Whether the offer's dialog is up, or a look is waiting for another dialog to close.
#[derive(Default)]
struct OfferPresenter {
    open: bool,
    waiting: bool,
}

impl Global for OfferPresenter {}

/// Shows the offer whenever there is one, once no other dialog is open (the sign-in dialog
/// closing). Turned on from Profile, the check runs without the question.
pub fn watch(window: &mut Window, cx: &mut App) {
    let state = AppState::global(cx);
    window.observe(&state, cx, |_, window, cx| present_if_needed(window, cx)).detach();
}

fn present_if_needed(window: &mut Window, cx: &mut App) {
    let Some(offer) = AppState::global(cx).update(cx, |state, cx| state.biometric_offer(cx)) else { return };
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
    if !offer.asks {
        AppState::global(cx).update(cx, |state, cx| state.answer_biometric_offer(true, cx)).detach();
        return;
    }
    cx.global_mut::<OfferPresenter>().open = true;
    open(offer, window, cx);
}

fn answer(accept: bool, window: &mut Window, cx: &mut App) {
    window.close_dialog(cx);
    cx.global_mut::<OfferPresenter>().open = false;
    AppState::global(cx).update(cx, |state, cx| state.answer_biometric_offer(accept, cx)).detach();
}

/// "Sign in with Touch ID next time?" — Not now / **Use Touch ID**.
fn open(offer: Offer, window: &mut Window, cx: &mut App) {
    let library = AppState::global(cx).read(cx).library_name();
    let method = offer.method;
    let message = Copy::Biometrics::offerMessage(&library, method);
    window.open_dialog(cx, move |dialog, _, _| {
        sheet_dialog(dialog)
            .w(px(440.))
            .close_button(false)
            .keyboard(false)
            .overlay_closable(false)
            .title(dialog_title(Copy::Biometrics::offerTitle(method)))
            .child(
                div()
                    .id("touch-id-offer")
                    .test_support()
                    .aria_label(SharedString::from(message.clone()))
                    .typography(Font::Body)
                    .text_color(theme::color(colors::ON_SHEET_SECONDARY))
                    .child(message.clone()),
            )
            .footer(
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .justify_end()
                    .gap(px(spacing::S))
                    .child(DialogButton::new(
                        "touch-id-offer-not-now",
                        Copy::Common::notNow,
                        on_click(|window, cx| answer(false, window, cx)),
                    ))
                    .child(
                        DialogButton::new(
                            "touch-id-offer-use",
                            Copy::Biometrics::r#use(method),
                            on_click(|window, cx| answer(true, window, cx)),
                        )
                        .confirming(true),
                    ),
            )
    });
}

/// Profile, under the Account card while signed in and the check is available: "Sign in with
/// Touch ID", "No PIN to type when City of Canning signs you out.", and the switch.
pub fn profile_section(_window: &mut Window, cx: &mut App) -> Option<AnyElement> {
    let status = super::status(cx);
    let state = AppState::global(cx).read(cx);
    if !state.is_signed_in() {
        return None;
    }
    let method = status.method()?;
    let title: SharedString = Copy::Biometrics::signInWith(method).into();
    let subtitle = Copy::Biometrics::switchSubtitle(&state.library_name());
    Some(
        div()
            .id("profile-touch-id")
            .test_support()
            .w_full()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(spacing::L))
            .p(px(spacing::L))
            .rounded(px(radius::CARD))
            .bg(theme::color(colors::SURFACE))
            .child(div().w(px(24.)).flex().justify_center().child(icon(
                Lucide::ShieldCheck,
                px(18.),
                theme::color(colors::TEXT_PRIMARY),
            )))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(
                        div()
                            .id("profile-touch-id-title")
                            .test_support()
                            .role(Role::Label)
                            .aria_label(title.clone())
                            .typography(Font::BodyStrong)
                            .text_color(theme::color(colors::TEXT_PRIMARY))
                            .child(title.clone()),
                    )
                    .child(
                        div()
                            .typography(Font::Caption)
                            .text_color(theme::color(colors::TEXT_SECONDARY))
                            .child(subtitle),
                    ),
            )
            .child(
                Switch::new("profile-touch-id-switch")
                    .checked(status.is_switched_on())
                    .color(theme::color(colors::ACCENT))
                    .accessibility_label(title)
                    .on_click(|checked: &bool, _, cx| {
                        let on = *checked;
                        AppState::global(cx).update(cx, |state, cx| state.set_biometric_sign_in(on, cx));
                    }),
            )
            .into_any_element(),
    )
}
