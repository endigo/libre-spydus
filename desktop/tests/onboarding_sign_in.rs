//! Headless UI tests of the sign-in dialog (docs/native-apps.md §6 "Login", §5.7): opened from the
//! sidebar, a rejected PIN shows the library's error inline, demo / demo signs in and closes it.
//! The backend is a `DemoBackend` whose public reads are faked (nothing reaches the network).

mod support;

use std::sync::Arc;

use gpui_kit::test::TestWindowExt;
use gpui_kit::{AppContext, TestAppContext};
use librespydus_desktop::backend::LibraryBackend;
use librespydus_desktop::backend::demo::{DemoBackend, DemoDelays, DemoFixtures};
use librespydus_desktop::backend::fake::FakeBackend;
use librespydus_desktop::state::Dependencies;
use librespydus_desktop::state::library::canning;

/// Signed out at Canning, on the demo account (demo / demo signs in).
fn demo_app(
    cx: &mut TestAppContext,
) -> (gpui_kit::Entity<librespydus_desktop::state::AppState>, gpui_kit::WindowHandle<gpui_kit::base::Root>) {
    let demo = Arc::new(DemoBackend::new(
        FakeBackend::new("https://canning.spydus.com"),
        DemoFixtures::bundled("https://canning.spydus.com", chrono::Local::now().date_naive()).unwrap(),
        DemoDelays::NONE,
        false,
    ));
    let deps = Dependencies::in_memory(Arc::new(move |_| -> Arc<dyn LibraryBackend> { demo.clone() }));
    let state = support::app_state(cx, deps);
    state.update(cx, |state, cx| state.select_library(canning(), cx));
    (state, support::open_root(cx))
}

#[gpui_kit::test]
fn a_rejected_pin_shows_the_error_and_demo_signs_in(cx: &mut TestAppContext) {
    let (state, window) = demo_app(cx);
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        window.click("sidebar-sign-in", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("sign-in-form").is_some());
        assert_eq!(window.find("sign-in-title").label(), Some("Sign in"));
        // Nothing typed: Sign in does nothing.
        window.click("sign-in-submit", cx);
        window.click("sign-in-card", cx);
        window.input("demo", cx);
        window.click("sign-in-pin", cx);
        window.input("nope", cx);
        // The reveal toggle shows and hides the PIN.
        assert_eq!(window.find("sign-in-reveal-pin").label(), Some("Show PIN"));
        window.click("sign-in-reveal-pin", cx);
    })
    .unwrap();
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find("sign-in-reveal-pin").label(), Some("Hide PIN"));
        window.click("sign-in-submit", cx);
        window.render_frame(cx);
        // Busy until the library answers.
        assert_eq!(window.find("sign-in-submit").label(), Some("Signing in…"));
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find("sign-in-error").label(), Some("Card number or PIN not recognised."));
        assert_eq!(window.find("sign-in-submit").label(), Some("Sign in"));
    })
    .unwrap();
    assert!(!state.read_with(cx, |state, _| state.is_signed_in()));

    // Typing clears the error; Enter in the PIN field signs in.
    cx.update_window(window.into(), |_, window, cx| {
        window.click("sign-in-pin", cx);
        window.press("cmd-a", cx);
        window.input("demo", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("sign-in-error").is_none());
        window.press("enter", cx);
    })
    .unwrap();
    cx.run_until_parked();
    assert!(state.read_with(cx, |state, _| state.is_signed_in()));
    assert_eq!(state.read_with(cx, |state, _| state.account_name()).as_deref(), Some("Test Borrower"));
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("sign-in-form").is_none(), "success closes the dialog");
        assert_eq!(window.find("sidebar-account").label(), Some("Test Borrower"));
    })
    .unwrap();
}

#[gpui_kit::test]
fn a_second_request_doesnt_stack_another_dialog(cx: &mut TestAppContext) {
    let (state, window) = demo_app(cx);
    state.update(cx, |state, cx| state.request_sign_in(cx));
    cx.run_until_parked();
    state.update(cx, |state, cx| state.request_sign_in(cx));
    cx.run_until_parked();
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("sign-in-form").is_some());
        window.click("sign-in-close", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("sign-in-form").is_none(), "one close leaves no sign-in dialog");
    })
    .unwrap();
}
