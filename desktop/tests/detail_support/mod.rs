//! Helpers for the book page's headless UI tests: a public backend with scripted answers (records,
//! availability, related lists, holdings, saved pages), apps on it or on a `DemoBackend` over it,
//! and a window whose root is one `DetailScreen`.
#![allow(dead_code)]

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, MutexGuard};

use futures::FutureExt;
use futures::channel::oneshot;
use gpui_kit::base::Root;
use gpui_kit::{
    AppContext, Bounds, Entity, TestAppContext, WindowBounds, WindowHandle, WindowOptions, point, px, size,
};
use librespydus_core::{
    AccountLinks, AccountSnapshot, Availability, EditOptions, EditOverrides, Holding, PatronList, RecordDetail,
    RecordRelated, RelatedItem, RelatedList, Reservations, ReserveOptions, ReserveResult, SavedBook, SearchResults,
    SearchSort, SpydusError,
};
use librespydus_desktop::backend::demo::{DemoBackend, DemoDelays, DemoFixtures};
use librespydus_desktop::backend::{BackendFuture, LibraryBackend};
use librespydus_desktop::features::detail::DetailScreen;
use librespydus_desktop::state::formatting::today;
use librespydus_desktop::state::library::canning;
use librespydus_desktop::state::{AppState, BookRef, Dependencies};

pub const ORIGIN: &str = "https://canning.spydus.com";

#[derive(Default)]
struct Script {
    records: HashMap<String, RecordDetail>,
    availability: HashMap<String, Availability>,
    related: HashMap<String, RelatedList>,
    holdings: HashMap<String, Vec<Holding>>,
    search: Option<SearchResults>,
    saved: HashMap<String, SavedBook>,
    saves: Vec<(String, SavedBook)>,
    record_calls: Vec<String>,
    availability_calls: Vec<String>,
    hold_records: bool,
    held: VecDeque<(String, oneshot::Sender<Result<RecordDetail, SpydusError>>)>,
}

/// Public reads from a script; every account call says "not signed in".
pub struct ScriptedBackend {
    script: Mutex<Script>,
}

impl ScriptedBackend {
    pub fn new() -> Arc<Self> {
        Arc::new(Self { script: Mutex::default() })
    }

    fn script(&self) -> MutexGuard<'_, Script> {
        self.script.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn record(&self, url: &str, record: RecordDetail) {
        self.script().records.insert(url.to_owned(), record);
    }

    pub fn availability(&self, url: &str, availability: Availability) {
        self.script().availability.insert(url.to_owned(), availability);
    }

    pub fn related(&self, url: &str, list: RelatedList) {
        self.script().related.insert(url.to_owned(), list);
    }

    pub fn holdings(&self, url: &str, holdings: Vec<Holding>) {
        self.script().holdings.insert(url.to_owned(), holdings);
    }

    pub fn saved(&self, key: &str, book: SavedBook) {
        self.script().saved.insert(key.to_owned(), book);
    }

    /// Every `save_book` so far.
    pub fn saves(&self) -> Vec<(String, SavedBook)> {
        self.script().saves.clone()
    }

    pub fn record_calls(&self) -> Vec<String> {
        self.script().record_calls.clone()
    }

    pub fn availability_calls(&self) -> Vec<String> {
        self.script().availability_calls.clone()
    }

    /// `true`: each later `record` waits for `release_records`.
    pub fn hold_records(&self, hold: bool) {
        self.script().hold_records = hold;
    }

    /// Answers every waiting `record` call from the script.
    pub fn release_records(&self) -> usize {
        let held: Vec<_> = self.script().held.drain(..).collect();
        let count = held.len();
        for (url, sender) in held {
            let _ = sender.send(self.answer_record(&url));
        }
        count
    }

    fn answer_record(&self, url: &str) -> Result<RecordDetail, SpydusError> {
        self.script().records.get(url).cloned().ok_or(SpydusError::Upstream { message: format!("no record {url}") })
    }
}

fn ready<T: Send + 'static>(result: Result<T, SpydusError>) -> BackendFuture<T> {
    futures::future::ready(result).boxed()
}

impl LibraryBackend for ScriptedBackend {
    fn origin(&self) -> String {
        ORIGIN.to_owned()
    }

    fn set_verification_cookie(&self, _cookie: Option<String>) {}

    fn search(&self, _query: String) -> BackendFuture<SearchResults> {
        let results = self.script().search.clone().unwrap_or(SearchResults { no_results: true, ..Default::default() });
        ready(Ok(results))
    }

    fn search_sorted(&self, query: String, _sort: SearchSort) -> BackendFuture<SearchResults> {
        self.search(query)
    }

    fn page(&self, url: String) -> BackendFuture<SearchResults> {
        self.search(url)
    }

    fn availability(&self, url: String) -> BackendFuture<Availability> {
        let mut script = self.script();
        script.availability_calls.push(url.clone());
        ready(
            script
                .availability
                .get(&url)
                .cloned()
                .ok_or(SpydusError::Upstream { message: format!("no availability {url}") }),
        )
    }

    fn record(&self, url: String) -> BackendFuture<RecordDetail> {
        let mut script = self.script();
        script.record_calls.push(url.clone());
        if script.hold_records {
            let (sender, receiver) = oneshot::channel();
            script.held.push_back((url, sender));
            return receiver
                .map(|answer| answer.unwrap_or(Err(SpydusError::Network { message: "dropped".into() })))
                .boxed();
        }
        drop(script);
        ready(self.answer_record(&url))
    }

    fn related(&self, url: String) -> BackendFuture<RelatedList> {
        ready(
            self.script().related.get(&url).cloned().ok_or(SpydusError::Upstream { message: format!("no list {url}") }),
        )
    }

    fn holdings(&self, url: String) -> BackendFuture<Vec<Holding>> {
        ready(Ok(self.script().holdings.get(&url).cloned().unwrap_or_default()))
    }

    fn saved_book(&self, key: &str) -> Option<SavedBook> {
        self.script().saved.get(key).cloned()
    }

    fn save_book(&self, key: String, book: SavedBook) -> futures::future::BoxFuture<'static, ()> {
        self.script().saves.push((key, book));
        futures::future::ready(()).boxed()
    }

    fn login(&self, _username: String, _password: String) -> BackendFuture<String> {
        ready(Err(SpydusError::CredentialsRejected))
    }

    fn account(&self, _session: String) -> BackendFuture<AccountLinks> {
        ready(Err(SpydusError::NotLoggedIn))
    }

    fn loans(&self, _session: String) -> BackendFuture<PatronList> {
        ready(Err(SpydusError::NotLoggedIn))
    }

    fn reservations(&self, _session: String) -> BackendFuture<Reservations> {
        ready(Err(SpydusError::NotLoggedIn))
    }

    fn account_snapshot(&self, _session: String) -> BackendFuture<AccountSnapshot> {
        ready(Err(SpydusError::NotLoggedIn))
    }

    fn reserve_options(&self, _url: String, _session: String) -> BackendFuture<ReserveOptions> {
        ready(Err(SpydusError::NotLoggedIn))
    }

    fn place_reservation(
        &self,
        _url: String,
        _session: String,
        _xdays: Option<String>,
        _pickup: Option<String>,
    ) -> BackendFuture<ReserveResult> {
        ready(Err(SpydusError::NotLoggedIn))
    }

    fn cancel_reservation(&self, _url: String, _session: String) -> BackendFuture<ReserveResult> {
        ready(Err(SpydusError::NotLoggedIn))
    }

    fn edit_options(&self, _url: String, _session: String) -> BackendFuture<EditOptions> {
        ready(Err(SpydusError::NotLoggedIn))
    }

    fn edit_reservation(
        &self,
        _url: String,
        _session: String,
        _overrides: EditOverrides,
    ) -> BackendFuture<ReserveResult> {
        ready(Err(SpydusError::NotLoggedIn))
    }
}

// ---- Apps ----

fn init(cx: &mut TestAppContext, deps: Dependencies) -> Entity<AppState> {
    let state = cx.update(|cx| {
        gpui_kit::init(cx);
        librespydus_desktop::theme::apply(cx);
        AppState::init(deps, cx)
    });
    state.update(cx, |state, cx| state.select_library(canning(), cx));
    cx.run_until_parked();
    state
}

/// Signed out, on `backend`.
pub fn signed_out_app(cx: &mut TestAppContext, backend: Arc<dyn LibraryBackend>) -> Entity<AppState> {
    init(cx, Dependencies::in_memory(Arc::new(move |_| backend.clone())))
}

/// Demo mode, signed in as "Test Borrower": the demo account on `public` reads (no delays).
pub fn demo_app(cx: &mut TestAppContext, public: Arc<dyn LibraryBackend>) -> (Entity<AppState>, DemoBackend) {
    let fixtures = DemoFixtures::bundled(ORIGIN, today()).expect("demo fixtures");
    let demo = DemoBackend::new(public, fixtures, DemoDelays::NONE, false);
    let backend = demo.clone();
    let mut deps = Dependencies::in_memory(Arc::new(move |_| -> Arc<dyn LibraryBackend> { Arc::new(backend.clone()) }));
    deps.is_demo = true;
    let state = init(cx, deps);
    (state, demo)
}

/// A 1280×820 window whose root is the book page for `book`.
pub fn open_detail(cx: &mut TestAppContext, book: BookRef) -> (WindowHandle<Root>, Entity<DetailScreen>) {
    open_detail_sized(cx, book, 1280., 820.)
}

pub fn open_detail_sized(
    cx: &mut TestAppContext,
    book: BookRef,
    width: f32,
    height: f32,
) -> (WindowHandle<Root>, Entity<DetailScreen>) {
    let (window, screen) = cx.update(|cx| {
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: point(px(0.), px(0.)),
                size: size(px(width), px(height)),
            })),
            ..Default::default()
        };
        let (window, screen) =
            gpui_kit::open_window(options, cx, |window, cx| cx.new(|cx| DetailScreen::new(book, window, cx)))
                .expect("window");
        (window.downcast::<Root>().expect("Base Root"), screen)
    });
    cx.run_until_parked();
    (window, screen)
}

/// Runs `body` on a freshly rendered frame of `window`.
pub fn with_frame<R>(
    cx: &mut TestAppContext,
    window: WindowHandle<Root>,
    body: impl FnOnce(&mut gpui_kit::Window, &mut gpui_kit::App) -> R,
) -> R {
    cx.run_until_parked();
    let result = cx
        .update_window(window.into(), |_, window, cx| {
            gpui_kit::test::TestWindowExt::render_frame(window, cx);
            body(window, cx)
        })
        .expect("window");
    cx.run_until_parked();
    result
}

/// Clicks `id` on a fresh frame.
pub fn click(cx: &mut TestAppContext, window: WindowHandle<Root>, id: &str) {
    let id = id.to_owned();
    with_frame(cx, window, move |window, cx| gpui_kit::test::TestWindowExt::click(window, id, cx));
}

/// The accessibility label of `id`, or `None` when it isn't on screen.
pub fn label(cx: &mut TestAppContext, window: WindowHandle<Root>, id: &str) -> Option<String> {
    let id = id.to_owned();
    with_frame(cx, window, move |window, _| {
        gpui_kit::test::TestWindowExt::try_find(window, id).and_then(|found| found.label().map(str::to_owned))
    })
}

pub fn exists(cx: &mut TestAppContext, window: WindowHandle<Root>, id: &str) -> bool {
    let id = id.to_owned();
    with_frame(cx, window, move |window, _| gpui_kit::test::TestWindowExt::try_find(window, id).is_some())
}

// ---- Data ----

pub fn related_item(brn: &str, title: &str) -> RelatedItem {
    RelatedItem {
        brn: Some(brn.into()),
        title: Some(title.into()),
        bib_url: Some(format!("{ORIGIN}/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&BRN={brn}")),
        author: Some("Haig, Matt".into()),
        ..Default::default()
    }
}

pub fn record(bib_irn: &str, brn: &str, title: &str, related: RecordRelated) -> RecordDetail {
    RecordDetail {
        bib_irn: Some(bib_irn.into()),
        brn: Some(brn.into()),
        title: Some(title.into()),
        summary: Some(format!("All about {title}.")),
        bookmark_url: Some(format!("{ORIGIN}/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&BRN={brn}")),
        related,
        ..Default::default()
    }
}

pub fn counts(available: i64, copies: i64, bib_irn: &str) -> Availability {
    Availability {
        bib_record_irn: Some(bib_irn.into()),
        counts: HashMap::from([("available".to_owned(), available), ("copies".to_owned(), copies)]),
        copies: Some(copies),
        ..Default::default()
    }
}
