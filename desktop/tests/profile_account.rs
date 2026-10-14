//! Headless UI tests of Profile (docs/native-apps.md §6 "Profile"): the library card, signed out
//! with Sign in, signed in with the name and Sign out behind a confirmation, and (debug builds)
//! Expire session leading to the sign-in dialog.

mod support;

use std::sync::Arc;

use gpui_kit::test::TestWindowExt;
use gpui_kit::{AppContext, ScrollDelta, TestAppContext, point, px};
use librespydus_desktop::backend::LibraryBackend;
use librespydus_desktop::backend::demo::{DemoBackend, DemoDelays, DemoFixtures};
use librespydus_desktop::backend::fake::FakeBackend;
use librespydus_desktop::state::library::canning;
use librespydus_desktop::state::{Dependencies, Section};

/// Demo mode at Canning: signed in as "Test Borrower".
fn demo_app(
    cx: &mut TestAppContext,
) -> (gpui_kit::Entity<librespydus_desktop::state::AppState>, gpui_kit::WindowHandle<gpui_kit::base::Root>) {
    let demo = Arc::new(DemoBackend::new(
        FakeBackend::new("https://canning.spydus.com"),
        DemoFixtures::bundled("https://canning.spydus.com", chrono::Local::now().date_naive()).unwrap(),
        DemoDelays::NONE,
        false,
    ));
    let mut deps = Dependencies::in_memory(Arc::new(move |_| -> Arc<dyn LibraryBackend> { demo.clone() }));
    deps.is_demo = true;
    let state = support::app_state(cx, deps);
    state.update(cx, |state, cx| state.select_library(canning(), cx));
    let window = support::open_root(cx);
    state.update(cx, |state, cx| state.select(Section::Profile, cx));
    cx.run_until_parked();
    (state, window)
}

#[gpui_kit::test]
fn sign_out_asks_first(cx: &mut TestAppContext) {
    let (state, window) = demo_app(cx);
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find("profile-library-name").label(), Some("City of Canning"));
        assert_eq!(window.find("profile-account-name").label(), Some("Test Borrower"));
        assert!(window.try_find("profile-version").is_some());
        window.click("profile-sign-out", cx);
    })
    .unwrap();
    cx.run_until_parked();
    // Cancel keeps the account.
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find("confirm-ok").label(), Some("Sign out"));
        window.click("confirm-cancel", cx);
    })
    .unwrap();
    cx.run_until_parked();
    assert!(state.read_with(cx, |state, _| state.is_signed_in()));
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("confirm-ok").is_none());
        window.click("profile-sign-out", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        window.click("confirm-ok", cx);
    })
    .unwrap();
    cx.run_until_parked();
    assert!(!state.read_with(cx, |state, _| state.is_signed_in()));
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("profile-signed-out").is_some());
        assert!(window.try_find("profile-sign-out").is_none());
        // Sign in asks the shell for the dialog.
        window.click("profile-sign-in", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("sign-in-form").is_some());
    })
    .unwrap();
}

#[gpui_kit::test]
fn change_library_opens_the_picker(cx: &mut TestAppContext) {
    let (_state, window) = demo_app(cx);
    cx.update(|cx| {
        librespydus_desktop::features::onboarding::set_directory_fetch(
            Arc::new(|| futures::FutureExt::boxed(futures::future::ready(Ok(vec![canning()])))),
            cx,
        )
    });
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        window.click("profile-change-library", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("library-picker-dialog").is_some());
        assert_eq!(window.find("library-canning.spydus.com").selected(), Some(true));
    })
    .unwrap();
}

#[gpui_kit::test]
fn expire_session_signs_out_and_asks_to_sign_in(cx: &mut TestAppContext) {
    let (state, window) = demo_app(cx);
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        window.scroll("profile-screen", ScrollDelta::Pixels(point(px(0.), px(-2000.))), cx);
    })
    .unwrap();
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        window.click("profile-expire-session", cx);
    })
    .unwrap();
    cx.run_until_parked();
    assert!(!state.read_with(cx, |state, _| state.is_signed_in()));
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("sign-in-form").is_some());
        assert!(window.try_find("profile-signed-out").is_some());
    })
    .unwrap();
}
