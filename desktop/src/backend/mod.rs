//! Everything the app asks of one library (docs/desktop-app.md §5.2; iOS `LibraryBackend.swift`,
//! method for method). Screens hold `Arc<dyn LibraryBackend>` and never call `SpydusClient`:
//! `CoreBackend` wraps the real core, `DemoBackend` serves the demo account, `FakeBackend` answers
//! tests.
//!
//! Async methods return a `'static` boxed future, awaitable from GPUI's executor (`cx.spawn`).
//! Errors are the core's `SpydusError`; route them through `AppState::report`. Mutations
//! (`place_reservation`, `cancel_reservation`, `edit_reservation`, `renew_loan`) return `ok: false`
//! results as data when the library refuses; that's not an error.

pub mod core;
#[cfg(debug_assertions)]
pub mod demo;
#[cfg(debug_assertions)]
pub mod demo_catalogue;
#[cfg(debug_assertions)]
pub mod fake;

use futures::FutureExt;
use futures::future::BoxFuture;
use librespydus_core::barcode::BarcodeSymbology;
use librespydus_core::{
    AccountLinks, AccountSnapshot, Availability, EditOptions, EditOverrides, FeaturedCollection, FeaturedLink, Holding,
    PatronList, RecordDetail, RelatedList, RenewResult, Reservations, ReserveOptions, ReserveResult, SavedBook,
    SearchResults, SearchSort, SpydusError,
};

pub use self::core::CoreBackend;

/// A backend call's result, awaitable from any executor.
pub type BackendFuture<T> = BoxFuture<'static, Result<T, SpydusError>>;

pub trait LibraryBackend: Send + Sync + 'static {
    /// Normalised origin, e.g. `https://canning.spydus.com` (no trailing slash).
    fn origin(&self) -> String;
    /// True for `DemoBackend` (debug demo mode).
    fn is_demo(&self) -> bool {
        false
    }
    /// `"aws-waf-token=<value>"` after human verification, or `None` to clear. Drops the core's caches.
    fn set_verification_cookie(&self, cookie: Option<String>);

    // ---- Public reads (GET only) ----

    fn search(&self, query: String) -> BackendFuture<SearchResults>;
    /// `search` with an explicit order (`Relevance` is the same request as `search`).
    fn search_sorted(&self, query: String, sort: SearchSort) -> BackendFuture<SearchResults>;
    /// Next/previous page: pass `SearchResults.next_page_url` / `prev_page_url`.
    fn page(&self, url: String) -> BackendFuture<SearchResults>;
    /// A format's `availability_url`.
    fn availability(&self, url: String) -> BackendFuture<Availability>;
    /// `SearchRecord.detail_url`, `patron_item_record_url(item)` for a loan or reservation,
    /// `RelatedItem.bib_url`, a format's `availability_url`, or a stored `BookRef.url`.
    fn record(&self, url: String) -> BackendFuture<RecordDetail>;
    /// One of `RecordDetail.related.*`.
    fn related(&self, url: String) -> BackendFuture<RelatedList>;
    /// The symbology the library's card scanners read (§5.9); Code 128 when it has none. Never
    /// fails; cached in the core.
    fn card_barcode_symbology(&self) -> BoxFuture<'static, BarcodeSymbology> {
        futures::future::ready(BarcodeSymbology::Code128).boxed()
    }
    /// The library's "Featured" collections (§5.10); empty for most libraries.
    fn featured(&self) -> BackendFuture<Vec<FeaturedLink>> {
        ok(Vec::new())
    }
    /// A `Collection` link's page → its sections; none for a page of outside links.
    fn collection(&self, _url: String) -> BackendFuture<FeaturedCollection> {
        ok(FeaturedCollection::default())
    }
    /// Every copy of a record: an `Availability.holdings_url`.
    fn holdings(&self, _url: String) -> BackendFuture<Vec<Holding>> {
        ok(Vec::new())
    }

    // ---- Saved pages (§5.13: this computer only, never the network) ----

    /// The book page saved under `key` (`BookRef::saved_page_key`), if any.
    fn saved_book(&self, _key: &str) -> Option<SavedBook> {
        None
    }
    /// Saves a book page after a fresh load. Never fails.
    fn save_book(&self, _key: String, _book: SavedBook) -> BoxFuture<'static, ()> {
        futures::future::ready(()).boxed()
    }
    /// A Home shelf's first results page saved under its `Shelf.key`, if any.
    fn saved_shelf(&self, _key: &str) -> Option<SearchResults> {
        None
    }
    fn save_shelf(&self, _key: String, _results: SearchResults) -> BoxFuture<'static, ()> {
        futures::future::ready(()).boxed()
    }
    /// The `featured()` links saved last (empty: the library has none), if any.
    fn saved_featured(&self) -> Option<Vec<FeaturedLink>> {
        None
    }
    fn save_featured(&self, _links: Vec<FeaturedLink>) -> BoxFuture<'static, ()> {
        futures::future::ready(()).boxed()
    }

    // ---- Account (needs a session from `login`) ----

    fn login(&self, username: String, password: String) -> BackendFuture<String>;
    fn account(&self, session: String) -> BackendFuture<AccountLinks>;
    fn loans(&self, session: String) -> BackendFuture<PatronList>;
    fn reservations(&self, session: String) -> BackendFuture<Reservations>;
    /// Name, loans and both reservation lists in one call: what `AccountStore` shows and saves.
    fn account_snapshot(&self, session: String) -> BackendFuture<AccountSnapshot>;
    fn reserve_options(&self, url: String, session: String) -> BackendFuture<ReserveOptions>;
    fn place_reservation(
        &self,
        url: String,
        session: String,
        xdays: Option<String>,
        pickup: Option<String>,
    ) -> BackendFuture<ReserveResult>;
    fn cancel_reservation(&self, url: String, session: String) -> BackendFuture<ReserveResult>;
    fn edit_options(&self, url: String, session: String) -> BackendFuture<EditOptions>;
    fn edit_reservation(&self, url: String, session: String, overrides: EditOverrides) -> BackendFuture<ReserveResult>;
    /// MUTATING, only after the reader confirms (§5.12): renews the loan whose "Renew loan" action
    /// is `url`. Takes a few seconds; `ok` says whether the due date moved.
    fn renew_loan(&self, _url: String, _session: String) -> BackendFuture<RenewResult> {
        ok(RenewResult { ok: false, message: None, due: None })
    }

    // ---- My Picks (§5.11; needs a session, not cached) ----

    /// The Pick list as a results list: `url` `None` for the first page, else its `next_page_url`.
    fn picks(&self, _session: String, _url: Option<String>) -> BackendFuture<SearchResults> {
        ok(SearchResults { query: Some(String::new()), total_results: Some(0), no_results: true, ..Default::default() })
    }
    /// MUTATING, only on the reader's click: returns the list's new size.
    fn save_pick(&self, _irn: String, _session: String) -> BackendFuture<i64> {
        ok(0)
    }
    /// MUTATING, only on the reader's click: returns how many remain.
    fn remove_pick(&self, _irn: String, _session: String) -> BackendFuture<i64> {
        ok(0)
    }
}

/// An answer that's ready now.
pub fn ok<T: Send + 'static>(value: T) -> BackendFuture<T> {
    futures::future::ready(Ok(value)).boxed()
}

/// A failure that's ready now.
pub fn err<T: Send + 'static>(error: SpydusError) -> BackendFuture<T> {
    futures::future::ready(Err(error)).boxed()
}
