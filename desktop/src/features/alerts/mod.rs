//! Ready-to-collect alerts and due-date reminders as desktop notifications (docs/desktop-app.md
//! §11.3, E3; docs/native-apps.md §5.8; a port of iOS `AlertCenter.swift` and
//! `AlertMorningCheck.swift`).
//!
//! - **Checks:** after every fresh account snapshot (`account_refreshed`), at start on the saved
//!   one, and at 07:45 local time each day while the app runs (the morning check fetches the
//!   account, then processes it). Only while signed in; demo data only when a debug env var asks.
//! - **Ready:** one notification per reservation newly ready (`ready_alerts`), keys remembered
//!   per library. **Due:** from 07:45, the reminders whose day is today (`due_reminders`, 2 days
//!   ahead), each once a day. See `store::AlertSettings::process`.
//! - **Delivery:** macOS notifications through gpui-kit (`Notification::system()`); clicking one
//!   opens Your Books on the right list with the book page on top (`AppState::open_alert`). Without
//!   a notification center (an unbundled build) or with notifications turned off, an in-app toast
//!   says the same and opens the same.
//! - **State:** `alerts.json` (`store`); sign-out forgets that library.
//! - **Profile:** the Notifications section (`section`).
//!
//! Debug builds: `LS_ALERTS_CHECK_NOW=1` runs the morning check at start; `LS_DEMO_NEW_READY=1`
//! makes the demo account gain one ready reservation on its next refresh; either lets demo data
//! raise alerts. `LS_ALERTS_PERMISSION=denied|allowed|ask` fakes the system's answer.

pub mod content;
#[cfg(debug_assertions)]
pub mod debug;
#[cfg(feature = "evidence")]
mod evidence;
mod section;
pub mod store;
pub mod system;

use std::rc::Rc;
use std::time::Duration;

use chrono::{Local, NaiveDateTime};
use gpui_kit::component::WindowExt;
use gpui_kit::component::notification::Notification;
use gpui_kit::{
    AnyElement, AnyWindowHandle, App, AppContext, Context, Entity, Global, SharedString, Subscription, Task, Window,
};
use librespydus_core::AccountSnapshot;

use crate::state::formatting::today;
use crate::state::{AppState, Paths, Route, Section};

pub use self::content::{AlertKind, AlertNote, AlertTap};
pub use self::store::{AlertSettings, AlertStore};
pub use self::system::{AlertSystem, FakeSystem, Permission};

/// The morning check's timer wakes at least this often to compare the wall clock, so a Mac that
/// slept through 07:45 runs the check when it wakes.
pub const CLOCK_CHECK_INTERVAL: Duration = Duration::from_secs(60);

/// A notification as it went out: to the notification center, or as an in-app toast.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Posted {
    pub note: AlertNote,
    pub to_system: bool,
}

struct GlobalAlerts(Entity<AlertCenter>);

impl Global for GlobalAlerts {}

/// Toasts and notifications posted by the alerts, keyed by their alert id.
struct AlertNotice;

pub struct AlertCenter {
    store: AlertStore,
    system: Rc<dyn AlertSystem>,
    /// The permission as last read.
    permission: Permission,
    /// Notifications are off for the app in System Settings while the reader wants alerts:
    /// Profile shows "Notifications are turned off for LibreSpydus." with Open System Settings.
    permission_denied: bool,
    /// The switch whose permission request is on screen (it shows as on meanwhile).
    asking: Option<AlertKind>,
    is_demo: bool,
    /// Demo data raises alerts only when a debug env var asks.
    allow_demo: bool,
    /// Local wall-clock time.
    clock: Rc<dyn Fn() -> NaiveDateTime>,
    window: Option<AnyWindowHandle>,
    posted: Vec<Posted>,
    /// When the next morning check runs (local).
    next_check: NaiveDateTime,
    _timer: Option<Task<()>>,
    _activation: Option<Subscription>,
}

impl AlertCenter {
    fn new(paths: &Paths, is_demo: bool) -> Self {
        let clock: Rc<dyn Fn() -> NaiveDateTime> = Rc::new(|| Local::now().naive_local());
        let next_check = content::next_alert_time(clock());
        Self {
            // copy-exempt: a file name
            store: AlertStore::load(paths.data.as_ref().map(|dir| dir.join("alerts.json")), is_demo),
            system: system::default_system(),
            permission: Permission::NotDetermined,
            permission_denied: false,
            asking: None,
            is_demo,
            allow_demo: debug_allows_demo(),
            clock,
            window: None,
            posted: Vec::new(),
            next_check,
            _timer: None,
            _activation: None,
        }
    }

    /// The alerts' state, made on first use for the app's data dir.
    fn ensure(paths: &Paths, is_demo: bool, cx: &mut App) -> Entity<AlertCenter> {
        if let Some(global) = cx.try_global::<GlobalAlerts>() {
            return global.0.clone();
        }
        let center = cx.new(|_| AlertCenter::new(paths, is_demo));
        cx.set_global(GlobalAlerts(center.clone()));
        center
    }

    pub fn settings(&self) -> &AlertSettings {
        self.store.settings()
    }

    pub fn permission(&self) -> Permission {
        self.permission
    }

    pub fn permission_denied(&self) -> bool {
        self.permission_denied
    }

    pub fn asking(&self) -> Option<AlertKind> {
        self.asking
    }

    /// What went out since launch, oldest first.
    pub fn posted(&self) -> &[Posted] {
        &self.posted
    }

    pub fn next_check(&self) -> NaiveDateTime {
        self.next_check
    }

    /// Tests and debug hooks.
    pub fn set_system(&mut self, system: Rc<dyn AlertSystem>) {
        self.system = system;
    }

    pub fn set_clock(&mut self, clock: Rc<dyn Fn() -> NaiveDateTime>) {
        self.next_check = content::next_alert_time(clock());
        self.clock = clock;
    }

    pub fn set_allow_demo(&mut self, allow: bool) {
        self.allow_demo = allow;
    }

    /// A snapshot of `origin`'s account: posts what's new, as the switches say.
    fn process(&mut self, snapshot: &AccountSnapshot, origin: &str, cx: &mut Context<Self>) {
        if (self.is_demo && !self.allow_demo) || !self.settings().any_enabled() {
            return;
        }
        let now = (self.clock)();
        let notes = self.store.update(|settings| settings.process(snapshot, origin, now));
        if notes.is_empty() {
            return;
        }
        // The permission can change in System Settings at any time: read it before posting.
        let system = self.system.clone();
        let permission = system.permission();
        cx.spawn(async move |this, cx| {
            let permission = permission.await;
            let to_system = system.has_notification_center() && permission == Permission::Allowed;
            let Ok(window) = this.update(cx, |this, _| {
                this.permission = permission;
                this.posted.extend(notes.iter().map(|note| Posted { note: note.clone(), to_system }));
                this.window
            }) else {
                return;
            };
            if let Some(window) = window {
                cx.update(|cx| deliver(&notes, to_system, window, cx));
            }
        })
        .detach();
    }

    /// The switch `kind` was turned on (permission granted) or off, for the selected library's
    /// saved account.
    fn apply_switch(&mut self, kind: AlertKind, on: bool, origin: Option<&str>, saved: Option<&AccountSnapshot>) {
        self.store.update(|settings| match kind {
            AlertKind::Ready => settings.set_ready(on, origin, saved),
            AlertKind::Due => settings.set_due(on),
        });
    }

    /// Reads the permission (at start, and when the window comes back to the front: the reader
    /// may have been to System Settings). Allowed hides the "turned off" line; turned off while a
    /// switch is on shows it.
    fn refresh_permission(&mut self, cx: &mut Context<Self>) {
        let permission = self.system.permission();
        cx.spawn(async move |this, cx| {
            let permission = permission.await;
            let _ = this.update(cx, |this, cx| {
                this.permission = permission;
                this.permission_denied =
                    permission == Permission::Denied && (this.permission_denied || this.settings().any_enabled());
                cx.notify();
            });
            cx.update(|cx| cx.refresh_windows());
        })
        .detach();
    }

    /// How long the morning timer sleeps: until the next check, at most `CLOCK_CHECK_INTERVAL`.
    fn wait_before_next_check(&self) -> Duration {
        let until = (self.next_check - (self.clock)()).to_std().unwrap_or(Duration::ZERO);
        until.min(CLOCK_CHECK_INTERVAL)
    }

    /// Whether the morning check is due now; if so the next one is the following 07:45.
    pub fn take_morning_check(&mut self) -> bool {
        let now = (self.clock)();
        if !content::morning_check_due(now, self.next_check) {
            return false;
        }
        self.next_check = content::next_alert_time(now);
        true
    }

    fn start_timer(&mut self, cx: &mut Context<Self>) {
        self.next_check = content::next_alert_time((self.clock)());
        self._timer = Some(cx.spawn(async move |this, cx| {
            loop {
                let Ok(wait) = this.update(cx, |this, _| this.wait_before_next_check()) else { break };
                cx.background_executor().timer(wait.max(Duration::from_millis(500))).await;
                let Ok(due) = this.update(cx, |this, _| this.take_morning_check()) else { break };
                if due {
                    cx.update(run_morning_check);
                }
            }
        }));
    }
}

/// The alerts' state (made on first use).
pub fn center(cx: &mut App) -> Entity<AlertCenter> {
    let state = AppState::global(cx);
    let (paths, is_demo) = {
        let state = state.read(cx);
        (state.deps.paths.clone(), state.is_demo)
    };
    AlertCenter::ensure(&paths, is_demo, cx)
}

// ---- Hooks (docs/desktop-app.md §11.1) ----

/// A fresh account snapshot was just applied (`AppState::account().snapshot()`).
pub fn account_refreshed(state: &mut AppState, cx: &mut Context<AppState>) {
    if !state.is_signed_in() || state.library.is_none() {
        return;
    }
    let Some(snapshot) = state.account.snapshot().cloned() else { return };
    let origin = state.backend.origin();
    let center = AlertCenter::ensure(&state.deps.paths, state.is_demo, cx);
    center.update(cx, |center, cx| center.process(&snapshot, &origin, cx));
}

/// `AppState::sign_out` for `origin`: that library's alerted keys and reminder days go.
pub fn did_sign_out(state: &mut AppState, origin: &str, cx: &mut Context<AppState>) {
    let center = AlertCenter::ensure(&state.deps.paths, state.is_demo, cx);
    center.update(cx, |center, _| center.store.update(|settings| settings.forget(origin)));
}

/// Profile: the Notifications section while signed in, or nothing.
pub fn profile_section(window: &mut Window, cx: &mut App) -> Option<AnyElement> {
    section::render(window, cx)
}

/// Called once by the shell at start: the window to post from, the permission, the morning
/// check's timer, and a check of the saved account.
pub fn start(window: &mut Window, cx: &mut App) {
    let center = center(cx);
    center.update(cx, |center, cx| {
        center.window = Some(window.window_handle());
        center._activation = Some(cx.observe_window_activation(window, |center, window, cx| {
            if window.is_window_active() {
                center.refresh_permission(cx);
            }
        }));
        center.refresh_permission(cx);
        center.start_timer(cx);
    });
    #[cfg(debug_assertions)]
    let checked = debug::start(cx);
    #[cfg(not(debug_assertions))]
    let checked = false;
    if !checked {
        check_now(cx);
    }
    #[cfg(feature = "evidence")]
    evidence::schedule(window, cx);
}

// ---- Operations ----

/// Profile's switch `kind`, for the selected library's saved account. On asks for permission
/// (refused: it stays off and Profile says so), then checks the saved account; off just turns
/// it off.
pub fn set_switch(kind: AlertKind, on: bool, cx: &mut App) {
    let state = AppState::global(cx);
    let (origin, saved) = {
        let state = state.read(cx);
        (state.library().map(|_| state.backend().origin()), state.account().snapshot().cloned())
    };
    let center = center(cx);
    let (system, asking, enabled) = {
        let center = center.read(cx);
        let settings = center.settings();
        let enabled = match kind {
            AlertKind::Ready => settings.ready_enabled,
            AlertKind::Due => settings.due_enabled,
        };
        (center.system.clone(), center.asking, enabled)
    };
    if asking.is_some() || on == enabled {
        return;
    }
    if !on {
        center.update(cx, |center, _| center.apply_switch(kind, false, origin.as_deref(), saved.as_ref()));
        cx.refresh_windows();
        return;
    }
    if !system.has_notification_center() {
        center.update(cx, |center, _| center.apply_switch(kind, true, origin.as_deref(), saved.as_ref()));
        cx.refresh_windows();
        check_now(cx);
        return;
    }
    center.update(cx, |center, _| center.asking = Some(kind));
    cx.refresh_windows();
    let request = system.request_permission();
    cx.spawn(async move |cx| {
        let granted = request.await;
        center.update(cx, |center, _| {
            center.asking = None;
            center.permission = if granted { Permission::Allowed } else { Permission::Denied };
            center.permission_denied = !granted;
            if granted {
                center.apply_switch(kind, true, origin.as_deref(), saved.as_ref());
            }
        });
        cx.update(|cx| {
            cx.refresh_windows();
            if granted {
                check_now(cx);
            }
        });
    })
    .detach();
}

/// Processes the selected library's saved account now (start, a switch turned on, after the
/// morning check's fetch).
pub fn check_now(cx: &mut App) {
    let state = AppState::global(cx);
    let (snapshot, origin) = {
        let state = state.read(cx);
        if !state.is_signed_in() || state.library().is_none() {
            return;
        }
        let Some(snapshot) = state.account().snapshot().cloned() else { return };
        (snapshot, state.backend().origin())
    };
    center(cx).update(cx, |center, cx| center.process(&snapshot, &origin, cx));
}

/// The morning check (07:45, or debug `LS_ALERTS_CHECK_NOW`): fetch the account and save it like
/// any refresh, so the app shows fresh data; the alerts process it on the way
/// (`account_refreshed`), then the saved account once more so a failed fetch still gives the
/// day's due reminders. A failure is silent; an expired session signs out as in the app.
pub fn run_morning_check(cx: &mut App) {
    let state = AppState::global(cx);
    let center = center(cx);
    let go = {
        let (state, center) = (state.read(cx), center.read(cx));
        state.is_signed_in()
            && state.library().is_some()
            && center.settings().any_enabled()
            && (!state.is_demo() || center.allow_demo)
    };
    if !go {
        log::info!("alerts: morning check has nothing to do");
        return;
    }
    log::info!("alerts: morning check fetching the account");
    let refresh = state.update(cx, |state, cx| state.refresh_account(true, cx));
    cx.spawn(async move |cx| {
        refresh.await;
        cx.update(check_now);
    })
    .detach();
}

/// Posts `notes` from `window`: to the notification center, or as in-app toasts.
fn deliver(notes: &[AlertNote], to_system: bool, window: AnyWindowHandle, cx: &mut App) {
    let _ = window.update(cx, |_, window, cx| {
        for note in notes {
            let tap = note.tap.clone();
            let notification = Notification::info(SharedString::from(note.body.clone()))
                .title(SharedString::from(note.title.clone()))
                .id1::<AlertNotice>(SharedString::from(note.id.clone()))
                .on_click(move |_, _, cx| open_alert(&tap, cx));
            let notification = if to_system { notification.system() } else { notification.autohide(false) };
            window.push_notification(notification, cx);
        }
    });
}

/// A clicked notification or toast (§5.8 "Tap").
pub fn open_alert(tap: &AlertTap, cx: &mut App) {
    AppState::global(cx).update(cx, |state, cx| state.open_alert(tap, cx));
}

impl AppState {
    /// A clicked alert: Your Books on its list (Reservations / Loans) with the book page of the
    /// reservation or loan it names pushed on top, built from the saved account the way clicking
    /// that row opens it, so Back lands on the list. Just the list when the alert names several
    /// loans, an item no longer in the account, or another library than the selected one.
    pub fn open_alert(&mut self, tap: &AlertTap, cx: &mut Context<Self>) {
        self.open_your_books(tap.segment(), cx);
        if self.library.is_none() || tap.origin != self.backend.origin() {
            return;
        }
        let Some(book) = self.account.snapshot().and_then(|snapshot| content::book_for_tap(tap, snapshot, today()))
        else {
            return;
        };
        self.push_on(Section::YourBooks, Route::Detail(book), cx);
    }
}

#[cfg(debug_assertions)]
fn debug_allows_demo() -> bool {
    debug::allows_demo()
}

#[cfg(not(debug_assertions))]
fn debug_allows_demo() -> bool {
    false
}
