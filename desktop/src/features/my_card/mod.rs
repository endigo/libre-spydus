//! My card (D2; docs/native-apps.md §5.9; iOS `MyCardSheet.swift` + `MyCardModel.swift`): the
//! library card as a barcode to scan at the desk or self-checkout, on a white card. Asks for the
//! number when there's none; "Edit card number" opens the same form. Signed out, it asks to sign
//! in. The barcode is drawn by the app from the core's `card_barcode` modules, in the library's
//! own symbology (Code 128, Code 39 or Codabar).

pub mod barcode;

use std::time::Duration;

use futures::FutureExt;
use gpui_kit::component::WindowExt;
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::{
    AnyElement, App, AppContext, Context, Entity, InteractiveElement, IntoElement, ParentElement, Render, Role,
    SharedString, StatefulInteractiveElement, Styled, Subscription, Task, TestSupportExt, Window, div,
    prelude::FluentBuilder, px,
};
use librespydus_core::barcode::{BarcodeSymbology, CardBarcode, card_barcode};

use self::barcode::BarcodeBars;
use crate::copy::Copy;
use crate::features::onboarding::LibraryLogo;
use crate::features::onboarding::sheet::{self, SheetButton};
use crate::state::AppState;
use crate::theme::{self, Font, colors, radius, spacing};
use crate::ui::{FontExt, Lucide, dialog_title, icon, on_click, sheet_dialog, style::shadow};

const DIALOG_WIDTH: f32 = 480.;
/// The card's padding; the barcode runs edge to edge inside the card (its white is quiet zone).
const CARD_PADDING: f32 = 20.;
const BARCODE_WIDTH: f32 = DIALOG_WIDTH - 2. * 16.;
const BARCODE_HEIGHT: f32 = 100.;
/// The library's symbology is asked for once; past this it's "Couldn't load the barcode" + Retry.
const SYMBOLOGY_TIMEOUT: Duration = Duration::from_secs(20);

pub fn open_my_card(window: &mut Window, cx: &mut App) {
    let card = cx.new(|cx| MyCard::new(window, cx));
    let shown = card.clone();
    window.open_dialog(cx, move |dialog, _, _| {
        sheet_dialog(dialog)
            .w(px(DIALOG_WIDTH))
            .close_button(false)
            // Enter saves the number field (it reaches the dialog too): never close on it.
            .on_ok(|_, _, _| false)
            .title(sheet::dialog_header(dialog_title(Copy::MyCard::title), "my-card-close"))
            .child(shown.clone())
    });
    if card.read(cx).mode(cx) == Mode::Entry {
        let draft = card.read(cx).draft.clone();
        draft.update(cx, |draft, cx| draft.focus(window, cx));
    }
}

/// What the dialog shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Signed out: a Sign in prompt.
    SignIn,
    /// The account (and its card number) is still loading.
    Loading,
    Card,
    /// No number from either source, or "Edit card number".
    Entry,
}

/// What the card's barcode area shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Barcode {
    /// The library's symbology isn't known yet (first open per library).
    Loading,
    /// The symbology didn't come (Retry).
    Failed,
    Bars(CardBarcode),
    /// `card_barcode` can't encode the number: it's shown as text instead.
    Unencodable(String),
}

/// The barcode area for `number` (iOS `MyCardModel.barcode(for:symbology:)`).
pub fn barcode_for(number: &str, symbology: Option<BarcodeSymbology>, failed: bool) -> Barcode {
    let Some(symbology) = symbology else {
        // Code 128 is every symbology's fallback: a number it can't encode needn't wait.
        if card_barcode(number.to_owned(), BarcodeSymbology::Code128).is_none() {
            return Barcode::Unencodable(number.to_owned());
        }
        return if failed { Barcode::Failed } else { Barcode::Loading };
    };
    card_barcode(number.to_owned(), symbology).map_or_else(|| Barcode::Unencodable(number.to_owned()), Barcode::Bars)
}

pub struct MyCard {
    /// "Edit card number" was clicked: the form, whatever the card shows.
    editing: bool,
    draft: Entity<InputState>,
    /// The form edits a number already on the card, so it offers Cancel back to the card.
    editing_existing: bool,
    symbology: Option<BarcodeSymbology>,
    symbology_failed: bool,
    loading: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl MyCard {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let state = AppState::global(cx);
        let draft = cx.new(|cx| InputState::new(window, cx).placeholder(Copy::MyCard::numberPlaceholder));
        let subscriptions = vec![
            cx.subscribe_in(&draft, window, |this, _, event: &InputEvent, window, cx| match event {
                InputEvent::PressEnter { .. } => this.save(window, cx),
                InputEvent::Change => cx.notify(),
                _ => {}
            }),
            cx.observe(&state, |_, _, cx| cx.notify()),
        ];
        let symbology = state.read(cx).known_card_symbology();
        let mut card = Self {
            editing: false,
            draft,
            editing_existing: false,
            symbology,
            symbology_failed: false,
            loading: None,
            _subscriptions: subscriptions,
        };
        card.load_symbology(cx);
        card
    }

    pub fn mode(&self, cx: &App) -> Mode {
        let state = AppState::global(cx).read(cx);
        let account = state.account();
        if !state.is_signed_in() {
            Mode::SignIn
        } else if self.editing {
            Mode::Entry
        } else if state.card_number().is_some() {
            Mode::Card
        } else if account.snapshot().is_none() && account.is_refreshing() {
            Mode::Loading
        } else {
            Mode::Entry
        }
    }

    pub fn draft(&self) -> &Entity<InputState> {
        &self.draft
    }

    /// Reads the library's symbology (instant after the first time: the core caches it).
    fn load_symbology(&mut self, cx: &mut Context<Self>) {
        self.symbology_failed = false;
        #[cfg(feature = "evidence")]
        if self.symbology.is_none()
            && crate::features::onboarding::evidence_step("my_card").as_deref() == Some("barcode-failed")
        {
            // Screenshots of the Retry state (debug builds).
            self.symbology_failed = true;
            return;
        }
        let fetch = AppState::global(cx).update(cx, |state, cx| state.card_symbology(cx));
        let timeout = cx.background_executor().timer(SYMBOLOGY_TIMEOUT);
        self.loading = Some(cx.spawn(async move |this, cx| {
            let result = futures::future::select(fetch, timeout.boxed()).await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    futures::future::Either::Left((symbology, _)) => this.symbology = Some(symbology),
                    futures::future::Either::Right(_) => this.symbology_failed = this.symbology.is_none(),
                }
                this.loading = None;
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn retry(&mut self, cx: &mut Context<Self>) {
        self.load_symbology(cx);
    }

    fn can_save(&self, cx: &App) -> bool {
        !self.draft.read(cx).value().trim().is_empty()
    }

    /// "Edit card number": the form, filled in with the current number.
    fn edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let current = AppState::global(cx).read(cx).card_number();
        self.editing_existing = current.is_some();
        self.draft.update(cx, |draft, cx| {
            draft.set_value(current.unwrap_or_default(), window, cx);
            draft.focus(window, cx);
        });
        self.editing = true;
        cx.notify();
    }

    fn cancel_editing(&mut self, cx: &mut Context<Self>) {
        if self.editing_existing {
            self.editing_existing = false;
            self.editing = false;
            cx.notify();
        }
    }

    /// Saves the number for this library and shows the card; nothing while blank.
    fn save(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        if !self.can_save(cx) {
            return;
        }
        let number = self.draft.read(cx).value().trim().to_owned();
        AppState::global(cx).update(cx, |state, cx| state.set_card_number(Some(&number), cx));
        self.editing_existing = false;
        self.editing = false;
        cx.notify();
    }

    fn render_signed_out(&self) -> AnyElement {
        div()
            .id("my-card-signed-out")
            .test_support()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(spacing::M))
            .py(px(spacing::L))
            .child(icon(Lucide::Barcode, px(40.), theme::color(colors::ON_SHEET_SECONDARY)))
            .child(
                div()
                    .typography(Font::Section)
                    .text_color(theme::color(colors::ON_SHEET))
                    .child(Copy::Profile::notSignedInTitle),
            )
            .child(
                div()
                    .typography(Font::Body)
                    .text_color(theme::color(colors::ON_SHEET_SECONDARY))
                    .text_center()
                    .child(Copy::MyCard::signInMessage),
            )
            .child(div().w(px(220.)).pt(px(spacing::S)).child(
                SheetButton::new("my-card-sign-in", Copy::Account::signIn).on_click(on_click(|window, cx| {
                    window.close_dialog(cx);
                    AppState::global(cx).update(cx, |state, cx| state.request_sign_in(cx));
                })),
            ))
            .into_any_element()
    }

    fn render_entry(&self, cx: &mut Context<Self>) -> AnyElement {
        let entity = cx.entity();
        let cancel = cx.entity();
        div()
            .id("my-card-entry")
            .test_support()
            .flex()
            .flex_col()
            .gap(px(spacing::L))
            .child(sheet::labelled_field(
                Copy::MyCard::numberLabel,
                sheet::field_box()
                    .id("my-card-number-field")
                    .test_support()
                    .child(sheet::sheet_input(&self.draft, Copy::MyCard::numberLabel)),
            ))
            .child(
                div()
                    .typography(Font::Caption)
                    .text_color(theme::color(colors::ON_SHEET_SECONDARY))
                    .child(Copy::MyCard::numberNote),
            )
            .child(
                SheetButton::new("my-card-save", Copy::Common::save)
                    .disabled(!self.can_save(cx))
                    .on_click(on_click(move |window, cx| entity.update(cx, |card, cx| card.save(window, cx)))),
            )
            .when(self.editing_existing, |column| {
                column.child(
                    SheetButton::new("my-card-cancel", Copy::Common::cancel)
                        .filled(false)
                        .on_click(on_click(move |_, cx| cancel.update(cx, |card, cx| card.cancel_editing(cx)))),
                )
            })
            .into_any_element()
    }

    fn render_card(&self, number: String, cx: &mut Context<Self>) -> AnyElement {
        let state = AppState::global(cx).read(cx);
        let library_name = state.library_name();
        let logo = state.library().and_then(|library| library.logo_url.clone());
        let borrower = state.account_name();
        let expires = state.card_expires();
        let barcode = barcode_for(&number, self.symbology, self.symbology_failed);
        let entity = cx.entity();
        let expiry = expires.map(|date| {
            div()
                .typography(Font::Caption)
                .text_color(theme::color(colors::ON_SHEET_SECONDARY))
                .child(Copy::MyCard::expires(&date))
        });
        let number_text = |text: String| {
            div()
                .typography(Font::BodyStrong)
                // copy-exempt: a font family name
                .font_family("Menlo")
                .text_color(theme::color(colors::ON_SHEET))
                .child(text)
        };
        let area = match barcode {
            Barcode::Loading => div()
                .id("my-card-barcode")
                .test_support()
                .role(Role::ProgressIndicator)
                .aria_label(SharedString::from(Copy::MyCard::loadingBarcode(&number)))
                .flex()
                .flex_col()
                .items_center()
                .gap(px(spacing::S))
                .child(div().h(px(BARCODE_HEIGHT)).flex().items_center().child(
                    gpui_kit::component::spinner::Spinner::new().color(theme::color(colors::ON_SHEET_SECONDARY)),
                ))
                .child(number_text(number.clone()))
                .children(expiry),
            Barcode::Failed => div()
                .id("my-card-barcode")
                .test_support()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(spacing::S))
                .child(
                    div()
                        .h(px(BARCODE_HEIGHT))
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .gap(px(spacing::S))
                        .child(
                            div()
                                .typography(Font::Body)
                                .text_color(theme::color(colors::ON_SHEET_SECONDARY))
                                .child(Copy::MyCard::barcodeFailed),
                        )
                        .child(
                            div().w(px(160.)).child(
                                SheetButton::new("my-card-retry", Copy::Common::retry)
                                    .filled(false)
                                    .on_click(on_click(move |_, cx| entity.update(cx, |card, cx| card.retry(cx)))),
                            ),
                        ),
                )
                .child(number_text(number.clone()))
                .children(expiry),
            Barcode::Bars(barcode) => div()
                .id("my-card-barcode")
                .test_support()
                .role(Role::Image)
                .aria_label(SharedString::from(Copy::MyCard::barcode(&number)))
                .flex()
                .flex_col()
                .items_center()
                .gap(px(spacing::S))
                .child(BarcodeBars::new(barcode.modules, BARCODE_WIDTH - 2., BARCODE_HEIGHT))
                .child(
                    div()
                        .id("my-card-number")
                        .test_support()
                        .aria_label(SharedString::from(barcode.text.clone()))
                        .child(number_text(barcode.text)),
                )
                .children(expiry),
            Barcode::Unencodable(text) => div()
                .id("my-card-barcode")
                .test_support()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(spacing::S))
                .px(px(CARD_PADDING))
                .child(
                    div()
                        .text_size(px(28.))
                        .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                        // copy-exempt: a font family name
                        .font_family("Menlo")
                        .text_color(theme::color(colors::ON_SHEET))
                        .text_center()
                        .child(text),
                )
                .child(
                    div()
                        .typography(Font::Caption)
                        .text_color(theme::color(colors::ON_SHEET_SECONDARY))
                        .text_center()
                        .child(Copy::MyCard::cannotEncode),
                ),
        };
        let edit = cx.entity();
        div()
            .id("my-card")
            .test_support()
            .flex()
            .flex_col()
            .gap(px(spacing::L))
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .rounded(px(radius::CARD))
                    .bg(theme::color(colors::SHEET))
                    .border_1()
                    .border_color(theme::color(colors::CHIP_ON_SHEET))
                    .shadow(shadow((0x000000, 0.12, 12., 4.)))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(spacing::M))
                            .px(px(CARD_PADDING))
                            .pt(px(CARD_PADDING))
                            .child(LibraryLogo::new("my-card-logo", library_name.clone(), logo).size(40.))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .typography(Font::BodyStrong)
                                            .text_color(theme::color(colors::ON_SHEET))
                                            .child(library_name),
                                    )
                                    .child(
                                        div()
                                            .typography(Font::Caption)
                                            .text_color(theme::color(colors::ON_SHEET_SECONDARY))
                                            .child(Copy::MyCard::libraryCard),
                                    ),
                            ),
                    )
                    .when_some(borrower, |card, name| {
                        card.child(
                            div()
                                .id("my-card-name")
                                .test_support()
                                .aria_label(SharedString::from(name.clone()))
                                .px(px(CARD_PADDING))
                                .pt(px(spacing::L))
                                .typography(Font::Section)
                                .text_color(theme::color(colors::ON_SHEET))
                                .child(name),
                        )
                    })
                    .child(div().pt(px(spacing::XL)).pb(px(CARD_PADDING)).child(area)),
            )
            .child(
                div()
                    .typography(Font::Body)
                    .text_color(theme::color(colors::ON_SHEET_SECONDARY))
                    .text_center()
                    .child(Copy::MyCard::showAtDesk),
            )
            .child(
                SheetButton::new("my-card-edit", Copy::MyCard::editNumber)
                    .filled(false)
                    .on_click(on_click(move |window, cx| edit.update(cx, |card, cx| card.edit(window, cx)))),
            )
            .into_any_element()
    }
}

impl Render for MyCard {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let number = AppState::global(cx).read(cx).card_number();
        let content = match (self.mode(cx), number) {
            (Mode::SignIn, _) => self.render_signed_out(),
            (Mode::Card, Some(number)) => self.render_card(number, cx),
            (Mode::Loading, _) => div()
                .h(px(220.))
                .child(sheet::sheet_loading("my-card-loading", Copy::Profile::loadingAccount))
                .into_any_element(),
            _ => self.render_entry(cx),
        };
        div().id("my-card-dialog").test_support().w_full().pb(px(spacing::S)).child(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_barcode_area_follows_the_symbology() {
        assert_eq!(barcode_for("DEMO0012345", None, false), Barcode::Loading);
        assert_eq!(barcode_for("DEMO0012345", None, true), Barcode::Failed);
        assert!(
            matches!(barcode_for("DEMO0012345", Some(BarcodeSymbology::Code39), false), Barcode::Bars(b) if b.symbology == BarcodeSymbology::Code39)
        );
        // Codabar can't encode letters: Code 128 instead.
        assert!(
            matches!(barcode_for("DEMO0012345", Some(BarcodeSymbology::Codabar), false), Barcode::Bars(b) if b.symbology == BarcodeSymbology::Code128)
        );
        assert!(
            matches!(barcode_for("1234", Some(BarcodeSymbology::Codabar), false), Barcode::Bars(b) if b.symbology == BarcodeSymbology::Codabar)
        );
        assert_eq!(barcode_for("ÉÉ", None, false), Barcode::Unencodable("ÉÉ".into()));
    }
}
