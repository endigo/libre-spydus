//! Headless UI tests of "Confirm you're human" (E2; docs/desktop-app.md §11.2): a library that
//! answers `HumanVerificationRequired` brings up the dialog; Open check starts the helper (a fake
//! launcher here); its cookie completes the check and Home and Search retry; Cancel and a closed
//! helper window; "unavailable" after two checks that didn't help; Profile's verified card.

mod search_support;
mod support;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use futures::FutureExt;
use futures::channel::oneshot;
use gpui_kit::base::Root;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{AppContext, Entity, TestAppContext, WindowHandle};
use librespydus_core::{SearchSort, SpydusError};
use librespydus_desktop::backend::LibraryBackend;
use librespydus_desktop::features::verification::{
    self, HelperArgs, HelperLauncher, HelperOutcome, HelperRun, Phase, Verification,
};
use librespydus_desktop::state::library::canning;
use librespydus_desktop::state::{AppState, Dependencies, ErrorKind, Section};
use search_support::{ORIGIN, ScriptedBackend, page, records};

const COOKIE: &str = "aws-waf-token=from-the-helper";
/// Home's New arrivals.
const ARRIVALS: &str = "search:fiction:newest";

/// Records every launch; the test answers each one.
#[derive(Default)]
struct FakeLauncher {
    launches: Mutex<Vec<HelperArgs>>,
    answers: Mutex<Vec<oneshot::Sender<HelperOutcome>>>,
    stops: Arc<AtomicUsize>,
}

impl FakeLauncher {
    fn launches(&self) -> Vec<HelperArgs> {
        self.launches.lock().unwrap().clone()
    }

    fn answer(&self, outcome: HelperOutcome) {
        let sender = self.answers.lock().unwrap().pop().expect("a running helper");
        let _ = sender.send(outcome);
    }

    fn stops(&self) -> usize {
        self.stops.load(Ordering::SeqCst)
    }
}

impl HelperLauncher for FakeLauncher {
    fn launch(&self, args: &HelperArgs) -> HelperRun {
        self.launches.lock().unwrap().push(args.clone());
        let (sender, receiver) = oneshot::channel();
        self.answers.lock().unwrap().push(sender);
        let stops = self.stops.clone();
        HelperRun {
            outcome: receiver.map(|answer| answer.unwrap_or(HelperOutcome::Closed)).boxed(),
            stop: Box::new(move || {
                stops.fetch_add(1, Ordering::SeqCst);
            }),
        }
    }
}

fn verification_required() -> SpydusError {
    SpydusError::HumanVerificationRequired
}

/// Canning, signed out, on `backend`, with `launcher` in place of the helper process.
fn app(
    cx: &mut TestAppContext,
    backend: &Arc<ScriptedBackend>,
    launcher: &Arc<FakeLauncher>,
) -> (Entity<AppState>, WindowHandle<Root>) {
    let backend = backend.clone();
    let deps = Dependencies::in_memory(Arc::new(move |_| -> Arc<dyn LibraryBackend> { backend.clone() }));
    let state = support::app_state(cx, deps);
    let launcher: Arc<dyn HelperLauncher> = launcher.clone();
    cx.update(|cx| verification::set_launcher(launcher, cx));
    state.update(cx, |state, cx| state.select_library(canning(), cx));
    let window = support::open_root(cx);
    (state, window)
}

fn frame(
    cx: &mut TestAppContext,
    window: WindowHandle<Root>,
    check: impl FnOnce(&mut gpui_kit::Window, &mut gpui_kit::App),
) {
    cx.run_until_parked();
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        check(window, cx);
    })
    .unwrap();
    cx.run_until_parked();
}

fn exists(cx: &mut TestAppContext, window: WindowHandle<Root>, id: &'static str) -> bool {
    let mut found = false;
    frame(cx, window, |window, _| found = window.try_find(id).is_some());
    found
}

fn click(cx: &mut TestAppContext, window: WindowHandle<Root>, id: &'static str) {
    frame(cx, window, |window, cx| window.click(id, cx));
}

fn phase(cx: &mut TestAppContext) -> Phase {
    cx.update(|cx| Verification::global(cx).read(cx).phase())
}

#[gpui_kit::test]
fn a_check_on_home_completes_and_the_shelf_retries(cx: &mut TestAppContext) {
    let backend = ScriptedBackend::new();
    backend.on_sorted("fiction", SearchSort::Newest, Err(verification_required()));
    let launcher = Arc::new(FakeLauncher::default());
    let (state, window) = app(cx, &backend, &launcher);

    // Home's shelf asked for the check: the dialog, with the desktop wording.
    assert!(exists(cx, window, "verification-message"));
    let mut message = String::new();
    frame(cx, window, |window, _| message = window.find("verification-message").label().unwrap_or_default().to_owned());
    assert_eq!(
        message, "City of Canning asks you to confirm you’re human. Complete the check in the window that opens.",
        "{message}"
    );
    assert!(exists(cx, window, "home-new-arrivals-error"), "the shelf shows the error behind the dialog");
    assert_eq!(backend.count(ARRIVALS), 1);

    // Open check: the helper starts for this library with the core's User-Agent and its name.
    click(cx, window, "verification-open");
    assert_eq!(phase(cx), Phase::Checking);
    let launches = launcher.launches();
    assert_eq!(launches.len(), 1);
    assert_eq!(launches[0].origin, ORIGIN);
    assert_eq!(launches[0].user_agent, librespydus_core::user_agent());
    assert_eq!(launches[0].title, "City of Canning");
    assert!(exists(cx, window, "verification-waiting"));
    assert!(!exists(cx, window, "verification-open"), "no second window while one is open");

    // The reader finishes; the library now answers.
    backend.on_sorted("fiction", SearchSort::Newest, Ok(page(records("New", 3), 3, None)));
    launcher.answer(HelperOutcome::Cookie(COOKIE.into()));
    assert!(!exists(cx, window, "verification-message"), "the dialog closes");
    assert_eq!(phase(cx), Phase::Idle);
    assert!(state.read_with(cx, |state, _| state.is_verified()));
    assert_eq!(state.read_with(cx, |state, _| state.verification_generation()), 1);
    assert_eq!(backend.count(ARRIVALS), 2, "New arrivals loads again");
    assert!(!exists(cx, window, "home-new-arrivals-error"));

    // Profile: verified on this computer, and Clear verification.
    click(cx, window, Section::Profile.id());
    assert!(exists(cx, window, "profile-verified"));
    let mut label = String::new();
    frame(cx, window, |window, _| label = window.find("profile-verified").label().unwrap_or_default().to_owned());
    assert_eq!(label, "Verified on this computer");
    assert!(exists(cx, window, "profile-clear-verification"));
}

#[gpui_kit::test]
fn a_search_retries_after_the_check(cx: &mut TestAppContext) {
    let backend = ScriptedBackend::new();
    backend.on_search("dune", Err(verification_required()));
    let launcher = Arc::new(FakeLauncher::default());
    let (state, window) = app(cx, &backend, &launcher);

    click(cx, window, Section::Search.id());
    frame(cx, window, |window, cx| {
        window.input("dune", cx);
        window.press("enter", cx);
    });
    assert!(exists(cx, window, "verification-message"));
    assert!(exists(cx, window, "results-error-state"));
    assert_eq!(backend.count("search:dune:"), 1);

    click(cx, window, "verification-open");
    backend.on_search("dune", Ok(page(records("Dune", 2), 2, None)));
    launcher.answer(HelperOutcome::Cookie(COOKIE.into()));
    assert!(!exists(cx, window, "verification-message"));
    assert!(state.read_with(cx, |state, _| state.is_verified()));
    assert_eq!(backend.count("search:dune:"), 2, "the search runs again");
    assert!(exists(cx, window, "result-0"));
}

#[gpui_kit::test]
fn cancel_closes_the_dialog_and_the_helper(cx: &mut TestAppContext) {
    let backend = ScriptedBackend::new();
    backend.on_sorted("fiction", SearchSort::Newest, Err(verification_required()));
    let launcher = Arc::new(FakeLauncher::default());
    let (state, window) = app(cx, &backend, &launcher);

    // Cancel before opening the check.
    assert!(exists(cx, window, "verification-message"));
    click(cx, window, "verification-cancel");
    assert!(!exists(cx, window, "verification-message"));
    assert_eq!(phase(cx), Phase::Idle);
    assert!(launcher.launches().is_empty());
    assert!(!state.read_with(cx, |state, _| state.is_verified()));

    // A later failure asks again; Cancel while the helper is open stops it.
    state.update(cx, |state, cx| {
        state.report(&verification_required(), cx);
    });
    assert!(exists(cx, window, "verification-message"));
    click(cx, window, "verification-open");
    assert_eq!(launcher.stops(), 0);
    click(cx, window, "verification-cancel");
    assert_eq!(launcher.stops(), 1, "the helper window closes");
    assert!(!exists(cx, window, "verification-message"));
    assert_eq!(phase(cx), Phase::Idle);
    // Its late answer changes nothing.
    launcher.answer(HelperOutcome::Cookie(COOKIE.into()));
    cx.run_until_parked();
    assert!(!state.read_with(cx, |state, _| state.is_verified()));
    assert!(!exists(cx, window, "verification-message"));
}

#[gpui_kit::test]
fn closing_the_helper_window_comes_back_to_the_dialog_with_a_note(cx: &mut TestAppContext) {
    let backend = ScriptedBackend::new();
    backend.on_sorted("fiction", SearchSort::Newest, Err(verification_required()));
    let launcher = Arc::new(FakeLauncher::default());
    let (state, window) = app(cx, &backend, &launcher);

    click(cx, window, "verification-open");
    launcher.answer(HelperOutcome::Closed);
    assert!(exists(cx, window, "verification-message"));
    let mut note = String::new();
    frame(cx, window, |window, _| note = window.find("verification-note").label().unwrap_or_default().to_owned());
    assert_eq!(note, "The check window closed before the check was finished.");
    assert!(exists(cx, window, "verification-open"), "Open check again");

    // A helper that couldn't start says so.
    click(cx, window, "verification-open");
    launcher.answer(HelperOutcome::Failed("spawn".into()));
    frame(cx, window, |window, _| note = window.find("verification-note").label().unwrap_or_default().to_owned());
    assert_eq!(note, "The check window couldn’t open.");
    assert!(!state.read_with(cx, |state, _| state.is_verified()));
    assert_eq!(launcher.launches().len(), 2);
}

#[gpui_kit::test]
fn the_library_is_unavailable_after_two_checks_that_did_not_help(cx: &mut TestAppContext) {
    let backend = ScriptedBackend::new();
    backend.on_sorted("fiction", SearchSort::Newest, Err(verification_required()));
    let launcher = Arc::new(FakeLauncher::default());
    let (state, window) = app(cx, &backend, &launcher);

    // Two completed checks; the library keeps asking, so the dialog comes back once.
    for completed in 1..=2 {
        assert!(exists(cx, window, "verification-message"), "check {completed}");
        click(cx, window, "verification-open");
        launcher.answer(HelperOutcome::Cookie(format!("aws-waf-token=try-{completed}")));
        cx.run_until_parked();
        assert_eq!(cx.update(|cx| Verification::global(cx).read(cx).completions(ORIGIN)), completed);
    }
    assert_eq!(backend.count(ARRIVALS), 3, "the shelf retried after each check");
    assert!(!exists(cx, window, "verification-message"), "no third dialog");
    assert_eq!(phase(cx), Phase::Idle);
    assert_eq!(launcher.launches().len(), 2);

    // What a screen shows now: iOS's verificationUnavailable.
    let error = state.update(cx, |state, cx| state.report(&verification_required(), cx));
    assert_eq!(error.kind, ErrorKind::VerificationUnavailable);
    assert_eq!(error.message, "City of Canning can’t be used from the app right now. Try its website instead.");
    assert!(!exists(cx, window, "verification-message"));

    // Clear verification (Profile) starts the count again.
    state.update(cx, |state, cx| state.clear_verification(cx));
    cx.run_until_parked();
    assert_eq!(cx.update(|cx| Verification::global(cx).read(cx).completions(ORIGIN)), 0);
    let error = state.update(cx, |state, cx| state.report(&verification_required(), cx));
    assert_eq!(error.kind, ErrorKind::VerificationRequired);
    assert!(exists(cx, window, "verification-message"));
}
