//! Demo mode (`--demo`; docs/cli.md §6, docs/native-apps.md §5.4): a port of the desktop app's
//! `backend/demo.rs` (itself a port of iOS `DemoBackend.swift` and `DemoData.swift`), minus the
//! saved pages and the Featured collections the CLI doesn't show.
//!
//! Public reads (search, page, availability, record, copies) go to a real backend: public GETs
//! are allowed. The demo account's own books are the exception: the demo catalogue answers for
//! them offline, the way the OPAC does. Every account call is answered from the iOS demo fixtures
//! (`ios/LibreSpydus/Resources/DemoData/*.json`, included at build time, so there is one copy)
//! and never touches the network.
//!
//! `login("demo", "demo")` succeeds, anything else is `CredentialsRejected`. Mutations succeed
//! after `DemoDelays::mutation` (300 ms) and update the in-memory lists: cancel removes the row,
//! reserve adds a pending row, edit changes the pickup location. A renewal moves the loan's due
//! date 21 days later, except for IRN `18000002`, which the "library" refuses. Changes last for
//! the one run.
//!
//! My Picks is an in-memory list seeded with two demo records ("The Midnight Library" as a work,
//! "The Love Library" as an edition-level card); `save_pick` adds a card for any IRN this backend
//! has seen, `remove_pick` takes it out, and an emptied list is a genuine `no_results` page.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use chrono::{Days, NaiveDate, Utc};
use librespydus_core::app::ACCOUNT_SNAPSHOT_VERSION;
use librespydus_core::{
    AccountLinks, AccountSnapshot, Availability, EditOptions, EditOverrides, Holding, PatronAction, PatronItem,
    PatronList, RecordDetail, RecordFormat, RenewResult, Reservations, ReserveOptions, ReserveResult, SearchRecord,
    SearchResults, SearchSort, SpydusError,
};
use serde::Deserialize;

use super::demo_catalogue::{self, DemoCatalogue};
use super::{BackendFuture, LibraryBackend, ready};
use crate::formatting::{due_date, non_empty};
use crate::patron::{DUE, PICKUP_LOCATION, PatronItemExt};

pub const USERNAME: &str = "demo";
pub const PASSWORD: &str = "demo";
pub const SESSION: &str = "demo-session";
/// The demo account's library card number and expiry, in its snapshot like a real account's
/// Personal details.
pub const CARD_NUMBER: &str = "DEMO0012345";
pub const CARD_EXPIRES: &str = "12 Mar 2027";
/// The demo Pick list's first entries: a work ("The Midnight Library") and an edition ("The Love
/// Library", edition-level like a Featured list's card).
pub const SEED_PICK_WORK: &str = "15642035";
pub const SEED_PICK_EDITION: &str = "19242648";
/// Records per Pick list page, as the library's `NRECS=20`.
pub const PICKS_PAGE_SIZE: usize = 20;
/// The demo loan the library won't renew: "Whistler", due in 2 days, reserved by another borrower.
pub const UNRENEWABLE_LOAN_IRN: &str = "18000002";
/// The library's reason for not renewing it, in its own words.
pub const RENEW_REFUSAL: &str = "This item cannot be renewed as it is reserved by another borrower.";
/// How much later a renewed demo loan is due.
pub const RENEWAL_DAYS: u64 = 21;

/// How long the demo "library" takes.
#[derive(Debug, Clone, Copy)]
pub struct DemoDelays {
    pub mutation: Duration,
}

impl Default for DemoDelays {
    fn default() -> Self {
        Self { mutation: Duration::from_millis(300) }
    }
}

impl DemoDelays {
    /// No waiting (tests).
    pub const NONE: DemoDelays = DemoDelays { mutation: Duration::ZERO };
}

/// The account fixtures `DemoBackend` serves, decoded from `ios/LibreSpydus/Resources/DemoData/`.
#[derive(Debug, Clone)]
pub struct DemoFixtures {
    pub account: AccountLinks,
    pub loans: Vec<PatronItem>,
    pub ready: Vec<PatronItem>,
    pub pending: Vec<PatronItem>,
    pub reserve_options: ReserveOptions,
    pub edit_options: EditOptions,
    pub catalogue: DemoCatalogue,
}

/// The origin every URL in the bundled JSON uses; rewritten to the demo backend's origin.
pub const FIXTURE_ORIGIN: &str = "https://canning.spydus.com";

const ACCOUNT_JSON: &str = include_str!("../../../ios/LibreSpydus/Resources/DemoData/demo-account.json");
const LOANS_JSON: &str = include_str!("../../../ios/LibreSpydus/Resources/DemoData/demo-loans.json");
const RESERVATIONS_JSON: &str = include_str!("../../../ios/LibreSpydus/Resources/DemoData/demo-reservations.json");
const RESERVE_OPTIONS_JSON: &str =
    include_str!("../../../ios/LibreSpydus/Resources/DemoData/demo-reserve-options.json");
const EDIT_OPTIONS_JSON: &str = include_str!("../../../ios/LibreSpydus/Resources/DemoData/demo-edit-options.json");
const CATALOGUE_JSON: &str = include_str!("../../../ios/LibreSpydus/Resources/DemoData/demo-catalogue.json");

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoansDto {
    loans: Vec<LoanDto>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoanDto {
    due_in_days: i64,
    item: PatronItem,
}

impl DemoFixtures {
    /// The bundled fixtures, with URLs moved to `origin` and loan due dates set from `today`.
    pub fn bundled(origin: &str, today: NaiveDate) -> serde_json::Result<Self> {
        let moved = |json: &str| {
            if origin == FIXTURE_ORIGIN { json.to_owned() } else { json.replace(FIXTURE_ORIGIN, origin) }
        };
        let loans: LoansDto = serde_json::from_str(&moved(LOANS_JSON))?;
        let loans = loans
            .loans
            .into_iter()
            .map(|loan| {
                let mut item = loan.item;
                let due = today + chrono::Duration::days(loan.due_in_days);
                item.fields.insert(DUE.into(), due_date::format(due));
                item
            })
            .collect();
        let reservations: Reservations = serde_json::from_str(&moved(RESERVATIONS_JSON))?;
        Ok(Self {
            account: serde_json::from_str(&moved(ACCOUNT_JSON))?,
            loans,
            ready: reservations.ready.items,
            pending: reservations.pending.items,
            reserve_options: serde_json::from_str(&moved(RESERVE_OPTIONS_JSON))?,
            edit_options: serde_json::from_str(&moved(EDIT_OPTIONS_JSON))?,
            catalogue: DemoCatalogue::from_json(&moved(CATALOGUE_JSON), origin)?,
        })
    }
}

struct State {
    fixtures: DemoFixtures,
    /// Search records seen, by `RecordFormat.reserve_url`, so a demo reservation shows the right
    /// title and links to the right bib.
    reservable: HashMap<String, (SearchRecord, RecordFormat)>,
    reserved_urls: HashSet<String>,
    next_irn: u64,
    /// The demo account's Pick list, in the order saved.
    pick_list: Vec<SearchRecord>,
    /// Search records seen by `work_irn`, and records by `bib_irn`: what `save_pick` lists.
    seen_records: HashMap<String, SearchRecord>,
    seen_details: HashMap<String, RecordDetail>,
}

struct Inner {
    public: Arc<dyn LibraryBackend>,
    origin: String,
    delays: DemoDelays,
    today: NaiveDate,
    state: Mutex<State>,
}

impl Inner {
    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn remember(&self, records: &[SearchRecord]) {
        let mut state = self.state();
        for record in records {
            if let Some(irn) = non_empty(record.work_irn.as_deref()) {
                state.seen_records.insert(irn, record.clone());
            }
            for format in &record.formats {
                if let Some(reserve_url) = &format.reserve_url {
                    state.reservable.insert(reserve_url.clone(), (record.clone(), format.clone()));
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct DemoBackend {
    inner: Arc<Inner>,
}

impl DemoBackend {
    pub fn new(public: Arc<dyn LibraryBackend>, fixtures: DemoFixtures, delays: DemoDelays, today: NaiveDate) -> Self {
        let origin = public.origin();
        let pick_list = seed_picks(&fixtures.catalogue, &origin);
        Self {
            inner: Arc::new(Inner {
                public,
                origin,
                delays,
                today,
                state: Mutex::new(State {
                    fixtures,
                    reservable: HashMap::new(),
                    reserved_urls: HashSet::new(),
                    next_irn: 19_100_000,
                    pick_list,
                    seen_records: HashMap::new(),
                    seen_details: HashMap::new(),
                }),
            }),
        }
    }

    /// The demo backend the CLI uses: real public reads through `public`, the bundled fixtures.
    pub fn live(public: Arc<dyn LibraryBackend>, today: NaiveDate) -> Self {
        let fixtures = DemoFixtures::bundled(&public.origin(), today).expect("the demo fixtures decode");
        Self::new(public, fixtures, DemoDelays::default(), today)
    }

    /// The fixtures as they are now (tests).
    pub fn fixtures(&self) -> DemoFixtures {
        self.inner.state().fixtures.clone()
    }
}

async fn wait(delay: Duration) {
    if !delay.is_zero() {
        tokio::time::sleep(delay).await;
    }
}

fn check(session: &str) -> Result<(), SpydusError> {
    if session == SESSION { Ok(()) } else { Err(SpydusError::NotLoggedIn) }
}

fn validated_irn(irn: &str) -> Option<String> {
    non_empty(Some(irn)).filter(|irn| irn.bytes().all(|byte| byte.is_ascii_digit()))
}

impl LibraryBackend for DemoBackend {
    fn origin(&self) -> String {
        self.inner.origin.clone()
    }

    fn is_demo(&self) -> bool {
        true
    }

    fn set_verification_cookie(&self, cookie: Option<String>) {
        self.inner.public.set_verification_cookie(cookie);
    }

    fn note_records(&self, records: &[SearchRecord]) {
        self.inner.remember(records);
    }

    // ---- Public reads → the real backend ----

    fn search_sorted(&self, query: String, sort: SearchSort) -> BackendFuture<SearchResults> {
        let inner = self.inner.clone();
        // The lock goes before `remember` takes it again.
        let demo = inner.state().fixtures.catalogue.search(&query);
        if let Some(results) = demo {
            inner.remember(&results.records);
            return ready(Ok(results));
        }
        let fetch = inner.public.search_sorted(query, sort);
        Box::pin(async move {
            let results = fetch.await?;
            inner.remember(&results.records);
            Ok(results)
        })
    }

    fn page(&self, url: String) -> BackendFuture<SearchResults> {
        let inner = self.inner.clone();
        let fetch = inner.public.page(url);
        Box::pin(async move {
            let results = fetch.await?;
            inner.remember(&results.records);
            Ok(results)
        })
    }

    fn availability(&self, url: String) -> BackendFuture<Availability> {
        let demo = self.inner.state().fixtures.catalogue.availability(&url);
        if let Some(availability) = demo {
            return ready(Ok(availability));
        }
        self.inner.public.availability(url)
    }

    fn record(&self, url: String) -> BackendFuture<RecordDetail> {
        let inner = self.inner.clone();
        let demo = inner.state().fixtures.catalogue.record(&url);
        Box::pin(async move {
            let record = match demo {
                Some(record) => record,
                None => inner.public.record(url).await?,
            };
            if let Some(irn) = non_empty(record.bib_irn.as_deref()) {
                inner.state().seen_details.insert(irn, record.clone());
            }
            Ok(record)
        })
    }

    fn holdings(&self, url: String) -> BackendFuture<Vec<Holding>> {
        self.inner.public.holdings(url)
    }

    // ---- Account → fixtures ----

    fn login(&self, username: String, password: String) -> BackendFuture<String> {
        let delay = self.inner.delays.mutation;
        Box::pin(async move {
            wait(delay).await;
            if username.trim().to_lowercase() == USERNAME && password == PASSWORD {
                Ok(SESSION.to_owned())
            } else {
                Err(SpydusError::CredentialsRejected)
            }
        })
    }

    fn account(&self, session: String) -> BackendFuture<AccountLinks> {
        ready(check(&session).map(|()| self.inner.state().fixtures.account.clone()))
    }

    fn account_snapshot(&self, session: String) -> BackendFuture<AccountSnapshot> {
        let result = check(&session).map(|()| {
            let state = self.inner.state();
            AccountSnapshot {
                version: ACCOUNT_SNAPSHOT_VERSION,
                account_name: state.fixtures.account.name.clone(),
                loans: loans(&state.fixtures),
                reservations: reservations(&state.fixtures),
                fetched_at_ms: Utc::now().timestamp_millis(),
                card_number: Some(CARD_NUMBER.into()),
                card_expires: Some(CARD_EXPIRES.into()),
            }
        });
        ready(result)
    }

    fn reserve_options(&self, _url: String, session: String) -> BackendFuture<ReserveOptions> {
        ready(check(&session).map(|()| self.inner.state().fixtures.reserve_options.clone()))
    }

    fn place_reservation(
        &self,
        url: String,
        session: String,
        _xdays: Option<String>,
        pickup: Option<String>,
    ) -> BackendFuture<ReserveResult> {
        let inner = self.inner.clone();
        Box::pin(async move {
            check(&session)?;
            wait(inner.delays.mutation).await;
            let (record, format, irn) = {
                let mut state = inner.state();
                if !state.reserved_urls.insert(url.clone()) {
                    return Ok(ReserveResult {
                        ok: false,
                        message: Some("You already have a reservation for this title.".into()),
                    });
                }
                let seen = state.reservable.get(&url).cloned();
                let irn = state.next_irn;
                state.next_irn += 1;
                (seen.as_ref().map(|(record, _)| record.clone()), seen.map(|(_, format)| format), irn)
            };
            // Like the library's own row: the reserved edition's bib, linked through the patron's set.
            let bib_irn = edition_irn(&inner, format.as_ref()).await;
            let mut state = inner.state();
            let pickup = pickup
                .or_else(|| state.fixtures.reserve_options.pickup_location.clone())
                .unwrap_or_else(|| "Your library".into());
            let item = PatronItem {
                irn: Some(irn.to_string()),
                bib_irn: bib_irn.clone(),
                title: Some(record.as_ref().and_then(|r| r.title.clone()).unwrap_or_else(|| "Reserved title".into())),
                bib_url: bib_irn
                    .as_deref()
                    .map(|irn| demo_catalogue::session_set_url(&inner.origin, irn))
                    .or_else(|| record.as_ref().and_then(|r| r.detail_url.clone())),
                author: record.as_ref().and_then(|r| r.author.clone()),
                year: record.as_ref().and_then(|r| r.year.clone()),
                cover_urls: record.as_ref().map(|r| r.cover_urls.clone()).unwrap_or_default(),
                isbn: record.as_ref().and_then(|r| r.isbn.clone()),
                fields: HashMap::from([
                    ("Status".to_owned(), "Pending".to_owned()),
                    ("Rank".to_owned(), "1".to_owned()),
                    ("Placed".to_owned(), due_date::format(inner.today)),
                    (PICKUP_LOCATION.to_owned(), pickup),
                ]),
                actions: reservation_actions(&inner.origin, irn),
            };
            state.fixtures.pending.insert(0, item);
            Ok(ReserveResult { ok: true, message: Some("Reservation placed".into()) })
        })
    }

    fn cancel_reservation(&self, url: String, session: String) -> BackendFuture<ReserveResult> {
        let inner = self.inner.clone();
        Box::pin(async move {
            check(&session)?;
            wait(inner.delays.mutation).await;
            let mut state = inner.state();
            let before = state.fixtures.ready.len() + state.fixtures.pending.len();
            state.fixtures.ready.retain(|item| !item.has_action(&url));
            state.fixtures.pending.retain(|item| !item.has_action(&url));
            if state.fixtures.ready.len() + state.fixtures.pending.len() == before {
                return Ok(ReserveResult {
                    ok: false,
                    message: Some("This reservation can’t be cancelled right now.".into()),
                });
            }
            Ok(ReserveResult { ok: true, message: Some("Reservation cancelled".into()) })
        })
    }

    fn edit_options(&self, url: String, session: String) -> BackendFuture<EditOptions> {
        let result = check(&session).map(|()| {
            let state = self.inner.state();
            let mut options = state.fixtures.edit_options.clone();
            let row = state.fixtures.ready.iter().chain(&state.fixtures.pending).find(|item| item.has_action(&url));
            if let Some(pickup) = row.and_then(|row| row.fields.get(PICKUP_LOCATION))
                && let Some(choice) =
                    options.pickup_choices.iter().find(|choice| choice.name.eq_ignore_ascii_case(pickup))
            {
                options.pickup_selected = Some(choice.code.clone());
            }
            options
        });
        ready(result)
    }

    fn edit_reservation(&self, url: String, session: String, overrides: EditOverrides) -> BackendFuture<ReserveResult> {
        let inner = self.inner.clone();
        Box::pin(async move {
            check(&session)?;
            wait(inner.delays.mutation).await;
            let mut state = inner.state();
            let pickup_name = overrides.pickup.as_ref().and_then(|code| {
                state
                    .fixtures
                    .edit_options
                    .pickup_choices
                    .iter()
                    .find(|choice| &choice.code == code)
                    .map(|c| c.name.clone())
            });
            let mut found = false;
            let fixtures = &mut state.fixtures;
            for item in fixtures.ready.iter_mut().chain(fixtures.pending.iter_mut()) {
                if item.has_action(&url) {
                    found = true;
                    if let Some(name) = &pickup_name {
                        item.fields.insert(PICKUP_LOCATION.into(), name.clone());
                    }
                }
            }
            if !found {
                return Ok(ReserveResult {
                    ok: false,
                    message: Some("This reservation can’t be edited right now.".into()),
                });
            }
            Ok(ReserveResult { ok: true, message: Some("Reservation updated".into()) })
        })
    }

    /// Moves the loan's due date 21 days later, unless it's `UNRENEWABLE_LOAN_IRN` (refused with
    /// the library's reason, the date unchanged). A link no longer on the list renews nothing.
    fn renew_loan(&self, url: String, session: String) -> BackendFuture<RenewResult> {
        let inner = self.inner.clone();
        Box::pin(async move {
            check(&session)?;
            wait(inner.delays.mutation).await;
            let mut state = inner.state();
            let Some(loan) = state.fixtures.loans.iter_mut().find(|item| item.has_action(&url)) else {
                return Ok(RenewResult {
                    ok: false,
                    message: Some("This loan isn’t on your loans list any more.".into()),
                    due: None,
                });
            };
            let due = loan.fields.get(DUE).cloned();
            if loan.irn.as_deref() == Some(UNRENEWABLE_LOAN_IRN) {
                return Ok(RenewResult { ok: false, message: Some(RENEW_REFUSAL.into()), due });
            }
            let Some(renewed) =
                due.as_deref().and_then(due_date::parse).and_then(|d| d.checked_add_days(Days::new(RENEWAL_DAYS)))
            else {
                return Ok(RenewResult {
                    ok: false, message: Some("The library didn’t renew this loan.".into()), due
                });
            };
            let text = due_date::format(renewed);
            loan.fields.insert(DUE.into(), text.clone());
            Ok(RenewResult { ok: true, message: None, due: Some(text) })
        })
    }

    // ---- My Picks → in memory ----

    fn picks(&self, session: String, url: Option<String>) -> BackendFuture<SearchResults> {
        let inner = self.inner.clone();
        Box::pin(async move {
            check(&session)?;
            let page_number = url.as_deref().and_then(picks_page).unwrap_or(1);
            let results = {
                let state = inner.state();
                let list = &state.pick_list;
                let start = ((page_number - 1) * PICKS_PAGE_SIZE).min(list.len());
                let end = (start + PICKS_PAGE_SIZE).min(list.len());
                let records = list[start..end]
                    .iter()
                    .enumerate()
                    .map(|(offset, record)| {
                        let mut record = record.clone();
                        record.rank = Some((start + offset + 1) as i64);
                        record
                    })
                    .collect();
                SearchResults {
                    query: None,
                    set_id: Some(demo_catalogue::SET_ID.into()),
                    total_results: Some(list.len() as i64),
                    showing: None,
                    next_page_url: (end < list.len()).then(|| picks_url(&inner.origin, page_number + 1)),
                    prev_page_url: (page_number > 1).then(|| picks_url(&inner.origin, page_number - 1)),
                    no_results: list.is_empty(),
                    records,
                }
            };
            inner.remember(&results.records);
            Ok(results)
        })
    }

    fn save_pick(&self, irn: String, session: String) -> BackendFuture<i64> {
        let inner = self.inner.clone();
        Box::pin(async move {
            check(&session)?;
            wait(inner.delays.mutation).await;
            if validated_irn(&irn).as_deref() != Some(irn.as_str()) {
                return Err(SpydusError::Parse { message: format!("not an IRN: {irn}") });
            }
            let mut state = inner.state();
            if !state.pick_list.iter().any(|record| record.work_irn.as_deref() == Some(irn.as_str())) {
                // The library's answer for an IRN it doesn't have.
                let card = pick_card(&state, &irn, &inner.origin)
                    .ok_or(SpydusError::Upstream { message: "None found".into() })?;
                state.pick_list.push(card);
            }
            Ok(state.pick_list.len() as i64)
        })
    }

    fn remove_pick(&self, irn: String, session: String) -> BackendFuture<i64> {
        let inner = self.inner.clone();
        Box::pin(async move {
            check(&session)?;
            wait(inner.delays.mutation).await;
            if validated_irn(&irn).as_deref() != Some(irn.as_str()) {
                return Err(SpydusError::Parse { message: format!("not an IRN: {irn}") });
            }
            let mut state = inner.state();
            state.pick_list.retain(|record| record.work_irn.as_deref() != Some(irn.as_str()));
            Ok(state.pick_list.len() as i64)
        })
    }
}

fn loans(fixtures: &DemoFixtures) -> PatronList {
    PatronList { total: Some(fixtures.loans.len() as i64), items: fixtures.loans.clone() }
}

fn reservations(fixtures: &DemoFixtures) -> Reservations {
    Reservations {
        ready: PatronList { total: Some(fixtures.ready.len() as i64), items: fixtures.ready.clone() },
        pending: PatronList { total: Some(fixtures.pending.len() as i64), items: fixtures.pending.clone() },
    }
}

/// The bib IRN `format` resolves to: the catalogue's, else the library's availability page (a
/// public GET); `None` if that fails.
async fn edition_irn(inner: &Inner, format: Option<&RecordFormat>) -> Option<String> {
    let url = non_empty(format?.availability_url.as_deref())?;
    if let Some(irn) = inner.state().fixtures.catalogue.bib_irn(&url) {
        return Some(irn);
    }
    let availability = inner.public.availability(url).await.ok()?;
    non_empty(availability.bib_record_irn.as_deref())
}

fn reservation_actions(origin: &str, irn: u64) -> Vec<PatronAction> {
    let base = format!("{origin}/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/78500000/0");
    vec![
        PatronAction { label: "Cancel reservation".into(), href: format!("{base}/C/5/RSVCENQ?SVL={irn}&SETDESC=x") },
        PatronAction { label: "Edit".into(), href: format!("{base}/ER/1/RSVCENQ?SVL={irn}&SETDESC=x") },
    ]
}

/// The card the Pick list shows for `irn`: a results card seen with that `work_irn`, a demo work,
/// else an edition-level card for a demo edition or a record seen with that `bib_irn`.
fn pick_card(state: &State, irn: &str, origin: &str) -> Option<SearchRecord> {
    if let Some(record) = state.seen_records.get(irn) {
        return Some(record.clone());
    }
    let catalogue = &state.fixtures.catalogue;
    if let Some(work) = catalogue.works.iter().find(|work| work.record.work_irn.as_deref() == Some(irn)) {
        return Some(work.record.clone());
    }
    for work in &catalogue.works {
        if let Some(edition) = work.editions.iter().find(|edition| edition.bib_irn == irn) {
            return Some(edition_card(irn, &edition.record, Some(&work.record), origin));
        }
    }
    state.seen_details.get(irn).map(|record| edition_card(irn, record, None, origin))
}

fn seed_picks(catalogue: &DemoCatalogue, origin: &str) -> Vec<SearchRecord> {
    let work = catalogue
        .works
        .iter()
        .find(|work| work.record.work_irn.as_deref() == Some(SEED_PICK_WORK))
        .map(|w| w.record.clone());
    let edition = catalogue.works.iter().find_map(|work| {
        work.editions
            .iter()
            .find(|edition| edition.bib_irn == SEED_PICK_EDITION)
            .map(|edition| edition_card(&edition.bib_irn, &edition.record, Some(&work.record), origin))
    });
    [work, edition].into_iter().flatten().collect()
}

/// An edition as Featured and Pick lists show it: its `work_irn` is the bib's IRN, and its one
/// format has no code or label, the card's reserve link and `…/BIBENQ?SETLVL=&IRN=…&FMT=RECFMT`.
pub fn edition_card(irn: &str, record: &RecordDetail, work: Option<&SearchRecord>, origin: &str) -> SearchRecord {
    let author = record
        .details
        .iter()
        .find(|row| ["author", "creator"].contains(&row.label.to_lowercase().as_str()))
        .and_then(|row| row.values.first())
        .map(|value| value.text.clone());
    let base = format!("{origin}/cgi-bin/spydus.exe");
    let set = demo_catalogue::SET_ID;
    SearchRecord {
        rank: Some(1),
        work_irn: Some(irn.to_owned()),
        title: non_empty(record.title.as_deref()).or_else(|| work.and_then(|w| w.title.clone())),
        detail_url: Some(format!("{base}/FULL/OPAC/ALLENQ/{set}/{irn},1")),
        author: work.and_then(|w| w.author.clone()).or(author),
        author_role: work.and_then(|w| w.author_role.clone()),
        author_url: None,
        year: work.and_then(|w| w.year.clone()),
        language: work.and_then(|w| w.language.clone()),
        summary: None,
        isbn: record.isbn.clone().or_else(|| work.and_then(|w| w.isbn.clone())),
        cover_urls: if record.cover_urls.is_empty() {
            work.map(|w| w.cover_urls.clone()).unwrap_or_default()
        } else {
            record.cover_urls.clone()
        },
        formats: vec![RecordFormat {
            code: String::new(),
            label: None,
            reserve_url: Some(format!("{base}/PGM/OPAC/CCOPT/{set}/0/V/0/R/9/ALLENQ?SVL={irn}")),
            availability_url: Some(format!("{base}/ENQ/OPAC/BIBENQ?SETLVL=&IRN={irn}&FMT=RECFMT")),
        }],
    }
}

/// The demo list's page `page` (the library's own list is `SVL/OPAC/ALLENQ?ISGLB=1&NRECS=20`).
pub fn picks_url(origin: &str, page: usize) -> String {
    format!("{origin}/cgi-bin/spydus.exe/SVL/OPAC/ALLENQ?ISGLB=1&NRECS={PICKS_PAGE_SIZE}&DEMOPAGE={page}")
}

fn picks_page(url: &str) -> Option<usize> {
    let query = url.split_once('?')?.1;
    query.split('&').find_map(|pair| pair.strip_prefix("DEMOPAGE=")?.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::fake::FakeBackend;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 14).unwrap()
    }

    fn backend() -> DemoBackend {
        let public = FakeBackend::new(FIXTURE_ORIGIN);
        let fixtures = DemoFixtures::bundled(FIXTURE_ORIGIN, today()).unwrap();
        DemoBackend::new(public, fixtures, DemoDelays::NONE, today())
    }

    #[test]
    fn fixtures_decode_with_due_dates_from_today() {
        let fixtures = DemoFixtures::bundled(FIXTURE_ORIGIN, today()).unwrap();
        assert_eq!(fixtures.account.name.as_deref(), Some("Test Borrower"));
        assert!(!fixtures.loans.is_empty() && !fixtures.ready.is_empty() && !fixtures.pending.is_empty());
        let first_due = fixtures.loans[0].fields.get("Due").unwrap();
        assert_eq!(due_date::parse(first_due), Some(today() - chrono::Duration::days(2)));
        assert!(!fixtures.catalogue.works.is_empty());
        let moved = DemoFixtures::bundled("https://cumberland.spydus.com", today()).unwrap();
        assert!(moved.loans[0].bib_url.as_deref().unwrap().starts_with("https://cumberland.spydus.com/"));
    }

    #[tokio::test]
    async fn only_demo_demo_signs_in() {
        let demo = backend();
        assert_eq!(demo.login(" Demo ".into(), "demo".into()).await.unwrap(), SESSION);
        assert!(matches!(demo.login("demo".into(), "nope".into()).await, Err(SpydusError::CredentialsRejected)));
        assert!(matches!(demo.account_snapshot("other-session".into()).await, Err(SpydusError::NotLoggedIn)));
        let snapshot = demo.account_snapshot(SESSION.into()).await.unwrap();
        assert_eq!(snapshot.account_name.as_deref(), Some("Test Borrower"));
        assert_eq!(snapshot.card_number.as_deref(), Some(CARD_NUMBER));
        assert_eq!(snapshot.loans.items.len(), 5);
    }

    #[tokio::test]
    async fn mutations_change_the_lists() {
        let demo = backend();
        let ready = demo.fixtures().ready[0].clone();
        let cancel = ready.cancel_href().unwrap();
        assert!(demo.cancel_reservation(cancel.clone(), SESSION.into()).await.unwrap().ok);
        assert!(!demo.fixtures().ready.iter().any(|item| item.irn == ready.irn));
        assert!(!demo.cancel_reservation(cancel, SESSION.into()).await.unwrap().ok);

        let results = demo.search_sorted("midnight library".into(), SearchSort::Relevance).await.unwrap();
        let format = results.records[0].formats[0].clone();
        let before = demo.fixtures().pending.len();
        let placed =
            demo.place_reservation(format.reserve_url.clone().unwrap(), SESSION.into(), None, None).await.unwrap();
        assert!(placed.ok);
        let pending = demo.fixtures().pending;
        assert_eq!(pending.len(), before + 1);
        assert_eq!(pending[0].title.as_deref(), Some("The Midnight Library"));
        assert_eq!(pending[0].fields.get("Pickup location").map(String::as_str), Some("RIVERTON LIBRARY"));
        assert!(!demo.place_reservation(format.reserve_url.unwrap(), SESSION.into(), None, None).await.unwrap().ok);

        let edit = pending[0].edit_href().unwrap();
        let overrides = EditOverrides { pickup: Some("25353".into()), ..Default::default() };
        assert!(demo.edit_reservation(edit.clone(), SESSION.into(), overrides).await.unwrap().ok);
        assert_eq!(
            demo.fixtures().pending[0].fields.get("Pickup location").map(String::as_str),
            Some("CANNINGTON LIBRARY")
        );
        assert_eq!(demo.edit_options(edit, SESSION.into()).await.unwrap().pickup_selected.as_deref(), Some("25353"));
    }

    #[tokio::test]
    async fn renewals_move_the_date_except_for_the_reserved_loan() {
        let demo = backend();
        let loans = demo.fixtures().loans;
        let renewable = loans.iter().find(|loan| loan.irn.as_deref() != Some(UNRENEWABLE_LOAN_IRN)).unwrap();
        let due = due_date::parse(renewable.fields.get("Due").unwrap()).unwrap();
        let result = demo.renew_loan(renewable.renew_href().unwrap(), SESSION.into()).await.unwrap();
        assert!(result.ok);
        let expected = due_date::format(due + chrono::Duration::days(21));
        assert_eq!(result.due.as_deref(), Some(expected.as_str()));
        let refused = loans.iter().find(|loan| loan.irn.as_deref() == Some(UNRENEWABLE_LOAN_IRN)).unwrap();
        let result = demo.renew_loan(refused.renew_href().unwrap(), SESSION.into()).await.unwrap();
        assert!(!result.ok);
        assert_eq!(result.message.as_deref(), Some(RENEW_REFUSAL));
        let gone = demo.renew_loan("https://canning.spydus.com/nope".into(), SESSION.into()).await.unwrap();
        assert!(!gone.ok && gone.due.is_none());
    }

    #[tokio::test]
    async fn picks_start_seeded_and_follow_saves_and_removes() {
        let demo = backend();
        let first = demo.picks(SESSION.into(), None).await.unwrap();
        let irns: Vec<_> = first.records.iter().filter_map(|r| r.work_irn.clone()).collect();
        assert_eq!(irns, [SEED_PICK_WORK, SEED_PICK_EDITION]);
        assert_eq!(demo.remove_pick(SEED_PICK_WORK.into(), SESSION.into()).await.unwrap(), 1);
        assert_eq!(demo.remove_pick(SEED_PICK_EDITION.into(), SESSION.into()).await.unwrap(), 0);
        assert!(demo.picks(SESSION.into(), None).await.unwrap().no_results);
        assert_eq!(demo.save_pick(SEED_PICK_WORK.into(), SESSION.into()).await.unwrap(), 1);
        assert!(matches!(demo.save_pick("99999999".into(), SESSION.into()).await, Err(SpydusError::Upstream { .. })));
        assert!(matches!(demo.save_pick("abc".into(), SESSION.into()).await, Err(SpydusError::Parse { .. })));
    }

    #[tokio::test]
    async fn the_catalogue_answers_for_account_books() {
        let demo = backend();
        let loan = demo.fixtures().loans[0].clone();
        let empty = demo.record(loan.bib_url.clone().unwrap()).await.unwrap();
        assert!(empty.title.is_none());
        let url = librespydus_core::app::patron_item_record_url(loan.clone()).unwrap();
        let full = demo.record(url).await.unwrap();
        assert_eq!(full.bib_irn, loan.bib_irn);
    }
}
