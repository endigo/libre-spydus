//! Sign back in with Touch ID (docs/desktop-app.md §11.4, E4; native-apps.md §5.14; a port of
//! iOS `BiometricSignIn.swift`, `BiometricVault.swift` and `AfterSignInOffer.swift`).
//!
//! - **The offer:** after a typed sign-in, when the check is available, this library has no
//!   details and the reader hasn't said Not now: "Sign in with Touch ID next time?" (`view.rs`).
//!   My Picks' copy offer is held back until it's answered.
//! - **Profile:** a switch under the Account card. Off deletes the details; on opens the sign-in
//!   dialog with `turnOnNote`, and its next typed sign-in saves without asking.
//! - **Signing back in:** `NotLoggedIn` with details runs the check, then `AppState::sign_in`
//!   with them, keeping the session on screen until it ends.
//! - **The sign-in dialog** (`onboarding::sign_in`) gets "Sign in with Touch ID" and
//!   `privacyNoteBiometrics`, and starts the check at once when it can.
//!
//! State lives in this module's GPUI global (`Biometrics`), never in `AppState`; every change
//! notifies `AppState`, which the shell, Profile and the sign-in dialog observe.

pub mod check;
pub mod declined;
#[cfg(feature = "evidence")]
mod evidence;
pub mod vault;
mod view;

use std::path::PathBuf;
use std::sync::Arc;

use gpui_kit::{AnyElement, App, Context, Global, Task, Window};

pub use self::check::{BiometricCheck, NoCheck, SystemCheck};
#[cfg(debug_assertions)]
pub use self::check::{FakeCheck, FakeOutcome};
#[cfg(debug_assertions)]
pub use self::vault::FakeVault;
pub use self::vault::{
    BiometricFailure, BiometricVault, Biometry, DEMO_SIGN_IN_SERVICE, KeychainVault, MemoryVault, SIGN_IN_SERVICE,
    SignInDetails,
};

use self::declined::DeclinedStore;
use crate::copy::Copy;
use crate::state::{AppState, ErrorKind, Paths, PicksCopyOffer, UserFacingError};

/// "Sign in with Touch ID next time?" after a typed sign-in. The PIN stays in memory only until
/// the answer.
#[derive(Clone, PartialEq, Eq)]
pub struct Offer {
    pub origin: String,
    /// `{method}`: "Touch ID".
    pub method: &'static str,
    details: SignInDetails,
    /// `false` when Profile's switch asked for it: the check runs without the question, once the
    /// sign-in dialog has gone.
    pub asks: bool,
}

impl Offer {
    pub fn username(&self) -> &str {
        &self.details.username
    }
}

/// How a sign-in with the check ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BiometricSignInResult {
    SignedIn,
    /// Cancelled, "Enter PIN", not recognised, unavailable, or the details were unreadable.
    Declined,
    /// The library answered with an error (`CredentialsRejected`: the details are gone).
    Failed(UserFacingError),
    /// The library changed meanwhile.
    Stale,
}

/// What the screens show (`status`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    /// The check this computer can run now.
    pub biometry: Option<Biometry>,
    /// The selected library has details kept.
    pub has_details: bool,
    /// A sign-back-in with the check is under way.
    pub running: bool,
    /// Profile's switch was turned on: the next typed sign-in saves without asking.
    pub turning_on: bool,
    /// The offer waiting for an answer: `(method, asks)`.
    pub offer: Option<(&'static str, bool)>,
}

impl Status {
    /// `{method}` while the check is available.
    pub fn method(&self) -> Option<&'static str> {
        self.biometry.map(Biometry::name)
    }

    /// The selected library can sign in with the check now.
    pub fn is_available(&self) -> bool {
        self.biometry.is_some() && self.has_details
    }

    /// Profile's switch: on while details are kept, or while turning it on is under way.
    pub fn is_switched_on(&self) -> bool {
        self.has_details || self.turning_on || self.offer.is_some_and(|(_, asks)| !asks)
    }
}

/// This feature's state for the selected library (iOS `BiometricSignIn`).
pub struct Biometrics {
    vault: Arc<dyn BiometricVault>,
    check: Arc<dyn BiometricCheck>,
    declined: DeclinedStore,
    /// The library the fields below are for (`None`: none selected).
    origin: Option<String>,
    /// The username of the selected library's details; `None`: none kept.
    username: Option<String>,
    offer: Option<Offer>,
    /// My Picks' copy offer, held back until this one is answered.
    held_picks_offer: Option<PicksCopyOffer>,
    running: bool,
    /// `AppState::sign_in` is running for this feature: `did_sign_in` isn't a typed sign-in.
    own_sign_in: bool,
    turning_on: bool,
    /// Bumped each time Profile's switch is turned on, so only the dialog it opened ends it.
    turn_on_id: u64,
    /// The sign-in form's error after a sign-back-in that didn't work (taken once).
    form_error: Option<UserFacingError>,
    /// The sign-in dialog doesn't start the check by itself the next time it opens.
    skips_automatic_check: bool,
}

impl Global for Biometrics {}

impl Biometrics {
    pub fn new(vault: Arc<dyn BiometricVault>, check: Arc<dyn BiometricCheck>, declined_file: Option<PathBuf>) -> Self {
        Self {
            vault,
            check,
            declined: DeclinedStore::at(declined_file),
            origin: None,
            username: None,
            offer: None,
            held_picks_offer: None,
            running: false,
            own_sign_in: false,
            turning_on: false,
            turn_on_id: 0,
            form_error: None,
            skips_automatic_check: false,
        }
    }

    /// The app's own: the login keychain, or memory in demo mode and whenever secrets stay in
    /// memory (`--data-dir`, tests); the system check, or `LS_FAKE_TOUCH_ID` (debug builds).
    /// An app with no files at all (`Paths::memory()`: other features' tests) gets no check, as
    /// iOS tests get `NoBiometricVault`, so they never offer Touch ID on a Mac that has it.
    fn for_app(state: &AppState) -> Self {
        let paths = &state.deps.paths;
        if paths.data.is_none() {
            return Self::new(Arc::new(MemoryVault::new()), Arc::new(NoCheck), None);
        }
        // `--data-dir` (and tests) keep secrets in memory; only the real folders use the keychain.
        let secrets_in_memory = state.is_demo() || *paths != Paths::new(None);
        let vault: Arc<dyn BiometricVault> = if state.is_demo() && demo_keychain() {
            Arc::new(KeychainVault::new(DEMO_SIGN_IN_SERVICE))
        } else if secrets_in_memory {
            Arc::new(MemoryVault::new())
        } else {
            Arc::new(KeychainVault::new(SIGN_IN_SERVICE))
        };
        let mut this = Self::new(vault, system_check(), None);
        this.declined = DeclinedStore::for_paths(paths);
        this
    }

    fn biometry(&self) -> Option<Biometry> {
        self.check.biometry()
    }

    fn has_details(&self) -> bool {
        self.username.is_some()
    }

    fn is_available(&self) -> bool {
        self.has_details() && self.biometry().is_some()
    }

    fn status(&self) -> Status {
        Status {
            biometry: self.biometry(),
            has_details: self.has_details(),
            running: self.running,
            turning_on: self.turning_on,
            offer: self.offer.as_ref().map(|offer| (offer.method, offer.asks)),
        }
    }

    /// Follows the selected library; anything under way for the previous one is dropped.
    fn sync(&mut self, origin: Option<String>) {
        if self.origin == origin {
            return;
        }
        self.username = origin.as_deref().and_then(|origin| self.vault.username(origin));
        self.origin = origin;
        self.offer = None;
        self.held_picks_offer = None;
        self.running = false;
        self.own_sign_in = false;
        self.turning_on = false;
        self.form_error = None;
        self.skips_automatic_check = false;
    }

    /// Reads the username again (another process or the Keychain may have changed it).
    fn reload_username(&mut self) {
        self.username = self.origin.as_deref().and_then(|origin| self.vault.username(origin));
    }

    /// A typed sign-in to `origin` succeeded (iOS `didSignInByTyping`): details kept for another
    /// username go, then the details are saved without asking (Profile's switch) or offered,
    /// unless the reader said Not now. `true` when an offer is now waiting.
    fn did_sign_in_by_typing(&mut self, details: SignInDetails, origin: &str, demo: bool) -> bool {
        if self.origin.as_deref() != Some(origin) {
            return false;
        }
        self.form_error = None;
        self.skips_automatic_check = false;
        if let Some(kept) = &self.username
            && !same_username(kept, &details.username)
        {
            self.vault.delete(origin);
            self.username = None;
        }
        let turning_on = std::mem::take(&mut self.turning_on);
        let Some(biometry) = self.biometry() else { return false };
        if self.has_details() || (!turning_on && self.declined.contains(origin, demo)) {
            return false;
        }
        self.offer = Some(Offer { origin: origin.to_owned(), method: biometry.name(), details, asks: !turning_on });
        true
    }

    /// Signed out of `origin`: its details and its Not now go.
    fn forget(&mut self, origin: &str, demo: bool) {
        self.vault.delete(origin);
        self.declined.set_declined(origin, demo, false);
        if self.origin.as_deref() == Some(origin) {
            self.username = None;
            self.offer = None;
            self.held_picks_offer = None;
            self.turning_on = false;
            self.form_error = None;
            self.skips_automatic_check = false;
        }
    }

    fn delete_details(&mut self) {
        if let Some(origin) = &self.origin {
            self.vault.delete(origin);
        }
        self.username = None;
    }

    fn take_form_error(&mut self) -> Option<UserFacingError> {
        self.form_error.take()
    }

    /// Whether the sign-in dialog starts the check at once (once per opening).
    fn take_automatic_check(&mut self) -> bool {
        let skips = std::mem::take(&mut self.skips_automatic_check);
        self.is_available() && !skips && !self.turning_on && !self.running
    }
}

/// The username a sign-in used, case and spaces aside.
pub fn same_username(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}

/// `LS_DEMO_KEYCHAIN=1` (debug builds, with `--demo`): demo mode keeps the details in the
/// keychain under `com.librespydus.desktop.signin.demo`, so the real store runs.
fn demo_keychain() -> bool {
    cfg!(debug_assertions) && std::env::var("LS_DEMO_KEYCHAIN").is_ok_and(|value| value == "1")
}

#[cfg(debug_assertions)]
fn system_check() -> Arc<dyn BiometricCheck> {
    match FakeCheck::from_env() {
        Some(fake) => Arc::new(fake),
        None => Arc::new(SystemCheck),
    }
}

#[cfg(not(debug_assertions))]
fn system_check() -> Arc<dyn BiometricCheck> {
    Arc::new(SystemCheck)
}

/// The selected library's origin, `None` before one is chosen.
fn current_origin(state: &AppState) -> Option<String> {
    state.library().map(|_| state.backend.origin())
}

/// Runs `f` on this feature's state for `state`'s library (made on first use).
fn with<R>(state: &AppState, cx: &mut App, f: impl FnOnce(&mut Biometrics) -> R) -> R {
    if !cx.has_global::<Biometrics>() {
        cx.set_global(Biometrics::for_app(state));
    }
    let biometrics = cx.global_mut::<Biometrics>();
    biometrics.sync(current_origin(state));
    f(biometrics)
}

/// Tests (and anything else that wants its own store or check): replaces this feature's state.
/// Call it after `AppState::init`. `declined_file`: where Not now is kept (`None`: memory).
pub fn install(
    vault: Arc<dyn BiometricVault>,
    check: Arc<dyn BiometricCheck>,
    declined_file: Option<PathBuf>,
    cx: &mut App,
) {
    cx.set_global(Biometrics::new(vault, check, declined_file));
}

/// What the screens show for the selected library.
pub fn status(cx: &mut App) -> Status {
    AppState::global(cx).update(cx, |state, cx| with(state, cx, |biometrics| biometrics.status()))
}

/// Updates `AppState` with this feature's state for its library, and notifies its observers.
fn update<R>(cx: &mut App, f: impl FnOnce(&mut AppState, &mut Context<AppState>) -> R) -> R {
    AppState::global(cx).update(cx, |state, cx| {
        let result = f(state, cx);
        cx.notify();
        result
    })
}

// ---- Hooks (docs/desktop-app.md §11.1) ----

/// `AppState::report` on `NotLoggedIn`: `true` when a Touch ID sign-in started instead of
/// clearing the session (the session and saved account stay on screen until it ends).
pub fn sign_back_in(state: &mut AppState, cx: &mut Context<AppState>) -> bool {
    enum Start {
        No,
        AlreadyRunning,
        Now,
    }
    let start = with(state, cx, |biometrics| {
        biometrics.reload_username();
        if !biometrics.is_available() {
            Start::No
        } else if biometrics.running {
            Start::AlreadyRunning
        } else {
            biometrics.running = true;
            Start::Now
        }
    });
    match start {
        Start::No => return false,
        Start::AlreadyRunning => return true,
        Start::Now => {}
    }
    let origin = state.backend.origin();
    let signing_in = state.sign_in_with_biometrics(cx);
    cx.spawn(async move |this, cx| {
        let result = signing_in.await;
        #[cfg(debug_assertions)]
        eprintln!("🔐 Touch ID sign-back-in: {result:?}");
        let _ = this.update(cx, |state, cx| {
            if state.backend.origin() != origin {
                return;
            }
            with(state, cx, |biometrics| biometrics.running = false);
            match result {
                BiometricSignInResult::SignedIn | BiometricSignInResult::Stale => {}
                BiometricSignInResult::Declined => state.sign_out_after_check(None, cx),
                BiometricSignInResult::Failed(error) => state.sign_out_after_check(Some(error), cx),
            }
            cx.notify();
        });
    })
    .detach();
    cx.notify();
    true
}

/// `AppState::sign_in` succeeded with these details (a typed sign-in, or this feature's own).
pub fn did_sign_in(state: &mut AppState, username: &str, pin: &str, cx: &mut Context<AppState>) {
    let origin = state.backend.origin();
    let demo = state.is_demo();
    let offered = with(state, cx, |biometrics| {
        if biometrics.own_sign_in {
            biometrics.form_error = None;
            return false;
        }
        biometrics.did_sign_in_by_typing(SignInDetails::new(username, pin), &origin, demo)
    });
    if offered {
        // My Picks' copy offer waits until this one is answered.
        let held = state.picks_copy_offer.take();
        with(state, cx, |biometrics| biometrics.held_picks_offer = held);
    }
    cx.notify();
}

/// `AppState::sign_out` for `origin`: its details and its Not now go.
pub fn did_sign_out(state: &mut AppState, origin: &str, cx: &mut Context<AppState>) {
    let demo = state.is_demo();
    with(state, cx, |biometrics| biometrics.forget(origin, demo));
    cx.notify();
}

/// Profile: the Touch ID switch, or nothing.
pub fn profile_section(window: &mut Window, cx: &mut App) -> Option<AnyElement> {
    view::profile_section(window, cx)
}

/// Called once by the shell at start: the offer after a typed sign-in.
pub fn watch(window: &mut Window, cx: &mut App) {
    view::watch(window, cx);
    #[cfg(feature = "evidence")]
    evidence::start(window, cx);
}

/// `--reset`: deletes every library's saved details (`secrets_in_memory`: nothing to delete).
/// The Not now answers are in the data dir, which `--reset` deletes too.
pub fn reset(secrets_in_memory: bool) {
    if !secrets_in_memory {
        KeychainVault::new(SIGN_IN_SERVICE).delete_all();
    }
    if demo_keychain() {
        KeychainVault::new(DEMO_SIGN_IN_SERVICE).delete_all();
    }
}

// ---- The sign-in dialog (`onboarding::sign_in`) ----

/// What the sign-in dialog does as it opens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignInOpening {
    /// The error to show (after a sign-back-in that didn't work).
    pub error: Option<UserFacingError>,
    /// Start the check at once.
    pub check_now: bool,
    /// Hand it back to `sign_in_closed`.
    pub token: u64,
}

/// For the sign-in dialog when it opens (once per opening).
pub fn sign_in_opened(cx: &mut App) -> SignInOpening {
    AppState::global(cx).update(cx, |state, cx| {
        with(state, cx, |biometrics| SignInOpening {
            error: biometrics.take_form_error(),
            check_now: biometrics.take_automatic_check(),
            token: biometrics.turn_on_id,
        })
    })
}

/// The sign-in dialog opened with `token` closed: turning on from Profile ends with the dialog
/// it opened (not with one that closed since).
pub fn sign_in_closed(token: u64, cx: &mut App) {
    let ends =
        cx.try_global::<Biometrics>().is_some_and(|biometrics| biometrics.turning_on && biometrics.turn_on_id == token);
    if ends {
        update(cx, |state, cx| with(state, cx, |biometrics| biometrics.turning_on = false));
    }
}

// ---- Flows (iOS `extension AppModel`) ----

impl AppState {
    /// The check, then `sign_in` with the kept details exactly as from the form, without the
    /// offers. The sign-in dialog's "Sign in with Touch ID" button and the sign-back-in both use
    /// it. `CredentialsRejected` (the PIN changed) deletes the details.
    pub fn sign_in_with_biometrics(&mut self, cx: &mut Context<Self>) -> Task<BiometricSignInResult> {
        let origin = self.backend.origin();
        let reason = Copy::Biometrics::reason(&self.library_name());
        let ready = with(self, cx, |biometrics| {
            biometrics.has_details().then(|| (biometrics.check.clone(), biometrics.vault.clone()))
        });
        let Some((check, vault)) = ready else { return Task::ready(BiometricSignInResult::Declined) };
        cx.spawn(async move |this, cx| {
            let read = match check.run(reason).await {
                Ok(()) => vault.load(&origin),
                Err(failure) => Err(failure),
            };
            let started = this.update(cx, |state, cx| {
                if state.backend.origin() != origin {
                    return Err(BiometricSignInResult::Stale);
                }
                match read {
                    Ok(details) => {
                        with(state, cx, |biometrics| biometrics.own_sign_in = true);
                        Ok(state.sign_in(details.username, details.pin, cx))
                    }
                    Err(failure) => {
                        if failure == BiometricFailure::Missing {
                            with(state, cx, |biometrics| biometrics.username = None);
                            cx.notify();
                        }
                        Err(BiometricSignInResult::Declined)
                    }
                }
            });
            let signing_in = match started {
                Ok(Ok(task)) => task,
                Ok(Err(result)) => return result,
                Err(_) => return BiometricSignInResult::Stale,
            };
            let result = signing_in.await;
            this.update(cx, |state, cx| {
                if state.backend.origin() != origin {
                    return BiometricSignInResult::Stale;
                }
                with(state, cx, |biometrics| biometrics.own_sign_in = false);
                match result {
                    Ok(()) => BiometricSignInResult::SignedIn,
                    Err(error) => {
                        if error.kind == ErrorKind::CredentialsRejected {
                            with(state, cx, Biometrics::delete_details);
                        }
                        cx.notify();
                        BiometricSignInResult::Failed(error)
                    }
                }
            })
            .unwrap_or(BiometricSignInResult::Stale)
        })
    }

    /// Profile's "Sign in with Touch ID" switch. On opens the sign-in dialog, whose next typed
    /// sign-in saves the details; off deletes them at once (no check).
    pub fn set_biometric_sign_in(&mut self, on: bool, cx: &mut Context<Self>) {
        with(self, cx, |biometrics| {
            if biometrics.origin.is_none() {
                return;
            }
            if on {
                biometrics.turning_on = true;
                biometrics.turn_on_id += 1;
            } else {
                biometrics.turning_on = false;
                biometrics.delete_details();
            }
        });
        if on {
            self.request_sign_in(cx);
        }
        cx.notify();
    }

    /// The reader answered the offer. Use → the check, then the details are saved (a cancelled
    /// or failed check saves nothing and isn't remembered); Not now → remembered for this
    /// library until sign-out. Then My Picks' copy offer may show.
    pub fn answer_biometric_offer(&mut self, accept: bool, cx: &mut Context<Self>) -> Task<()> {
        let demo = self.is_demo();
        let Some((offer, check, vault)) = with(self, cx, |biometrics| {
            biometrics.offer.take().map(|offer| (offer, biometrics.check.clone(), biometrics.vault.clone()))
        }) else {
            return Task::ready(());
        };
        cx.notify();
        if !accept {
            with(self, cx, |biometrics| biometrics.declined.set_declined(&offer.origin, demo, true));
            self.restore_picks_offer(cx);
            return Task::ready(());
        }
        let reason = Copy::Biometrics::reason(&self.library_name());
        cx.spawn(async move |this, cx| {
            let saved = match check.run(reason).await {
                Ok(()) => vault.store(&offer.origin, &offer.details),
                Err(failure) => Err(failure),
            };
            let _ = this.update(cx, |state, cx| {
                if saved.is_ok() {
                    with(state, cx, |biometrics| {
                        if biometrics.origin.as_deref() == Some(offer.origin.as_str()) {
                            biometrics.username = Some(offer.details.username.clone());
                        }
                    });
                }
                state.restore_picks_offer(cx);
                cx.notify();
            });
        })
    }

    /// The offer waiting for an answer, if any.
    pub fn biometric_offer(&self, cx: &mut App) -> Option<Offer> {
        with(self, cx, |biometrics| biometrics.offer.clone())
    }

    /// My Picks' copy offer comes back once Touch ID's is answered, if it still applies.
    fn restore_picks_offer(&mut self, cx: &mut Context<Self>) {
        let Some(held) = with(self, cx, |biometrics| biometrics.held_picks_offer.take()) else { return };
        if self.is_signed_in() && held.origin == self.backend.origin() && self.picks_copy_offer.is_none() {
            self.picks_copy_offer = Some(held);
            cx.notify();
        }
    }

    /// A sign-back-in with the check didn't work: signed out as before, and the sign-in dialog
    /// shows the form (with the library's error) instead of asking for the check again. A
    /// verification challenge has opened its own dialog (`report`), so the form shows no error.
    fn sign_out_after_check(&mut self, error: Option<UserFacingError>, cx: &mut Context<Self>) {
        self.clear_session(cx);
        let shown =
            error.filter(|error| !matches!(error.kind, ErrorKind::NotLoggedIn | ErrorKind::VerificationRequired));
        with(self, cx, |biometrics| {
            biometrics.form_error = shown;
            biometrics.skips_automatic_check = true;
        });
        self.request_sign_in(cx);
    }
}
