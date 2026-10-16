//! The real backend: one `SpydusClient` for one library origin (the desktop app's
//! `backend/core.rs`, without the timing log and saved pages).

use std::sync::Arc;

use librespydus_core::{
    AccountLinks, AccountSnapshot, Availability, EditOptions, EditOverrides, Holding, RecordDetail, RenewResult,
    ReserveOptions, ReserveResult, SearchResults, SearchSort, SpydusClient,
};

use super::{BackendFuture, LibraryBackend};

pub struct CoreBackend {
    client: Arc<SpydusClient>,
    origin: String,
}

impl CoreBackend {
    /// `origin` is a `LibraryInfo.origin` (`None`: Canning).
    pub fn new(origin: Option<&str>) -> Self {
        let client = SpydusClient::new(origin.map(str::to_owned));
        let origin = client.origin();
        Self { client, origin }
    }
}

impl LibraryBackend for CoreBackend {
    fn origin(&self) -> String {
        self.origin.clone()
    }

    fn set_verification_cookie(&self, cookie: Option<String>) {
        self.client.set_verification_cookie(cookie);
    }

    fn search_sorted(&self, query: String, sort: SearchSort) -> BackendFuture<SearchResults> {
        let c = self.client.clone();
        Box::pin(async move { c.search_sorted(query, sort).await })
    }

    fn page(&self, url: String) -> BackendFuture<SearchResults> {
        let c = self.client.clone();
        Box::pin(async move { c.page(url).await })
    }

    fn availability(&self, url: String) -> BackendFuture<Availability> {
        let c = self.client.clone();
        Box::pin(async move { c.availability(url).await })
    }

    fn record(&self, url: String) -> BackendFuture<RecordDetail> {
        let c = self.client.clone();
        Box::pin(async move { c.record(url).await })
    }

    fn holdings(&self, url: String) -> BackendFuture<Vec<Holding>> {
        let c = self.client.clone();
        Box::pin(async move { c.holdings(url).await })
    }

    fn login(&self, username: String, password: String) -> BackendFuture<String> {
        let c = self.client.clone();
        Box::pin(async move { c.login(username, password).await })
    }

    fn account(&self, session: String) -> BackendFuture<AccountLinks> {
        let c = self.client.clone();
        Box::pin(async move { c.account(session).await })
    }

    fn account_snapshot(&self, session: String) -> BackendFuture<AccountSnapshot> {
        let c = self.client.clone();
        Box::pin(async move { c.account_snapshot(session).await })
    }

    fn reserve_options(&self, url: String, session: String) -> BackendFuture<ReserveOptions> {
        let c = self.client.clone();
        Box::pin(async move { c.reserve_options(url, session).await })
    }

    fn place_reservation(
        &self,
        url: String,
        session: String,
        xdays: Option<String>,
        pickup: Option<String>,
    ) -> BackendFuture<ReserveResult> {
        let c = self.client.clone();
        Box::pin(async move { c.place_reservation(url, session, xdays, pickup).await })
    }

    fn cancel_reservation(&self, url: String, session: String) -> BackendFuture<ReserveResult> {
        let c = self.client.clone();
        Box::pin(async move { c.cancel_reservation(url, session).await })
    }

    fn edit_options(&self, url: String, session: String) -> BackendFuture<EditOptions> {
        let c = self.client.clone();
        Box::pin(async move { c.edit_options(url, session).await })
    }

    fn edit_reservation(&self, url: String, session: String, overrides: EditOverrides) -> BackendFuture<ReserveResult> {
        let c = self.client.clone();
        Box::pin(async move { c.edit_reservation(url, session, overrides).await })
    }

    fn renew_loan(&self, url: String, session: String) -> BackendFuture<RenewResult> {
        let c = self.client.clone();
        Box::pin(async move { c.renew_loan(url, session).await })
    }

    fn picks(&self, session: String, url: Option<String>) -> BackendFuture<SearchResults> {
        let c = self.client.clone();
        Box::pin(async move { c.picks(session, url).await })
    }

    fn save_pick(&self, irn: String, session: String) -> BackendFuture<i64> {
        let c = self.client.clone();
        Box::pin(async move { c.save_pick(irn, session).await })
    }

    fn remove_pick(&self, irn: String, session: String) -> BackendFuture<i64> {
        let c = self.client.clone();
        Box::pin(async move { c.remove_pick(irn, session).await })
    }
}
