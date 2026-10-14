//! My Picks on the library account (docs/native-apps.md §5.11; a port of iOS `PicksStore.swift`):
//! the reader's Pick list, which the My Picks section shows while signed in. `AppState` owns one
//! and points it at the current origin and session, like the account; sign-out, an expired
//! session and a library change clear it.
//!
//! - **The list and the picked set** come from the same pages: `AppState::refresh_picks` loads the
//!   first page, then the rest in the background up to `MAX_PAGES`, collecting every record's
//!   `work_irn` into the set, so a book page knows whether it's picked.
//! - **`AppState::set_picked_irn`** flips the IRN at once, calls `save_pick` / `remove_pick`, and
//!   flips back on failure, with a notice either way. A load that overlaps a change may predate
//!   it, so the change is laid over what it brings back.
//! - **The copy-once offer** (`AppState::copy_picks`) adds this computer's picks to the account.
//!
//! Saves and removes are MUTATING: only ever started by the reader.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use gpui_kit::{AsyncApp, Context, Task, WeakEntity};
use librespydus_core::{RecordDetail, SearchResults, SpydusError};

use crate::backend::LibraryBackend;
use crate::copy::Copy;
use crate::state::book_ref::BookRef;
use crate::state::error::UserFacingError;
use crate::state::formatting::non_empty;
use crate::state::results::{SearchResultItem, merge, unrecognised_page};
use crate::state::running::Running;
use crate::state::{AppState, NoticeTone};

/// The picked set loads at most this many pages (20 records each).
pub const MAX_PAGES: usize = 10;

#[derive(Debug, Clone, PartialEq, Default)]
pub enum PicksPhase {
    /// Signed out, or not loaded yet.
    #[default]
    Idle,
    /// The first page, with nothing on screen yet.
    Loading,
    Loaded,
    /// The library's "you do not have any items in your Pick list" page.
    Empty,
    Failed(UserFacingError),
}

#[derive(Clone)]
struct PicksAccount {
    session: String,
    backend: Arc<dyn LibraryBackend>,
    origin: String,
}

#[derive(Default)]
pub struct PicksStore {
    phase: PicksPhase,
    items: Vec<SearchResultItem>,
    next_page_url: Option<String>,
    is_loading_more: bool,
    load_more_error: Option<UserFacingError>,
    irns: HashSet<String>,
    is_stale: bool,
    account: Option<PicksAccount>,
    generation: u64,
    list_generation: u64,
    first_page: Option<Running>,
    more_pages: Option<Running>,
    next_page: Option<Running>,
    requested_pages: HashSet<String>,
    pages_loaded: usize,
    loaded_irns: HashSet<String>,
    /// Saves (true) and removes (false) in flight.
    pending: HashMap<String, bool>,
    /// Saves and removes done since the current load began: its pages may predate them.
    committed: HashMap<String, bool>,
}

impl PicksStore {
    pub fn phase(&self) -> &PicksPhase {
        &self.phase
    }

    /// The list's rows, as search results show them.
    pub fn items(&self) -> &[SearchResultItem] {
        &self.items
    }

    pub fn next_page_url(&self) -> Option<&str> {
        self.next_page_url.as_deref()
    }

    pub fn is_loading_more(&self) -> bool {
        self.is_loading_more
    }

    /// The last next-page failure: the footer offers Retry.
    pub fn load_more_error(&self) -> Option<&UserFacingError> {
        self.load_more_error.as_ref()
    }

    /// A book was saved that the list doesn't show yet: My Picks loads it again when shown.
    pub fn is_stale(&self) -> bool {
        self.is_stale
    }

    /// Signed in to a library (a session to use).
    pub fn is_active(&self) -> bool {
        self.account.is_some()
    }

    /// Whether `irn` is on the list (or being saved).
    pub fn contains(&self, irn: &str) -> bool {
        self.irns.contains(irn)
    }

    /// A save or remove of `irn` is in flight.
    pub fn is_changing(&self, irn: &str) -> bool {
        self.pending.contains_key(irn)
    }

    /// The current library and session. A new origin or session starts empty; `None` for either
    /// forgets the list.
    pub fn use_account(&mut self, origin: Option<&str>, session: Option<&str>, backend: Arc<dyn LibraryBackend>) {
        if let (Some(account), Some(origin), Some(session)) = (&mut self.account, origin, session)
            && account.origin == origin
            && account.session == session
        {
            account.backend = backend;
            return;
        }
        self.reset();
        if let (Some(origin), Some(session)) = (origin, session) {
            self.account = Some(PicksAccount { session: session.to_owned(), backend, origin: origin.to_owned() });
        }
    }

    /// `base` with the changes in flight and those done since the load began laid over it.
    fn overlaid(&self, base: &HashSet<String>) -> HashSet<String> {
        let mut set = base.clone();
        let mut changes = self.committed.clone();
        changes.extend(self.pending.iter().map(|(irn, picked)| (irn.clone(), *picked)));
        for (irn, picked) in changes {
            if picked {
                set.insert(irn);
            } else {
                set.remove(&irn);
            }
        }
        set
    }

    /// A row stays unless it's being (or was, since the load began) removed.
    fn is_kept(&self, item: &SearchResultItem) -> bool {
        let Some(irn) = pick_irn(&item.book) else { return true };
        self.pending.get(&irn).or_else(|| self.committed.get(&irn)).copied().unwrap_or(true)
    }

    fn kept(&self, items: Vec<SearchResultItem>) -> Vec<SearchResultItem> {
        items.into_iter().filter(|item| self.is_kept(item)).collect()
    }

    /// A save done since the load began that the first page doesn't show: load again later.
    fn mark_stale_if_missing_saves(&mut self) {
        let shown: HashSet<String> = self.items.iter().filter_map(|item| pick_irn(&item.book)).collect();
        if self.committed.iter().any(|(irn, picked)| *picked && !shown.contains(irn)) {
            self.is_stale = true;
        }
    }

    fn update_phase(&mut self) {
        self.phase =
            if self.items.is_empty() && self.next_page_url.is_none() { PicksPhase::Empty } else { PicksPhase::Loaded };
    }

    /// Takes `irn`'s rows out of the list at once; returns them with their positions.
    fn remove_rows(&mut self, irn: &str) -> Vec<(usize, SearchResultItem)> {
        let removed: Vec<(usize, SearchResultItem)> = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| pick_irn(&item.book).as_deref() == Some(irn))
            .map(|(index, item)| (index, item.clone()))
            .collect();
        if removed.is_empty() {
            return removed;
        }
        self.items.retain(|item| pick_irn(&item.book).as_deref() != Some(irn));
        if self.phase == PicksPhase::Loaded {
            self.update_phase();
        }
        removed
    }

    fn restore(&mut self, rows: Vec<(usize, SearchResultItem)>) {
        if rows.is_empty() {
            return;
        }
        for (index, item) in rows {
            if !self.items.iter().any(|existing| existing.id == item.id) {
                let at = index.min(self.items.len());
                self.items.insert(at, item);
            }
        }
        if self.phase == PicksPhase::Empty {
            self.phase = PicksPhase::Loaded;
        }
    }

    /// Forgets the list and lets anything in flight finish unseen.
    fn reset(&mut self) {
        let generation = self.generation + 1;
        let list_generation = self.list_generation + 1;
        *self = Self { generation, list_generation, ..Default::default() };
    }
}

/// What a pick is keyed on (§5.11): a ref keyed `work:<IRN>` (a results card) or `bib:<IRN>` (a
/// loan or reservation) → that IRN; `None` for `brn:` / `url:` / `title:` refs.
pub fn pick_irn(book: &BookRef) -> Option<String> {
    ["work:", "bib:"].iter().find_map(|prefix| book.id.strip_prefix(prefix)).and_then(|irn| validated_irn(Some(irn)))
}

/// The ref's own IRN, else the loaded record's `bib_irn` (a related item or a link). `None` while
/// the record is still loading: the bookmark waits.
pub fn pick_irn_with_record(book: &BookRef, record: Option<&RecordDetail>) -> Option<String> {
    pick_irn(book).or_else(|| validated_irn(record.and_then(|record| record.bib_irn.as_deref())))
}

/// The IRN when it's all digits (the core refuses anything else), trimmed.
pub fn validated_irn(irn: Option<&str>) -> Option<String> {
    non_empty(irn).filter(|irn| irn.bytes().all(|byte| byte.is_ascii_digit()))
}

fn page_irns(page: &SearchResults) -> HashSet<String> {
    page.records.iter().filter_map(|record| validated_irn(record.work_irn.as_deref())).collect()
}

/// Errors after which the next save would fail the same way.
fn stops_copy(error: &SpydusError) -> bool {
    matches!(error, SpydusError::NotLoggedIn | SpydusError::HumanVerificationRequired | SpydusError::Network { .. })
}

/// The copy-once offer after an interactive sign-in: this many local picks for `origin`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PicksCopyOffer {
    pub origin: String,
    pub count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CopySummary {
    pub added: usize,
    pub failed: usize,
}

/// "Added 3 books to My Picks", plus " · 1 couldn’t be added".
pub fn copied_text(added: usize, failed: usize) -> String {
    let text = Copy::Picks::added(added as i64);
    if failed > 0 { format!("{text}{}{}", Copy::Common::separator, Copy::Picks::notAdded(failed as i64)) } else { text }
}

impl AppState {
    /// Loads the Pick list when it hasn't been, the last load failed, or a pick saved since is
    /// missing from it; joins a load already running.
    pub fn load_picks_if_needed(&mut self, cx: &mut Context<Self>) -> Task<()> {
        if self.picks.account.is_none() {
            return Task::ready(());
        }
        match self.picks.phase {
            PicksPhase::Idle | PicksPhase::Loading | PicksPhase::Failed(_) => self.refresh_picks(cx),
            PicksPhase::Loaded | PicksPhase::Empty if self.picks.is_stale => self.refresh_picks(cx),
            _ => Task::ready(()),
        }
    }

    /// Loads the list again from its first page (Retry, after the copy), or joins the load
    /// running. Ends once the first page has settled; the rest of the picked set keeps loading.
    pub fn refresh_picks(&mut self, cx: &mut Context<Self>) -> Task<()> {
        if self.picks.account.is_none() {
            return Task::ready(());
        }
        if let Some(running) = &self.picks.first_page {
            let wait = running.wait();
            return cx.spawn(async move |_, _| wait.await);
        }
        self.picks.list_generation += 1;
        let list = self.picks.list_generation;
        let (done, running) = Running::new();
        self.picks.first_page = Some(running.clone());
        cx.spawn(async move |this, cx| {
            load_first_page(this, list, cx).await;
            drop(done);
        })
        .detach();
        let wait = running.wait();
        cx.spawn(async move |_, _| wait.await)
    }

    /// Waits for every load in flight, the background pages included (tests, the copy).
    pub fn settle_picks(&self) -> impl std::future::Future<Output = ()> + use<> {
        let waits: Vec<_> = [&self.picks.first_page, &self.picks.more_pages, &self.picks.next_page]
            .into_iter()
            .flatten()
            .map(Running::wait)
            .collect();
        async move {
            for wait in waits {
                wait.await;
            }
        }
    }

    /// The next page, as the list scrolls. Ignored after a page failed and at the end.
    pub fn load_more_picks(&mut self, cx: &mut Context<Self>) -> Task<()> {
        if self.picks.load_more_error.is_some() {
            return Task::ready(());
        }
        self.fetch_next_picks_page(cx)
    }

    pub fn retry_load_more_picks(&mut self, cx: &mut Context<Self>) -> Task<()> {
        self.picks.load_more_error = None;
        self.fetch_next_picks_page(cx)
    }

    /// Saves (`picked`) or removes `irn` for the reader: flips at once (a removed row leaves the
    /// list), calls the library, and flips back on failure. Shows "Saved to My Picks" / "Removed
    /// from My Picks", or "Couldn’t update My Picks". Ignored while a change to the same IRN is in
    /// flight. The task says whether it went through.
    pub fn set_picked_irn(&mut self, picked: bool, irn: String, cx: &mut Context<Self>) -> Task<bool> {
        let Some(account) = self.picks.account.clone() else { return Task::ready(false) };
        if self.picks.pending.contains_key(&irn) {
            return Task::ready(false);
        }
        let generation = self.picks.generation;
        let was_picked = self.picks.irns.contains(&irn);
        self.picks.pending.insert(irn.clone(), picked);
        if picked {
            self.picks.irns.insert(irn.clone());
        } else {
            self.picks.irns.remove(&irn);
        }
        let removed_rows = if picked { Vec::new() } else { self.picks.remove_rows(&irn) };
        cx.notify();
        let call = if picked {
            account.backend.save_pick(irn.clone(), account.session.clone())
        } else {
            account.backend.remove_pick(irn.clone(), account.session.clone())
        };
        cx.spawn(async move |this, cx| {
            let result = call.await;
            this.update(cx, |this, cx| {
                if generation != this.picks.generation {
                    return result.is_ok();
                }
                this.picks.pending.remove(&irn);
                cx.notify();
                match result {
                    Ok(_) => {
                        this.picks.committed.insert(irn.clone(), picked);
                        if picked
                            && !this
                                .picks
                                .items
                                .iter()
                                .any(|item| pick_irn(&item.book).as_deref() == Some(irn.as_str()))
                        {
                            this.picks.is_stale = true;
                        }
                        let message = if picked { Copy::Picks::saved } else { Copy::Picks::removed };
                        this.show_notice(message, NoticeTone::Success, cx);
                        true
                    }
                    Err(error) => {
                        if was_picked {
                            this.picks.irns.insert(irn.clone());
                        } else {
                            this.picks.irns.remove(&irn);
                        }
                        this.picks.restore(removed_rows);
                        this.report(&error, cx);
                        // A lost session signed out (and cleared the store): the sign-in says it all.
                        if generation == this.picks.generation {
                            this.show_notice(Copy::Picks::updateFailed, NoticeTone::Error, cx);
                        }
                        false
                    }
                }
            })
            .unwrap_or(false)
        })
    }

    /// A row's remove (the "…" menu, right-click): the row leaves at once and comes back if the
    /// library refuses.
    pub fn remove_pick_item(&mut self, item: &SearchResultItem, cx: &mut Context<Self>) -> Task<bool> {
        match pick_irn(&item.book) {
            Some(irn) => self.set_picked_irn(false, irn, cx),
            None => Task::ready(false),
        }
    }

    /// Saves `books` to the list one at a time, in order (the caller passes the oldest first). A
    /// ref keyed `work:` / `bib:` saves that IRN; another loads its record for the `bib_irn`, else
    /// is skipped. A lost session, a verification challenge or a lost connection stops the copy.
    /// Then shows "Added {k} books to My Picks" (" · {m} couldn’t be added") and loads the list
    /// again. `None` when signed out.
    pub fn copy_picks(&mut self, books: Vec<BookRef>, cx: &mut Context<Self>) -> Task<Option<CopySummary>> {
        let Some(account) = self.picks.account.clone() else { return Task::ready(None) };
        if books.is_empty() {
            return Task::ready(None);
        }
        let generation = self.picks.generation;
        self.show_notice(&Copy::Picks::adding(books.len() as i64), NoticeTone::Progress, cx);
        cx.spawn(async move |this, cx| {
            let mut summary = CopySummary::default();
            let current = |this: &WeakEntity<AppState>, cx: &mut AsyncApp| {
                this.update(cx, |this, _| this.picks.generation == generation).unwrap_or(false)
            };
            for (index, book) in books.iter().enumerate() {
                if !current(&this, cx) {
                    break;
                }
                let mut irn = pick_irn(book);
                if irn.is_none()
                    && let Some(url) = non_empty(book.url.as_deref())
                {
                    // A related item or a link: its record says which bib it is. A failure just skips it.
                    irn = validated_irn(
                        account.backend.record(url).await.ok().and_then(|record| record.bib_irn).as_deref(),
                    );
                }
                let Some(irn) = irn else {
                    summary.failed += 1;
                    continue;
                };
                match account.backend.save_pick(irn.clone(), account.session.clone()).await {
                    Ok(_) => {
                        if !current(&this, cx) {
                            break;
                        }
                        summary.added += 1;
                        let _ = this.update(cx, |this, cx| {
                            this.picks.irns.insert(irn.clone());
                            this.picks.committed.insert(irn, true);
                            cx.notify();
                        });
                    }
                    Err(error) => {
                        summary.failed += 1;
                        if stops_copy(&error) {
                            summary.failed += books.len() - index - 1;
                            let _ = this.update(cx, |this, cx| this.report(&error, cx));
                            break;
                        }
                    }
                }
            }
            let refresh = this
                .update(cx, |this, cx| {
                    let tone = if summary.added == 0 {
                        NoticeTone::Error
                    } else if summary.failed == 0 {
                        NoticeTone::Success
                    } else {
                        NoticeTone::Info
                    };
                    this.show_notice(&copied_text(summary.added, summary.failed), tone, cx);
                    (this.picks.generation == generation && summary.added > 0).then(|| {
                        this.picks.is_stale = true;
                        this.refresh_picks(cx)
                    })
                })
                .ok()
                .flatten();
            if let Some(refresh) = refresh {
                refresh.await;
            }
            Some(summary)
        })
    }

    fn fetch_next_picks_page(&mut self, cx: &mut Context<Self>) -> Task<()> {
        if let Some(running) = &self.picks.next_page {
            let wait = running.wait();
            return cx.spawn(async move |_, _| wait.await);
        }
        // While the list loads again from its first page, the old next link waits.
        let store = &mut self.picks;
        let (Some(account), Some(url)) = (store.account.clone(), store.next_page_url.clone()) else {
            return Task::ready(());
        };
        if store.first_page.is_some() || store.phase != PicksPhase::Loaded || store.requested_pages.contains(&url) {
            return Task::ready(());
        }
        let generation = store.generation;
        let list = store.list_generation;
        store.is_loading_more = true;
        store.requested_pages.insert(url.clone());
        let (done, running) = Running::new();
        store.next_page = Some(running.clone());
        let fetch = account.backend.picks(account.session.clone(), Some(url.clone()));
        cx.spawn(async move |this, cx| {
            let result = fetch.await;
            let _ = this.update(cx, |this, cx| {
                if list == this.picks.list_generation {
                    this.picks.is_loading_more = false;
                    this.picks.next_page = None;
                }
                if generation != this.picks.generation || list != this.picks.list_generation {
                    return;
                }
                match result {
                    Ok(page) => {
                        let store = &mut this.picks;
                        store.pages_loaded += 1;
                        let irns = page_irns(&page);
                        store.loaded_irns.extend(irns.iter().cloned());
                        let overlaid = store.overlaid(&irns);
                        store.irns.extend(overlaid);
                        let merged = merge(&store.items, &page.records);
                        store.items = store.kept(merged);
                        // A next link back to a page already fetched would loop forever.
                        store.next_page_url = non_empty(page.next_page_url.as_deref())
                            .filter(|next| !store.requested_pages.contains(next));
                    }
                    Err(error) => {
                        this.picks.requested_pages.remove(&url);
                        let shown = this.report(&error, cx);
                        if generation == this.picks.generation {
                            this.picks.load_more_error = Some(shown);
                        }
                    }
                }
                cx.notify();
            });
            drop(done);
        })
        .detach();
        cx.notify();
        let wait = running.wait();
        cx.spawn(async move |_, _| wait.await)
    }
}

async fn load_first_page(this: WeakEntity<AppState>, list: u64, cx: &mut AsyncApp) {
    let start = this.update(cx, |this, cx| {
        let store = &mut this.picks;
        let Some(account) = store.account.clone() else {
            if list == store.list_generation {
                store.first_page = None;
            }
            return None;
        };
        store.committed.clear();
        if store.items.is_empty() {
            store.phase = PicksPhase::Loading;
        }
        cx.notify();
        Some((account.backend.picks(account.session.clone(), None), store.generation))
    });
    let Ok(Some((fetch, generation))) = start else { return };
    let result = fetch.await;
    let more = this.update(cx, |this, cx| {
        if list == this.picks.list_generation {
            this.picks.first_page = None;
        }
        if generation != this.picks.generation || list != this.picks.list_generation {
            return false;
        }
        cx.notify();
        match result {
            Ok(page) => {
                let store = &mut this.picks;
                store.load_more_error = None;
                store.is_loading_more = false;
                store.requested_pages.clear();
                store.pages_loaded = 1;
                store.loaded_irns = page_irns(&page);
                store.is_stale = false;
                if page.records.is_empty() && !page.no_results {
                    store.items.clear();
                    store.next_page_url = None;
                    store.phase = PicksPhase::Failed(unrecognised_page());
                    return false;
                }
                let merged = merge(&[], &page.records);
                store.items = store.kept(merged);
                store.next_page_url = non_empty(page.next_page_url.as_deref());
                store.mark_stale_if_missing_saves();
                store.update_phase();
                let loaded = store.loaded_irns.clone();
                let overlaid = store.overlaid(&loaded);
                if store.next_page_url.is_none() {
                    store.irns = overlaid;
                    false
                } else {
                    store.irns.extend(overlaid);
                    true
                }
            }
            Err(error) => {
                let shown = this.report(&error, cx);
                // NotLoggedIn signed out.
                if generation != this.picks.generation {
                    return false;
                }
                if this.picks.items.is_empty() {
                    this.picks.phase = PicksPhase::Failed(shown);
                } else {
                    this.show_notice(&shown.message, NoticeTone::Error, cx);
                }
                false
            }
        }
    });
    if matches!(more, Ok(true)) {
        let _ = this.update(cx, |this, cx| {
            let (done, running) = Running::new();
            this.picks.more_pages = Some(running);
            cx.spawn(async move |this, cx| {
                load_more_pages(this, list, cx).await;
                drop(done);
            })
            .detach();
        });
    }
}

/// The rest of the picked set after the first page, up to `MAX_PAGES` in all; the set is exact
/// once they're in (picks removed on the website leave it).
async fn load_more_pages(this: WeakEntity<AppState>, list: u64, cx: &mut AsyncApp) {
    loop {
        let step = this.update(cx, |this, cx| {
            let store = &this.picks;
            let more = list == store.list_generation
                && store.account.is_some()
                && store.pages_loaded < MAX_PAGES
                && store.next_page_url.is_some()
                && store.load_more_error.is_none();
            let before = store.pages_loaded;
            more.then(|| (before, this.fetch_next_picks_page(cx)))
        });
        let Ok(Some((before, fetch))) = step else { break };
        fetch.await;
        // Nothing came back (a cancelled fetch): stop here; scrolling asks again.
        let loaded = this.update(cx, |this, _| this.picks.pages_loaded).unwrap_or(before);
        if loaded == before {
            let _ = this.update(cx, |this, _| {
                if list == this.picks.list_generation {
                    this.picks.more_pages = None;
                }
            });
            return;
        }
    }
    let _ = this.update(cx, |this, cx| {
        let store = &mut this.picks;
        if list != store.list_generation {
            return;
        }
        store.more_pages = None;
        if store.account.is_some() && store.load_more_error.is_none() {
            let loaded = store.loaded_irns.clone();
            store.irns = store.overlaid(&loaded);
            cx.notify();
        }
    });
}
