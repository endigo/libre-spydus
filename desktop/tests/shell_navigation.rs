//! Headless UI tests of the shell (docs/desktop-app.md §9): the sidebar, the stacks and Back, and
//! the sidebar footer following sign-in.

mod support;

use gpui_kit::test::TestWindowExt;
use gpui_kit::{AppContext, TestAppContext};
use librespydus_desktop::backend::fake::{FAKE_PASSWORD, FakeBackend};
use librespydus_desktop::state::{BookRef, Route, Section};

#[gpui_kit::test]
fn sidebar_items_switch_sections_and_back_pops(cx: &mut TestAppContext) {
    let fake = FakeBackend::new("https://canning.spydus.com");
    let (state, window) = support::signed_out_app(cx, &fake);
    for section in [Section::YourBooks, Section::Search, Section::MyPicks, Section::Profile, Section::Home] {
        cx.update_window(window.into(), |_, window, cx| {
            window.render_frame(cx);
            window.click(section.id(), cx);
        })
        .unwrap();
        assert_eq!(state.read_with(cx, |state, _| state.section()), section);
    }

    // No Back at a section's root; a pushed route shows it, and Back pops it.
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("toolbar-back").is_none());
    })
    .unwrap();
    state.update(cx, |state, cx| {
        state.push(Route::Detail(BookRef::from_url("https://canning.spydus.com/x", "A book")), cx)
    });
    assert_eq!(state.read_with(cx, |state, _| state.stack(Section::Home).len()), 1);
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("detail-screen").is_some());
        window.click("toolbar-back", cx);
    })
    .unwrap();
    assert!(state.read_with(cx, |state, _| state.stack(Section::Home).is_empty()));
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("toolbar-back").is_none());
        assert!(window.try_find("home-screen").is_some());
    })
    .unwrap();

    // Clicking the selected section again goes back to its root.
    state.update(cx, |state, cx| state.push(Route::Detail(BookRef::new("work:1", "B")), cx));
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        window.click(Section::Home.id(), cx);
    })
    .unwrap();
    assert!(state.read_with(cx, |state, _| state.stack(Section::Home).is_empty()));
}

#[gpui_kit::test]
fn signing_in_through_app_state_updates_the_sidebar_footer(cx: &mut TestAppContext) {
    let fake = FakeBackend::new("https://canning.spydus.com");
    let (state, window) = support::signed_out_app(cx, &fake);
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("sidebar-sign-in").is_some());
        assert!(window.try_find("sidebar-account").is_none());
    })
    .unwrap();

    // A wrong PIN is an error and changes nothing.
    let rejected = state.update(cx, |state, cx| state.sign_in("someone".into(), "nope".into(), cx));
    cx.run_until_parked();
    let error = cx.foreground_executor().block_test(rejected).unwrap_err();
    assert_eq!(error.message, "Card number or PIN not recognised.");
    assert!(!state.read_with(cx, |state, _| state.is_signed_in()));

    let task = state.update(cx, |state, cx| state.sign_in("someone".into(), FAKE_PASSWORD.into(), cx));
    cx.run_until_parked();
    cx.foreground_executor().block_test(task).unwrap();
    assert!(state.read_with(cx, |state, _| state.is_signed_in()));
    assert_eq!(state.read_with(cx, |state, _| state.account_name()).as_deref(), Some("Fake Borrower"));
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find("sidebar-account").label(), Some("Fake Borrower"));
        assert!(window.try_find("sidebar-sign-in").is_none());
    })
    .unwrap();

    // Signing out puts Sign in back.
    state.update(cx, |state, cx| state.sign_out(cx));
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("sidebar-sign-in").is_some());
        assert!(window.try_find("sidebar-account").is_none());
    })
    .unwrap();
}
