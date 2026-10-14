//! A scripted `LibraryBackend` for D3's UI tests (Home, Search, Featured): answers per query, sort
//! and page URL, records every call, and never touches the network. Included as a module by
//! `search_ui.rs`, `home_ui.rs` and `featured_ui.rs`.
#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use librespydus_core::{
    AccountLinks, AccountSnapshot, Availability, EditOptions, EditOverrides, FeaturedCollection, FeaturedLink,
    PatronList, RecordDetail, RelatedList, Reservations, ReserveOptions, ReserveResult, SearchRecord, SearchResults,
    SearchSort, SpydusError,
};
use librespydus_desktop::backend::fake::clone_error;
use librespydus_desktop::backend::{BackendFuture, LibraryBackend, err, ok};

/// `save_*`'s answer: done at once.
mod futures_util_ready {
    pub type Ready = futures::future::BoxFuture<'static, ()>;

    pub fn done() -> Ready {
        Box::pin(async {})
    }
}

pub const ORIGIN: &str = "https://canning.spydus.com";

#[derive(Default)]
struct Script {
    /// By query (any sort unless `sorted` has it).
    searches: HashMap<String, Result<SearchResults, SpydusError>>,
    sorted: HashMap<(String, bool), Result<SearchResults, SpydusError>>,
    pages: HashMap<String, Result<SearchResults, SpydusError>>,
    featured: Option<Result<Vec<FeaturedLink>, SpydusError>>,
    collections: HashMap<String, FeaturedCollection>,
    saved_shelves: HashMap<String, SearchResults>,
    saved_featured: Option<Vec<FeaturedLink>>,
    calls: Vec<String>,
    hold_pages: bool,
    held: Vec<(String, futures::channel::oneshot::Sender<Result<SearchResults, SpydusError>>)>,
}

pub struct ScriptedBackend {
    script: Mutex<Script>,
}

fn is_newest(sort: SearchSort) -> bool {
    matches!(sort, SearchSort::Newest)
}

fn answer(result: Option<&Result<SearchResults, SpydusError>>) -> BackendFuture<SearchResults> {
    match result {
        Some(Ok(results)) => ok(results.clone()),
        Some(Err(error)) => err(clone_error(error)),
        None => ok(no_results()),
    }
}

impl ScriptedBackend {
    pub fn new() -> Arc<Self> {
        Arc::new(Self { script: Mutex::default() })
    }

    fn script(&self) -> MutexGuard<'_, Script> {
        self.script.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn on_search(&self, query: &str, result: Result<SearchResults, SpydusError>) {
        self.script().searches.insert(query.into(), result);
    }

    pub fn on_sorted(&self, query: &str, sort: SearchSort, result: Result<SearchResults, SpydusError>) {
        self.script().sorted.insert((query.into(), is_newest(sort)), result);
    }

    pub fn on_page(&self, url: &str, result: Result<SearchResults, SpydusError>) {
        self.script().pages.insert(url.into(), result);
    }

    pub fn on_featured(&self, result: Result<Vec<FeaturedLink>, SpydusError>) {
        self.script().featured = Some(result);
    }

    pub fn on_collection(&self, url: &str, collection: FeaturedCollection) {
        self.script().collections.insert(url.into(), collection);
    }

    /// A Home shelf saved on this computer (§5.13).
    pub fn on_saved_shelf(&self, key: &str, results: SearchResults) {
        self.script().saved_shelves.insert(key.into(), results);
    }

    pub fn on_saved_featured(&self, links: Vec<FeaturedLink>) {
        self.script().saved_featured = Some(links);
    }

    /// `true`: every later `page` waits for `release_pages`.
    pub fn hold_pages(&self, hold: bool) {
        self.script().hold_pages = hold;
    }

    /// How many `page` calls are waiting.
    pub fn held_pages(&self) -> usize {
        self.script().held.len()
    }

    /// Answers every waiting `page` call as scripted.
    pub fn release_pages(&self) {
        let held = std::mem::take(&mut self.script().held);
        for (url, sender) in held {
            let answer = match self.script().pages.get(&url) {
                Some(Ok(results)) => Ok(results.clone()),
                Some(Err(error)) => Err(clone_error(error)),
                None => Ok(no_results()),
            };
            let _ = sender.send(answer);
        }
    }

    /// Every call, as `search:<q>:<relevance|newest>`, `page:<url>`, `featured`, `collection:<url>`.
    pub fn calls(&self) -> Vec<String> {
        self.script().calls.clone()
    }

    pub fn count(&self, prefix: &str) -> usize {
        self.calls().iter().filter(|call| call.starts_with(prefix)).count()
    }
}

/// Records titled `{prefix} 1…n`, each its own work.
pub fn records(prefix: &str, n: usize) -> Vec<SearchRecord> {
    (1..=n)
        .map(|index| SearchRecord {
            work_irn: Some(format!("{prefix}-{index}")),
            title: Some(format!("{prefix} {index}")),
            author: Some("Haig, Matt, author".into()),
            year: Some("2024".into()),
            detail_url: Some(format!("{ORIGIN}/detail/{prefix}/{index}")),
            ..Default::default()
        })
        .collect()
}

/// A results page with `records`, `total` and a next page.
pub fn page(records: Vec<SearchRecord>, total: i64, next: Option<&str>) -> SearchResults {
    SearchResults {
        query: Some("q".into()),
        total_results: Some(total),
        next_page_url: next.map(str::to_owned),
        records,
        ..Default::default()
    }
}

pub fn no_results() -> SearchResults {
    SearchResults { no_results: true, total_results: Some(0), ..Default::default() }
}

/// No records and not the "no records" page.
pub fn unrecognised() -> SearchResults {
    SearchResults::default()
}

impl LibraryBackend for ScriptedBackend {
    fn origin(&self) -> String {
        ORIGIN.into()
    }

    fn set_verification_cookie(&self, _cookie: Option<String>) {}

    fn search(&self, query: String) -> BackendFuture<SearchResults> {
        self.search_sorted(query, SearchSort::Relevance)
    }

    fn search_sorted(&self, query: String, sort: SearchSort) -> BackendFuture<SearchResults> {
        let mut script = self.script();
        let order = if is_newest(sort) { "newest" } else { "relevance" };
        script.calls.push(format!("search:{query}:{order}"));
        let sorted = script.sorted.get(&(query.clone(), is_newest(sort)));
        answer(sorted.or_else(|| script.searches.get(&query)))
    }

    fn page(&self, url: String) -> BackendFuture<SearchResults> {
        let mut script = self.script();
        script.calls.push(format!("page:{url}"));
        if script.hold_pages {
            let (sender, receiver) = futures::channel::oneshot::channel();
            script.held.push((url, sender));
            return Box::pin(async move {
                receiver.await.unwrap_or(Err(SpydusError::Network { message: "dropped".into() }))
            });
        }
        answer(script.pages.get(&url))
    }

    fn availability(&self, _url: String) -> BackendFuture<Availability> {
        err(SpydusError::Upstream { message: "scripted".into() })
    }

    fn record(&self, _url: String) -> BackendFuture<RecordDetail> {
        err(SpydusError::Upstream { message: "scripted".into() })
    }

    fn related(&self, _url: String) -> BackendFuture<RelatedList> {
        ok(RelatedList::default())
    }

    fn featured(&self) -> BackendFuture<Vec<FeaturedLink>> {
        let mut script = self.script();
        script.calls.push("featured".into());
        match &script.featured {
            Some(Ok(links)) => ok(links.clone()),
            Some(Err(error)) => err(clone_error(error)),
            None => ok(Vec::new()),
        }
    }

    fn collection(&self, url: String) -> BackendFuture<FeaturedCollection> {
        let mut script = self.script();
        script.calls.push(format!("collection:{url}"));
        ok(script.collections.get(&url).cloned().unwrap_or_default())
    }

    fn saved_shelf(&self, key: &str) -> Option<SearchResults> {
        self.script().saved_shelves.get(key).cloned()
    }

    fn save_shelf(&self, key: String, results: SearchResults) -> futures_util_ready::Ready {
        let mut script = self.script();
        script.calls.push(format!("save_shelf:{key}"));
        script.saved_shelves.insert(key, results);
        futures_util_ready::done()
    }

    fn saved_featured(&self) -> Option<Vec<FeaturedLink>> {
        self.script().saved_featured.clone()
    }

    fn save_featured(&self, links: Vec<FeaturedLink>) -> futures_util_ready::Ready {
        let mut script = self.script();
        script.calls.push("save_featured".into());
        script.saved_featured = Some(links);
        futures_util_ready::done()
    }

    fn login(&self, _username: String, _password: String) -> BackendFuture<String> {
        err(SpydusError::CredentialsRejected)
    }

    fn account(&self, _session: String) -> BackendFuture<AccountLinks> {
        err(SpydusError::NotLoggedIn)
    }

    fn loans(&self, _session: String) -> BackendFuture<PatronList> {
        err(SpydusError::NotLoggedIn)
    }

    fn reservations(&self, _session: String) -> BackendFuture<Reservations> {
        err(SpydusError::NotLoggedIn)
    }

    fn account_snapshot(&self, _session: String) -> BackendFuture<AccountSnapshot> {
        err(SpydusError::NotLoggedIn)
    }

    fn reserve_options(&self, _url: String, _session: String) -> BackendFuture<ReserveOptions> {
        err(SpydusError::NotLoggedIn)
    }

    fn place_reservation(
        &self,
        _url: String,
        _session: String,
        _xdays: Option<String>,
        _pickup: Option<String>,
    ) -> BackendFuture<ReserveResult> {
        err(SpydusError::NotLoggedIn)
    }

    fn cancel_reservation(&self, _url: String, _session: String) -> BackendFuture<ReserveResult> {
        err(SpydusError::NotLoggedIn)
    }

    fn edit_options(&self, _url: String, _session: String) -> BackendFuture<EditOptions> {
        err(SpydusError::NotLoggedIn)
    }

    fn edit_reservation(
        &self,
        _url: String,
        _session: String,
        _overrides: EditOverrides,
    ) -> BackendFuture<ReserveResult> {
        err(SpydusError::NotLoggedIn)
    }
}
