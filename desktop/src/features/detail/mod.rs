//! The book page (D4; docs/desktop-app.md §6, docs/native-apps.md §6 "Book detail"; a port of iOS
//! `BookDetailModel` / `BookDetailView`): a navy panel with the cover, title, author, availability,
//! format chips, Reserve (or the "Already yours" pill) and the My Picks bookmark; beside it (below
//! it in a narrow window) a white sheet with the synopsis, subjects, details, branch holdings and
//! the related carousels.
//!
//! Loading follows iOS: the page saved on this computer shows at once (§5.13), then the record is
//! fetched, then a ref without formats re-finds them by search and a loan's or reservation's page
//! picks the patron's edition; each selected format's availability and copies load once; the
//! carousels load with the fresh record. The page is saved after each fresh result.

mod held;
mod logic;
mod reserve;
mod sheet;
mod view;

#[cfg(all(debug_assertions, feature = "evidence"))]
mod evidence;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use gpui_kit::component::WindowExt;
use gpui_kit::{AppContext, Context, Entity, Subscription, Task, Window};
use librespydus_core::app::account_holding;
use librespydus_core::{Availability, RecordDetail, RelatedList, SearchSort, SpydusError};

use crate::backend::LibraryBackend;
use crate::copy::Copy;
use crate::state::formatting::{non_empty, today};
use crate::state::picks::pick_irn_with_record;
use crate::state::{AppState, BookRef, ErrorKind, FormatRef, Loadable, NoticeTone, ReloadKey, UserFacingError};
use crate::ui::ConfirmDialog;

pub use self::held::{HeldStatus, HeldTone};
pub use self::logic::{AvailabilityLine, AvailabilityTone, FormatAvailability, HoldingRow, RelatedPanel};
pub use self::reserve::ReserveDialog;

use self::logic::{PanelState, SubjectChip};

/// The library's limit for one page's concurrent requests (docs/native-apps.md §2).
const MAX_CONCURRENT_AVAILABILITY: usize = 6;

/// Whether the formats came with the ref or are being looked up (formats live on search records
/// only, docs/native-apps.md §4).
#[derive(Debug, Clone, PartialEq)]
pub enum FormatLookup {
    Provided,
    /// Waiting for the record before searching.
    Pending,
    Searching,
    Found,
    /// No confident match: availability and Reserve are hidden.
    NotFound,
    Failed(UserFacingError),
}

/// The availability line under the byline.
#[derive(Debug, Clone, PartialEq)]
pub enum HeaderAvailability {
    Loading(&'static str),
    Line(AvailabilityLine),
    AvailabilityFailed,
    LookupFailed,
    /// No formats matched: "Check availability on the library’s website".
    CheckWebsite,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeldSearch {
    Idle,
    Running,
    Done,
}

pub struct DetailScreen {
    /// The ref as opened (its URL picks which record to load).
    opened: BookRef,
    /// What the panel draws: `opened`, filled in from the record (bookmark URL, cover, title).
    shown: BookRef,
    record: Loadable<RecordDetail>,
    formats: Vec<FormatRef>,
    selected: Option<usize>,
    lookup: FormatLookup,
    /// By `availability_url`, so switching back to a format doesn't fetch it again.
    availability: HashMap<String, Loadable<FormatAvailability>>,
    related: HashMap<RelatedPanel, Loadable<Vec<BookRef>>>,
    /// The selected format's branch holdings (`holdings(holdings_url)`), and the URL they came from.
    holdings: Loadable<Vec<HoldingRow>>,
    holdings_url: Option<String>,
    /// The related links whose panels were requested (a record upgrade brings new ones).
    related_requested_for: Option<librespydus_core::RecordRelated>,
    /// The page saved on this computer is on screen and its fresh record hasn't come back (§5.13).
    shows_saved_record: bool,
    /// A fresh record is loading behind the saved one.
    refreshing_record: bool,
    /// Carousels still showing saved (or an earlier record's) items until their fresh list arrives.
    stale_related: HashSet<RelatedPanel>,
    /// The newest list each carousel loaded (fresh, else saved), unfiltered: what a save stores.
    related_lists: HashMap<RelatedPanel, RelatedList>,
    /// A format was picked on screen: the patron's edition is no longer selected for them.
    format_picked_on_screen: bool,
    held_search: HeldSearch,
    /// The reload key the last load ran under (docs/desktop-app.md §5.3).
    load_key: Option<ReloadKey>,
    /// A renewal from the "On loan" pill is running (§5.12): a spinner in the pill.
    renewing: bool,
    /// Reserve clicked while signed out: reserve this format once signed in.
    pending_reserve: Option<FormatRef>,
    waiting_for_sign_in: bool,
    synopsis_expanded: bool,
    _observe: Subscription,
}

impl DetailScreen {
    pub fn new(book: BookRef, window: &mut Window, cx: &mut Context<Self>) -> Self {
        #[cfg(feature = "evidence")]
        let book = view::evidence_edition_level(book);
        let state = AppState::global(cx);
        let observe = cx.observe_in(&state, window, |this, _, window, cx| this.state_changed(window, cx));
        let formats = book.formats.clone();
        let mut screen = Self {
            opened: book.clone(),
            shown: book,
            record: Loadable::Idle,
            selected: logic::default_format_index(&formats),
            lookup: if formats.is_empty() { FormatLookup::Pending } else { FormatLookup::Provided },
            formats,
            availability: HashMap::new(),
            related: HashMap::new(),
            holdings: Loadable::Idle,
            holdings_url: None,
            related_requested_for: None,
            shows_saved_record: false,
            refreshing_record: false,
            stale_related: HashSet::new(),
            related_lists: HashMap::new(),
            format_picked_on_screen: false,
            held_search: HeldSearch::Idle,
            load_key: None,
            renewing: false,
            pending_reserve: None,
            waiting_for_sign_in: false,
            synopsis_expanded: false,
            _observe: observe,
        };
        // Before any request, so the first frame already shows it.
        screen.show_saved_page(&state.read(cx).backend());
        #[cfg(all(debug_assertions, feature = "evidence"))]
        let stall = evidence::stalls();
        #[cfg(not(all(debug_assertions, feature = "evidence")))]
        let stall = false;
        if !stall {
            cx.spawn(async move |this, cx| {
                let _ = this.update(cx, |this, cx| this.load(cx));
            })
            .detach();
        }
        #[cfg(all(debug_assertions, feature = "evidence"))]
        evidence::schedule(window, cx);
        screen
    }

    /// The ref the page was opened with.
    pub fn book(&self) -> &BookRef {
        &self.opened
    }

    /// The ref as the panel draws it (filled in from the record).
    pub fn shown_book(&self) -> &BookRef {
        &self.shown
    }

    pub fn record(&self) -> &Loadable<RecordDetail> {
        &self.record
    }

    pub fn formats(&self) -> &[FormatRef] {
        &self.formats
    }

    pub fn format_lookup(&self) -> &FormatLookup {
        &self.lookup
    }

    pub fn selected_format(&self) -> Option<&FormatRef> {
        self.selected.and_then(|index| self.formats.get(index))
    }

    pub fn is_renewing(&self) -> bool {
        self.renewing
    }

    // ---- Derived ----

    /// A Featured list's item (§5.10): one format without a code; no format chips.
    fn is_edition_level(&self) -> bool {
        crate::state::book_ref::is_edition_level(&self.formats)
    }

    /// The format Reserve reserves: the selected one, when it has a reserve URL.
    pub fn reservable_format(&self) -> Option<&FormatRef> {
        if !matches!(self.lookup, FormatLookup::Provided | FormatLookup::Found) {
            return None;
        }
        self.selected_format().filter(|format| non_empty(format.reserve_url.as_deref()).is_some())
    }

    pub fn header_availability(&self) -> HeaderAvailability {
        match &self.lookup {
            FormatLookup::Pending => {
                return if self.record.error().is_none() {
                    HeaderAvailability::Loading(Copy::Detail::findingFormats)
                } else {
                    HeaderAvailability::None
                };
            }
            FormatLookup::Searching => return HeaderAvailability::Loading(Copy::Detail::findingFormats),
            FormatLookup::NotFound => return HeaderAvailability::CheckWebsite,
            FormatLookup::Failed(_) => return HeaderAvailability::LookupFailed,
            FormatLookup::Provided | FormatLookup::Found => {}
        }
        let Some(format) = self.selected_format() else { return HeaderAvailability::None };
        let Some(url) = non_empty(format.availability_url.as_deref()) else {
            return HeaderAvailability::Line(AvailabilityLine::new(None, format));
        };
        match self.availability.get(&url) {
            None | Some(Loadable::Idle | Loadable::Loading) => {
                HeaderAvailability::Loading(Copy::Detail::checkingAvailability)
            }
            Some(Loadable::Loaded(loaded)) => {
                HeaderAvailability::Line(AvailabilityLine::new(Some(&loaded.summary), format))
            }
            Some(Loadable::Failed(_)) => HeaderAvailability::AvailabilityFailed,
        }
    }

    fn known_irn(&self, format: &FormatRef) -> Option<String> {
        let url = format.availability_url.as_deref()?;
        self.availability.get(url)?.value()?.bib_record_irn.clone()
    }

    /// The bib IRNs of the edition on screen, to find it in the account. While the patron's
    /// edition is being picked, the opened row's IRN stands, so the pill doesn't flicker to Reserve.
    fn holding_irns(&self) -> Vec<String> {
        let others: HashSet<String> = self
            .formats
            .iter()
            .enumerate()
            .filter(|(index, _)| Some(*index) != self.selected)
            .filter_map(|(_, format)| self.known_irn(format))
            .collect();
        let selected = if self.held_search == HeldSearch::Running {
            None
        } else {
            self.selected_format().and_then(|format| self.known_irn(format))
        };
        let record_irn = self.record.value().and_then(|record| record.bib_irn.clone());
        logic::holding_irns(selected.as_deref(), record_irn.as_deref(), self.opened.bib_irn(), &others)
    }

    /// The page can't tell yet which edition is on screen: the patron's is being picked, or the
    /// selected format's availability (without one, the record) hasn't come back.
    fn is_identifying_edition(&self) -> bool {
        if self.held_search == HeldSearch::Running {
            return true;
        }
        if let Some(url) = self.selected_format().and_then(|format| non_empty(format.availability_url.as_deref())) {
            return !matches!(self.availability.get(&url), Some(Loadable::Loaded(_) | Loadable::Failed(_)));
        }
        !matches!(self.record, Loadable::Loaded(_) | Loadable::Failed(_))
    }

    /// The patron's loan or reservation of the edition on screen, from the saved account (`None`
    /// when signed out or before the account has loaded): it replaces Reserve.
    pub fn held(&self, cx: &gpui_kit::App) -> Option<HeldStatus> {
        let state = AppState::global(cx).read(cx);
        if !state.is_signed_in() {
            return None;
        }
        let snapshot = state.account().snapshot()?;
        let irns = self.holding_irns();
        if irns.is_empty() {
            return None;
        }
        account_holding(snapshot.clone(), irns).map(|holding| HeldStatus::new(&holding, today()))
    }

    /// Signed in with loans or reservations, and the edition on screen isn't identified yet: Reserve
    /// keeps its place but doesn't show, rather than flash before "On loan" replaces it.
    fn awaiting_holding(&self, cx: &gpui_kit::App) -> bool {
        let state = AppState::global(cx).read(cx);
        let Some(snapshot) = state.account().snapshot().filter(|_| state.is_signed_in()) else { return false };
        let has_items = !(snapshot.loans.items.is_empty()
            && snapshot.reservations.ready.items.is_empty()
            && snapshot.reservations.pending.items.is_empty());
        has_items && self.is_identifying_edition()
    }

    /// What My Picks keys this book on when signed in (§5.11): the opened card's own IRN, else the
    /// loaded record's; `None` until the record has loaded (the bookmark waits).
    fn pick_irn(&self) -> Option<String> {
        pick_irn_with_record(&self.opened, self.record.value())
    }

    /// What a pick on this computer stores: the stable bookmark URL once known, and the formats.
    fn favourite_ref(&self) -> BookRef {
        let mut saved = self.shown.clone();
        saved.formats = self.formats.clone();
        saved
    }

    /// Whether the book is in My Picks: the account's when signed in, this computer's when not.
    pub fn is_picked(&self, cx: &gpui_kit::App) -> bool {
        let state = AppState::global(cx).read(cx);
        if state.picks().is_active() {
            self.pick_irn().is_some_and(|irn| state.picks().contains(&irn))
        } else {
            state.local_picks().contains(&self.shown.id)
        }
    }

    fn subjects(&self) -> Vec<SubjectChip> {
        self.record.value().map(|record| logic::subjects(&record.details, logic::SUBJECT_LIMIT)).unwrap_or_default()
    }

    fn panel_state(&self, panel: RelatedPanel) -> PanelState {
        match self.related.get(&panel) {
            None | Some(Loadable::Idle | Loadable::Loading) => PanelState::Waiting,
            Some(Loadable::Loaded(items)) => PanelState::Items(items.len()),
            Some(Loadable::Failed(_)) => PanelState::Failed,
        }
    }

    pub fn visible_panels(&self) -> Vec<RelatedPanel> {
        let Some(record) = self.record.value() else { return Vec::new() };
        let states = RelatedPanel::ALL.into_iter().map(|panel| (panel, self.panel_state(panel))).collect();
        logic::visible_panels(&record.related, &states)
    }

    fn has_related_panels(&self) -> bool {
        self.record
            .value()
            .is_some_and(|record| RelatedPanel::ALL.into_iter().any(|panel| panel.url(&record.related).is_some()))
    }

    /// Where an edition-level item's copies are listed, once its availability has come back
    /// (`None` before, and for any other book: iOS `copiesSource`).
    fn holdings_source(&self) -> Option<String> {
        let format = self.selected_format().filter(|_| logic::shows_copies(&self.formats))?;
        let url = non_empty(format.availability_url.as_deref())?;
        self.availability.get(&url)?.value()?.holdings_url.clone()
    }

    // ---- Actions ----

    /// A format chip was clicked.
    pub fn select_format(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.formats.len() {
            return;
        }
        self.format_picked_on_screen = true;
        self.selected = Some(index);
        self.ensure_availability(cx);
        cx.notify();
    }

    /// The bookmark: saves or removes the book in My Picks, with a toast either way (§5.11).
    pub fn toggle_pick(&mut self, cx: &mut Context<Self>) {
        let picked = self.is_picked(cx);
        let state = AppState::global(cx);
        if state.read(cx).picks().is_active() {
            let Some(irn) = self.pick_irn() else { return };
            if state.read(cx).picks().is_changing(&irn) {
                return;
            }
            state.update(cx, |state, cx| state.set_picked_irn(!picked, irn, cx)).detach();
        } else {
            let book = self.favourite_ref();
            state.update(cx, |state, cx| state.set_picked(book, !picked, cx)).detach();
        }
        cx.notify();
    }

    /// Reserve: the dialog when signed in; signed out, sign in first, then the dialog.
    pub fn reserve(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(format) = self.reservable_format().cloned() else { return };
        if AppState::global(cx).read(cx).is_signed_in() {
            self.open_reserve(format, window, cx);
        } else {
            self.wait_for_sign_in(format, window, cx);
            AppState::global(cx).update(cx, |state, cx| state.request_sign_in(cx));
        }
    }

    fn open_reserve(
        &mut self,
        format: FormatRef,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<ReserveDialog> {
        reserve::open(format, self.shown.clone(), cx.entity().downgrade(), window, cx)
    }

    /// Remembers `format` and opens the Reserve dialog once the reader has signed in and the
    /// sign-in dialog has closed; forgets it when sign-in is closed without signing in.
    pub(crate) fn wait_for_sign_in(&mut self, format: FormatRef, window: &mut Window, cx: &mut Context<Self>) {
        self.pending_reserve = Some(format);
        if self.waiting_for_sign_in {
            return;
        }
        self.waiting_for_sign_in = true;
        cx.spawn_in(window, async move |this, cx| {
            let mut dialog_seen = false;
            // At most about two minutes.
            for _ in 0..400 {
                cx.background_executor().timer(Duration::from_millis(300)).await;
                let done = this.update_in(cx, |this, window, cx| {
                    let Some(format) = this.pending_reserve.clone() else { return true };
                    let dialog_open = window.has_active_dialog(cx);
                    let state = AppState::global(cx).read(cx);
                    dialog_seen |= dialog_open;
                    if !state.is_signed_in() {
                        // Sign-in was closed without signing in: forget the reservation.
                        if dialog_seen && !dialog_open {
                            this.pending_reserve = None;
                            return true;
                        }
                        return false;
                    }
                    // Signed in: wait for the sign-in dialog to go and the account to say whether
                    // the patron has this edition already.
                    if dialog_open || (state.account().snapshot().is_none() && state.account().is_refreshing()) {
                        return false;
                    }
                    this.pending_reserve = None;
                    if this.held(cx).is_none() {
                        this.open_reserve(format, window, cx);
                    }
                    true
                });
                if done.unwrap_or(true) {
                    break;
                }
            }
            let _ = this.update(cx, |this, _| this.waiting_for_sign_in = false);
        })
        .detach();
    }

    /// The held pill: a renewable loan asks to renew it (§5.12); any other opens Your Books.
    pub fn tap_held(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(held) = self.held(cx) else { return };
        match held.renewable_loan {
            Some(loan) => {
                if self.renewing {
                    return;
                }
                let screen = cx.entity().downgrade();
                let item = loan.clone();
                ConfirmDialog::new(
                    Copy::YourBooks::renewTitle,
                    Copy::YourBooks::renew,
                    crate::ui::on_click(move |_, cx| {
                        let _ = screen.update(cx, |screen, cx| screen.renew(item.clone(), cx));
                    }),
                )
                .message(held::renew_message(&loan, today()))
                .cancel_label(Copy::Common::notNow)
                .open(window, cx);
            }
            None => AppState::global(cx).update(cx, |state, cx| state.open_your_books(held.segment, cx)),
        }
    }

    /// Renews `loan` after the reader confirmed, as Your Books does: "Renewing…", then the result
    /// as a toast; the pill follows the account to the new due date.
    pub fn renew(&mut self, loan: librespydus_core::PatronItem, cx: &mut Context<Self>) {
        if self.renewing {
            return;
        }
        self.renewing = true;
        cx.notify();
        let state = AppState::global(cx);
        state.update(cx, |state, cx| state.show_notice(Copy::YourBooks::renewing, NoticeTone::Progress, cx));
        let task = state.update(cx, |state, cx| state.renew(&loan, cx));
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.renewing = false;
                let state = AppState::global(cx);
                match result {
                    Ok(result) => {
                        let tone = if result.ok { NoticeTone::Success } else { NoticeTone::Error };
                        let text = held::renew_result_text(&result, today());
                        state.update(cx, |state, cx| state.show_notice(&text, tone, cx));
                    }
                    // A lost session signs out and asks to sign in again: that says enough.
                    Err(error) if error.kind == ErrorKind::NotLoggedIn => {}
                    Err(_) => state
                        .update(cx, |state, cx| state.show_notice(Copy::YourBooks::renewFailed, NoticeTone::Error, cx)),
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub fn toggle_synopsis(&mut self, cx: &mut Context<Self>) {
        self.synopsis_expanded = !self.synopsis_expanded;
        cx.notify();
    }

    // ---- Loading ----

    fn state_changed(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let key = AppState::global(cx).read(cx).reload_key();
        if self.load_key.as_ref().is_some_and(|loaded| *loaded != key) {
            // Verification, sign-in or sign-out: load what's missing or failed again.
            self.load(cx);
            self.retry_failed_related(cx);
            self.ensure_availability(cx);
        }
        cx.notify();
    }

    /// Loads what's missing: the record, then (for a ref without formats) its formats, then for a
    /// loan or reservation the format that is the patron's edition. Safe to call again after a
    /// failure. The page saved on this computer shows first; its record is then fetched again
    /// while the formats are found from it.
    pub fn load(&mut self, cx: &mut Context<Self>) {
        let state = AppState::global(cx).read(cx);
        self.load_key = Some(state.reload_key());
        let backend = state.backend();
        if self.record.is_loading() || self.refreshing_record {
            return;
        }
        self.ensure_availability(cx);
        if self.shows_saved_record {
            self.refreshing_record = true;
            let record = self.load_record(backend.clone(), cx);
            let formats = self.load_formats(backend, cx);
            cx.spawn(async move |this, cx| {
                futures::join!(record, formats);
                let _ = this.update(cx, |this, cx| {
                    this.refreshing_record = false;
                    cx.notify();
                });
            })
            .detach();
            return;
        }
        let record = if self.record.value().is_none() { Some(self.load_record(backend.clone(), cx)) } else { None };
        cx.spawn(async move |this, cx| {
            if let Some(record) = record {
                record.await;
            }
            if let Ok(formats) = this.update(cx, |this, cx| this.load_formats(backend, cx)) {
                formats.await;
            }
        })
        .detach();
    }

    /// With the record on screen: re-finds a ref's missing formats, then picks the patron's edition.
    fn load_formats(&mut self, backend: Arc<dyn LibraryBackend>, cx: &mut Context<Self>) -> Task<()> {
        let refind = matches!(self.lookup, FormatLookup::Pending | FormatLookup::Failed(_));
        cx.spawn(async move |this, cx| {
            if refind && let Ok(task) = this.update(cx, |this, cx| this.refind_formats(backend.clone(), cx)) {
                task.await;
            }
            if let Ok(task) = this.update(cx, |this, cx| this.select_held_format(backend, cx)) {
                task.await;
            }
        })
    }

    fn load_record(&mut self, backend: Arc<dyn LibraryBackend>, cx: &mut Context<Self>) -> Task<()> {
        let urls = logic::candidate_urls(self.opened.url.as_deref(), self.opened.bib_irn(), self.selected_format());
        if urls.is_empty() {
            if !self.shows_saved_record {
                self.record =
                    Loadable::Failed(UserFacingError::new(ErrorKind::Unexpected, Copy::Detail::noRecordLink, None));
                cx.notify();
            }
            return Task::ready(());
        }
        if !self.shows_saved_record {
            self.record = Loadable::Loading;
            cx.notify();
        }
        cx.spawn(async move |this, cx| {
            let mut failure: Option<SpydusError> = None;
            for url in urls {
                match backend.record(url.clone()).await {
                    Ok(detail) if !logic::is_empty_record(&detail) => {
                        let _ = this.update(cx, |this, cx| this.apply_fresh(detail, cx));
                        return;
                    }
                    // The OPAC's answer to a link it won't show (a loan's set without the session).
                    // copy-exempt: technical detail for the error's Details disclosure
                    Ok(_) => failure = Some(SpydusError::Upstream { message: format!("empty record from {url}") }),
                    Err(error) => {
                        // A challenge or a lost connection fails the other URL the same way.
                        let stop =
                            matches!(error, SpydusError::HumanVerificationRequired | SpydusError::Network { .. });
                        failure = Some(error);
                        if stop {
                            break;
                        }
                    }
                }
            }
            let _ = this.update(cx, |this, cx| {
                let shown = failure.map(|failure| report(&failure, cx));
                // The saved page stays on screen, without an error.
                if this.shows_saved_record {
                    return;
                }
                this.record = shown.map_or(Loadable::Idle, Loadable::Failed);
                cx.notify();
            });
        })
    }

    /// Searches for the record's formats (title + author) and keeps them only on a confident match.
    fn refind_formats(&mut self, backend: Arc<dyn LibraryBackend>, cx: &mut Context<Self>) -> Task<()> {
        if !self.formats.is_empty() {
            return Task::ready(());
        }
        let Some(detail) = self.record.value().cloned() else { return Task::ready(()) };
        let Some(query) = logic::format_search_query(&self.shown, Some(&detail)) else {
            self.lookup = FormatLookup::NotFound;
            cx.notify();
            return Task::ready(());
        };
        self.lookup = FormatLookup::Searching;
        cx.notify();
        let search = backend.search_sorted(query, SearchSort::Relevance);
        cx.spawn(async move |this, cx| {
            let result = search.await;
            let found = this.update(cx, |this, cx| {
                let found = match result {
                    Ok(results) => match logic::matching_record(&this.shown, Some(&detail), &results.records) {
                        Some(hit) => {
                            this.formats = hit.formats.iter().map(FormatRef::from).collect();
                            this.selected = logic::default_format_index(&this.formats);
                            if this.shown.cover_url.is_none() {
                                this.shown.cover_url = hit.cover_urls.first().cloned();
                            }
                            this.lookup = FormatLookup::Found;
                            this.ensure_availability(cx);
                            true
                        }
                        None => {
                            this.lookup = FormatLookup::NotFound;
                            false
                        }
                    },
                    Err(error) => {
                        this.lookup = FormatLookup::Failed(report(&error, cx));
                        false
                    }
                };
                cx.notify();
                found
            });
            if let Ok(true) = found
                && let Ok(task) = this.update(cx, |this, cx| this.upgrade_work_level_record(backend, cx))
            {
                task.await;
            }
        })
    }

    /// Looks the formats up again (the availability line's Retry after a failed lookup).
    pub fn retry_lookup(&mut self, cx: &mut Context<Self>) {
        let backend = AppState::global(cx).read(cx).backend();
        self.refind_formats(backend, cx).detach();
    }

    /// A work-level record (opened by a work URL) has no subjects, imprint or similar titles; once
    /// formats are known, the selected one's `availability_url` loads the full bib record. Kept as
    /// is if that fails.
    fn upgrade_work_level_record(&mut self, backend: Arc<dyn LibraryBackend>, cx: &mut Context<Self>) -> Task<()> {
        let Some(current) = self.record.value() else { return Task::ready(()) };
        if !(logic::is_work_level(current.bookmark_url.as_deref()) || logic::is_work_level(self.opened.url.as_deref()))
        {
            return Task::ready(());
        }
        let Some(url) = self.selected_format().and_then(|format| non_empty(format.availability_url.as_deref())) else {
            return Task::ready(());
        };
        let fetch = backend.record(url);
        cx.spawn(async move |this, cx| {
            if let Ok(detail) = fetch.await
                && !logic::is_empty_record(&detail)
            {
                let _ = this.update(cx, |this, cx| this.apply_fresh(detail, cx));
            }
        })
    }

    /// The selected format's availability (once per URL; again after a failure), then its copies.
    pub fn ensure_availability(&mut self, cx: &mut Context<Self>) {
        let url = self.selected_format().and_then(|format| non_empty(format.availability_url.as_deref()));
        let Some(url) = url.filter(|url| self.needs_availability(url)) else {
            self.ensure_holdings(cx);
            return;
        };
        self.availability.insert(url.clone(), Loadable::Loading);
        cx.notify();
        let fetch = AppState::global(cx).read(cx).backend().availability(url.clone());
        cx.spawn(async move |this, cx| {
            let result = fetch.await;
            let _ = this.update(cx, |this, cx| {
                this.set_availability(result, url, cx);
                this.ensure_holdings(cx);
                cx.notify();
            });
        })
        .detach();
    }

    /// The selected format's copies at each branch, once per URL; again after a failure.
    pub fn ensure_holdings(&mut self, cx: &mut Context<Self>) {
        let Some(url) = self.holdings_source() else { return };
        if self.holdings_url.as_deref() == Some(url.as_str())
            && matches!(self.holdings, Loadable::Loaded(_) | Loadable::Loading)
        {
            return;
        }
        self.holdings_url = Some(url.clone());
        self.holdings = Loadable::Loading;
        cx.notify();
        let fetch = AppState::global(cx).read(cx).backend().holdings(url.clone());
        cx.spawn(async move |this, cx| {
            let result = fetch.await;
            let _ = this.update(cx, |this, cx| {
                if this.holdings_url.as_deref() != Some(url.as_str()) {
                    return;
                }
                this.holdings = match result {
                    Ok(holdings) => Loadable::Loaded(holdings.iter().map(HoldingRow::from).collect()),
                    Err(error) => Loadable::Failed(report(&error, cx)),
                };
                cx.notify();
            });
        })
        .detach();
    }

    /// Retry on the copies list.
    pub fn retry_holdings(&mut self, cx: &mut Context<Self>) {
        self.holdings = Loadable::Idle;
        self.ensure_holdings(cx);
    }

    /// Opened from Your Books or Home's loans (a `bib:` ref): once the formats are known, selects
    /// the one that is the patron's edition (its `bib_record_irn` is the opened IRN), from every
    /// format's availability, at most 6 at a time and each fetched once. No match keeps the default;
    /// so does a format picked on screen meanwhile.
    fn select_held_format(&mut self, backend: Arc<dyn LibraryBackend>, cx: &mut Context<Self>) -> Task<()> {
        let Some(irn) = self.opened.bib_irn().map(str::to_owned) else { return Task::ready(()) };
        if self.held_search != HeldSearch::Idle
            || self.formats.len() < 2
            || !matches!(self.lookup, FormatLookup::Provided | FormatLookup::Found)
        {
            return Task::ready(());
        }
        self.held_search = HeldSearch::Running;
        let urls: Vec<String> =
            self.formats.iter().filter_map(|format| non_empty(format.availability_url.as_deref())).collect();
        let missing: Vec<String> = urls.iter().filter(|url| self.needs_availability(url)).cloned().collect();
        for url in &missing {
            self.availability.insert(url.clone(), Loadable::Loading);
        }
        cx.notify();
        cx.spawn(async move |this, cx| {
            let mut fetches = futures::stream::iter(missing)
                .map(move |url| {
                    let fetch = backend.availability(url.clone());
                    async move { (url, fetch.await) }
                })
                .buffer_unordered(MAX_CONCURRENT_AVAILABILITY);
            while let Some((url, result)) = fetches.next().await {
                if this.update(cx, |this, cx| this.set_availability(result, url, cx)).is_err() {
                    return;
                }
            }
            let _ = this.update(cx, |this, cx| {
                let found =
                    this.formats.iter().position(|format| this.known_irn(format).as_deref() == Some(irn.as_str()));
                if let Some(index) = found
                    && !this.format_picked_on_screen
                {
                    this.selected = Some(index);
                }
                // A format that failed may be the one: look again on the next load.
                let settled = urls
                    .iter()
                    .all(|url| matches!(this.availability.get(url), Some(Loadable::Loaded(_) | Loadable::Loading)));
                this.held_search = if found.is_some() || settled { HeldSearch::Done } else { HeldSearch::Idle };
                this.ensure_availability(cx);
                cx.notify();
            });
        })
    }

    /// Starts loading the related panels once per record. The saved record's links are from an
    /// older search set: those wait for the fresh record.
    fn start_related(&mut self, cx: &mut Context<Self>) {
        if self.shows_saved_record {
            return;
        }
        let Some(links) = self.record.value().map(|record| record.related.clone()) else { return };
        if self.related_requested_for.as_ref() == Some(&links) {
            return;
        }
        self.related_requested_for = Some(links);
        self.load_related(cx);
    }

    /// After a reload: load panels that failed again (if they were requested).
    fn retry_failed_related(&mut self, cx: &mut Context<Self>) {
        if self.related_requested_for.is_some() && self.related.values().any(|state| state.error().is_some()) {
            self.load_related(cx);
        }
    }

    /// Every related panel the record links to, concurrently (at most four GETs). A failed or
    /// empty panel is hidden, never reported; one showing saved items keeps them when it fails.
    /// The page is saved again once fresh lists came back.
    fn load_related(&mut self, cx: &mut Context<Self>) {
        let Some(detail) = self.record.value().cloned() else { return };
        let pending: Vec<(RelatedPanel, String)> = RelatedPanel::ALL
            .into_iter()
            .filter_map(|panel| {
                let url = panel.url(&detail.related)?;
                match self.related.get(&panel) {
                    Some(Loadable::Loading) => None,
                    Some(Loadable::Loaded(_)) if !self.stale_related.contains(&panel) => None,
                    _ => Some((panel, url)),
                }
            })
            .collect();
        if pending.is_empty() {
            return;
        }
        for (panel, _) in &pending {
            if !self.stale_related.contains(panel) {
                self.related.insert(*panel, Loadable::Loading);
            }
        }
        cx.notify();
        let backend = AppState::global(cx).read(cx).backend();
        let mut fetches: futures::stream::FuturesUnordered<_> = pending
            .into_iter()
            .map(|(panel, url)| {
                let fetch = backend.related(url);
                async move { (panel, fetch.await) }
            })
            .collect();
        let links = detail.related.clone();
        let brn = detail.brn.clone();
        cx.spawn(async move |this, cx| {
            let mut loaded_any = false;
            while let Some((panel, result)) = fetches.next().await {
                let applied = this.update(cx, |this, cx| {
                    if this.record.value().map(|record| &record.related) != Some(&links) {
                        return false;
                    }
                    match result {
                        Ok(list) => {
                            this.stale_related.remove(&panel);
                            this.related.insert(panel, Loadable::Loaded(RelatedPanel::items(&list, brn.as_deref())));
                            this.related_lists.insert(panel, list);
                            loaded_any = true;
                        }
                        Err(error) => {
                            if !this.stale_related.contains(&panel) {
                                this.related.insert(panel, Loadable::Failed(UserFacingError::from_core(&error, "")));
                            }
                        }
                    }
                    cx.notify();
                    true
                });
                if !matches!(applied, Ok(true)) {
                    return;
                }
            }
            if loaded_any {
                let _ = this.update(cx, |this, cx| this.save_page(cx));
            }
        })
        .detach();
    }

    /// Shows the page saved on this computer for this book (§5.13), before the first request: its
    /// record, and its lists as the carousels' items until fresh ones arrive.
    fn show_saved_page(&mut self, backend: &Arc<dyn LibraryBackend>) {
        if self.record.value().is_some() {
            return;
        }
        let Some(key) = self.opened.saved_page_key() else { return };
        let Some(saved) = backend.saved_book(key).filter(|saved| !logic::is_empty_record(&saved.record)) else {
            return;
        };
        self.apply(saved.record.clone());
        self.shows_saved_record = true;
        for panel in RelatedPanel::ALL {
            let Some(list) = panel.list(&saved) else { continue };
            self.related.insert(panel, Loadable::Loaded(RelatedPanel::items(list, saved.record.brn.as_deref())));
            self.related_lists.insert(panel, list.clone());
            self.stale_related.insert(panel);
        }
    }

    /// A record just fetched: shown and saved. A work-level record (no subjects or related titles)
    /// never replaces a bib record on screen.
    fn apply_fresh(&mut self, detail: RecordDetail, cx: &mut Context<Self>) {
        self.shows_saved_record = false;
        let keeps_bib = logic::is_work_level(detail.bookmark_url.as_deref())
            && self.record.value().is_some_and(|current| !logic::is_work_level(current.bookmark_url.as_deref()));
        if !keeps_bib {
            self.apply(detail);
        }
        self.start_related(cx);
        if !keeps_bib {
            self.save_page(cx);
        }
        cx.notify();
    }

    /// Saves the page on this computer (§5.13) after something fresh came back: the record on
    /// screen and, for each carousel it links to, the newest list that loaded.
    fn save_page(&mut self, cx: &mut Context<Self>) {
        let Some(key) = self.opened.saved_page_key() else { return };
        let Some(detail) = self.record.value() else { return };
        let lists: HashMap<RelatedPanel, RelatedList> = self
            .related_lists
            .iter()
            .filter(|(panel, _)| panel.url(&detail.related).is_some())
            .map(|(panel, list)| (*panel, list.clone()))
            .collect();
        let save = AppState::global(cx)
            .read(cx)
            .backend()
            .save_book(key.to_owned(), logic::saved_book(detail.clone(), &lists));
        cx.background_spawn(save).detach();
    }

    /// Not loaded or loading yet (each format's availability is fetched once; again after a failure).
    fn needs_availability(&self, url: &str) -> bool {
        !matches!(self.availability.get(url), Some(Loadable::Loaded(_) | Loadable::Loading))
    }

    fn set_availability(&mut self, result: Result<Availability, SpydusError>, url: String, cx: &mut Context<Self>) {
        let state = match result {
            Ok(loaded) => Loadable::Loaded(FormatAvailability::from(loaded)),
            Err(error) => Loadable::Failed(report(&error, cx)),
        };
        self.availability.insert(url, state);
        cx.notify();
    }

    fn apply(&mut self, detail: RecordDetail) {
        if let Some(current) = self.record.value()
            && current.related != detail.related
        {
            // New links (a fresh record's carry a new search set): carousels with items keep them
            // until their new lists arrive; the others load again.
            self.stale_related =
                self.related.iter().filter(|(_, state)| state.value().is_some()).map(|(panel, _)| *panel).collect();
            let stale = self.stale_related.clone();
            self.related.retain(|panel, _| stale.contains(panel));
        }
        let mut updated = self.shown.updated_with(&detail);
        if (updated.title == Copy::Common::untitled || updated.title.trim().is_empty())
            && let Some(title) = non_empty(detail.title.as_deref())
        {
            updated.title = title;
        }
        if updated.author.is_none() {
            updated.author = logic::author_in(Some(&detail));
        }
        self.shown = updated;
        self.record = Loadable::Loaded(detail);
    }
}

/// Routes a failed call through the app (sign-in, verification) and returns what to show.
fn report(error: &SpydusError, cx: &mut gpui_kit::App) -> UserFacingError {
    AppState::global(cx).update(cx, |state, cx| state.report(error, cx))
}
