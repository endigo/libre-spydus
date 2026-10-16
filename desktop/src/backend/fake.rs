//! An offline `LibraryBackend` for tests (iOS `TestSupport.swift`): canned public reads, a fake
//! account, and a record of every call. It never touches the network or the core's runtime, so it
//! runs on GPUI's test executor.
//!
//! `login` accepts any username with the password `ok`. `hold_snapshots(true)` parks each
//! `account_snapshot` until the test answers it with `release_snapshot`, to test what happens
//! while a fetch is in flight.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard};

use futures::FutureExt;
use futures::channel::oneshot;
use librespydus_core::app::ACCOUNT_SNAPSHOT_VERSION;
use librespydus_core::{
    AccountLinks, AccountSnapshot, Availability, EditOptions, EditOverrides, PatronAction, PatronItem, PatronList,
    RecordDetail, RecordFormat, RelatedList, RenewResult, Reservations, ReserveOptions, ReserveResult, SearchRecord,
    SearchResults, SearchSort, SpydusError,
};

use super::{BackendFuture, LibraryBackend, err, ok};

pub const FAKE_SESSION: &str = "fake-session";
pub const FAKE_PASSWORD: &str = "ok";

#[derive(Default)]
struct State {
    verification_cookies: Vec<Option<String>>,
    searches: Vec<String>,
    logins: Vec<String>,
    snapshot_calls: usize,
    hold_snapshots: bool,
    held: VecDeque<oneshot::Sender<Result<AccountSnapshot, SpydusError>>>,
    picks: Vec<String>,
    pick_calls: Vec<(String, bool)>,
}

pub struct FakeBackend {
    origin: String,
    state: Mutex<State>,
    pub search_result: Mutex<SearchResults>,
    /// The account's name (`Err`: the account call fails with it).
    pub account_result: Mutex<Result<AccountLinks, SpydusError>>,
    /// Thrown by `login` instead of its answer (a lost connection, say).
    pub login_error: Mutex<Option<SpydusError>>,
    /// What `account_snapshot` answers when snapshots aren't held (`None`: from `account_result`).
    pub snapshot_result: Mutex<Option<Result<AccountSnapshot, SpydusError>>>,
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
            state: Mutex::default(),
            search_result: Mutex::new(results),
            account_result: Mutex::new(Ok(AccountLinks { name: Some("Fake Borrower".into()), ..Default::default() })),
            login_error: Mutex::new(None),
            snapshot_result: Mutex::new(None),
        })
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn verification_cookies(&self) -> Vec<Option<String>> {
        self.state().verification_cookies.clone()
    }

    pub fn searches(&self) -> Vec<String> {
        self.state().searches.clone()
    }

    pub fn logins(&self) -> Vec<String> {
        self.state().logins.clone()
    }

    /// How many times `account_snapshot` was called.
    pub fn snapshot_calls(&self) -> usize {
        self.state().snapshot_calls
    }

    /// `true`: every later `account_snapshot` waits for `release_snapshot`.
    pub fn hold_snapshots(&self, hold: bool) {
        self.state().hold_snapshots = hold;
    }

    /// How many `account_snapshot` calls are waiting.
    pub fn held_snapshots(&self) -> usize {
        self.state().held.len()
    }

    /// Answers the oldest waiting `account_snapshot`; false when none waits.
    pub fn release_snapshot(&self, result: Result<AccountSnapshot, SpydusError>) -> bool {
        let sender = self.state().held.pop_front();
        sender.is_some_and(|sender| sender.send(result).is_ok())
    }

    /// The fake Pick list's IRNs, and every save (true) / remove (false) asked for.
    pub fn picks_list(&self) -> Vec<String> {
        self.state().picks.clone()
    }

    pub fn pick_calls(&self) -> Vec<(String, bool)> {
        self.state().pick_calls.clone()
    }

    fn default_snapshot(&self) -> Result<AccountSnapshot, SpydusError> {
        let account = self.account_result.lock().unwrap_or_else(|p| p.into_inner());
        match &*account {
            Ok(links) => Ok(test_data::snapshot(links.name.as_deref(), &[], &[])),
            Err(error) => Err(clone_error(error)),
        }
    }
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

impl LibraryBackend for FakeBackend {
    fn origin(&self) -> String {
        self.origin.clone()
    }

    fn set_verification_cookie(&self, cookie: Option<String>) {
        self.state().verification_cookies.push(cookie);
    }

    fn search(&self, query: String) -> BackendFuture<SearchResults> {
        self.state().searches.push(query);
        ok(self.search_result.lock().unwrap_or_else(|p| p.into_inner()).clone())
    }

    fn search_sorted(&self, query: String, _sort: SearchSort) -> BackendFuture<SearchResults> {
        self.search(query)
    }

    fn page(&self, _url: String) -> BackendFuture<SearchResults> {
        ok(self.search_result.lock().unwrap_or_else(|p| p.into_inner()).clone())
    }

    fn availability(&self, _url: String) -> BackendFuture<Availability> {
        err(SpydusError::Upstream { message: "fake".into() })
    }

    fn record(&self, _url: String) -> BackendFuture<RecordDetail> {
        err(SpydusError::Upstream { message: "fake".into() })
    }

    fn related(&self, _url: String) -> BackendFuture<RelatedList> {
        ok(RelatedList::default())
    }

    fn login(&self, username: String, password: String) -> BackendFuture<String> {
        self.state().logins.push(username);
        if let Some(error) = &*self.login_error.lock().unwrap_or_else(|p| p.into_inner()) {
            return err(clone_error(error));
        }
        if password != FAKE_PASSWORD {
            return err(SpydusError::CredentialsRejected);
        }
        ok(FAKE_SESSION.to_owned())
    }

    fn account(&self, _session: String) -> BackendFuture<AccountLinks> {
        match &*self.account_result.lock().unwrap_or_else(|p| p.into_inner()) {
            Ok(links) => ok(links.clone()),
            Err(error) => err(clone_error(error)),
        }
    }

    fn loans(&self, _session: String) -> BackendFuture<PatronList> {
        ok(PatronList { total: Some(0), items: Vec::new() })
    }

    fn reservations(&self, _session: String) -> BackendFuture<Reservations> {
        ok(Reservations::default())
    }

    fn account_snapshot(&self, _session: String) -> BackendFuture<AccountSnapshot> {
        let mut state = self.state();
        state.snapshot_calls += 1;
        if state.hold_snapshots {
            let (sender, receiver) = oneshot::channel();
            state.held.push_back(sender);
            return receiver
                .map(|answer| answer.unwrap_or(Err(SpydusError::Network { message: "fake: dropped".into() })))
                .boxed();
        }
        drop(state);
        let canned =
            self.snapshot_result.lock().unwrap_or_else(|p| p.into_inner()).as_ref().map(|result| match result {
                Ok(snapshot) => Ok(snapshot.clone()),
                Err(error) => Err(clone_error(error)),
            });
        futures::future::ready(canned.unwrap_or_else(|| self.default_snapshot())).boxed()
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
        ok(ReserveResult { ok: false, message: None })
    }

    fn cancel_reservation(&self, _url: String, _session: String) -> BackendFuture<ReserveResult> {
        ok(ReserveResult { ok: false, message: None })
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
        ok(ReserveResult { ok: false, message: None })
    }

    fn renew_loan(&self, _url: String, _session: String) -> BackendFuture<RenewResult> {
        ok(RenewResult { ok: false, message: None, due: None })
    }

    /// The saved IRNs as one page of cards.
    fn picks(&self, _session: String, _url: Option<String>) -> BackendFuture<SearchResults> {
        let picks = self.state().picks.clone();
        let records: Vec<SearchRecord> = picks
            .iter()
            .map(|irn| SearchRecord {
                work_irn: Some(irn.clone()),
                title: Some(format!("Pick {irn}")),
                ..Default::default()
            })
            .collect();
        ok(SearchResults {
            total_results: Some(records.len() as i64),
            no_results: records.is_empty(),
            records,
            ..Default::default()
        })
    }

    fn save_pick(&self, irn: String, _session: String) -> BackendFuture<i64> {
        let mut state = self.state();
        state.pick_calls.push((irn.clone(), true));
        if !state.picks.contains(&irn) {
            state.picks.push(irn);
        }
        ok(state.picks.len() as i64)
    }

    fn remove_pick(&self, irn: String, _session: String) -> BackendFuture<i64> {
        let mut state = self.state();
        state.pick_calls.push((irn.clone(), false));
        state.picks.retain(|saved| *saved != irn);
        ok(state.picks.len() as i64)
    }
}

/// Canned data for tests (iOS `TestData`).
pub mod test_data {
    use super::*;

    /// An account snapshot with these loan titles (and reservation titles, all pending).
    pub fn snapshot(name: Option<&str>, loans: &[&str], reservations: &[&str]) -> AccountSnapshot {
        snapshot_at(name, loans, reservations, chrono::Utc::now().timestamp_millis())
    }

    pub fn snapshot_at(
        name: Option<&str>,
        loans: &[&str],
        reservations: &[&str],
        fetched_at_ms: i64,
    ) -> AccountSnapshot {
        fn items(titles: &[&str], prefix: &str) -> PatronList {
            PatronList {
                total: Some(titles.len() as i64),
                items: titles
                    .iter()
                    .enumerate()
                    .map(|(index, title)| PatronItem {
                        irn: Some(format!("{prefix}{index}")),
                        title: Some((*title).to_owned()),
                        actions: if prefix == "r" {
                            vec![PatronAction {
                                label: "Cancel reservation".into(),
                                href: format!("https://canning.spydus.com/cancel?SVL={index}"),
                            }]
                        } else {
                            Vec::new()
                        },
                        ..Default::default()
                    })
                    .collect(),
            }
        }
        AccountSnapshot {
            version: ACCOUNT_SNAPSHOT_VERSION,
            account_name: name.map(str::to_owned),
            loans: items(loans, "l"),
            reservations: Reservations {
                ready: PatronList { total: Some(0), items: Vec::new() },
                pending: items(reservations, "r"),
            },
            fetched_at_ms,
            card_number: None,
            card_expires: None,
        }
    }

    pub const RESERVE_URL: &str = "https://canning.spydus.com/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/78529863/0/V/0/R/9/ALLWRKENQ?SVL=15974354&RECFMT=BK";

    pub fn search_record() -> SearchRecord {
        SearchRecord {
            rank: Some(1),
            work_irn: Some("15974354".into()),
            title: Some("Baby-Sitter's Club graphic novels. 18, Jessi Ramsey, pet-sitter".into()),
            detail_url: Some(
                "https://canning.spydus.com/cgi-bin/spydus.exe/FULL/OPAC/ALLWRKENQ/78529863/15974354,1".into(),
            ),
            author: Some("Crenshaw, Ellen T.".into()),
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
                    availability_url: Some(
                        "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/78529863?FMT=RECFMT&QRY=BK"
                            .into(),
                    ),
                },
                RecordFormat {
                    code: "EBK".into(),
                    label: Some("eBook".into()),
                    reserve_url: None,
                    availability_url: Some(
                        "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/78529863?FMT=RECFMT&QRY=EBK"
                            .into(),
                    ),
                },
            ],
            ..Default::default()
        }
    }
}
