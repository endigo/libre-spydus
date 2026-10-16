//! The Reserve dialog for one format (docs/native-apps.md §6; iOS `ReserveSheet.swift`):
//! `reserve_options` → Collect from, availability, the expiry choice → Confirm →
//! `place_reservation` → the result (an `ok: false` answer is shown, not thrown) →
//! `account_did_change`. White like every dialog: neutral buttons, the confirming one bold, no coral.
//!
//! Workers exercise it in demo mode only (docs/desktop-app.md §2): never against a real library.

use gpui_kit::component::WindowExt;
use gpui_kit::{
    AppContext, Context, CursorStyle, Entity, InteractiveElement, IntoElement, ParentElement, Render, Role,
    SharedString, StatefulInteractiveElement, Styled, TestSupportExt, WeakEntity, Window, div, prelude::FluentBuilder,
    px,
};
use librespydus_core::{ReserveOptions, ReserveResult, SpydusError};

use crate::copy::Copy;
use crate::state::formatting::{byline, non_empty};
use crate::state::{AppState, BookRef, ErrorKind, FormatRef, Loadable, UserFacingError};
use crate::theme::{self, Font, colors, cover, radius, spacing};
use crate::ui::{
    Chip, ChipTone, CoverImage, DialogButton, FontExt, Lucide, dialog_title, icon, on_click, sheet_dialog,
};

use super::DetailScreen;
use super::logic::{display_title, title_cased};
use super::sheet::{SheetBanner, SheetBannerTone, SheetError, SheetLoading};

pub struct ReserveDialog {
    format: FormatRef,
    book: BookRef,
    screen: WeakEntity<DetailScreen>,
    options: Loadable<ReserveOptions>,
    /// `ExpiryChoice.days` sent as `xdays`; the first choice by default.
    selected_days: Option<String>,
    submitting: bool,
    /// The library's answer (`ok: false` is data to show, not an error).
    result: Option<ReserveResult>,
    /// A thrown failure on Confirm (network, unexpected page); Confirm can be tried again.
    submit_error: Option<UserFacingError>,
}

/// Opens the Reserve dialog for `format` of `book`, and loads its options.
pub fn open(
    format: FormatRef,
    book: BookRef,
    screen: WeakEntity<DetailScreen>,
    window: &mut Window,
    cx: &mut gpui_kit::App,
) -> Entity<ReserveDialog> {
    let dialog = cx.new(|_| ReserveDialog {
        format,
        book,
        screen,
        options: Loadable::Idle,
        selected_days: None,
        submitting: false,
        result: None,
        submit_error: None,
    });
    dialog.update(cx, |dialog, cx| dialog.load_options(window, cx));
    let content = dialog.clone();
    window.open_dialog(cx, move |base, _, _| {
        sheet_dialog(base).w(px(500.)).title(dialog_title(Copy::Reserve::title)).child(content.clone())
    });
    dialog
}

impl ReserveDialog {
    pub fn options(&self) -> &Loadable<ReserveOptions> {
        &self.options
    }

    pub fn result(&self) -> Option<&ReserveResult> {
        self.result.as_ref()
    }

    /// Placed: Confirm is replaced by Done, so a reservation can't be sent twice.
    fn is_complete(&self) -> bool {
        self.result.as_ref().is_some_and(|result| result.ok)
    }

    fn load_options(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(url) = non_empty(self.format.reserve_url.as_deref()) else {
            self.options =
                Loadable::Failed(UserFacingError::new(ErrorKind::Unexpected, Copy::Reserve::notFromApp, None));
            return;
        };
        let state = AppState::global(cx);
        let Some(session) = state.read(cx).session().map(str::to_owned) else {
            self.needs_sign_in(SpydusError::NotLoggedIn, window, cx);
            return;
        };
        self.options = Loadable::Loading;
        cx.notify();
        let fetch = state.read(cx).backend().reserve_options(url, session);
        cx.spawn_in(window, async move |this, cx| {
            let result = fetch.await;
            let _ = this.update_in(cx, |this, window, cx| {
                match result {
                    Ok(options) => {
                        this.selected_days = options.expiry_choices.first().map(|choice| choice.days.clone());
                        this.options = Loadable::Loaded(options);
                    }
                    Err(SpydusError::NotLoggedIn) => this.needs_sign_in(SpydusError::NotLoggedIn, window, cx),
                    Err(error) => {
                        this.options =
                            Loadable::Failed(AppState::global(cx).update(cx, |state, cx| state.report(&error, cx)))
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Confirm: `place_reservation` with the chosen expiry, once.
    pub fn confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(url) = non_empty(self.format.reserve_url.as_deref()) else { return };
        if self.submitting || self.is_complete() {
            return;
        }
        let state = AppState::global(cx);
        let Some(session) = state.read(cx).session().map(str::to_owned) else {
            self.needs_sign_in(SpydusError::NotLoggedIn, window, cx);
            return;
        };
        self.submitting = true;
        self.submit_error = None;
        self.result = None;
        cx.notify();
        let place = state.read(cx).backend().place_reservation(url, session, self.selected_days.clone(), None);
        cx.spawn_in(window, async move |this, cx| {
            let result = place.await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.submitting = false;
                match result {
                    Ok(result) => {
                        if result.ok {
                            // The new reservation shows in Your Books (and on this page) without a refresh.
                            AppState::global(cx).update(cx, |state, cx| state.account_did_change(cx));
                        }
                        this.result = Some(result);
                    }
                    Err(SpydusError::NotLoggedIn) => this.needs_sign_in(SpydusError::NotLoggedIn, window, cx),
                    Err(error) => {
                        this.submit_error = Some(AppState::global(cx).update(cx, |state, cx| state.report(&error, cx)))
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// The session is gone: this dialog closes, sign-in opens, and the page reserves once signed in.
    fn needs_sign_in(&mut self, error: SpydusError, window: &mut Window, cx: &mut Context<Self>) {
        window.close_dialog(cx);
        let format = self.format.clone();
        let _ = self.screen.update(cx, |screen, cx| screen.wait_for_sign_in(format, window, cx));
        AppState::global(cx).update(cx, |state, cx| {
            state.report(&error, cx);
        });
    }

    fn select_days(&mut self, days: String, cx: &mut Context<Self>) {
        self.selected_days = Some(days);
        cx.notify();
    }

    /// The result's words: the library's message, else "Reservation placed" / "The library
    /// couldn’t place this reservation."
    fn message(result: &ReserveResult) -> String {
        non_empty(result.message.as_deref())
            .unwrap_or_else(|| (if result.ok { Copy::Reserve::placed } else { Copy::Reserve::placeFailed }).to_owned())
    }

    fn summary(&self) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .items_start()
            .gap(px(spacing::L))
            .child(
                CoverImage::new("reserve-cover", self.book.cover_url.clone(), self.book.title.clone())
                    .author(self.book.author.clone())
                    .width(cover::SMALL_WIDTH),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(spacing::XS))
                    .child(
                        div()
                            .typography(Font::BodyStrong)
                            .text_color(theme::color(colors::ON_SHEET))
                            .line_clamp(3)
                            .text_ellipsis()
                            .child(display_title(&self.book.title)),
                    )
                    .children(byline(self.book.author.as_deref()).map(|line| {
                        div().typography(Font::Caption).text_color(theme::color(colors::ON_SHEET_SECONDARY)).child(line)
                    }))
                    // An edition-level item's one format has no name to show.
                    .when(!self.format.is_edition_level(), |column| {
                        column.child(
                            div().pt(px(spacing::XS)).flex().child(
                                Chip::new(self.format.display_label())
                                    .tone(ChipTone::OnSheet)
                                    .icon(if self.format.is_digital() { Lucide::Cloud } else { Lucide::BookOpen }),
                            ),
                        )
                    }),
            )
    }

    fn info_row(id: &'static str, label: &'static str, value: String, name: Lucide) -> impl IntoElement {
        div()
            .id(id)
            .test_support()
            .role(Role::Group)
            .aria_label(SharedString::from(format!("{label}{}{value}", Copy::Common::separator)))
            .flex()
            .flex_row()
            .items_start()
            .gap(px(spacing::M))
            .py(px(spacing::M))
            .child(div().pt(px(2.)).child(icon(name, px(18.), theme::color(colors::ON_SHEET_SECONDARY))))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(
                        div()
                            .typography(Font::Caption)
                            .text_color(theme::color(colors::ON_SHEET_SECONDARY))
                            .child(label),
                    )
                    .child(div().typography(Font::Body).text_color(theme::color(colors::ON_SHEET)).child(value)),
            )
    }

    fn divider() -> impl IntoElement {
        div().h(px(1.)).w_full().bg(theme::color(colors::CHIP_ON_SHEET))
    }

    fn form(&self, options: &ReserveOptions, cx: &mut Context<Self>) -> impl IntoElement {
        let pickup = non_empty(options.pickup_location.as_deref())
            .map_or_else(|| Copy::Reserve::usualBranch.to_owned(), |branch| title_cased(&branch));
        let locked = self.submitting || self.is_complete();
        let entity = cx.entity();
        div()
            .flex()
            .flex_col()
            .px(px(spacing::L))
            .rounded(px(radius::CARD))
            .border_1()
            .border_color(theme::color(colors::CHIP_ON_SHEET))
            .child(Self::info_row("reserve-collect-from", Copy::Reserve::collectFrom, pickup, Lucide::MapPinned))
            .when_some(non_empty(options.available_summary.as_deref()), |form, available| {
                form.child(Self::divider()).child(Self::info_row(
                    "reserve-availability",
                    Copy::Reserve::availability,
                    available,
                    Lucide::LibraryBig,
                ))
            })
            .when(!options.expiry_choices.is_empty(), |form| {
                let selected = self.selected_days.clone();
                form.child(Self::divider()).child(
                    div()
                        .flex()
                        .flex_row()
                        .items_start()
                        .gap(px(spacing::M))
                        .py(px(spacing::M))
                        .child(div().pt(px(2.)).child(icon(
                            Lucide::Calendar,
                            px(18.),
                            theme::color(colors::ON_SHEET_SECONDARY),
                        )))
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .gap(px(spacing::S))
                                .child(
                                    div()
                                        .typography(Font::Caption)
                                        .text_color(theme::color(colors::ON_SHEET_SECONDARY))
                                        .child(Copy::Reserve::notNeededAfter),
                                )
                                .child(
                                    div()
                                        .id("reserve-expiry")
                                        .test_support()
                                        .role(Role::RadioGroup)
                                        .aria_label(SharedString::from(Copy::Reserve::notNeededAfter))
                                        .flex()
                                        .flex_row()
                                        .flex_wrap()
                                        .gap(px(spacing::S))
                                        .children(options.expiry_choices.iter().enumerate().map(|(index, choice)| {
                                            let chosen = selected.as_deref() == Some(choice.days.as_str());
                                            let days = choice.days.clone();
                                            let entity = entity.clone();
                                            div()
                                                .id(SharedString::from(format!("reserve-expiry-{index}")))
                                                .test_support()
                                                .role(Role::RadioButton)
                                                .aria_label(SharedString::from(choice.label.clone()))
                                                .aria_selected(chosen)
                                                .px(px(spacing::M))
                                                .py(px(spacing::XS + 2.))
                                                .rounded(px(radius::PILL))
                                                .typography(Font::Caption)
                                                .when(chosen, |chip| {
                                                    chip.bg(theme::color(colors::ON_SHEET))
                                                        .text_color(theme::color(colors::SHEET))
                                                })
                                                .when(!chosen, |chip| {
                                                    chip.bg(theme::color(colors::CHIP_ON_SHEET))
                                                        .text_color(theme::color(colors::ON_SHEET))
                                                })
                                                .when(!locked, |chip| {
                                                    chip.cursor(CursorStyle::PointingHand).on_click(move |_, _, cx| {
                                                        entity.update(cx, |dialog, cx| {
                                                            dialog.select_days(days.clone(), cx)
                                                        })
                                                    })
                                                })
                                                .child(choice.label.clone())
                                        })),
                                ),
                        ),
                )
            })
    }

    fn footer(&self, options: &ReserveOptions, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        let finished =
            !options.can_reserve || self.is_complete() || self.result.as_ref().is_some_and(|result| !result.ok);
        let close_label = if self.is_complete() { Copy::Common::done } else { Copy::Common::close };
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(spacing::M))
            .when_some(self.result.as_ref(), |footer, result| {
                let tone = if result.ok { SheetBannerTone::Success } else { SheetBannerTone::Error };
                footer.child(SheetBanner::new("reserve-result", Self::message(result), tone))
            })
            .when_some(self.submit_error.as_ref(), |footer, error| {
                footer.child(SheetBanner::new("reserve-error", error.message.clone(), SheetBannerTone::Error))
            })
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .justify_end()
                    .gap(px(spacing::S))
                    .when(finished, |row| {
                        row.child(
                            DialogButton::new(
                                "reserve-close",
                                close_label,
                                on_click(|window, cx| window.close_dialog(cx)),
                            )
                            .confirming(true),
                        )
                    })
                    .when(!finished, |row| {
                        let submitting = self.submitting;
                        row.child(DialogButton::new(
                            "reserve-cancel",
                            Copy::Common::cancel,
                            on_click(|window, cx| window.close_dialog(cx)),
                        ))
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap(px(spacing::S))
                                .when(submitting, |cell| {
                                    cell.child(
                                        gpui_kit::component::spinner::Spinner::new()
                                            .color(theme::color(colors::ON_SHEET_SECONDARY)),
                                    )
                                })
                                .child(
                                    DialogButton::new(
                                        "reserve-confirm",
                                        Copy::Reserve::confirm,
                                        on_click(move |window, cx| {
                                            entity.update(cx, |dialog, cx| dialog.confirm(window, cx));
                                        }),
                                    )
                                    .confirming(true),
                                ),
                        )
                    }),
            )
    }
}

impl Render for ReserveDialog {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = div().w_full().flex().flex_col().gap(px(spacing::XL)).child(self.summary());
        let retry_entity = cx.entity();
        let body = match &self.options {
            Loadable::Idle | Loadable::Loading => {
                body.child(SheetLoading::new("reserve-loading", Copy::Reserve::checkingOptions))
            }
            Loadable::Failed(error) => {
                body.child(SheetError::new("reserve-options-error", error).retry(on_click(move |window, cx| {
                    retry_entity.update(cx, |dialog, cx| dialog.load_options(window, cx))
                })))
            }
            Loadable::Loaded(options) => {
                let options = options.clone();
                let content = if options.can_reserve {
                    self.form(&options, cx).into_any_element()
                } else {
                    let message =
                        non_empty(options.message.as_deref()).unwrap_or_else(|| Copy::Reserve::cantReserve.to_owned());
                    SheetBanner::new("reserve-cant", message, SheetBannerTone::Info).into_any_element()
                };
                body.child(content).child(self.footer(&options, cx))
            }
        };
        let footer = match &self.options {
            Loadable::Loaded(_) => None,
            _ => Some(
                div().w_full().flex().justify_end().child(
                    DialogButton::new(
                        "reserve-close",
                        Copy::Common::close,
                        on_click(|window, cx| window.close_dialog(cx)),
                    )
                    .confirming(true),
                ),
            ),
        };
        div()
            .id("reserve-dialog")
            .test_support()
            .role(Role::Dialog)
            .aria_label(SharedString::from(Copy::Reserve::title))
            .w_full()
            .child(body.children(footer))
    }
}
