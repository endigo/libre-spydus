//! Sign in (docs/native-apps.md §6 "Login"; iOS `LoginView.swift` + `LoginFormModel.swift`): the
//! library's logo and name, "Username / library card number" and a masked "PIN / password" with a
//! reveal toggle → `AppState::sign_in`. Enter moves on from the first field and signs in from the
//! second; errors (§5.7) show inline; success closes the dialog.

use gpui_kit::component::WindowExt;
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::{
    App, AppContext, Context, CursorStyle, Entity, Global, InteractiveElement, IntoElement, ParentElement, Render,
    Role, SharedString, StatefulInteractiveElement, Styled, Subscription, TestSupportExt, WeakEntity, Window, div,
    prelude::FluentBuilder, px,
};

use super::logo::LibraryLogo;
use super::sheet::{self, SheetButton};
use crate::copy::Copy;
use crate::state::{AppState, UserFacingError};
use crate::theme::{self, Font, colors, spacing};
use crate::ui::{FontExt, Lucide, icon, on_click, sheet_dialog};

pub const DIALOG_WIDTH: f32 = 440.;

pub struct SignInForm {
    card: Entity<InputState>,
    pin: Entity<InputState>,
    pin_masked: bool,
    signing_in: bool,
    /// Shown inline under the fields.
    error: Option<UserFacingError>,
    _subscriptions: Vec<Subscription>,
}

/// The open sign-in dialog's form, so a second request doesn't stack another dialog on it.
struct OpenSignIn(WeakEntity<SignInForm>);

impl Global for OpenSignIn {}

/// Opens the sign-in dialog, unless one is already open.
pub fn open(window: &mut Window, cx: &mut App) {
    if cx.try_global::<OpenSignIn>().and_then(|open| open.0.upgrade()).is_some() && window.has_active_dialog(cx) {
        return;
    }
    let form = cx.new(|cx| SignInForm::new(window, cx));
    cx.set_global(OpenSignIn(form.downgrade()));
    let shown = form.clone();
    window.open_dialog(cx, move |dialog, _, _| {
        sheet_dialog(dialog)
            .w(px(DIALOG_WIDTH))
            .close_button(false)
            // Enter belongs to the fields (it reaches the dialog too): never close on it.
            .on_ok(|_, _, _| false)
            .child(shown.clone())
    });
    form.update(cx, |form, cx| form.card.update(cx, |card, cx| card.focus(window, cx)));
    #[cfg(feature = "evidence")]
    evidence_fill(&form, window, cx);
}

impl SignInForm {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let card = cx.new(|cx| InputState::new(window, cx).placeholder(Copy::Login::cardPlaceholder));
        let pin = cx.new(|cx| InputState::new(window, cx).placeholder(Copy::Login::pinPlaceholder).masked(true));
        let subscriptions = vec![
            cx.subscribe_in(&card, window, |this, _, event: &InputEvent, window, cx| match event {
                InputEvent::Change => this.clear_error(cx),
                InputEvent::PressEnter { .. } => {
                    if this.pin.read(cx).value().is_empty() {
                        this.pin.update(cx, |pin, cx| pin.focus(window, cx));
                    } else {
                        this.submit(window, cx);
                    }
                }
                _ => {}
            }),
            cx.subscribe_in(&pin, window, |this, _, event: &InputEvent, window, cx| match event {
                InputEvent::Change => this.clear_error(cx),
                InputEvent::PressEnter { .. } => this.submit(window, cx),
                _ => {}
            }),
            cx.observe(&AppState::global(cx), |_, _, cx| cx.notify()),
        ];
        Self { card, pin, pin_masked: true, signing_in: false, error: None, _subscriptions: subscriptions }
    }

    pub fn card(&self) -> &Entity<InputState> {
        &self.card
    }

    pub fn pin(&self) -> &Entity<InputState> {
        &self.pin
    }

    pub fn error(&self) -> Option<&UserFacingError> {
        self.error.as_ref()
    }

    pub fn is_signing_in(&self) -> bool {
        self.signing_in
    }

    fn trimmed_card(&self, cx: &App) -> String {
        self.card.read(cx).value().trim().to_owned()
    }

    pub fn can_submit(&self, cx: &App) -> bool {
        !self.trimmed_card(cx).is_empty() && !self.pin.read(cx).value().is_empty() && !self.signing_in
    }

    /// A field changed: the error goes, and the Sign in button follows the fields.
    fn clear_error(&mut self, cx: &mut Context<Self>) {
        self.error = None;
        cx.notify();
    }

    fn set_fields_disabled(&self, disabled: bool, cx: &mut Context<Self>) {
        for field in [&self.card, &self.pin] {
            field.update(cx, |field, cx| field.set_disabled(disabled, cx));
        }
    }

    /// Signs in; the dialog closes on success.
    pub fn submit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.can_submit(cx) {
            return;
        }
        let username = self.trimmed_card(cx);
        let password = self.pin.read(cx).value().to_string();
        self.signing_in = true;
        self.error = None;
        self.set_fields_disabled(true, cx);
        cx.notify();
        let task = AppState::global(cx).update(cx, |state, cx| state.sign_in(username, password, cx));
        cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.signing_in = false;
                this.set_fields_disabled(false, cx);
                match result {
                    Ok(()) => {
                        this.pin.update(cx, |pin, cx| pin.set_value("", window, cx));
                        window.close_dialog(cx);
                    }
                    Err(error) => {
                        this.error = Some(error);
                        this.pin.update(cx, |pin, cx| pin.focus(window, cx));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn toggle_pin_mask(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.pin_masked = !self.pin_masked;
        let masked = self.pin_masked;
        self.pin.update(cx, |pin, cx| pin.set_masked(masked, window, cx));
        cx.notify();
    }
}

impl Render for SignInForm {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = AppState::global(cx).read(cx);
        let library_name = state.library_name();
        let library = state.library().cloned();
        let demo = state.is_demo();
        let can_submit = self.can_submit(cx);
        let reveal_label = if self.pin_masked { Copy::Login::showPin } else { Copy::Login::hidePin };
        let entity = cx.entity();
        let reveal = div()
            .id("sign-in-reveal-pin")
            .test_support()
            .role(Role::Button)
            .aria_label(SharedString::from(reveal_label))
            .cursor(CursorStyle::PointingHand)
            .p(px(spacing::XS))
            .rounded(px(6.))
            .hover(|style| style.bg(theme::color_alpha(colors::ON_SHEET_SECONDARY, 0.12)))
            .on_click(cx.listener(|this, _, window, cx| this.toggle_pin_mask(window, cx)))
            .child(icon(
                if self.pin_masked { Lucide::Eye } else { Lucide::EyeOff },
                px(16.),
                theme::color(colors::ON_SHEET_SECONDARY),
            ));
        div()
            .id("sign-in-form")
            .test_support()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(spacing::L))
            .pt(px(spacing::S))
            .pb(px(spacing::S))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(spacing::M))
                    .when_some(library, |row, library| {
                        row.child(LibraryLogo::library("sign-in-logo", &library).size(52.))
                    })
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(
                                div()
                                    .id("sign-in-title")
                                    .test_support()
                                    .role(Role::Heading)
                                    .aria_label(SharedString::from(Copy::Login::title))
                                    .typography(Font::Title)
                                    .text_color(theme::color(colors::ON_SHEET))
                                    .child(Copy::Login::title),
                            )
                            .child(
                                div()
                                    .typography(Font::Body)
                                    .text_color(theme::color(colors::ON_SHEET_SECONDARY))
                                    .child(library_name.clone()),
                            ),
                    )
                    .child(div().self_start().child(sheet::close_button("sign-in-close"))),
            )
            .child(sheet::labelled_field(
                Copy::Login::cardLabel,
                sheet::field_box()
                    .id("sign-in-card")
                    .test_support()
                    .child(sheet::sheet_input(&self.card, Copy::Login::cardLabel)),
            ))
            .child(sheet::labelled_field(
                Copy::Login::pinLabel,
                sheet::field_box()
                    .id("sign-in-pin")
                    .test_support()
                    .child(sheet::sheet_input(&self.pin, Copy::Login::pinLabel).suffix(reveal)),
            ))
            .when_some(self.error.as_ref(), |column, error| {
                column.child(sheet::error_note("sign-in-error", error.message.clone()))
            })
            .child(
                SheetButton::new(
                    "sign-in-submit",
                    if self.signing_in { Copy::Account::signingIn } else { Copy::Account::signIn },
                )
                .loading(self.signing_in)
                .disabled(!can_submit && !self.signing_in)
                .on_click(on_click(move |window, cx| entity.update(cx, |form, cx| form.submit(window, cx)))),
            )
            .child(
                div()
                    .typography(Font::Caption)
                    .text_color(theme::color(colors::ON_SHEET_SECONDARY))
                    .child(Copy::Login::privacyNote(&library_name)),
            )
            .when(cfg!(debug_assertions) && demo, |column| {
                // Debug builds only (exempt from copy): how to sign in to the demo account.
                column.child(div().id("sign-in-demo-hint").test_support().child(sheet::info_note("Demo: demo / demo"))) // copy-exempt: debug-only demo hint
            })
    }
}

/// Screenshots (`onboarding::evidence_step`, **demo mode only**, never a real library):
/// `sign-in-error` submits a wrong PIN after 0.5 s; `sign-in-busy` submits demo / demo after 3 s.
#[cfg(feature = "evidence")]
fn evidence_fill(form: &Entity<SignInForm>, window: &mut Window, cx: &mut App) {
    if !AppState::global(cx).read(cx).is_demo() {
        return;
    }
    // copy-exempt: the demo account's sign-in details (screenshots)
    let (card, pin, delay) = match super::evidence_step("onboarding").as_deref() {
        Some("sign-in-error") => ("demo", "wrong", 0.5),
        Some("sign-in-busy") => ("demo", "demo", 3.0),
        _ => return,
    };
    // Demo mode starts signed in: show the dialog over a signed-out app, as a reader sees it.
    AppState::global(cx).update(cx, |state, cx| state.sign_out(cx));
    form.update(cx, |form, cx| {
        form.card.update(cx, |field, cx| field.set_value(card, window, cx));
        form.pin.update(cx, |field, cx| field.set_value(pin, window, cx));
        cx.notify();
    });
    let form = form.downgrade();
    window
        .spawn(cx, async move |cx| {
            cx.background_executor().timer(std::time::Duration::from_secs_f64(delay)).await;
            let _ = cx.update(|window, cx| {
                if let Some(form) = form.upgrade() {
                    form.update(cx, |form, cx| form.submit(window, cx));
                }
            });
        })
        .detach();
}
