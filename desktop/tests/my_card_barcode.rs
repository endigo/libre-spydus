//! Headless UI tests of My card (docs/native-apps.md §5.9): the demo account's card number
//! (DEMO0012345) as a barcode with the borrower's name and expiry; Edit card number; and a Sign in
//! prompt while signed out.

mod support;

use std::sync::Arc;

use gpui_kit::test::TestWindowExt;
use gpui_kit::{AppContext, TestAppContext};
use librespydus_desktop::backend::LibraryBackend;
use librespydus_desktop::backend::demo::{CARD_EXPIRES, CARD_NUMBER, DemoBackend, DemoDelays, DemoFixtures};
use librespydus_desktop::backend::fake::FakeBackend;
use librespydus_desktop::features::my_card::open_my_card;
use librespydus_desktop::state::Dependencies;
use librespydus_desktop::state::library::canning;

fn demo_app(
    cx: &mut TestAppContext,
    signed_in: bool,
) -> (gpui_kit::Entity<librespydus_desktop::state::AppState>, gpui_kit::WindowHandle<gpui_kit::base::Root>) {
    let demo = Arc::new(DemoBackend::new(
        FakeBackend::new("https://canning.spydus.com"),
        DemoFixtures::bundled("https://canning.spydus.com", chrono::Local::now().date_naive()).unwrap(),
        DemoDelays::NONE,
        false,
    ));
    let mut deps = Dependencies::in_memory(Arc::new(move |_| -> Arc<dyn LibraryBackend> { demo.clone() }));
    deps.is_demo = signed_in;
    let state = support::app_state(cx, deps);
    state.update(cx, |state, cx| state.select_library(canning(), cx));
    let window = support::open_root(cx);
    cx.run_until_parked();
    (state, window)
}

#[gpui_kit::test]
fn the_demo_card_shows_as_a_barcode(cx: &mut TestAppContext) {
    let (state, window) = demo_app(cx, true);
    assert_eq!(state.read_with(cx, |state, _| state.card_number()).as_deref(), Some(CARD_NUMBER));
    cx.update_window(window.into(), |_, window, cx| open_my_card(window, cx)).unwrap();
    cx.run_until_parked();
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find("my-card-barcode").label(), Some("Barcode for card DEMO0012345"));
        assert_eq!(window.find("my-card-number").label(), Some(CARD_NUMBER));
        assert_eq!(window.find("my-card-name").label(), Some("Test Borrower"));
        assert!(window.try_find("my-card-entry").is_none());
        window.click("my-card-edit", cx);
    })
    .unwrap();
    assert_eq!(state.read_with(cx, |state, _| state.card_expires()).as_deref(), Some(CARD_EXPIRES));

    // Edit card number: the form, filled in; Save shows the new number.
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("my-card-entry").is_some());
        assert!(window.try_find("my-card-cancel").is_some());
        window.click("my-card-number-field", cx);
        window.press("cmd-a", cx);
        window.input("1234", cx);
        window.render_frame(cx);
        window.click("my-card-save", cx);
    })
    .unwrap();
    cx.run_until_parked();
    assert_eq!(state.read_with(cx, |state, _| state.card_number()).as_deref(), Some("1234"));
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find("my-card-number").label(), Some("1234"));
        assert_eq!(window.find("my-card-barcode").label(), Some("Barcode for card 1234"));
    })
    .unwrap();
}

#[gpui_kit::test]
fn signed_out_it_asks_to_sign_in(cx: &mut TestAppContext) {
    let (_state, window) = demo_app(cx, false);
    cx.update_window(window.into(), |_, window, cx| open_my_card(window, cx)).unwrap();
    cx.run_until_parked();
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("my-card-signed-out").is_some());
        window.click("my-card-sign-in", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("my-card-signed-out").is_none());
        assert!(window.try_find("sign-in-form").is_some());
    })
    .unwrap();
}
