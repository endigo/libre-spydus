//! An offline `LibraryBackend` for tests: canned public reads and a fake account. It never
//! touches the network. `login` accepts any username with the password `ok`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use librespydus_core::app::ACCOUNT_SNAPSHOT_VERSION;
use librespydus_core::{
    AccountLinks, AccountSnapshot, Availability, EditOptions, EditOverrides, Holding, RecordDetail, RecordFormat,
    RenewResult, ReserveOptions, ReserveResult, SearchRecord, SearchResults, SearchSort, SpydusError,
};

use super::{BackendFuture, LibraryBackend, clone_error, ready};

pub const FAKE_SESSION: &str = "fake-session";
pub const FAKE_PASSWORD: &str = "ok";

pub struct FakeBackend {
    origin: String,
    pub search_result: Mutex<SearchResults>,
    /// Pages by URL (`next_page_url`s of `search_result`).
    pub pages: Mutex<HashMap<String, SearchResults>>,
    pub records: Mutex<HashMap<String, RecordDetail>>,
    pub availabilities: Mutex<HashMap<String, Availability>>,
    pub holdings: Mutex<HashMap<String, Vec<Holding>>>,
    /// What every public read fails with instead, when set.
    pub public_error: Mutex<Option<SpydusError>>,
    pub searches: Mutex<Vec<(String, SearchSort)>>,
    pub cookies: Mutex<Vec<Option<String>>>,
}

impl FakeBackend {
    pub fn new(origin: &str) -> Arc<Self> {
        Self::with_records(origin, vec![test_data::search_record()])
    }

    pub fn with_records(origin: &str, records: Vec<SearchRecord>) -> Arc<Self> {
        let results = SearchResults {
            query: Some("q".into()),
            set_id: Some("1".into()),
            total_results: Some(records.len() as i64),
            no_results: records.is_empty(),
            records,
            ..Default::default()
        };
        Arc::new(Self {
            origin: origin.trim_end_matches('/').to_owned(),
            search_result: Mutex::new(results),
            pages: Mutex::default(),
            records: Mutex::default(),
            availabilities: Mutex::default(),
            holdings: Mutex::default(),
            public_error: Mutex::new(None),
            searches: Mutex::default(),
            cookies: Mutex::default(),
        })
    }

    fn failing(&self) -> Option<SpydusError> {
        self.public_error.lock().unwrap().as_ref().map(clone_error)
    }
}

impl LibraryBackend for FakeBackend {
    fn origin(&self) -> String {
        self.origin.clone()
    }

    fn set_verification_cookie(&self, cookie: Option<String>) {
        self.cookies.lock().unwrap().push(cookie);
    }

    fn search_sorted(&self, query: String, sort: SearchSort) -> BackendFuture<SearchResults> {
        self.searches.lock().unwrap().push((query, sort));
        if let Some(error) = self.failing() {
            return ready(Err(error));
        }
        ready(Ok(self.search_result.lock().unwrap().clone()))
    }

    fn page(&self, url: String) -> BackendFuture<SearchResults> {
        if let Some(error) = self.failing() {
            return ready(Err(error));
        }
        let page = self.pages.lock().unwrap().get(&url).cloned();
        ready(page.ok_or(SpydusError::Upstream { message: format!("no page {url}") }))
    }

    fn availability(&self, url: String) -> BackendFuture<Availability> {
        if let Some(error) = self.failing() {
            return ready(Err(error));
        }
        let availability = self.availabilities.lock().unwrap().get(&url).cloned();
        ready(availability.ok_or(SpydusError::Upstream { message: format!("no availability {url}") }))
    }

    fn record(&self, url: String) -> BackendFuture<RecordDetail> {
        if let Some(error) = self.failing() {
            return ready(Err(error));
        }
        let record = self.records.lock().unwrap().get(&url).cloned();
        ready(record.ok_or(SpydusError::Upstream { message: format!("no record {url}") }))
    }

    fn holdings(&self, url: String) -> BackendFuture<Vec<Holding>> {
        let holdings = self.holdings.lock().unwrap().get(&url).cloned();
        ready(holdings.ok_or(SpydusError::Upstream { message: format!("no holdings {url}") }))
    }

    fn login(&self, _username: String, password: String) -> BackendFuture<String> {
        if password != FAKE_PASSWORD {
            return ready(Err(SpydusError::CredentialsRejected));
        }
        ready(Ok(FAKE_SESSION.to_owned()))
    }

    fn account(&self, session: String) -> BackendFuture<AccountLinks> {
        if session != FAKE_SESSION {
            return ready(Err(SpydusError::NotLoggedIn));
        }
        ready(Ok(AccountLinks { name: Some("Fake Borrower".into()), ..Default::default() }))
    }

    fn account_snapshot(&self, session: String) -> BackendFuture<AccountSnapshot> {
        if session != FAKE_SESSION {
            return ready(Err(SpydusError::NotLoggedIn));
        }
        ready(Ok(AccountSnapshot {
            version: ACCOUNT_SNAPSHOT_VERSION,
            account_name: Some("Fake Borrower".into()),
            fetched_at_ms: 0,
            ..Default::default()
        }))
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
        ready(Ok(ReserveResult { ok: false, message: None }))
    }

    fn cancel_reservation(&self, _url: String, _session: String) -> BackendFuture<ReserveResult> {
        ready(Ok(ReserveResult { ok: false, message: None }))
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
        ready(Ok(ReserveResult { ok: false, message: None }))
    }

    fn renew_loan(&self, _url: String, _session: String) -> BackendFuture<RenewResult> {
        ready(Ok(RenewResult { ok: false, message: None, due: None }))
    }

    fn picks(&self, _session: String, _url: Option<String>) -> BackendFuture<SearchResults> {
        ready(Ok(SearchResults { total_results: Some(0), no_results: true, ..Default::default() }))
    }

    fn save_pick(&self, _irn: String, _session: String) -> BackendFuture<i64> {
        ready(Ok(0))
    }

    fn remove_pick(&self, _irn: String, _session: String) -> BackendFuture<i64> {
        ready(Ok(0))
    }
}

/// Canned data for tests.
pub mod test_data {
    use super::*;

    pub const RESERVE_URL: &str = "https://canning.spydus.com/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/78529863/0/V/0/R/9/ALLWRKENQ?SVL=15974354&RECFMT=BK";
    pub const BK_AVAILABILITY_URL: &str =
        "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/78529863?FMT=RECFMT&QRY=BK";
    pub const EBK_AVAILABILITY_URL: &str =
        "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/78529863?FMT=RECFMT&QRY=EBK";
    pub const DETAIL_URL: &str =
        "https://canning.spydus.com/cgi-bin/spydus.exe/FULL/OPAC/ALLWRKENQ/78529863/15974354,1";

    pub fn search_record() -> SearchRecord {
        SearchRecord {
            rank: Some(1),
            work_irn: Some("15974354".into()),
            title: Some("Baby-Sitter's Club graphic novels. 18, Jessi Ramsey, pet-sitter".into()),
            detail_url: Some(DETAIL_URL.into()),
            author: Some("Crenshaw, Ellen T., author".into()),
            author_role: Some("author".into()),
            year: Some("2025".into()),
            language: Some("English".into()),
            isbn: Some("9781546144717".into()),
            cover_urls: vec![
                "https://secure.syndetics.com/index.aspx?isbn=9781546144717/lc.gif&client=citycanning".into(),
            ],
            formats: vec![
                RecordFormat {
                    code: "BK".into(),
                    label: Some("Book".into()),
                    reserve_url: Some(RESERVE_URL.into()),
                    availability_url: Some(BK_AVAILABILITY_URL.into()),
                },
                RecordFormat {
                    code: "EBK".into(),
                    label: Some("eBook".into()),
                    reserve_url: None,
                    availability_url: Some(EBK_AVAILABILITY_URL.into()),
                },
            ],
            ..Default::default()
        }
    }

    pub fn second_record() -> SearchRecord {
        SearchRecord {
            rank: Some(2),
            work_irn: Some("8753080".into()),
            title: Some("The midnight library".into()),
            detail_url: Some("https://canning.spydus.com/cgi-bin/spydus.exe/FULL/OPAC/ALLWRKENQ/78529863/8753080,1".into()),
            author: Some("Haig, Matt, author".into()),
            year: Some("2020".into()),
            formats: vec![RecordFormat {
                code: "BK".into(),
                label: Some("Book".into()),
                reserve_url: Some("https://canning.spydus.com/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/78529863/0/V/0/R/9/ALLWRKENQ?SVL=8753080&RECFMT=BK".into()),
                availability_url: Some("https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/78529863?FMT=RECFMT&QRY=BK2".into()),
            }],
            ..Default::default()
        }
    }
}
