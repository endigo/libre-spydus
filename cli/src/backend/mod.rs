//! Everything the CLI asks of one library: the shape of the desktop app's `LibraryBackend`
//! (`desktop/src/backend/mod.rs`), reduced to what the commands use. `CoreBackend` wraps the
//! real core, `DemoBackend` serves the demo account (`--demo`), and tests answer with a fake.
//!
//! Async methods return a `'static` boxed future so the trait stays object-safe. Errors are the
//! core's `SpydusError`. Mutations (`place_reservation`, `cancel_reservation`,
//! `edit_reservation`, `renew_loan`) return `ok: false` results as data when the library refuses;
//! that's not an error.

pub mod core;
pub mod demo;
pub mod demo_catalogue;
#[cfg(test)]
pub mod fake;

use std::future::Future;
use std::pin::Pin;

use librespydus_core::{
    AccountLinks, AccountSnapshot, Availability, EditOptions, EditOverrides, Holding, RecordDetail, RenewResult,
    ReserveOptions, ReserveResult, SearchRecord, SearchResults, SearchSort, SpydusError,
};

pub use self::core::CoreBackend;
pub use self::demo::DemoBackend;

/// A backend call's result, awaitable from the CLI's runtime.
pub type BackendFuture<T> = Pin<Box<dyn Future<Output = Result<T, SpydusError>> + Send + 'static>>;

pub trait LibraryBackend: Send + Sync + 'static {
    /// Normalised origin, e.g. `https://canning.spydus.com` (no trailing slash).
    fn origin(&self) -> String;
    /// True for `DemoBackend`.
    fn is_demo(&self) -> bool {
        false
    }
    /// `"aws-waf-token=<value>"` after human verification, or `None` to clear.
    fn set_verification_cookie(&self, cookie: Option<String>);
    /// Records the CLI printed earlier (a saved list), so a backend that keeps state per run
    /// (the demo) knows them again. Nothing by default.
    fn note_records(&self, _records: &[SearchRecord]) {}

    // ---- Public reads (GET only) ----

    fn search_sorted(&self, query: String, sort: SearchSort) -> BackendFuture<SearchResults>;
    /// Another results page: a `SearchResults.next_page_url`.
    fn page(&self, url: String) -> BackendFuture<SearchResults>;
    /// A format's `availability_url`.
    fn availability(&self, url: String) -> BackendFuture<Availability>;
    /// A format's `availability_url`, a `SearchRecord.detail_url`, or a bib URL.
    fn record(&self, url: String) -> BackendFuture<RecordDetail>;
    /// Every copy of a record: an `Availability.holdings_url`.
    fn holdings(&self, url: String) -> BackendFuture<Vec<Holding>>;

    // ---- Account (needs a session from `login`) ----

    fn login(&self, username: String, password: String) -> BackendFuture<String>;
    fn account(&self, session: String) -> BackendFuture<AccountLinks>;
    /// Name, card, loans and both reservation lists in one call.
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
    /// MUTATING, only after the reader confirms: renews the loan whose "Renew loan" action is
    /// `url`. `ok` says whether the due date moved.
    fn renew_loan(&self, url: String, session: String) -> BackendFuture<RenewResult>;

    // ---- My Picks (needs a session, not cached) ----

    /// The Pick list as a results list: `url` `None` for the first page, else its `next_page_url`.
    fn picks(&self, session: String, url: Option<String>) -> BackendFuture<SearchResults>;
    /// MUTATING: returns the list's new size.
    fn save_pick(&self, irn: String, session: String) -> BackendFuture<i64>;
    /// MUTATING: returns how many remain.
    fn remove_pick(&self, irn: String, session: String) -> BackendFuture<i64>;
}

/// An answer that's ready now.
pub fn ready<T: Send + 'static>(result: Result<T, SpydusError>) -> BackendFuture<T> {
    Box::pin(std::future::ready(result))
}

/// `SpydusError` isn't `Clone`.
pub fn clone_error(error: &SpydusError) -> SpydusError {
    match error {
        SpydusError::Network { message } => SpydusError::Network { message: message.clone() },
        SpydusError::OriginForbidden => SpydusError::OriginForbidden,
        SpydusError::CredentialsRejected => SpydusError::CredentialsRejected,
        SpydusError::NotLoggedIn => SpydusError::NotLoggedIn,
        SpydusError::Upstream { message } => SpydusError::Upstream { message: message.clone() },
        SpydusError::Parse { message } => SpydusError::Parse { message: message.clone() },
        SpydusError::HumanVerificationRequired => SpydusError::HumanVerificationRequired,
    }
}
