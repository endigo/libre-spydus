//! Your Books (D5; docs/desktop-app.md §6, native-apps.md §6 "Your Books", §5.2a, §5.12; a port of
//! iOS `BorrowedView.swift` / `BorrowedViewModel.swift`): Loans | Reservations.
//!
//! Both lists come from the saved account (`AppState::account`, §5.2a): shown at once with
//! "Updated 12 min ago" under the segments, and fetched again in the background when it's out of
//! date on opening the section, with ⌘R or the refresh button, and after a change. A failed
//! refresh keeps the saved lists ("Couldn’t refresh · Showing the saved list").
//!
//! Each card's actions sit behind its "…" button and on right-click (the phones' swipe): a loan's
//! Renew (confirmed, §5.12; "Renewing…" and the result as toasts), a reservation's Edit (a dialog)
//! and Cancel (confirmed, red). A card click opens the book page.

mod changes;
pub mod edit;
#[cfg(feature = "evidence")]
mod evidence;
pub mod rows;

use std::collections::HashSet;
use std::time::Duration;

use chrono::Local;
use gpui_kit::component::Sizable;
use gpui_kit::{
    AnyElement, Context, CursorStyle, Entity, InteractiveElement, IntoElement, ParentElement, Render, Role,
    SharedString, StatefulInteractiveElement, Styled, Subscription, Task, TestSupportExt, WeakEntity, Window, div,
    prelude::FluentBuilder, px,
};
use librespydus_core::ReserveResult;

use crate::copy::Copy;
use crate::state::formatting::{non_empty, today, updated};
use crate::state::renew;
use crate::state::{AppState, NoticeTone, ReloadKey, Route, Section, YourBooksSegment};
use crate::theme::{self, Font, colors, radius, spacing};
use crate::ui::{
    BookRowCard, CardAction, Chip, ChipTone, ConfirmDialog, EmptyStateView, ErrorStateView, FontExt, LoadingView,
    Lucide, SectionHeader, card_columns, card_grid, card_menu_button, content_column, icon, on_click,
    with_context_menu,
};

use self::rows::{LoanRow, ReservationRow, ReservationSections};

/// Tall enough that a state view sits in the middle of the window.
const STATE_HEIGHT: f32 = 380.;

/// A cancel or edit result, shown as a banner above the list (iOS `BorrowedViewModel.Notice`).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Notice {
    id: u64,
    message: String,
    is_error: bool,
}

pub struct YourBooksScreen {
    state: Entity<AppState>,
    segment: YourBooksSegment,
    /// Reservations with a cancel in flight.
    busy_rows: HashSet<String>,
    /// Loans with a renewal in flight.
    renewing_rows: HashSet<String>,
    notice: Option<Notice>,
    next_notice: u64,
    /// What the screen last saw, to refresh when the section is shown again or the account changes.
    shown: bool,
    reload_key: ReloadKey,
    _observe: Subscription,
    /// Redraws every half minute so "Updated 3 min ago" keeps up.
    _ticker: Task<()>,
}

impl YourBooksScreen {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let state = AppState::global(cx);
        let observe = cx.observe_in(&state, window, |this, _, _, cx| this.state_changed(cx));
        let (segment, reload_key, shown) = state.update(cx, |state, _| {
            (
                state.take_pending_segment().unwrap_or_default(),
                state.reload_key(),
                state.section() == Section::YourBooks,
            )
        });
        let ticker = cx.spawn(async move |this: WeakEntity<Self>, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(30)).await;
                if this.update(cx, |_, cx| cx.notify()).is_err() {
                    break;
                }
            }
        });
        let screen = Self {
            state,
            segment,
            busy_rows: HashSet::new(),
            renewing_rows: HashSet::new(),
            notice: None,
            next_notice: 0,
            shown,
            reload_key,
            _observe: observe,
            _ticker: ticker,
        };
        screen.refresh(false, cx);
        #[cfg(feature = "evidence")]
        evidence::schedule(window, cx);
        screen
    }

    pub fn segment(&self) -> YourBooksSegment {
        self.segment
    }

    fn state_changed(&mut self, cx: &mut Context<Self>) {
        let (segment, section, key) = self.state.update(cx, |state, _| {
            let segment = (state.section() == Section::YourBooks).then(|| state.take_pending_segment()).flatten();
            (segment, state.section(), state.reload_key())
        });
        if let Some(segment) = segment {
            self.segment = segment;
        }
        let shown = section == Section::YourBooks;
        let appeared = shown && !self.shown;
        self.shown = shown;
        if key != self.reload_key {
            // Another library, sign-in or sign-out: what was in flight belongs to the old account.
            self.reload_key = key;
            self.busy_rows.clear();
            self.renewing_rows.clear();
            self.notice = None;
            self.refresh(false, cx);
        } else if appeared {
            // Back on the section: refresh in the background if the account is out of date.
            self.refresh(false, cx);
        }
        cx.notify();
    }

    /// Fetches the account (`force`: ⌘R, the button, Retry; else only when out of date).
    fn refresh(&self, force: bool, cx: &mut Context<Self>) {
        self.state.update(cx, |state, cx| {
            if state.is_signed_in() {
                state.refresh_account(force, cx).detach();
            }
        });
    }

    pub fn select_segment(&mut self, segment: YourBooksSegment, cx: &mut Context<Self>) {
        self.segment = segment;
        cx.notify();
    }

    fn show_notice(&mut self, message: String, is_error: bool, cx: &mut Context<Self>) {
        self.next_notice += 1;
        let id = self.next_notice;
        self.notice = Some(Notice { id, message, is_error });
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_secs(6)).await;
            let _ = this.update(cx, |this, cx| {
                if this.notice.as_ref().is_some_and(|notice| notice.id == id) {
                    this.notice = None;
                    cx.notify();
                }
            });
        })
        .detach();
        cx.notify();
    }

    fn open(&self, row_book: crate::state::BookRef, cx: &mut Context<Self>) {
        self.state.update(cx, |state, cx| state.push_on(Section::YourBooks, Route::Detail(row_book), cx));
    }

    // ---- Renew (§5.12) ----

    /// Asks to confirm renewing `row` (not while it's renewing).
    pub fn request_renew(&mut self, row: LoanRow, window: &mut Window, cx: &mut Context<Self>) {
        if !row.can_renew() || self.renewing_rows.contains(&row.id) {
            return;
        }
        let view = cx.entity().downgrade();
        let message = renew::confirm_message(&row.item, today());
        ConfirmDialog::new(
            Copy::YourBooks::renewTitle,
            Copy::YourBooks::renew,
            on_click(move |_, cx| {
                let _ = view.update(cx, |this, cx| this.renew(row.clone(), cx));
            }),
        )
        .message(message)
        .cancel_label(Copy::Common::notNow)
        .open(window, cx);
    }

    /// Renews `row` after the reader confirmed: the card shows progress and "Renewing…" while the
    /// library answers, then the result; the account is fetched again either way.
    pub fn renew(&mut self, row: LoanRow, cx: &mut Context<Self>) {
        if !row.can_renew() || self.renewing_rows.contains(&row.id) {
            return;
        }
        self.renewing_rows.insert(row.id.clone());
        let task = self.state.update(cx, |state, cx| {
            state.show_notice(Copy::YourBooks::renewing, NoticeTone::Progress, cx);
            state.renew(&row.item, cx)
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.renewing_rows.remove(&row.id);
                let (message, tone) = match &result {
                    Ok(result) => (
                        renew::result_text(result, today()),
                        if result.ok { NoticeTone::Success } else { NoticeTone::Error },
                    ),
                    Err(_) => (Copy::YourBooks::renewFailed.to_owned(), NoticeTone::Error),
                };
                this.state.update(cx, |state, cx| state.show_notice(&message, tone, cx));
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    // ---- Cancel ----

    /// Asks to confirm cancelling `row` (not while it's busy): red, "Keep reservation" keeps it.
    pub fn request_cancel(&mut self, row: ReservationRow, window: &mut Window, cx: &mut Context<Self>) {
        if row.cancel_href.is_none() || self.busy_rows.contains(&row.id) {
            return;
        }
        let view = cx.entity().downgrade();
        let message = row.cancel_message();
        ConfirmDialog::new(
            Copy::YourBooks::cancelTitle,
            Copy::YourBooks::cancelReservation,
            on_click(move |_, cx| {
                let _ = view.update(cx, |this, cx| this.cancel(row.clone(), cx));
            }),
        )
        .message(message)
        .cancel_label(Copy::YourBooks::keepReservation)
        .destructive(true)
        .open(window, cx);
    }

    /// Cancels `row` after the reader confirmed; on success the card goes at once.
    pub fn cancel(&mut self, row: ReservationRow, cx: &mut Context<Self>) {
        let Some(href) = row.cancel_href.clone() else { return };
        if self.busy_rows.contains(&row.id) {
            return;
        }
        let key = self.reload_key.clone();
        self.busy_rows.insert(row.id.clone());
        let task = self.state.update(cx, |state, cx| state.cancel_reservation(href, cx));
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.busy_rows.remove(&row.id);
                if this.state.read(cx).reload_key() != key {
                    return;
                }
                match result {
                    Ok(result) => {
                        let fallback =
                            if result.ok { Copy::YourBooks::cancelled } else { Copy::YourBooks::cancelFailed };
                        let message = non_empty(result.message.as_deref()).unwrap_or_else(|| fallback.to_owned());
                        this.show_notice(message, !result.ok, cx);
                    }
                    Err(error) => this.show_notice(error.message, true, cx),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    // ---- Edit ----

    /// Opens the edit dialog for `row` (not while it's busy).
    pub fn request_edit(&mut self, row: ReservationRow, window: &mut Window, cx: &mut Context<Self>) {
        if row.edit_href.is_none() || self.busy_rows.contains(&row.id) {
            return;
        }
        edit::open(row, cx.entity().downgrade(), window, cx);
    }

    /// The edit dialog saved: show the library's message.
    pub(crate) fn finish_edit(&mut self, _row_id: &str, result: Option<ReserveResult>, cx: &mut Context<Self>) {
        if let Some(result) = result {
            let message = non_empty(result.message.as_deref()).unwrap_or_else(|| Copy::YourBooks::updated.to_owned());
            self.show_notice(message, !result.ok, cx);
        }
    }

    // ---- Rendering ----

    fn header(&self, signed_in: bool, refreshing: bool, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .child(
                div()
                    .id("your-books-title")
                    .test_support()
                    .role(Role::Heading)
                    .aria_label(SharedString::from(Copy::YourBooks::title))
                    .typography(Font::Greeting)
                    .text_color(theme::color(colors::TEXT_PRIMARY))
                    .child(Copy::YourBooks::title),
            )
            .when(signed_in, |row| {
                row.child(
                    div()
                        .id("your-books-refresh")
                        .test_support()
                        .role(Role::Button)
                        .aria_label(SharedString::from(Copy::Desktop::refreshAccount))
                        .aria_description(SharedString::from(Copy::YourBooks::refreshHint))
                        .aria_keyshortcuts("Meta+R")
                        .cursor(CursorStyle::PointingHand)
                        .size(px(36.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(radius::PILL))
                        .bg(theme::color(colors::SURFACE))
                        .hover(|style| style.bg(theme::color(colors::SURFACE_ELEVATED)))
                        .on_click(cx.listener(|this, _, _, cx| this.refresh(true, cx)))
                        .map(|button| {
                            if refreshing {
                                button.child(
                                    gpui_kit::component::spinner::Spinner::new()
                                        .color(theme::color(colors::TEXT_SECONDARY)),
                                )
                            } else {
                                button.child(icon(Lucide::RefreshCw, px(16.), theme::color(colors::TEXT_PRIMARY)))
                            }
                        }),
                )
            })
    }

    fn segment_control(&self, lists: Option<&rows::Lists>, cx: &mut Context<Self>) -> impl IntoElement {
        let item = |segment: YourBooksSegment, title: String, cx: &mut Context<Self>| {
            let selected = self.segment == segment;
            let id = match segment {
                YourBooksSegment::Loans => "your-books-segment-loans",
                YourBooksSegment::Reservations => "your-books-segment-reservations",
            };
            div()
                .id(id)
                .test_support()
                .role(Role::Tab)
                .aria_label(SharedString::from(title.clone()))
                .aria_selected(selected)
                .cursor(CursorStyle::PointingHand)
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .h(px(32.))
                .px(px(spacing::L))
                .rounded(px(radius::PILL))
                .typography(if selected { Font::BodyStrong } else { Font::Body })
                .map(|tab| {
                    if selected {
                        tab.bg(theme::color(colors::TAB_BAR)).text_color(theme::color(colors::TAB_ACTIVE))
                    } else {
                        tab.text_color(theme::color(colors::TEXT_SECONDARY))
                            .hover(|style| style.text_color(theme::color(colors::TEXT_PRIMARY)))
                    }
                })
                .on_click(cx.listener(move |this, _, _, cx| this.select_segment(segment, cx)))
                .child(title)
        };
        let loans = lists.map_or_else(
            || Copy::YourBooks::loans.to_owned(),
            |lists| Copy::YourBooks::loansCount(lists.loans.len() as i64),
        );
        let reservations = lists.map_or_else(
            || Copy::YourBooks::reservations.to_owned(),
            |lists| Copy::YourBooks::reservationsCount(lists.reservations.count() as i64),
        );
        div()
            .id("your-books-segments")
            .role(Role::TabList)
            .aria_label(SharedString::from(Copy::YourBooks::show))
            .w(px(420.))
            .max_w_full()
            .flex()
            .flex_row()
            .gap(px(spacing::XS))
            .p(px(spacing::XS))
            .rounded(px(radius::PILL))
            .bg(theme::color(colors::SURFACE))
            .child(item(YourBooksSegment::Loans, loans, cx))
            .child(item(YourBooksSegment::Reservations, reservations, cx))
    }

    /// "Updated 12 min ago" (+ a spinner while refreshing), or the failed-refresh note with Retry;
    /// only once there's something on screen.
    fn status_line(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let account = self.state.read(cx).account();
        let updated_at = account.updated_at()?;
        let refreshing = account.is_refreshing();
        let failed = account.last_refresh_error().is_some();
        let line = div()
            .id("your-books-status")
            .test_support()
            .min_h(px(22.))
            .flex()
            .flex_row()
            .items_center()
            .gap(px(spacing::S))
            .typography(Font::Caption)
            .text_color(theme::color(colors::TEXT_MUTED));
        if failed && !refreshing {
            return Some(
                line.aria_label(SharedString::from(Copy::YourBooks::refreshFailed))
                    .child(icon(Lucide::TriangleAlert, px(14.), theme::color(colors::WARNING)))
                    .child(Copy::YourBooks::refreshFailed)
                    .child(
                        div()
                            .id("your-books-status-retry")
                            .test_support()
                            .role(Role::Button)
                            .aria_label(SharedString::from(Copy::Common::retry))
                            .aria_description(SharedString::from(Copy::YourBooks::refreshHint))
                            .cursor(CursorStyle::PointingHand)
                            .typography(Font::BodyStrong)
                            .text_color(theme::color(colors::PRIMARY))
                            .on_click(cx.listener(|this, _, _, cx| this.refresh(true, cx)))
                            .child(Copy::Common::retry),
                    )
                    .into_any_element(),
            );
        }
        let text = updated(updated_at.with_timezone(&Local), Local::now());
        let label = if refreshing { Copy::A11y::refreshing(&text) } else { text.clone() };
        Some(
            line.aria_label(SharedString::from(label))
                .child(text)
                .when(refreshing, |line| {
                    line.child(
                        gpui_kit::component::spinner::Spinner::new().small().color(theme::color(colors::TEXT_MUTED)),
                    )
                })
                .into_any_element(),
        )
    }

    fn notice_banner(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let notice = self.notice.clone()?;
        let tone = if notice.is_error { crate::ui::BannerTone::Error } else { crate::ui::BannerTone::Success };
        Some(
            div()
                .id("your-books-notice")
                .test_support()
                .role(Role::Status)
                .aria_label(SharedString::from(notice.message.clone()))
                .aria_description(SharedString::from(Copy::A11y::dismissHint))
                .cursor(CursorStyle::PointingHand)
                .on_click(cx.listener(|this, _, _, cx| {
                    this.notice = None;
                    cx.notify();
                }))
                .child(crate::ui::Banner::new(notice.message, tone)),
        )
    }

    /// No snapshot yet: loading, or why the first fetch failed.
    fn placeholder(&self, message: &'static str, cx: &mut Context<Self>) -> AnyElement {
        let account = self.state.read(cx).account();
        let body = div().w_full().h(px(STATE_HEIGHT));
        match account.last_refresh_error().filter(|_| !account.is_refreshing()).cloned() {
            Some(error) => body
                .child(ErrorStateView::new("your-books-error", &error).retry(on_click({
                    let state = self.state.clone();
                    move |_, cx| state.update(cx, |state, cx| state.refresh_account(true, cx).detach())
                })))
                .into_any_element(),
            None => body.child(LoadingView::new().message(message)).into_any_element(),
        }
    }

    fn footnote(text: &'static str) -> impl IntoElement {
        div()
            .w_full()
            .pt(px(spacing::S))
            .flex()
            .flex_row()
            .items_center()
            .justify_center()
            .gap(px(spacing::XS))
            .typography(Font::Caption)
            .text_color(theme::color(colors::TEXT_MUTED))
            .child(icon(Lucide::Info, px(13.), theme::color(colors::TEXT_MUTED)))
            .child(text)
    }

    fn loans(&self, loans: &[LoanRow], columns: u16, cx: &mut Context<Self>) -> AnyElement {
        if loans.is_empty() {
            let library = self.state.read(cx).library_name();
            return div()
                .id("loans-empty")
                .test_support()
                .w_full()
                .h(px(STATE_HEIGHT))
                .child(
                    EmptyStateView::new(Lucide::LibraryBig, Copy::YourBooks::noLoansTitle)
                        .message(Copy::YourBooks::noLoansMessage(&library)),
                )
                .into_any_element();
        }
        div()
            .id("loans-list")
            .flex()
            .flex_col()
            .gap(px(spacing::M))
            .child(card_grid(columns).children(loans.iter().map(|row| self.loan_card(row, cx))))
            .when(loans.iter().any(LoanRow::can_renew), |list| {
                list.child(Self::footnote(Copy::YourBooks::loanRightClickHint))
            })
            .into_any_element()
    }

    fn loan_actions(&self, row: &LoanRow, cx: &mut Context<Self>) -> Vec<CardAction> {
        if !row.can_renew() || self.renewing_rows.contains(&row.id) {
            return Vec::new();
        }
        let view = cx.entity().downgrade();
        let row = row.clone();
        vec![CardAction::new(Copy::YourBooks::renewLoan, move |window, cx| {
            let _ = view.update(cx, |this, cx| this.request_renew(row.clone(), window, cx));
        })]
    }

    fn loan_card(&self, row: &LoanRow, cx: &mut Context<Self>) -> AnyElement {
        let renewing = self.renewing_rows.contains(&row.id);
        let actions = self.loan_actions(row, cx);
        let id = format!("loan-{}", row.id);
        let meta_color = theme::color(if row.is_urgent() { colors::DANGER } else { colors::TEXT_SECONDARY });
        let chips = if row.is_overdue() {
            vec![Chip::new(Copy::Loans::overdue).tone(ChipTone::Danger).icon(Lucide::CircleAlert)]
        } else {
            Vec::new()
        };
        let book = row.book.clone();
        let mut card = BookRowCard::new(SharedString::from(id.clone()), &row.book)
            .chips(chips)
            .trailing(self.trailing(&id, renewing, actions.clone()))
            .on_click(on_click({
                let view = cx.entity().downgrade();
                move |_, cx| {
                    let _ = view.update(cx, |this, cx| this.open(book.clone(), cx));
                }
            }));
        if let Some(meta) = row.meta() {
            card = card.meta(meta, Some(meta_color));
        }
        let wrapper = div()
            .id(SharedString::from(format!("{id}-row")))
            .test_support()
            .when(renewing, |row| row.opacity(0.7))
            .child(card);
        if actions.is_empty() {
            wrapper.into_any_element()
        } else {
            with_context_menu(wrapper, actions).into_any_element()
        }
    }

    /// The card's "…" button, or a spinner while a change runs.
    fn trailing(&self, id: &str, busy: bool, actions: Vec<CardAction>) -> AnyElement {
        if busy {
            return div()
                .id(SharedString::from(format!("{id}-busy")))
                .test_support()
                .role(Role::ProgressIndicator)
                .aria_label(SharedString::from(Copy::A11y::updating))
                .p(px(spacing::S))
                .child(gpui_kit::component::spinner::Spinner::new().color(theme::color(colors::TEXT_SECONDARY)))
                .into_any_element();
        }
        if actions.is_empty() {
            return div().into_any_element();
        }
        div()
            .id(SharedString::from(format!("{id}-more")))
            .test_support()
            .child(card_menu_button(SharedString::from(format!("{id}-more-button")), actions))
            .into_any_element()
    }

    fn reservations(&self, sections: &ReservationSections, columns: u16, cx: &mut Context<Self>) -> AnyElement {
        if sections.is_empty() {
            return div()
                .id("reservations-empty")
                .test_support()
                .w_full()
                .h(px(STATE_HEIGHT))
                .child(
                    EmptyStateView::new(Lucide::BookMarked, Copy::YourBooks::noReservationsTitle)
                        .message(Copy::YourBooks::noReservationsMessage),
                )
                .into_any_element();
        }
        div()
            .id("reservations-list")
            .flex()
            .flex_col()
            .gap(px(spacing::M))
            .when(!sections.ready.is_empty(), |list| {
                list.child(div().pt(px(spacing::XS)).child(SectionHeader::new(
                    "reservations-ready",
                    Copy::YourBooks::readyCount(sections.ready.len() as i64),
                )))
                .child(card_grid(columns).children(sections.ready.iter().map(|row| self.reservation_card(row, cx))))
            })
            .when(!sections.pending.is_empty(), |list| {
                list.child(div().pt(px(spacing::XS)).child(SectionHeader::new(
                    "reservations-pending",
                    Copy::YourBooks::pendingCount(sections.pending.len() as i64),
                )))
                .child(card_grid(columns).children(sections.pending.iter().map(|row| self.reservation_card(row, cx))))
            })
            .when(sections.has_actions(), |list| list.child(Self::footnote(Copy::YourBooks::rightClickHint)))
            .into_any_element()
    }

    fn reservation_actions(&self, row: &ReservationRow, cx: &mut Context<Self>) -> Vec<CardAction> {
        if self.busy_rows.contains(&row.id) {
            return Vec::new();
        }
        let mut actions = Vec::new();
        if row.edit_href.is_some() {
            let view = cx.entity().downgrade();
            let row = row.clone();
            actions.push(CardAction::new(Copy::YourBooks::editReservation, move |window, cx| {
                let _ = view.update(cx, |this, cx| this.request_edit(row.clone(), window, cx));
            }));
        }
        if row.cancel_href.is_some() {
            let view = cx.entity().downgrade();
            let row = row.clone();
            actions.push(CardAction::new(Copy::YourBooks::cancelReservation, move |window, cx| {
                let _ = view.update(cx, |this, cx| this.request_cancel(row.clone(), window, cx));
            }));
        }
        actions
    }

    fn status_chip(row: &ReservationRow) -> Chip {
        if row.is_ready() {
            return Chip::new(row.status_label()).tone(ChipTone::Success).icon(Lucide::CircleCheck);
        }
        if row.is_in_transit() {
            return Chip::new(row.status_label()).tone(ChipTone::Warning).icon(Lucide::ArrowRight);
        }
        Chip::new(row.status_label()).icon(Lucide::Calendar)
    }

    fn reservation_card(&self, row: &ReservationRow, cx: &mut Context<Self>) -> AnyElement {
        let busy = self.busy_rows.contains(&row.id);
        let actions = self.reservation_actions(row, cx);
        let id = format!("reservation-{}", row.id);
        let book = row.book.clone();
        let mut card = BookRowCard::new(SharedString::from(id.clone()), &row.book)
            .chips(vec![Self::status_chip(row)])
            .highlighted(row.is_ready())
            .trailing(self.trailing(&id, busy, actions.clone()))
            .on_click(on_click({
                let view = cx.entity().downgrade();
                move |_, cx| {
                    let _ = view.update(cx, |this, cx| this.open(book.clone(), cx));
                }
            }));
        if let Some(meta) = row.meta() {
            let color = theme::color(if row.is_ready() { colors::TEXT_PRIMARY } else { colors::TEXT_SECONDARY });
            card = card.meta(meta, Some(color));
        }
        let wrapper = div()
            .id(SharedString::from(format!("{id}-row")))
            .test_support()
            .when(busy, |row| row.opacity(0.7))
            .child(card);
        if actions.is_empty() {
            wrapper.into_any_element()
        } else {
            with_context_menu(wrapper, actions).into_any_element()
        }
    }

    fn signed_out(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let library = self.state.read(cx).library_name();
        let state = self.state.clone();
        div().id("your-books-signed-out").test_support().w_full().h(px(STATE_HEIGHT + 80.)).child(
            EmptyStateView::new(Lucide::CircleUserRound, Copy::YourBooks::signedOutTitle)
                .message(Copy::YourBooks::signedOutMessage(&library))
                .action(
                    "your-books-sign-in",
                    Copy::Account::signIn,
                    on_click(move |_, cx| state.update(cx, |state, cx| state.request_sign_in(cx))),
                ),
        )
    }
}

impl Render for YourBooksScreen {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let columns = card_columns(window);
        let (signed_in, refreshing, lists) = {
            let state = self.state.read(cx);
            let lists = state.account().snapshot().map(|snapshot| rows::Lists::new(snapshot, today()));
            (state.is_signed_in(), state.account().is_refreshing(), lists)
        };
        // The title, Loans | Reservations and the status line stay put; only the cards scroll.
        let mut top = content_column().pt(px(spacing::XL)).pb(px(spacing::M)).flex().flex_col().gap(px(spacing::L));
        top = top.child(self.header(signed_in, refreshing, cx));
        let body = if signed_in {
            top = top
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(spacing::S))
                        .child(self.segment_control(lists.as_ref(), cx))
                        .children(self.status_line(cx)),
                )
                .children(self.notice_banner(cx));
            match (self.segment, &lists) {
                (YourBooksSegment::Loans, Some(lists)) => self.loans(&lists.loans, columns, cx),
                (YourBooksSegment::Reservations, Some(lists)) => self.reservations(&lists.reservations, columns, cx),
                (YourBooksSegment::Loans, None) => self.placeholder(Copy::YourBooks::loadingLoans, cx),
                (YourBooksSegment::Reservations, None) => self.placeholder(Copy::YourBooks::loadingReservations, cx),
            }
        } else {
            self.signed_out(cx).into_any_element()
        };
        div()
            .id("your-books-screen")
            .test_support()
            .size_full()
            .flex()
            .flex_col()
            .child(div().flex_none().child(top))
            .child(
                div()
                    .id("your-books-scroll")
                    .test_support()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .restrict_scroll_to_axis()
                    .child(content_column().pt(px(spacing::S)).pb(px(spacing::XXL)).child(body)),
            )
    }
}
