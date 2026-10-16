//! The real backend: one `SpydusClient` for one library origin, kept while that library is
//! selected (iOS `CoreBackend.swift`). Every call runs on the core's runtime
//! (`runtime::spawn_core`) and, in debug builds, prints a `⏱ app <call>: <ms> total` line.

use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use futures::FutureExt;
use futures::future::BoxFuture;
use librespydus_core::barcode::BarcodeSymbology;
use librespydus_core::{
    AccountLinks, AccountSnapshot, Availability, EditOptions, EditOverrides, FeaturedCollection, FeaturedLink, Holding,
    PatronList, RecordDetail, RelatedList, RenewResult, Reservations, ReserveOptions, ReserveResult, SavedBook,
    SearchResults, SearchSort, SpydusClient,
};

use super::{BackendFuture, LibraryBackend};
use crate::runtime::spawn_core;

pub struct CoreBackend {
    client: Arc<SpydusClient>,
    origin: String,
}

impl CoreBackend {
    /// `origin` is a `LibraryInfo.origin` (`None`: Canning). `saved_pages`: where the core keeps
    /// saved pages (§5.13), one folder per library under it; `None` saves nothing.
    pub fn new(origin: Option<&str>, saved_pages: Option<PathBuf>) -> Self {
        let client = SpydusClient::new(origin.map(str::to_owned));
        client.set_saved_pages_dir(saved_pages.map(|dir| dir.to_string_lossy().into_owned()));
        let origin = client.origin();
        Self { client, origin }
    }

    pub fn client(&self) -> &Arc<SpydusClient> {
        &self.client
    }

    /// Runs `call` with the client on the core's runtime, timed for the debug log.
    fn timed<T, F, Fut>(&self, operation: &'static str, call: F) -> BoxFuture<'static, T>
    where
        T: Send + 'static,
        F: FnOnce(Arc<SpydusClient>) -> Fut,
        Fut: Future<Output = T> + Send + 'static,
    {
        let future = call(self.client.clone());
        let started = Instant::now();
        spawn_core(future)
            .map(move |value| {
                crate::timing::app(operation, started.elapsed());
                value
            })
            .boxed()
    }
}

impl LibraryBackend for CoreBackend {
    fn origin(&self) -> String {
        self.origin.clone()
    }

    fn set_verification_cookie(&self, cookie: Option<String>) {
        self.client.set_verification_cookie(cookie);
    }

    fn search(&self, query: String) -> BackendFuture<SearchResults> {
        self.timed("search", |c| async move { c.search(query).await })
    }

    fn search_sorted(&self, query: String, sort: SearchSort) -> BackendFuture<SearchResults> {
        self.timed("searchSorted", |c| async move { c.search_sorted(query, sort).await })
    }

    fn page(&self, url: String) -> BackendFuture<SearchResults> {
        self.timed("page", |c| async move { c.page(url).await })
    }

    fn availability(&self, url: String) -> BackendFuture<Availability> {
        self.timed("availability", |c| async move { c.availability(url).await })
    }

    fn record(&self, url: String) -> BackendFuture<RecordDetail> {
        self.timed("record", |c| async move { c.record(url).await })
    }

    fn related(&self, url: String) -> BackendFuture<RelatedList> {
        self.timed("related", |c| async move { c.related(url).await })
    }

    fn card_barcode_symbology(&self) -> BoxFuture<'static, BarcodeSymbology> {
        self.timed("cardBarcodeSymbology", |c| async move { c.card_barcode_symbology().await })
    }

    fn featured(&self) -> BackendFuture<Vec<FeaturedLink>> {
        self.timed("featured", |c| async move { c.featured().await })
    }

    fn collection(&self, url: String) -> BackendFuture<FeaturedCollection> {
        self.timed("collection", |c| async move { c.collection(url).await })
    }

    fn holdings(&self, url: String) -> BackendFuture<Vec<Holding>> {
        self.timed("holdings", |c| async move { c.holdings(url).await })
    }

    fn saved_book(&self, key: &str) -> Option<SavedBook> {
        self.client.saved_book(key.to_owned())
    }

    fn save_book(&self, key: String, book: SavedBook) -> BoxFuture<'static, ()> {
        let client = self.client.clone();
        spawn_core(async move { client.save_book(key, book).await }).boxed()
    }

    fn saved_shelf(&self, key: &str) -> Option<SearchResults> {
        self.client.saved_shelf(key.to_owned())
    }

    fn save_shelf(&self, key: String, results: SearchResults) -> BoxFuture<'static, ()> {
        let client = self.client.clone();
        spawn_core(async move { client.save_shelf(key, results).await }).boxed()
    }

    fn saved_featured(&self) -> Option<Vec<FeaturedLink>> {
        self.client.saved_featured()
    }

    fn save_featured(&self, links: Vec<FeaturedLink>) -> BoxFuture<'static, ()> {
        let client = self.client.clone();
        spawn_core(async move { client.save_featured(links).await }).boxed()
    }

    fn login(&self, username: String, password: String) -> BackendFuture<String> {
        self.timed("login", |c| async move { c.login(username, password).await })
    }

    fn account(&self, session: String) -> BackendFuture<AccountLinks> {
        self.timed("account", |c| async move { c.account(session).await })
    }

    fn loans(&self, session: String) -> BackendFuture<PatronList> {
        self.timed("loans", |c| async move { c.loans(session).await })
    }

    fn reservations(&self, session: String) -> BackendFuture<Reservations> {
        self.timed("reservations", |c| async move { c.reservations(session).await })
    }

    fn account_snapshot(&self, session: String) -> BackendFuture<AccountSnapshot> {
        self.timed("accountSnapshot", |c| async move { c.account_snapshot(session).await })
    }

    fn reserve_options(&self, url: String, session: String) -> BackendFuture<ReserveOptions> {
        self.timed("reserveOptions", |c| async move { c.reserve_options(url, session).await })
    }

    fn place_reservation(
        &self,
        url: String,
        session: String,
        xdays: Option<String>,
        pickup: Option<String>,
    ) -> BackendFuture<ReserveResult> {
        self.timed("placeReservation", |c| async move { c.place_reservation(url, session, xdays, pickup).await })
    }

    fn cancel_reservation(&self, url: String, session: String) -> BackendFuture<ReserveResult> {
        self.timed("cancelReservation", |c| async move { c.cancel_reservation(url, session).await })
    }

    fn edit_options(&self, url: String, session: String) -> BackendFuture<EditOptions> {
        self.timed("editOptions", |c| async move { c.edit_options(url, session).await })
    }

    fn edit_reservation(&self, url: String, session: String, overrides: EditOverrides) -> BackendFuture<ReserveResult> {
        self.timed("editReservation", |c| async move { c.edit_reservation(url, session, overrides).await })
    }

    fn renew_loan(&self, url: String, session: String) -> BackendFuture<RenewResult> {
        self.timed("renewLoan", |c| async move { c.renew_loan(url, session).await })
    }

    fn picks(&self, session: String, url: Option<String>) -> BackendFuture<SearchResults> {
        self.timed("picks", |c| async move { c.picks(session, url).await })
    }

    fn save_pick(&self, irn: String, session: String) -> BackendFuture<i64> {
        self.timed("savePick", |c| async move { c.save_pick(irn, session).await })
    }

    fn remove_pick(&self, irn: String, session: String) -> BackendFuture<i64> {
        self.timed("removePick", |c| async move { c.remove_pick(irn, session).await })
    }
}
