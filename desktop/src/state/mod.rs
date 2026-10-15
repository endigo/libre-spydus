//! App-wide state (docs/desktop-app.md §5.3; a port of iOS `AppModel.swift`, minus the alerts and
//! Face ID): the library, the session and saved account, My Picks, the card, navigation and the
//! reload keys. One GPUI entity, reachable anywhere with `AppState::global(cx)`.
//!
//! Fields are `pub(crate)`, so a feature can add its own operations in its own `impl AppState`
//! block inside its feature directory (as the iOS app's `extension AppModel` files do).

pub mod account;
pub mod account_cache;
pub mod book_ref;
pub mod error;
pub mod formatting;
pub mod library;
pub mod local_picks;
pub mod paths;
pub mod patron;
pub mod picks;
pub mod results;
pub mod route;
pub mod running;
pub mod secrets;
pub mod settings;

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use gpui_kit::{App, AppContext, Context, Entity, Global, Task};
use librespydus_core::barcode::BarcodeSymbology;
use librespydus_core::{LibraryInfo, PatronItem, RenewResult, SpydusError};

use crate::backend::LibraryBackend;
use crate::copy::Copy;
use crate::runtime::spawn_core;

pub use self::account::AccountStore;
pub use self::book_ref::{BookRef, FormatRef};
pub use self::error::{ErrorKind, Loadable, UserFacingError};
pub use self::local_picks::LocalPicksStore;
pub use self::paths::Paths;
pub use self::picks::{PicksCopyOffer, PicksStore};
pub use self::route::{ResultsSource, Route, Section, YourBooksSegment};
pub use self::secrets::{MemorySecretStore, SecretStore};
pub use self::settings::Settings;

use self::account_cache::AccountSnapshotCache;
use self::formatting::non_empty;
use self::patron::PatronItemExt;
use self::picks::pick_irn;

/// Makes the backend for a library origin (`None`: Canning).
pub type BackendFactory = Arc<dyn Fn(Option<&str>) -> Arc<dyn LibraryBackend> + Send + Sync>;

/// Everything `AppState` persists to or creates; tests pass memory stores and a fake backend.
#[derive(Clone)]
pub struct Dependencies {
    pub paths: Paths,
    pub is_demo: bool,
    pub sessions: Arc<dyn SecretStore>,
    pub verification_cookies: Arc<dyn SecretStore>,
    pub card_numbers: Arc<dyn SecretStore>,
    pub make_backend: BackendFactory,
    pub now: Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>,
    /// Replace a placeholder library (`--library`) with its directory entry (one public GET).
    pub look_up_libraries: bool,
}

impl Dependencies {
    /// Memory only: no files, no keychain, no directory lookups (tests).
    pub fn in_memory(make_backend: BackendFactory) -> Self {
        Self {
            paths: Paths::memory(),
            is_demo: false,
            sessions: Arc::new(MemorySecretStore::new()),
            verification_cookies: Arc::new(MemorySecretStore::new()),
            card_numbers: Arc::new(MemorySecretStore::new()),
            make_backend,
            now: Arc::new(Utc::now),
            look_up_libraries: false,
        }
    }
}

/// `reload_key()`: re-run a screen's load when it changes (library change, verification,
/// sign-in/out, session loss).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReloadKey {
    pub origin: String,
    pub verification: u64,
    pub account: u64,
}

/// A short confirmation or failure the shell shows as a toast.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    pub id: u64,
    pub message: String,
    pub tone: NoticeTone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeTone {
    Success,
    Info,
    Error,
    /// Stays until the next notice replaces it.
    Progress,
}

struct GlobalAppState(Entity<AppState>);

impl Global for GlobalAppState {}

pub struct AppState {
    pub(crate) deps: Dependencies,
    pub(crate) settings: Settings,
    /// The selected library, persisted. `None` → onboarding.
    pub(crate) library: Option<LibraryInfo>,
    /// One backend per selected origin, rebuilt when the library changes.
    pub(crate) backend: Arc<dyn LibraryBackend>,
    /// The current origin's session (keychain; memory in demo mode and with `--data-dir`).
    pub(crate) session: Option<String>,
    pub(crate) account: AccountStore,
    /// Entered or edited on My card; deleted on sign-out, kept when the session expires.
    pub(crate) saved_card_number: Option<String>,
    pub(crate) card_symbologies: HashMap<String, BarcodeSymbology>,
    /// My Picks on this computer (signed out).
    pub(crate) local_picks: LocalPicksStore,
    /// My Picks on the library account (signed in).
    pub(crate) picks: PicksStore,
    pub(crate) picks_copy_offer: Option<PicksCopyOffer>,
    pub(crate) is_demo: bool,
    pub(crate) verification_cookie: Option<String>,
    // Navigation
    pub(crate) section: Section,
    pub(crate) stacks: HashMap<Section, Vec<Route>>,
    pub(crate) pending_search: Option<String>,
    pub(crate) pending_segment: Option<YourBooksSegment>,
    /// Dialogs the shell opens for AppState (`request_sign_in`, `request_library_picker`).
    pub(crate) sign_in_requested: bool,
    pub(crate) library_picker_requested: bool,
    pub(crate) notice: Option<Notice>,
    next_notice: u64,
    // Reload triggers
    pub(crate) verification_generation: u64,
    pub(crate) account_generation: u64,
    /// Bumped whenever `backend` is replaced, so late results for an old library are dropped.
    pub(crate) backend_generation: u64,
}

impl AppState {
    /// Creates the app's state and makes it the global one.
    pub fn init(deps: Dependencies, cx: &mut App) -> Entity<AppState> {
        let state = cx.new(|cx| AppState::new(deps, cx));
        cx.set_global(GlobalAppState(state.clone()));
        state
    }

    /// The app's state (`AppState::init` must have run).
    pub fn global(cx: &App) -> Entity<AppState> {
        cx.global::<GlobalAppState>().0.clone()
    }

    pub fn new(deps: Dependencies, cx: &mut Context<Self>) -> Self {
        let settings = Settings::load(deps.paths.settings_file());
        let library = settings.data.library.clone();
        let backend = (deps.make_backend)(library.as_ref().map(|info| info.origin.as_str()));
        // Demo mode never saves the account.
        let cache = if deps.is_demo { None } else { deps.paths.account_cache_dir().map(AccountSnapshotCache::new) };
        let mut state = Self {
            settings,
            library,
            backend,
            session: None,
            account: AccountStore::new(cache, deps.now.clone()),
            saved_card_number: None,
            card_symbologies: HashMap::new(),
            local_picks: LocalPicksStore::new(if deps.is_demo { None } else { deps.paths.my_picks_dir() }),
            picks: PicksStore::default(),
            picks_copy_offer: None,
            is_demo: deps.is_demo,
            verification_cookie: None,
            section: Section::Home,
            stacks: HashMap::new(),
            pending_search: None,
            pending_segment: None,
            sign_in_requested: false,
            library_picker_requested: false,
            notice: None,
            next_notice: 0,
            verification_generation: 0,
            account_generation: 0,
            backend_generation: 0,
            deps,
        };
        state.install_secrets();
        state.after_install(cx);
        state
    }

    // ---- Library ----

    pub fn library(&self) -> Option<&LibraryInfo> {
        self.library.as_ref()
    }

    /// The library's name ("your library" before one is chosen).
    pub fn library_name(&self) -> String {
        self.library.as_ref().map_or_else(|| Copy::Common::yourLibrary.to_owned(), |info| info.name.clone())
    }

    pub fn backend(&self) -> Arc<dyn LibraryBackend> {
        self.backend.clone()
    }

    pub fn is_demo(&self) -> bool {
        self.is_demo
    }

    pub fn needs_onboarding(&self) -> bool {
        self.library.is_none()
    }

    pub fn select_library(&mut self, info: LibraryInfo, cx: &mut Context<Self>) {
        self.library_picker_requested = false;
        let changed = self.library.as_ref().map(|current| current.origin.as_str()) != Some(info.origin.as_str());
        self.settings.set_library(&info);
        self.library = Some(info.clone());
        if changed {
            self.install((self.deps.make_backend)(Some(&info.origin)), cx);
        }
        cx.notify();
    }

    /// Selects a library by origin alone (`--library`); its name and region are guessed from the
    /// host until the directory entry is found (one public GET).
    pub fn select_library_origin(&mut self, origin: &str, cx: &mut Context<Self>) {
        let backend = (self.deps.make_backend)(Some(origin));
        let origin = backend.origin();
        if self.library.as_ref().is_some_and(|current| current.origin == origin) {
            return;
        }
        let info = library::placeholder(&origin);
        self.library_picker_requested = false;
        self.settings.set_library(&info);
        self.library = Some(info);
        self.install(backend, cx);
        self.refresh_library_info(cx);
        cx.notify();
    }

    /// Replaces a placeholder library with its Civica directory entry.
    pub fn refresh_library_info(&mut self, cx: &mut Context<Self>) {
        let Some(current) = self.library.clone().filter(library::is_placeholder) else { return };
        if !self.deps.look_up_libraries {
            return;
        }
        let lookup = spawn_core(librespydus_core::list_libraries(true));
        cx.spawn(async move |this, cx| {
            let Ok(libraries) = lookup.await else { return };
            let Some(entry) = libraries.into_iter().find(|info| info.origin == current.origin) else { return };
            let _ = this.update(cx, |this, cx| {
                if this.library.as_ref().is_some_and(|library| library.origin == entry.origin) {
                    this.settings.set_library(&entry);
                    this.library = Some(entry);
                    cx.notify();
                }
            });
        })
        .detach();
    }

    // ---- Session ----

    pub fn is_signed_in(&self) -> bool {
        self.session.is_some()
    }

    pub fn session(&self) -> Option<&str> {
        self.session.as_deref()
    }

    /// The borrower's name from the account snapshot.
    pub fn account_name(&self) -> Option<String> {
        self.account.snapshot().and_then(|snapshot| non_empty(snapshot.account_name.as_deref()))
    }

    /// Signs in and stores the session for this origin, then fetches the account. The error is
    /// what the sign-in dialog shows. After a typed sign-in this library's local picks may be
    /// offered to the account (`picks_copy_offer`).
    pub fn sign_in(
        &mut self,
        username: String,
        password: String,
        cx: &mut Context<Self>,
    ) -> Task<Result<(), UserFacingError>> {
        let generation = self.backend_generation;
        let backend = self.backend.clone();
        // Kept only until the sign-in settles, for Touch ID (`features::biometrics::did_sign_in`).
        let details = (username.clone(), password.clone());
        let login = backend.login(username, password);
        cx.spawn(async move |this, cx| {
            let result = login.await;
            let refresh = this
                .update(cx, |this, cx| {
                    if generation != this.backend_generation {
                        // Another library was chosen meanwhile.
                        return Err(UserFacingError::unexpected(None));
                    }
                    let token = result.map_err(|error| this.report(&error, cx))?;
                    let origin = backend.origin();
                    this.session = Some(token.clone());
                    this.deps.sessions.set_value(&origin, Some(&token));
                    this.note_secret_origin(&origin);
                    this.sign_in_requested = false;
                    this.account_generation += 1;
                    this.account.use_account(Some(&origin), Some(&token), backend.clone(), false);
                    this.picks.use_account(Some(&origin), Some(&token), backend.clone());
                    this.offer_picks_copy();
                    crate::features::biometrics::did_sign_in(this, &details.0, &details.1, cx);
                    this.load_picks_if_needed(cx).detach();
                    cx.notify();
                    Ok(this.refresh_account(true, cx))
                })
                .map_err(|_| UserFacingError::unexpected(None))??;
            refresh.await;
            Ok(())
        })
    }

    /// Signs out of this library: the card number goes with the account (an expired session keeps
    /// it).
    pub fn sign_out(&mut self, cx: &mut Context<Self>) {
        let origin = self.backend.origin();
        crate::features::biometrics::did_sign_out(self, &origin, cx);
        crate::features::alerts::did_sign_out(self, &origin, cx);
        self.deps.card_numbers.set_value(&origin, None);
        self.saved_card_number = None;
        self.clear_session(cx);
    }

    /// Signs out without forgetting the card number (an ended session).
    pub fn clear_session(&mut self, cx: &mut Context<Self>) {
        let origin = self.backend.origin();
        self.deps.sessions.set_value(&origin, None);
        self.session = None;
        self.account.clear(&origin);
        self.picks.use_account(None, None, self.backend.clone());
        self.picks_copy_offer = None;
        self.account_generation += 1;
        cx.notify();
    }

    // ---- Errors ----

    /// Routes a failed call (docs/native-apps.md §5.7) and returns what to show. `NotLoggedIn`
    /// clears the session and asks for sign-in, unless Touch ID signs back in instead
    /// (`features::biometrics`). `HumanVerificationRequired` goes to `features::verification`.
    pub fn report(&mut self, error: &SpydusError, cx: &mut Context<Self>) -> UserFacingError {
        let name = self.library_name();
        match error {
            SpydusError::HumanVerificationRequired => crate::features::verification::on_required(self, error, cx),
            SpydusError::NotLoggedIn => {
                if crate::features::biometrics::sign_back_in(self, cx) {
                    return UserFacingError::from_core(error, &name);
                }
                self.clear_session(cx);
                self.request_sign_in(cx);
                UserFacingError::from_core(error, &name)
            }
            _ => UserFacingError::from_core(error, &name),
        }
    }

    // ---- Account (§5.2a) ----

    pub fn account(&self) -> &AccountStore {
        &self.account
    }

    /// Renews `item` at the library after the reader confirmed (Your Books, or the book page's "On
    /// loan" pill; §5.12). The account is fetched again either way, and a renewed loan shows its
    /// new date at once. A loan without a renew link, or no session, renews nothing (`ok: false`).
    pub fn renew(&mut self, item: &PatronItem, cx: &mut Context<Self>) -> Task<Result<RenewResult, UserFacingError>> {
        let (Some(href), Some(session)) = (item.renew_href(), self.session.clone()) else {
            return Task::ready(Ok(RenewResult { ok: false, message: None, due: item.fields.get("Due").cloned() }));
        };
        let key = self.reload_key();
        let call = self.backend.renew_loan(href.clone(), session);
        let change = self.perform_account_change(call, |_| true, cx);
        cx.spawn(async move |this, cx| {
            let result = change.await;
            this.update(cx, |this, cx| match result {
                Ok(result) => {
                    if this.reload_key() == key
                        && result.ok
                        && let Some(due) = non_empty(result.due.as_deref())
                    {
                        this.update_account_snapshot(
                            |snapshot| {
                                for loan in &mut snapshot.loans.items {
                                    if loan.has_action(&href) {
                                        // copy-exempt: the loans list's field name (a key)
                                        loan.fields.insert("Due".into(), due.clone());
                                    }
                                }
                            },
                            cx,
                        );
                    }
                    Ok(result)
                }
                Err(error) => Err(this.report(&error, cx)),
            })
            .map_err(|_| UserFacingError::unexpected(None))?
        })
    }

    // ---- My Picks (§5.3, §5.11) ----

    /// Whether `book` is in My Picks: on the account when signed in (by its IRN), on this
    /// computer when not.
    pub fn is_picked(&self, book: &BookRef) -> bool {
        if self.picks.is_active() {
            pick_irn(book).is_some_and(|irn| self.picks.contains(&irn))
        } else {
            self.local_picks.contains(&book.id)
        }
    }

    /// Saves or removes `book`: on the account when signed in (a ref without an IRN needs
    /// `set_picked_irn` with its record's `bib_irn`), on this computer when not. The task says
    /// whether it went through.
    pub fn set_picked(&mut self, book: BookRef, picked: bool, cx: &mut Context<Self>) -> Task<bool> {
        if self.picks.is_active() {
            return match pick_irn(&book) {
                Some(irn) => self.set_picked_irn(picked, irn, cx),
                None => Task::ready(false),
            };
        }
        let now = (self.deps.now)();
        if picked {
            self.local_picks.add(&book, now);
        } else {
            self.local_picks.remove(&book.id);
        }
        let message = if picked { Copy::Picks::saved } else { Copy::Picks::removed };
        self.show_notice(message, NoticeTone::Success, cx);
        cx.notify();
        Task::ready(true)
    }

    pub fn picks(&self) -> &PicksStore {
        &self.picks
    }

    pub fn local_picks(&self) -> &LocalPicksStore {
        &self.local_picks
    }

    /// After an interactive sign-in: this library's local picks can go to the account, once.
    pub fn picks_copy_offer(&self) -> Option<&PicksCopyOffer> {
        self.picks_copy_offer.as_ref()
    }

    /// The reader answered the offer: remembered for this library either way. Add saves the local
    /// picks to the account, oldest first, and keeps them on this computer.
    pub fn answer_picks_copy_offer(&mut self, add: bool, cx: &mut Context<Self>) -> Task<()> {
        let Some(offer) = self.picks_copy_offer.take() else { return Task::ready(()) };
        self.settings.mark_picks_offered(&offer.origin, self.is_demo);
        cx.notify();
        let origin = self.backend.origin();
        if !add || !self.is_signed_in() || offer.origin != origin || self.local_picks.origin() != Some(origin.as_str())
        {
            return Task::ready(());
        }
        let books: Vec<BookRef> = self.local_picks.items().iter().rev().cloned().collect();
        let copy = self.copy_picks(books, cx);
        cx.spawn(async move |_, _| {
            copy.await;
        })
    }

    fn offer_picks_copy(&mut self) {
        let origin = self.backend.origin();
        if self.local_picks.origin() != Some(origin.as_str())
            || self.local_picks.items().is_empty()
            || self.settings.was_picks_offered(&origin, self.is_demo)
        {
            return;
        }
        self.picks_copy_offer = Some(PicksCopyOffer { origin, count: self.local_picks.items().len() });
    }

    // ---- Card (§5.9) ----

    /// The library card number: the one entered on My card, else the account's own.
    pub fn card_number(&self) -> Option<String> {
        card_number(self.saved_card_number.as_deref(), self.account_card_number().as_deref())
    }

    fn account_card_number(&self) -> Option<String> {
        self.account.snapshot().and_then(|snapshot| snapshot.card_number.clone())
    }

    /// When the card expires, while the card shown is the account's own.
    pub fn card_expires(&self) -> Option<String> {
        let expires = non_empty(self.account.snapshot()?.card_expires.as_deref())?;
        (self.card_number() == card_number(None, self.account_card_number().as_deref())).then_some(expires)
    }

    /// Saves the number entered on My card for this library (trimmed); a blank one is deleted.
    pub fn set_card_number(&mut self, number: Option<&str>, cx: &mut Context<Self>) {
        let value = non_empty(number);
        let origin = self.backend.origin();
        self.deps.card_numbers.set_value(&origin, value.as_deref());
        if value.is_some() {
            self.note_secret_origin(&origin);
        }
        self.saved_card_number = value;
        cx.notify();
    }

    /// The selected library's symbology if it has been read before.
    pub fn known_card_symbology(&self) -> Option<BarcodeSymbology> {
        self.card_symbologies.get(&self.backend.origin()).copied()
    }

    /// Asks the library which symbology its scanners read (one public GET, cached in the core).
    pub fn card_symbology(&mut self, cx: &mut Context<Self>) -> Task<BarcodeSymbology> {
        let origin = self.backend.origin();
        let fetch = self.backend.card_barcode_symbology();
        cx.spawn(async move |this, cx| {
            let symbology = fetch.await;
            let _ = this.update(cx, |this, _| this.card_symbologies.insert(origin, symbology));
            symbology
        })
    }

    // ---- Navigation (§5.5) ----

    pub fn section(&self) -> Section {
        self.section
    }

    /// A sidebar click: switch sections, or back to the root when it's already selected.
    pub fn select(&mut self, section: Section, cx: &mut Context<Self>) {
        if section == self.section {
            self.stacks.entry(section).or_default().clear();
        } else {
            self.section = section;
        }
        cx.notify();
    }

    /// Pushes `route` on the current section's stack.
    pub fn push(&mut self, route: Route, cx: &mut Context<Self>) {
        self.push_on(self.section, route, cx);
    }

    /// Pushes `route` on `section`'s stack and shows that section.
    pub fn push_on(&mut self, section: Section, route: Route, cx: &mut Context<Self>) {
        self.stacks.entry(section).or_default().push(route);
        self.section = section;
        cx.notify();
    }

    /// Back (⌘[): pops the current section's stack.
    pub fn pop(&mut self, cx: &mut Context<Self>) {
        if let Some(stack) = self.stacks.get_mut(&self.section)
            && stack.pop().is_some()
        {
            cx.notify();
        }
    }

    pub fn stack(&self, section: Section) -> &[Route] {
        self.stacks.get(&section).map_or(&[], Vec::as_slice)
    }

    /// Home's search field and ⌘F: Search, back at its root, which focuses and runs `query`.
    pub fn open_search(&mut self, query: Option<String>, cx: &mut Context<Self>) {
        self.pending_search = Some(query.unwrap_or_default());
        self.stacks.entry(Section::Search).or_default().clear();
        self.section = Section::Search;
        cx.notify();
    }

    /// For Search: the query handed over (`Some("")`: just focus the field), cleared once taken.
    pub fn take_pending_search(&mut self) -> Option<String> {
        self.pending_search.take()
    }

    /// A book page's "On loan" / "Reserved" pill: Your Books, back at its list, on `segment`.
    pub fn open_your_books(&mut self, segment: YourBooksSegment, cx: &mut Context<Self>) {
        self.pending_segment = Some(segment);
        self.stacks.entry(Section::YourBooks).or_default().clear();
        self.section = Section::YourBooks;
        cx.notify();
    }

    pub fn take_pending_segment(&mut self) -> Option<YourBooksSegment> {
        self.pending_segment.take()
    }

    /// Asks the shell to open the sign-in dialog.
    pub fn request_sign_in(&mut self, cx: &mut Context<Self>) {
        if self.library.is_some() {
            self.sign_in_requested = true;
            cx.notify();
        }
    }

    /// Asks the shell to open the library picker dialog.
    pub fn request_library_picker(&mut self, cx: &mut Context<Self>) {
        self.library_picker_requested = true;
        cx.notify();
    }

    /// For the shell: whether a sign-in dialog was asked for, cleared once taken.
    pub fn take_sign_in_request(&mut self) -> bool {
        std::mem::take(&mut self.sign_in_requested)
    }

    pub fn take_library_picker_request(&mut self) -> bool {
        std::mem::take(&mut self.library_picker_requested)
    }

    // ---- Notices ----

    /// Shows `message` as a toast (the shell's).
    pub fn show_notice(&mut self, message: &str, tone: NoticeTone, cx: &mut Context<Self>) {
        self.next_notice += 1;
        self.notice = Some(Notice { id: self.next_notice, message: message.to_owned(), tone });
        cx.notify();
    }

    pub fn notice(&self) -> Option<&Notice> {
        self.notice.as_ref()
    }

    // ---- Reloads ----

    pub fn reload_key(&self) -> ReloadKey {
        ReloadKey {
            origin: self.backend.origin(),
            verification: self.verification_generation,
            account: self.account_generation,
        }
    }

    pub fn account_generation(&self) -> u64 {
        self.account_generation
    }

    pub fn verification_generation(&self) -> u64 {
        self.verification_generation
    }

    // ---- Verification (no web view in v1; a cookie from a later helper applies here) ----

    pub fn is_verified(&self) -> bool {
        self.verification_cookie.is_some()
    }

    /// Stores and applies `"aws-waf-token=<value>"` for this origin, then lets screens retry.
    pub fn complete_verification(&mut self, cookie: String, cx: &mut Context<Self>) {
        let origin = self.backend.origin();
        self.deps.verification_cookies.set_value(&origin, Some(&cookie));
        self.note_secret_origin(&origin);
        self.backend.set_verification_cookie(Some(cookie.clone()));
        self.verification_cookie = Some(cookie);
        self.verification_generation += 1;
        cx.notify();
    }

    pub fn clear_verification(&mut self, cx: &mut Context<Self>) {
        let origin = self.backend.origin();
        self.deps.verification_cookies.set_value(&origin, None);
        self.backend.set_verification_cookie(None);
        self.verification_cookie = None;
        self.verification_generation += 1;
        cx.notify();
    }

    // ---- Recent searches ----

    pub fn recent_searches(&self) -> &[String] {
        &self.settings.data.recent_searches
    }

    pub fn add_recent_search(&mut self, query: &str, cx: &mut Context<Self>) {
        self.settings.add_recent_search(query);
        cx.notify();
    }

    pub fn remove_recent_search(&mut self, query: &str, cx: &mut Context<Self>) {
        self.settings.remove_recent_search(query);
        cx.notify();
    }

    pub fn clear_recent_searches(&mut self, cx: &mut Context<Self>) {
        self.settings.clear_recent_searches();
        cx.notify();
    }

    // ---- Private ----

    fn install(&mut self, backend: Arc<dyn LibraryBackend>, cx: &mut Context<Self>) {
        self.backend_generation += 1;
        self.backend = backend;
        self.stacks.clear();
        self.pending_search = None;
        self.pending_segment = None;
        self.picks_copy_offer = None;
        self.sign_in_requested = false;
        self.install_secrets();
        self.account_generation += 1;
        self.after_install(cx);
    }

    /// Loads this origin's verification cookie, session and card number, and points the stores at
    /// it (§5.2).
    fn install_secrets(&mut self) {
        let origin = self.backend.origin();
        self.verification_cookie = self.deps.verification_cookies.value(&origin);
        if let Some(cookie) = &self.verification_cookie {
            self.backend.set_verification_cookie(Some(cookie.clone()));
        }
        if self.library.is_none() {
            self.session = None;
            self.saved_card_number = None;
        } else if self.is_demo {
            // Demo mode starts signed in as the fixtures' "Test Borrower".
            self.session = Some(self.deps.sessions.value(&origin).unwrap_or_else(demo_session));
            self.saved_card_number = self.deps.card_numbers.value(&origin);
        } else {
            self.session = self.deps.sessions.value(&origin);
            self.saved_card_number = self.deps.card_numbers.value(&origin);
        }
        let current = self.library.as_ref().map(|_| origin.as_str());
        self.account.use_account(current, self.session.as_deref(), self.backend.clone(), false);
        self.picks.use_account(current, self.session.as_deref(), self.backend.clone());
        self.local_picks.load(current);
    }

    /// After launch and every library change: the saved account shows at once, a fresh one loads.
    fn after_install(&mut self, cx: &mut Context<Self>) {
        if self.session.is_some() {
            self.refresh_account(false, cx).detach();
            self.load_picks_if_needed(cx).detach();
        }
    }

    fn note_secret_origin(&mut self, origin: &str) {
        if !self.is_demo {
            self.settings.note_secret_origin(origin);
        }
    }
}

/// `saved` (entered on My card) wins over `account` (the library's own record); blank is none.
pub fn card_number(saved: Option<&str>, account: Option<&str>) -> Option<String> {
    non_empty(saved).or_else(|| non_empty(account))
}

#[cfg(debug_assertions)]
fn demo_session() -> String {
    crate::backend::demo::SESSION.to_owned()
}

#[cfg(not(debug_assertions))]
fn demo_session() -> String {
    String::new()
}
