//! Debug-only alert hooks (docs/desktop-app.md §11.1 "Debug toggles"; native-apps.md §5.6):
//!
//! - `LS_ALERTS_CHECK_NOW=1`: the morning check runs once the launch refresh has settled, instead
//!   of waiting for 07:45 (iOS `-LSMorningCheckNow`).
//! - `LS_DEMO_NEW_READY=1` (with `--demo`): the demo account gains one ready reservation on its
//!   next refresh, so a "Ready to collect" alert can be shown (iOS `-LSDemoNewReady`): "Hollow
//!   bones", pending at Riverton, is listed as Canning lists a ready one. Done by wrapping the
//!   demo backend, so `backend/demo.rs` stays as it is.
//!
//! Either one lets demo data raise alerts (never otherwise).

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use chrono::Days;
use futures::FutureExt;
use futures::future::BoxFuture;
use gpui_kit::App;
use librespydus_core::barcode::BarcodeSymbology;
use librespydus_core::{
    AccountLinks, AccountSnapshot, Availability, EditOptions, EditOverrides, FeaturedCollection, FeaturedLink, Holding,
    PatronList, RecordDetail, RelatedList, RenewResult, Reservations, ReserveOptions, ReserveResult, SavedBook,
    SearchResults, SearchSort,
};

use crate::backend::{BackendFuture, LibraryBackend};
use crate::state::AppState;
use crate::state::formatting::{due_date, today};

/// The pending demo reservation that becomes ready: "Hollow bones", for pickup at Riverton.
pub const NEW_READY_IRN: &str = "19000003";

fn flag(name: &str) -> bool {
    std::env::var(name).is_ok_and(|value| value == "1" || value.eq_ignore_ascii_case("yes"))
}

pub fn check_now_requested() -> bool {
    flag("LS_ALERTS_CHECK_NOW")
}

pub fn new_ready_requested() -> bool {
    flag("LS_DEMO_NEW_READY")
}

pub fn allows_demo() -> bool {
    check_now_requested() || new_ready_requested()
}

/// At start: installs the new-ready demo backend and runs the morning check, as asked. True when
/// the morning check takes the place of the start check.
pub fn start(cx: &mut App) -> bool {
    if new_ready_requested() {
        install_new_ready(cx);
    }
    if !check_now_requested() {
        return false;
    }
    // After the launch refresh (a check of its own), so the morning check's fetch is a new one.
    let state = AppState::global(cx);
    let launch = state.update(cx, |state, cx| state.refresh_account(false, cx));
    cx.spawn(async move |cx| {
        launch.await;
        log::info!("alerts: LS_ALERTS_CHECK_NOW");
        cx.update(super::run_morning_check);
    })
    .detach();
    true
}

/// Wraps the demo backend in `NewReadyBackend` for the current library.
pub fn install_new_ready(cx: &mut App) {
    AppState::global(cx).update(cx, |state, _| {
        if !state.is_demo || state.library.is_none() {
            return;
        }
        // The launch refresh hasn't asked for its snapshot yet (it does on its first poll), so it
        // goes through this backend and is the first one served; a snapshot already here counts.
        let served = u32::from(state.account.snapshot().is_some());
        let backend: Arc<dyn LibraryBackend> = Arc::new(NewReadyBackend::new(state.backend.clone(), served));
        let origin = backend.origin();
        state.backend = backend.clone();
        let session = state.session.clone();
        state.account.use_account(Some(&origin), session.as_deref(), backend.clone(), true);
        state.picks.use_account(Some(&origin), session.as_deref(), backend);
        log::info!("alerts: LS_DEMO_NEW_READY (a reservation is ready from the next refresh)");
    });
}

/// The demo backend, with "Hollow bones" ready to collect from the second account snapshot on.
pub struct NewReadyBackend {
    inner: Arc<dyn LibraryBackend>,
    served: AtomicU32,
}

impl NewReadyBackend {
    /// `served`: snapshots the plain backend has already given.
    pub fn new(inner: Arc<dyn LibraryBackend>, served: u32) -> Self {
        Self { inner, served: AtomicU32::new(served) }
    }
}

/// "Hollow bones" moves from pending to ready, listed the way Canning lists one: "Please pickup
/// from: RIVERTON LIBRARY by <a week from today>".
pub fn make_new_ready(snapshot: &mut AccountSnapshot) {
    let pending = &mut snapshot.reservations.pending;
    let Some(index) = pending.items.iter().position(|item| item.irn.as_deref() == Some(NEW_READY_IRN)) else {
        return;
    };
    let mut item = pending.items.remove(index);
    pending.total = Some(pending.items.len() as i64);
    let collect_by = today().checked_add_days(Days::new(7)).unwrap_or_else(today);
    item.fields.clear();
    let status = format!("Please pickup from: RIVERTON LIBRARY by {}", due_date::format(collect_by)); // copy-exempt: debug demo data
    item.fields.insert("Status".into(), status); // copy-exempt: a field name (a key)
    let ready = &mut snapshot.reservations.ready;
    ready.items.push(item);
    ready.total = Some(ready.items.len() as i64);
}

impl LibraryBackend for NewReadyBackend {
    fn origin(&self) -> String {
        self.inner.origin()
    }

    fn is_demo(&self) -> bool {
        self.inner.is_demo()
    }

    fn set_verification_cookie(&self, cookie: Option<String>) {
        self.inner.set_verification_cookie(cookie);
    }

    fn account_snapshot(&self, session: String) -> BackendFuture<AccountSnapshot> {
        let gains = self.served.fetch_add(1, Ordering::SeqCst) >= 1;
        let fetch = self.inner.account_snapshot(session);
        async move {
            let mut snapshot = fetch.await?;
            if gains {
                make_new_ready(&mut snapshot);
            }
            Ok(snapshot)
        }
        .boxed()
    }

    // ---- Everything else as the demo backend answers ----

    fn search(&self, query: String) -> BackendFuture<SearchResults> {
        self.inner.search(query)
    }
    fn search_sorted(&self, query: String, sort: SearchSort) -> BackendFuture<SearchResults> {
        self.inner.search_sorted(query, sort)
    }
    fn page(&self, url: String) -> BackendFuture<SearchResults> {
        self.inner.page(url)
    }
    fn availability(&self, url: String) -> BackendFuture<Availability> {
        self.inner.availability(url)
    }
    fn record(&self, url: String) -> BackendFuture<RecordDetail> {
        self.inner.record(url)
    }
    fn related(&self, url: String) -> BackendFuture<RelatedList> {
        self.inner.related(url)
    }
    fn card_barcode_symbology(&self) -> BoxFuture<'static, BarcodeSymbology> {
        self.inner.card_barcode_symbology()
    }
    fn featured(&self) -> BackendFuture<Vec<FeaturedLink>> {
        self.inner.featured()
    }
    fn collection(&self, url: String) -> BackendFuture<FeaturedCollection> {
        self.inner.collection(url)
    }
    fn holdings(&self, url: String) -> BackendFuture<Vec<Holding>> {
        self.inner.holdings(url)
    }
    fn saved_book(&self, key: &str) -> Option<SavedBook> {
        self.inner.saved_book(key)
    }
    fn save_book(&self, key: String, book: SavedBook) -> BoxFuture<'static, ()> {
        self.inner.save_book(key, book)
    }
    fn saved_shelf(&self, key: &str) -> Option<SearchResults> {
        self.inner.saved_shelf(key)
    }
    fn save_shelf(&self, key: String, results: SearchResults) -> BoxFuture<'static, ()> {
        self.inner.save_shelf(key, results)
    }
    fn saved_featured(&self) -> Option<Vec<FeaturedLink>> {
        self.inner.saved_featured()
    }
    fn save_featured(&self, links: Vec<FeaturedLink>) -> BoxFuture<'static, ()> {
        self.inner.save_featured(links)
    }
    fn login(&self, username: String, password: String) -> BackendFuture<String> {
        self.inner.login(username, password)
    }
    fn account(&self, session: String) -> BackendFuture<AccountLinks> {
        self.inner.account(session)
    }
    fn loans(&self, session: String) -> BackendFuture<PatronList> {
        self.inner.loans(session)
    }
    fn reservations(&self, session: String) -> BackendFuture<Reservations> {
        self.inner.reservations(session)
    }
    fn reserve_options(&self, url: String, session: String) -> BackendFuture<ReserveOptions> {
        self.inner.reserve_options(url, session)
    }
    fn place_reservation(
        &self,
        url: String,
        session: String,
        xdays: Option<String>,
        pickup: Option<String>,
    ) -> BackendFuture<ReserveResult> {
        self.inner.place_reservation(url, session, xdays, pickup)
    }
    fn cancel_reservation(&self, url: String, session: String) -> BackendFuture<ReserveResult> {
        self.inner.cancel_reservation(url, session)
    }
    fn edit_options(&self, url: String, session: String) -> BackendFuture<EditOptions> {
        self.inner.edit_options(url, session)
    }
    fn edit_reservation(&self, url: String, session: String, overrides: EditOverrides) -> BackendFuture<ReserveResult> {
        self.inner.edit_reservation(url, session, overrides)
    }
    fn renew_loan(&self, url: String, session: String) -> BackendFuture<RenewResult> {
        self.inner.renew_loan(url, session)
    }
    fn picks(&self, session: String, url: Option<String>) -> BackendFuture<SearchResults> {
        self.inner.picks(session, url)
    }
    fn save_pick(&self, irn: String, session: String) -> BackendFuture<i64> {
        self.inner.save_pick(irn, session)
    }
    fn remove_pick(&self, irn: String, session: String) -> BackendFuture<i64> {
        self.inner.remove_pick(irn, session)
    }
}
