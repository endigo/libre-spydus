//! Headless UI tests of the Edit reservation dialog's two dates (docs/native-apps.md §6 "Your
//! Books": delay until, not required after): pick a day in each calendar, switch a date off, and
//! check what `edit_reservation` receives. A demo account with no delays, behind a backend that
//! records every edit.

mod support;

use std::sync::{Arc, Mutex};

use chrono::{Datelike, Duration, Months, NaiveDate};
use futures::future::BoxFuture;
use gpui_kit::base::Root;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{AppContext, Entity, TestAppContext, Window, WindowHandle};
use librespydus_core::barcode::BarcodeSymbology;
use librespydus_core::{
    AccountLinks, AccountSnapshot, Availability, EditOptions, EditOverrides, PatronList, RecordDetail, RelatedList,
    RenewResult, Reservations, ReserveOptions, ReserveResult, SearchResults, SearchSort,
};
use librespydus_desktop::backend::demo::{DemoBackend, DemoDelays, DemoFixtures};
use librespydus_desktop::backend::fake::FakeBackend;
use librespydus_desktop::backend::{BackendFuture, LibraryBackend};
use librespydus_desktop::state::formatting::today;
use librespydus_desktop::state::library::canning;
use librespydus_desktop::state::{AppState, Dependencies, Section};

const ORIGIN: &str = "https://canning.spydus.com";
/// A demo reservation with Edit and Cancel, collected at Riverton (code 25363).
const RESERVATION: &str = "19000003";

/// The demo backend, recording what `edit_reservation` receives.
#[derive(Clone)]
struct Recorder {
    demo: DemoBackend,
    edits: Arc<Mutex<Vec<EditOverrides>>>,
}

impl LibraryBackend for Recorder {
    fn origin(&self) -> String {
        self.demo.origin()
    }
    fn is_demo(&self) -> bool {
        self.demo.is_demo()
    }
    fn set_verification_cookie(&self, cookie: Option<String>) {
        self.demo.set_verification_cookie(cookie)
    }
    fn search(&self, query: String) -> BackendFuture<SearchResults> {
        self.demo.search(query)
    }
    fn search_sorted(&self, query: String, sort: SearchSort) -> BackendFuture<SearchResults> {
        self.demo.search_sorted(query, sort)
    }
    fn page(&self, url: String) -> BackendFuture<SearchResults> {
        self.demo.page(url)
    }
    fn availability(&self, url: String) -> BackendFuture<Availability> {
        self.demo.availability(url)
    }
    fn record(&self, url: String) -> BackendFuture<RecordDetail> {
        self.demo.record(url)
    }
    fn related(&self, url: String) -> BackendFuture<RelatedList> {
        self.demo.related(url)
    }
    fn card_barcode_symbology(&self) -> BoxFuture<'static, BarcodeSymbology> {
        self.demo.card_barcode_symbology()
    }
    fn login(&self, username: String, password: String) -> BackendFuture<String> {
        self.demo.login(username, password)
    }
    fn account(&self, session: String) -> BackendFuture<AccountLinks> {
        self.demo.account(session)
    }
    fn loans(&self, session: String) -> BackendFuture<PatronList> {
        self.demo.loans(session)
    }
    fn reservations(&self, session: String) -> BackendFuture<Reservations> {
        self.demo.reservations(session)
    }
    fn account_snapshot(&self, session: String) -> BackendFuture<AccountSnapshot> {
        self.demo.account_snapshot(session)
    }
    fn reserve_options(&self, url: String, session: String) -> BackendFuture<ReserveOptions> {
        self.demo.reserve_options(url, session)
    }
    fn place_reservation(
        &self,
        url: String,
        session: String,
        xdays: Option<String>,
        pickup: Option<String>,
    ) -> BackendFuture<ReserveResult> {
        self.demo.place_reservation(url, session, xdays, pickup)
    }
    fn cancel_reservation(&self, url: String, session: String) -> BackendFuture<ReserveResult> {
        self.demo.cancel_reservation(url, session)
    }
    fn edit_options(&self, url: String, session: String) -> BackendFuture<EditOptions> {
        self.demo.edit_options(url, session)
    }
    fn edit_reservation(&self, url: String, session: String, overrides: EditOverrides) -> BackendFuture<ReserveResult> {
        self.edits.lock().unwrap().push(overrides.clone());
        self.demo.edit_reservation(url, session, overrides)
    }
    fn renew_loan(&self, url: String, session: String) -> BackendFuture<RenewResult> {
        self.demo.renew_loan(url, session)
    }
}

fn iso(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

/// Signed in to the demo account, on Your Books › Reservations, with the edit form's dates set to
/// `delay` and `expiry`.
fn demo_app(
    cx: &mut TestAppContext,
    delay: Option<NaiveDate>,
    expiry: Option<NaiveDate>,
) -> (WindowHandle<Root>, Arc<Mutex<Vec<EditOverrides>>>) {
    let mut fixtures = DemoFixtures::bundled(ORIGIN, today()).unwrap();
    fixtures.edit_options.delay_until = delay.map(iso);
    fixtures.edit_options.not_required_after = expiry.map(iso);
    let demo = DemoBackend::new(FakeBackend::new(ORIGIN), fixtures, DemoDelays::NONE, false);
    let recorder = Recorder { demo, edits: Arc::default() };
    let (backend, edits) = (recorder.clone(), recorder.edits.clone());
    let mut deps = Dependencies::in_memory(Arc::new(move |_| -> Arc<dyn LibraryBackend> { Arc::new(backend.clone()) }));
    deps.is_demo = true;
    let state: Entity<AppState> = support::app_state(cx, deps);
    state.update(cx, |state, cx| state.select_library(canning(), cx));
    let window = support::open_root(cx);
    state.update(cx, |state, cx| state.select(Section::YourBooks, cx));
    cx.run_until_parked();
    ui(cx, window, |window, cx| window.click("your-books-segment-reservations", cx));
    (window, edits)
}

fn ui(cx: &mut TestAppContext, window: WindowHandle<Root>, f: impl FnOnce(&mut Window, &mut gpui_kit::App)) {
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        f(window, cx);
    })
    .unwrap();
    cx.run_until_parked();
}

/// Opens Edit from the reservation's "…" menu.
fn open_edit(cx: &mut TestAppContext, window: WindowHandle<Root>) {
    ui(cx, window, |window, cx| window.click(format!("reservation-{RESERVATION}-more"), cx));
    ui(cx, window, |window, cx| {
        window.within("popup-menu").click(0, cx);
    });
    ui(cx, window, |window, _| assert!(window.try_find("edit-reservation").is_some()));
    // Let the dialog's slide-in (real time, well under half a second) settle, so clicks land
    // where the controls end up.
    std::thread::sleep(std::time::Duration::from_millis(500));
    ui(cx, window, |_, _| {});
}

/// Opens the `field` ("edit-delay" / "edit-expiry") calendar and clicks `day`.
fn pick(cx: &mut TestAppContext, window: WindowHandle<Root>, field: &str, day: NaiveDate) {
    ui(cx, window, |window, cx| window.click(format!("{field}-date"), cx));
    ui(cx, window, |window, cx| {
        // The calendar's day cells: `calendar-<date>-<month offset>-<week>`.
        let id = (0..6)
            .map(|week| format!("calendar-{day}-0-{week}"))
            .find(|id| window.try_find(id.clone()).is_some())
            .unwrap_or_else(|| panic!("no calendar cell for {day}"));
        window.click(id, cx);
    });
}

fn shows(cx: &mut TestAppContext, window: WindowHandle<Root>, id: &str) -> bool {
    let id = id.to_owned();
    let mut found = false;
    ui(cx, window, |window, _| found = window.try_find(id).is_some());
    found
}

/// Another day in the same month as `date`.
fn other_day(date: NaiveDate) -> NaiveDate {
    date.with_day(if date.day() == 15 { 16 } else { 15 }).unwrap()
}

#[gpui_kit::test]
fn setting_both_dates_sends_both(cx: &mut TestAppContext) {
    let delay = today() + Duration::days(10);
    let (window, edits) = demo_app(cx, Some(delay), None);
    open_edit(cx, window);
    // "Delay until" starts on with the form's date; "Not needed after" starts off.
    assert!(shows(cx, window, "edit-delay-date"));
    assert!(!shows(cx, window, "edit-expiry-date"));

    let new_delay = other_day(delay);
    pick(cx, window, "edit-delay", new_delay);
    ui(cx, window, |window, cx| window.click("edit-expiry-switch", cx));
    assert!(shows(cx, window, "edit-expiry-date"));
    // Switched on, it starts three months out; pick another day that month.
    let expiry_default = today().checked_add_months(Months::new(3)).unwrap();
    let new_expiry = other_day(expiry_default);
    pick(cx, window, "edit-expiry", new_expiry);
    ui(cx, window, |window, cx| window.click("edit-save", cx));
    ui(cx, window, |window, _| assert!(window.try_find("edit-reservation").is_none(), "closed after saving"));

    let edits = edits.lock().unwrap().clone();
    assert_eq!(
        edits,
        [EditOverrides {
            pickup: Some("25363".into()),
            delay_until: Some(iso(new_delay)),
            not_required_after: Some(iso(new_expiry)),
            clear_delay: false,
            clear_expiry: false,
        }]
    );
}

#[gpui_kit::test]
fn switching_dates_off_clears_them(cx: &mut TestAppContext) {
    let delay = today() + Duration::days(10);
    let expiry = today() + Duration::days(60);
    let (window, edits) = demo_app(cx, Some(delay), Some(expiry));
    open_edit(cx, window);
    assert!(shows(cx, window, "edit-delay-date") && shows(cx, window, "edit-expiry-date"));

    ui(cx, window, |window, cx| window.click("edit-delay-switch", cx));
    ui(cx, window, |window, cx| window.click("edit-expiry-switch", cx));
    assert!(!shows(cx, window, "edit-delay-date") && !shows(cx, window, "edit-expiry-date"));
    ui(cx, window, |window, cx| window.click("edit-save", cx));

    let edits = edits.lock().unwrap().clone();
    assert_eq!(
        edits,
        [EditOverrides {
            pickup: Some("25363".into()),
            delay_until: None,
            not_required_after: None,
            clear_delay: true,
            clear_expiry: true,
        }]
    );
}

#[gpui_kit::test]
fn an_unchanged_date_is_not_sent(cx: &mut TestAppContext) {
    let delay = today() + Duration::days(10);
    let (window, edits) = demo_app(cx, Some(delay), None);
    open_edit(cx, window);
    // Only the expiry changes: the delay the form already had isn't sent again.
    ui(cx, window, |window, cx| window.click("edit-expiry-switch", cx));
    let expiry = other_day(today().checked_add_months(Months::new(3)).unwrap());
    pick(cx, window, "edit-expiry", expiry);
    // Picking the delay's own day again changes nothing.
    pick(cx, window, "edit-delay", delay);
    ui(cx, window, |window, cx| window.click("edit-save", cx));

    let edits = edits.lock().unwrap().clone();
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].delay_until, None);
    assert!(!edits[0].clear_delay);
    assert_eq!(edits[0].not_required_after, Some(iso(expiry)));
}
