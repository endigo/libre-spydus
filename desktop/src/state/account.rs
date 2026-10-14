//! The signed-in account (name, loans, reservations), stale-while-revalidate (docs/native-apps.md
//! §5.2a; a port of iOS `AccountStore.swift`). `AppState` owns it and points it at the current
//! origin and session; screens show `snapshot()` at once (the saved one at launch) while
//! `AppState::refresh_account` fetches a new one in the background.
//!
//! One fetch runs at a time and callers share it. A result for an origin or session that is no
//! longer current is dropped, and so is one that may predate a reserve / cancel / edit / renew
//! (`AppState::perform_account_change`, `account_did_change`). A failed refresh keeps the
//! snapshot and sets `last_refresh_error`.

use std::future::Future;
use std::sync::Arc;

use chrono::{DateTime, TimeZone, Utc};
use gpui_kit::{AsyncApp, Context, Task, WeakEntity};
use librespydus_core::{AccountSnapshot, SpydusError};

use crate::backend::LibraryBackend;
use crate::state::AppState;
use crate::state::account_cache::AccountSnapshotCache;
use crate::state::error::UserFacingError;
use crate::state::running::{Done, Running};

/// Older than this (seconds), opening a screen or the app coming back fetches a new snapshot.
pub const STALE_AFTER_SECONDS: i64 = 2 * 60;

#[derive(Clone)]
struct AccountRef {
    origin: String,
    session: String,
    backend: Arc<dyn LibraryBackend>,
}

pub struct AccountStore {
    snapshot: Option<AccountSnapshot>,
    is_refreshing: bool,
    last_refresh_error: Option<UserFacingError>,
    account: Option<AccountRef>,
    /// `None`: in memory only (demo mode).
    cache: Option<AccountSnapshotCache>,
    now: Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>,
    refresh: Option<Running>,
    /// Bumped when the origin or session changes: a fetch for the old one is dropped.
    generation: u64,
    /// Bumped when a change to the account starts or ends: a fetch from before is dropped.
    version: u64,
    changes_in_flight: u32,
    /// Fetch again once the changes in flight have ended.
    refetch_after_changes: bool,
    /// The running fetch may be out of date: fetch again when it ends.
    fetch_again: bool,
}

impl AccountStore {
    pub fn new(cache: Option<AccountSnapshotCache>, now: Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>) -> Self {
        Self {
            snapshot: None,
            is_refreshing: false,
            last_refresh_error: None,
            account: None,
            cache,
            now,
            refresh: None,
            generation: 0,
            version: 0,
            changes_in_flight: 0,
            refetch_after_changes: false,
            fetch_again: false,
        }
    }

    /// The account as last fetched (or saved on this computer); `None` before the first one.
    pub fn snapshot(&self) -> Option<&AccountSnapshot> {
        self.snapshot.as_ref()
    }

    pub fn is_refreshing(&self) -> bool {
        self.is_refreshing
    }

    /// Why the last refresh failed; `None` after a success. With a snapshot the screens keep
    /// showing it ("Couldn’t refresh · Showing the saved list").
    pub fn last_refresh_error(&self) -> Option<&UserFacingError> {
        self.last_refresh_error.as_ref()
    }

    pub fn persists(&self) -> bool {
        self.cache.is_some()
    }

    /// When the snapshot was fetched.
    pub fn updated_at(&self) -> Option<DateTime<Utc>> {
        self.snapshot.as_ref().and_then(|snapshot| Utc.timestamp_millis_opt(snapshot.fetched_at_ms).single())
    }

    /// Whether opening a screen should fetch: nothing yet, the last refresh failed, or the
    /// snapshot is more than two minutes old (or from the future).
    pub fn needs_refresh(&self) -> bool {
        if self.account.is_none() {
            return false;
        }
        let Some(updated_at) = self.updated_at().filter(|_| self.last_refresh_error.is_none()) else {
            return true;
        };
        let age = ((self.now)() - updated_at).num_seconds();
        !(-STALE_AFTER_SECONDS..STALE_AFTER_SECONDS).contains(&age)
    }

    /// The current library and session (library change, launch, sign-in). Shows `origin`'s saved
    /// snapshot straight away; a library's saved data stays when another is selected. `None` for
    /// `origin` (no library yet) forgets the account without touching saved files. A new session
    /// at the same library starts afresh, unless `same_account`.
    pub fn use_account(
        &mut self,
        origin: Option<&str>,
        session: Option<&str>,
        backend: Arc<dyn LibraryBackend>,
        same_account: bool,
    ) {
        let Some(origin) = origin else {
            self.reset();
            self.account = None;
            return;
        };
        let Some(session) = session else {
            self.clear(origin);
            return;
        };
        if let Some(account) = &mut self.account
            && account.origin == origin
            && account.session == session
        {
            account.backend = backend;
            return;
        }
        let new_session = self.account.as_ref().is_some_and(|account| account.origin == origin);
        let kept = if new_session && same_account { self.snapshot.clone() } else { None };
        self.reset();
        self.account = Some(AccountRef { origin: origin.to_owned(), session: session.to_owned(), backend });
        if kept.is_some() {
            self.snapshot = kept;
        } else if new_session {
            // Signed in again at the same library, maybe with another card: the old data goes.
            if let Some(cache) = &self.cache {
                cache.remove(origin);
            }
        } else {
            self.snapshot = self.cache.as_ref().and_then(|cache| cache.load(origin));
        }
    }

    /// Signed out or the session expired: forgets `origin`'s account, on this computer too.
    pub fn clear(&mut self, origin: &str) {
        self.reset();
        self.account = None;
        if let Some(cache) = &self.cache {
            cache.remove(origin);
        }
    }

    /// Edits the snapshot in place (a cancelled reservation goes at once) until the next fetch
    /// replaces it; saved like a fetched one.
    pub fn update(&mut self, edit: impl FnOnce(&mut AccountSnapshot)) {
        let (Some(snapshot), Some(account)) = (&mut self.snapshot, &self.account) else { return };
        edit(snapshot);
        let (snapshot, origin) = (snapshot.clone(), account.origin.clone());
        self.save(&snapshot, &origin);
    }

    fn save(&self, snapshot: &AccountSnapshot, origin: &str) {
        // Losing the saved copy only costs a slower next launch; the screens still have it.
        if let Some(cache) = &self.cache
            && let Err(error) = cache.save(snapshot, origin)
        {
            log::warn!("couldn't save the account: {error}");
        }
    }

    /// Forgets the in-memory account and lets a running fetch finish unseen.
    fn reset(&mut self) {
        self.generation += 1;
        self.refresh = None;
        self.is_refreshing = false;
        self.fetch_again = false;
        self.refetch_after_changes = false;
        self.snapshot = None;
        self.last_refresh_error = None;
    }
}

enum Step {
    Fetch(futures::future::BoxFuture<'static, Result<AccountSnapshot, SpydusError>>, u64),
    /// No account any more: end the refresh.
    Finish,
    /// Another origin or session took over (and owns the flags now).
    Stop,
}

enum Next {
    Again,
    Done,
    Stop,
}

impl AppState {
    /// Fetches the account snapshot now (`force`) or only when it's out of date; joins the fetch
    /// already running instead of starting another. The task ends once the refresh has settled.
    pub fn refresh_account(&mut self, force: bool, cx: &mut Context<Self>) -> Task<()> {
        if let Some(running) = &self.account.refresh {
            let wait = running.wait();
            return cx.spawn(async move |_, _| wait.await);
        }
        if self.account.account.is_none() || !(force || self.account.needs_refresh()) {
            return Task::ready(());
        }
        let wait = self.start_account_refresh(cx);
        cx.spawn(async move |_, _| wait.await)
    }

    /// A reservation was placed (or the account otherwise changed outside Your Books): a fetch in
    /// flight may predate it, so it's dropped and the account fetched again.
    pub fn account_did_change(&mut self, cx: &mut Context<Self>) {
        self.account.version += 1;
        self.fetch_account_again_now(cx);
    }

    /// Runs a reserve / cancel / edit / renew. A fetch that overlaps it may not include it, so its
    /// result is dropped; once the library has `accepted` the change, the account is fetched again.
    pub fn perform_account_change<T: 'static>(
        &mut self,
        change: impl Future<Output = Result<T, SpydusError>> + 'static,
        accepted: impl FnOnce(&T) -> bool + 'static,
        cx: &mut Context<Self>,
    ) -> Task<Result<T, SpydusError>> {
        let generation = self.account.generation;
        self.account.changes_in_flight += 1;
        self.account.version += 1;
        cx.spawn(async move |this, cx| {
            let result = change.await;
            let changed = result.as_ref().is_ok_and(accepted);
            let _ = this.update(cx, |this, cx| this.end_account_change(changed, generation, cx));
            result
        })
    }

    /// Edits the saved account in place until the next fetch (`AccountStore::update`).
    pub fn update_account_snapshot(&mut self, edit: impl FnOnce(&mut AccountSnapshot), cx: &mut Context<Self>) {
        self.account.update(edit);
        cx.notify();
    }

    fn end_account_change(&mut self, changed: bool, generation: u64, cx: &mut Context<Self>) {
        let store = &mut self.account;
        store.changes_in_flight = store.changes_in_flight.saturating_sub(1);
        store.version += 1;
        if changed && generation == store.generation {
            store.refetch_after_changes = true;
        }
        if store.changes_in_flight > 0 || !store.refetch_after_changes {
            return;
        }
        store.refetch_after_changes = false;
        self.fetch_account_again_now(cx);
    }

    /// A fresh fetch, even while one is running (that one may be out of date).
    fn fetch_account_again_now(&mut self, cx: &mut Context<Self>) {
        if self.account.account.is_none() {
            return;
        }
        if self.account.refresh.is_some() {
            self.account.fetch_again = true;
        } else {
            drop(self.start_account_refresh(cx));
        }
    }

    fn start_account_refresh(&mut self, cx: &mut Context<Self>) -> impl Future<Output = ()> + Send + use<> {
        let generation = self.account.generation;
        self.account.is_refreshing = true;
        let (done, running) = Running::new();
        self.account.refresh = Some(running.clone());
        cx.spawn(async move |this, cx| run_account_refresh(this, generation, done, cx).await).detach();
        cx.notify();
        running.wait()
    }

    fn apply_account_result(
        &mut self,
        generation: u64,
        version: u64,
        result: Result<AccountSnapshot, SpydusError>,
        cx: &mut Context<Self>,
    ) -> Next {
        if generation != self.account.generation {
            return Next::Stop;
        }
        let store = &mut self.account;
        if version != store.version || store.changes_in_flight > 0 {
            // Possibly from before a change: fetch again, now or once the changes end.
            if store.changes_in_flight > 0 {
                store.refetch_after_changes = true;
            } else {
                store.fetch_again = true;
            }
            return if store.fetch_again { Next::Again } else { Next::Done };
        }
        match result {
            Ok(snapshot) => {
                let origin = store.account.as_ref().map(|account| account.origin.clone());
                if let Some(origin) = origin {
                    store.save(&snapshot, &origin);
                }
                store.snapshot = Some(snapshot);
                store.last_refresh_error = None;
            }
            Err(error) => {
                let shown = self.report(&error, cx);
                // `NotLoggedIn` signed out (a new generation).
                if generation != self.account.generation {
                    return Next::Stop;
                }
                self.account.last_refresh_error = Some(shown);
            }
        }
        cx.notify();
        if self.account.fetch_again { Next::Again } else { Next::Done }
    }
}

async fn run_account_refresh(this: WeakEntity<AppState>, generation: u64, done: Done, cx: &mut AsyncApp) {
    loop {
        let step = this.update(cx, |state, _| {
            let store = &mut state.account;
            store.fetch_again = false;
            if generation != store.generation {
                return Step::Stop;
            }
            match &store.account {
                None => Step::Finish,
                Some(account) => Step::Fetch(account.backend.account_snapshot(account.session.clone()), store.version),
            }
        });
        let (fetch, version) = match step {
            Ok(Step::Fetch(fetch, version)) => (fetch, version),
            Ok(Step::Finish) => break,
            Ok(Step::Stop) | Err(_) => return,
        };
        let result = fetch.await;
        match this.update(cx, |state, cx| state.apply_account_result(generation, version, result, cx)) {
            Ok(Next::Again) => continue,
            Ok(Next::Done) => break,
            Ok(Next::Stop) | Err(_) => return,
        }
    }
    let _ = this.update(cx, |state, cx| {
        if state.account.generation == generation {
            state.account.refresh = None;
            state.account.is_refreshing = false;
            cx.notify();
        }
    });
    drop(done);
}
