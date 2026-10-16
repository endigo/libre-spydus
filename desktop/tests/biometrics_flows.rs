//! Touch ID sign-back-in (docs/native-apps.md §5.14, docs/desktop-app.md §11.4), a port of iOS
//! `BiometricSignInTests` on `FakeVault` + `FakeCheck` and `FakeBackend` (`ok` is the PIN it
//! accepts): the offer after a typed sign-in only, Not now until sign-out, the picks offer after
//! this one, signing back in on `NotLoggedIn` and each way it can end, the sign-in dialog's button
//! and automatic check, Profile's switch, and sign-out.

mod support;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use gpui_kit::base::Root;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{AppContext, Entity, TestAppContext, Window, WindowHandle};
use librespydus_core::SpydusError;
use librespydus_desktop::backend::fake::{FAKE_PASSWORD, FAKE_SESSION, FakeBackend};
use librespydus_desktop::features::biometrics::{
    self, BiometricFailure, BiometricSignInResult, Biometry, FakeCheck, FakeOutcome, FakeVault, SignInDetails, Status,
};
use librespydus_desktop::state::library::canning;
use librespydus_desktop::state::{AppState, BookRef, Section};

const ORIGIN: &str = "https://canning.spydus.com";

fn kept() -> SignInDetails {
    SignInDetails::new("card", FAKE_PASSWORD)
}

struct TestApp {
    state: Entity<AppState>,
    window: WindowHandle<Root>,
    vault: Arc<FakeVault>,
    check: Arc<FakeCheck>,
    fake: Arc<FakeBackend>,
}

/// An app at Canning with `vault` and `check`; signed in when `session` is given; Not now kept in
/// `declined` (memory when `None`).
fn make(
    cx: &mut TestAppContext,
    vault: FakeVault,
    check: FakeCheck,
    session: Option<&str>,
    declined: Option<PathBuf>,
) -> TestApp {
    let fake = FakeBackend::new(ORIGIN);
    let deps = support::fake_deps(&fake);
    if let Some(session) = session {
        deps.sessions.set_value(ORIGIN, Some(session));
    }
    let state = support::app_state(cx, deps);
    let vault = Arc::new(vault);
    let check = Arc::new(check);
    cx.update(|cx| biometrics::install(vault.clone(), check.clone(), declined, cx));
    state.update(cx, |state, cx| state.select_library(canning(), cx));
    let window = support::open_root(cx);
    cx.run_until_parked();
    TestApp { state, window, vault, check, fake }
}

fn plain(cx: &mut TestAppContext) -> TestApp {
    make(cx, FakeVault::new(), FakeCheck::new(), None, None)
}

impl TestApp {
    fn ui(&self, cx: &mut TestAppContext, f: impl FnOnce(&mut Window, &mut gpui_kit::App)) {
        cx.update_window(self.window.into(), |_, window, cx| {
            window.render_frame(cx);
            f(window, cx);
        })
        .unwrap();
        cx.run_until_parked();
    }

    fn has(&self, cx: &mut TestAppContext, id: &'static str) -> bool {
        let mut found = false;
        self.ui(cx, |window, _| found = window.try_find(id).is_some());
        found
    }

    fn label(&self, cx: &mut TestAppContext, id: &'static str) -> Option<String> {
        let mut label = None;
        self.ui(cx, |window, _| label = window.try_find(id).and_then(|found| found.label().map(str::to_owned)));
        label
    }

    fn status(&self, cx: &mut TestAppContext) -> Status {
        cx.update(biometrics::status)
    }

    /// A typed sign-in (what the form calls), finished.
    fn sign_in(&self, cx: &mut TestAppContext, username: &str) {
        let task = self.state.update(cx, |state, cx| state.sign_in(username.into(), FAKE_PASSWORD.into(), cx));
        cx.run_until_parked();
        cx.foreground_executor().block_test(task).unwrap();
        cx.run_until_parked();
    }

    fn answer(&self, cx: &mut TestAppContext, accept: bool) {
        let task = self.state.update(cx, |state, cx| state.answer_biometric_offer(accept, cx));
        cx.run_until_parked();
        cx.foreground_executor().block_test(task);
        cx.run_until_parked();
    }

    fn session_ended(&self, cx: &mut TestAppContext) -> String {
        self.state.update(cx, |state, cx| state.report(&SpydusError::NotLoggedIn, cx)).message
    }

    fn session(&self, cx: &mut TestAppContext) -> Option<String> {
        self.state.read_with(cx, |state, _| state.session().map(str::to_owned))
    }

    fn signed_in(&self, cx: &mut TestAppContext) -> bool {
        self.state.read_with(cx, |state, _| state.is_signed_in())
    }

    /// Lets a dialog that waited for another one to close look again.
    fn settle(&self, cx: &mut TestAppContext) {
        cx.executor().advance_clock(Duration::from_millis(500));
        cx.run_until_parked();
    }
}

// ---- The offer ----

#[gpui_kit::test]
fn a_typed_sign_in_offers_to_keep_the_details(cx: &mut TestAppContext) {
    let app = plain(cx);
    app.sign_in(cx, "card");
    let offer = app.state.update(cx, |state, cx| state.biometric_offer(cx)).expect("an offer");
    assert!(offer.asks);
    assert_eq!(offer.method, "Touch ID");
    assert_eq!(offer.username(), "card");
    // Its dialog: title, the desktop's words, Not now / Use Touch ID.
    assert!(app.has(cx, "touch-id-offer"));
    assert!(app.label(cx, "touch-id-offer").unwrap().contains("stay on this computer, locked by Touch ID"));
    assert_eq!(app.label(cx, "touch-id-offer-use").as_deref(), Some("Use Touch ID"));
    assert_eq!(app.label(cx, "touch-id-offer-not-now").as_deref(), Some("Not now"));
    // Use → the check (with the library's reason), then saved.
    app.ui(cx, |window, cx| window.click("touch-id-offer-use", cx));
    assert!(!app.has(cx, "touch-id-offer"));
    assert_eq!(app.vault.details().get(ORIGIN), Some(&kept()));
    assert_eq!(app.check.runs(), 1);
    assert_eq!(app.check.reasons(), ["Sign in to City of Canning"]);
    let status = app.status(cx);
    assert!(status.has_details && status.offer.is_none());
}

#[gpui_kit::test]
fn a_restored_session_or_no_touch_id_offers_nothing(cx: &mut TestAppContext) {
    let restored = make(cx, FakeVault::new(), FakeCheck::new(), Some("token"), None);
    assert!(restored.signed_in(cx));
    assert!(restored.status(cx).offer.is_none());
    assert!(!restored.has(cx, "touch-id-offer"));
}

#[gpui_kit::test]
fn without_touch_id_nothing_is_offered_or_shown(cx: &mut TestAppContext) {
    let app = make(cx, FakeVault::new(), FakeCheck::with(None, FakeOutcome::Pass), None, None);
    app.sign_in(cx, "card");
    let status = app.status(cx);
    assert!(status.offer.is_none());
    assert_eq!(status.method(), None);
    app.state.update(cx, |state, cx| state.select(Section::Profile, cx));
    assert!(!app.has(cx, "profile-touch-id"));
}

#[gpui_kit::test]
fn a_cancelled_check_keeps_nothing_and_asks_again_next_time(cx: &mut TestAppContext) {
    let app = make(
        cx,
        FakeVault::new(),
        FakeCheck::with(Some(Biometry::TouchId), FakeOutcome::Fail(BiometricFailure::Declined)),
        None,
        None,
    );
    app.sign_in(cx, "card");
    app.answer(cx, true);
    assert!(app.vault.details().is_empty());
    assert!(!app.status(cx).has_details);
    app.sign_in(cx, "card");
    assert!(app.status(cx).offer.is_some());
}

#[gpui_kit::test]
fn not_now_is_remembered_until_sign_out(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("Biometrics.json");
    let app = make(cx, FakeVault::new(), FakeCheck::new(), None, Some(file.clone()));
    app.sign_in(cx, "card");
    app.ui(cx, |window, cx| window.click("touch-id-offer-not-now", cx));
    assert!(!app.has(cx, "touch-id-offer"));
    assert_eq!(app.check.runs(), 0);
    app.sign_in(cx, "card");
    assert!(app.status(cx).offer.is_none());

    // Remembered across launches (the same file).
    cx.update(|cx| biometrics::install(app.vault.clone(), app.check.clone(), Some(file.clone()), cx));
    app.sign_in(cx, "card");
    assert!(app.status(cx).offer.is_none());

    app.state.update(cx, |state, cx| state.sign_out(cx));
    app.sign_in(cx, "card");
    assert!(app.status(cx).offer.is_some());
}

#[gpui_kit::test]
fn my_picks_offer_waits_for_this_one(cx: &mut TestAppContext) {
    let app = plain(cx);
    app.state.update(cx, |state, cx| state.set_picked(BookRef::new("work:1", "First"), true, cx).detach());
    app.sign_in(cx, "card");
    app.settle(cx);
    // Touch ID's offer first; My Picks' is held back.
    assert!(app.has(cx, "touch-id-offer"));
    assert!(!app.has(cx, "picks-offer"));
    assert!(app.state.read_with(cx, |state, _| state.picks_copy_offer().is_none()));
    app.ui(cx, |window, cx| window.click("touch-id-offer-not-now", cx));
    app.settle(cx);
    assert!(app.state.read_with(cx, |state, _| state.picks_copy_offer().is_some()));
    assert!(app.has(cx, "picks-offer"));
    assert!(!app.has(cx, "touch-id-offer"));
}

#[gpui_kit::test]
fn a_typed_sign_in_with_another_username_deletes_the_details(cx: &mut TestAppContext) {
    let app = make(cx, FakeVault::with(ORIGIN, kept()), FakeCheck::new(), None, None);
    // The same username (case and spaces aside) keeps them, and there's nothing to offer.
    app.sign_in(cx, " CARD ");
    assert_eq!(app.vault.details().get(ORIGIN), Some(&kept()));
    assert!(app.status(cx).offer.is_none());

    app.sign_in(cx, "someone-else");
    assert!(app.vault.details().is_empty());
    assert!(!app.status(cx).has_details);
    let offer = app.state.update(cx, |state, cx| state.biometric_offer(cx)).unwrap();
    assert_eq!(offer.username(), "someone-else");
}

// ---- Signing back in ----

#[gpui_kit::test]
fn an_ended_session_signs_back_in_with_the_check(cx: &mut TestAppContext) {
    let app = make(cx, FakeVault::with(ORIGIN, kept()), FakeCheck::new(), Some("stale"), None);
    let generation = app.state.read_with(cx, |state, _| state.account_generation());
    app.session_ended(cx);
    // The session stays (and no dialog) while the check runs; more ended sessions are the same one.
    assert!(app.signed_in(cx));
    assert!(app.status(cx).running);
    app.session_ended(cx);
    cx.run_until_parked();

    assert!(!app.status(cx).running);
    assert_eq!(app.session(cx).as_deref(), Some(FAKE_SESSION));
    assert!(!app.has(cx, "sign-in-form"));
    assert!(app.state.read_with(cx, |state, _| state.account_generation()) > generation);
    assert_eq!(app.check.runs(), 1);
    assert_eq!(app.fake.logins(), ["card"]);
    assert!(app.status(cx).offer.is_none(), "no offer after a sign-in with the check");
}

#[gpui_kit::test]
fn while_the_check_runs_the_session_stays_on_screen(cx: &mut TestAppContext) {
    let app = make(
        cx,
        FakeVault::with(ORIGIN, kept()),
        FakeCheck::with(Some(Biometry::TouchId), FakeOutcome::Hang),
        Some("stale"),
        None,
    );
    app.session_ended(cx);
    cx.run_until_parked();
    assert!(app.signed_in(cx));
    assert!(app.status(cx).running);
    assert!(!app.has(cx, "sign-in-form"));
    assert!(app.fake.logins().is_empty());
}

#[gpui_kit::test]
fn a_cancelled_check_signs_out_and_shows_the_form(cx: &mut TestAppContext) {
    let app = make(
        cx,
        FakeVault::with(ORIGIN, kept()),
        FakeCheck::with(Some(Biometry::TouchId), FakeOutcome::Fail(BiometricFailure::Declined)),
        Some("stale"),
        None,
    );
    app.session_ended(cx);
    cx.run_until_parked();
    assert!(!app.status(cx).running);
    assert!(!app.signed_in(cx));
    // The form, with the Touch ID button but no second automatic check and no error.
    assert!(app.has(cx, "sign-in-form"));
    assert!(!app.has(cx, "sign-in-error"));
    assert_eq!(app.label(cx, "sign-in-touch-id").as_deref(), Some("Sign in with Touch ID"));
    assert_eq!(app.check.runs(), 1);
    assert!(app.status(cx).has_details, "kept");

    // The button asks again; this time it passes and the dialog closes.
    app.check.set_outcome(FakeOutcome::Pass);
    app.ui(cx, |window, cx| window.click("sign-in-touch-id", cx));
    assert_eq!(app.check.runs(), 2);
    assert_eq!(app.session(cx).as_deref(), Some(FAKE_SESSION));
    assert!(!app.has(cx, "sign-in-form"));
}

#[gpui_kit::test]
fn a_changed_pin_deletes_the_details_and_says_so(cx: &mut TestAppContext) {
    let app =
        make(cx, FakeVault::with(ORIGIN, SignInDetails::new("card", "old")), FakeCheck::new(), Some("stale"), None);
    app.session_ended(cx);
    cx.run_until_parked();
    assert!(!app.signed_in(cx));
    assert!(app.vault.details().is_empty());
    assert!(!app.status(cx).has_details);
    assert_eq!(app.label(cx, "sign-in-error").as_deref(), Some("Card number or PIN not recognised."));
    assert!(!app.has(cx, "sign-in-touch-id"));
}

#[gpui_kit::test]
fn another_error_keeps_the_details(cx: &mut TestAppContext) {
    let app = make(cx, FakeVault::with(ORIGIN, kept()), FakeCheck::new(), Some("stale"), None);
    *app.fake.login_error.lock().unwrap() = Some(SpydusError::Network { message: "offline".into() });
    app.session_ended(cx);
    cx.run_until_parked();
    assert!(!app.signed_in(cx));
    assert_eq!(app.vault.details().get(ORIGIN), Some(&kept()));
    assert_eq!(app.label(cx, "sign-in-error").as_deref(), Some("Can’t reach City of Canning. Check your connection."));
    assert_eq!(app.check.runs(), 1, "no second automatic check");
}

#[gpui_kit::test]
fn unreadable_details_are_gone(cx: &mut TestAppContext) {
    let vault = FakeVault::with(ORIGIN, kept());
    vault.set_load_failure(Some(BiometricFailure::Missing));
    let app = make(cx, vault, FakeCheck::new(), Some("stale"), None);
    app.session_ended(cx);
    cx.run_until_parked();
    assert!(app.has(cx, "sign-in-form"));
    assert!(!app.status(cx).has_details);
    assert!(!app.has(cx, "sign-in-touch-id"));
}

#[gpui_kit::test]
fn without_details_an_ended_session_asks_as_before(cx: &mut TestAppContext) {
    let app = make(cx, FakeVault::new(), FakeCheck::new(), Some("stale"), None);
    assert_eq!(app.session_ended(cx), "Please sign in again.");
    cx.run_until_parked();
    assert!(!app.signed_in(cx));
    assert!(app.has(cx, "sign-in-form"));
    assert_eq!(app.check.runs(), 0);
}

// ---- The sign-in dialog ----

#[gpui_kit::test]
fn the_dialog_starts_the_check_at_once_and_signs_in(cx: &mut TestAppContext) {
    let app = make(cx, FakeVault::with(ORIGIN, kept()), FakeCheck::new(), None, None);
    app.state.update(cx, |state, cx| state.request_sign_in(cx));
    cx.run_until_parked();
    assert_eq!(app.check.runs(), 1);
    assert_eq!(app.session(cx).as_deref(), Some(FAKE_SESSION));
    assert!(!app.has(cx, "sign-in-form"), "success closes the dialog");
}

#[gpui_kit::test]
fn the_dialogs_check_shows_the_librarys_error(cx: &mut TestAppContext) {
    let app = make(cx, FakeVault::with(ORIGIN, SignInDetails::new("card", "old")), FakeCheck::new(), None, None);
    app.state.update(cx, |state, cx| state.request_sign_in(cx));
    cx.run_until_parked();
    // The check passed; the library turned the old PIN down: the form says so, the details go.
    assert_eq!(app.check.runs(), 1);
    assert!(app.has(cx, "sign-in-form"));
    assert_eq!(app.label(cx, "sign-in-error").as_deref(), Some("Card number or PIN not recognised."));
    assert!(!app.has(cx, "sign-in-touch-id"));
    assert!(!app.status(cx).is_available());
    assert!(!app.signed_in(cx));
}

#[gpui_kit::test]
fn the_sign_in_with_the_check_reports_how_it_ended(cx: &mut TestAppContext) {
    let app = make(cx, FakeVault::with(ORIGIN, kept()), FakeCheck::new(), None, None);
    let task = app.state.update(cx, |state, cx| state.sign_in_with_biometrics(cx));
    cx.run_until_parked();
    assert_eq!(cx.foreground_executor().block_test(task), BiometricSignInResult::SignedIn);
    app.state.update(cx, |state, cx| state.clear_session(cx));
    app.check.set_outcome(FakeOutcome::Fail(BiometricFailure::Declined));
    let task = app.state.update(cx, |state, cx| state.sign_in_with_biometrics(cx));
    cx.run_until_parked();
    assert_eq!(cx.foreground_executor().block_test(task), BiometricSignInResult::Declined);
    assert!(app.status(cx).has_details);
}

#[gpui_kit::test]
fn the_privacy_note_says_so_when_touch_id_is_available(cx: &mut TestAppContext) {
    let app = plain(cx);
    app.state.update(cx, |state, cx| state.request_sign_in(cx));
    cx.run_until_parked();
    assert_eq!(
        app.label(cx, "sign-in-privacy-note").as_deref(),
        Some(
            "LibreSpydus signs in directly with City of Canning. It keeps your PIN on this computer only if you turn on Touch ID."
        )
    );
    // No details: no Touch ID button, no check.
    assert!(!app.has(cx, "sign-in-touch-id"));
    assert_eq!(app.check.runs(), 0);

    let none = make(cx, FakeVault::new(), FakeCheck::with(None, FakeOutcome::Pass), None, None);
    none.state.update(cx, |state, cx| state.request_sign_in(cx));
    cx.run_until_parked();
    assert_eq!(
        none.label(cx, "sign-in-privacy-note").as_deref(),
        Some("LibreSpydus signs in directly with City of Canning and doesn’t store your PIN.")
    );
}

// ---- Profile, sign-out ----

#[gpui_kit::test]
fn profiles_switch_turns_on_through_the_dialog_and_off_at_once(cx: &mut TestAppContext) {
    let app = make(cx, FakeVault::new(), FakeCheck::new(), Some("token"), None);
    app.state.update(cx, |state, cx| state.select(Section::Profile, cx));
    assert_eq!(app.label(cx, "profile-touch-id-title").as_deref(), Some("Sign in with Touch ID"));
    let mut checked = None;
    app.ui(cx, |window, _| checked = window.find("profile-touch-id-switch").checked());
    assert_eq!(checked, Some(false));

    // On: the sign-in dialog with the note, and no automatic check.
    app.ui(cx, |window, cx| window.click("profile-touch-id-switch", cx));
    assert!(app.has(cx, "sign-in-form"));
    assert!(app.has(cx, "sign-in-turn-on-note"));
    assert!(app.status(cx).turning_on);
    assert_eq!(app.check.runs(), 0);

    // Closing the dialog leaves it off. (Closed directly: a click on its × can land while the
    // dialog is still sliding in, which runs on real time.)
    app.ui(cx, |window, cx| {
        use gpui_kit::component::WindowExt;
        window.close_dialog(cx)
    });
    assert!(!app.has(cx, "sign-in-form"));
    assert!(!app.status(cx).turning_on);
    app.ui(cx, |window, _| checked = window.find("profile-touch-id-switch").checked());
    assert_eq!(checked, Some(false));

    // On again, and a typed sign-in: saved without the question, once the dialog has gone.
    app.ui(cx, |window, cx| window.click("profile-touch-id-switch", cx));
    app.ui(cx, |window, cx| {
        window.click("sign-in-card", cx);
        window.input("card", cx);
        window.click("sign-in-pin", cx);
        window.input(FAKE_PASSWORD, cx);
    });
    app.ui(cx, |window, cx| window.click("sign-in-submit", cx));
    app.settle(cx);
    assert!(!app.has(cx, "sign-in-form"));
    assert!(!app.has(cx, "touch-id-offer"), "no question");
    assert_eq!(app.vault.details().get(ORIGIN), Some(&kept()));
    assert_eq!(app.check.runs(), 1);
    app.ui(cx, |window, _| checked = window.find("profile-touch-id-switch").checked());
    assert_eq!(checked, Some(true));

    // Off deletes them at once, with no check.
    app.ui(cx, |window, cx| window.click("profile-touch-id-switch", cx));
    assert!(app.vault.details().is_empty());
    assert!(!app.status(cx).has_details);
    assert_eq!(app.check.runs(), 1);
}

#[gpui_kit::test]
fn profile_shows_no_switch_signed_out(cx: &mut TestAppContext) {
    let app = plain(cx);
    app.state.update(cx, |state, cx| state.select(Section::Profile, cx));
    assert!(!app.has(cx, "profile-touch-id"));
}

#[gpui_kit::test]
fn sign_out_forgets_the_details(cx: &mut TestAppContext) {
    let app = make(cx, FakeVault::with(ORIGIN, kept()), FakeCheck::new(), Some("token"), None);
    assert!(app.status(cx).has_details);
    app.state.update(cx, |state, cx| state.sign_out(cx));
    assert!(app.vault.details().is_empty());
    assert!(!app.status(cx).has_details);
}
