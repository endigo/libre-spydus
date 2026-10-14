//! Headless UI tests of Home (D3; docs/desktop-app.md §9): the shelves load on their own, saved
//! covers show first (§5.13), a genre tile pushes Results, and signed in (demo data) "Your loans"
//! shows the demo loans.

mod search_support;
mod support;

use std::sync::Arc;

use gpui_kit::base::Root;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{AppContext, Entity, TestAppContext, WindowHandle};
use librespydus_core::{FeaturedKind, FeaturedLink, SearchSort, SpydusError};
use librespydus_desktop::backend::LibraryBackend;
use librespydus_desktop::backend::demo::{DemoBackend, DemoDelays, DemoFixtures};
use librespydus_desktop::state::formatting::today;
use librespydus_desktop::state::library::canning;
use librespydus_desktop::state::{AppState, Dependencies, Route, Section};
use search_support::{ORIGIN, ScriptedBackend, page, records};

fn app_on(
    cx: &mut TestAppContext,
    backend: Arc<dyn LibraryBackend>,
    demo: bool,
) -> (Entity<AppState>, WindowHandle<Root>) {
    let mut deps = Dependencies::in_memory(Arc::new(move |_| -> Arc<dyn LibraryBackend> { backend.clone() }));
    deps.is_demo = demo;
    let state = support::app_state(cx, deps);
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

fn link(title: &str, kind: FeaturedKind, url: &str) -> FeaturedLink {
    FeaturedLink { title: title.into(), description: None, icon: Some("book".into()), kind, url: url.into() }
}

#[gpui_kit::test]
fn signed_out_home_shows_its_shelves_and_a_sign_in_card(cx: &mut TestAppContext) {
    let backend = ScriptedBackend::new();
    backend.on_sorted("fiction", SearchSort::Newest, Ok(page(records("New", 12), 500, None)));
    backend.on_featured(Ok(vec![link("What's New", FeaturedKind::Collection, "https://x/c")]));
    let (state, window) = app_on(cx, backend.clone(), false);
    frame(cx, window, |window, _| {
        assert_eq!(window.find("home-greeting").label(), Some("Hello!"));
        assert!(window.try_find("home-my-card").is_none());
        assert_eq!(window.find("home-new-arrival-0").label(), Some("New 1"));
        assert!(window.try_find("home-new-arrival-9").is_some());
        assert!(window.try_find("home-new-arrival-10").is_none(), "ten covers at most");
        assert_eq!(window.find("home-featured-0").label(), Some("What's New"));
        assert!(window.try_find("home-sign-in-card").is_some());
        assert!(window.try_find("home-loans-view-all").is_none());
    });
    let calls = backend.calls();
    assert!(calls.contains(&"search:fiction:newest".to_owned()));
    assert!(calls.contains(&"save_shelf:new_arrivals".to_owned()));
    assert!(calls.contains(&"save_featured".to_owned()));

    // View All: the shelf's results, newest first.
    frame(cx, window, |window, cx| window.click("home-new-arrivals-view-all", cx));
    let stack = state.read_with(cx, |state, _| state.stack(Section::Home).to_vec());
    assert_eq!(
        stack,
        [Route::Results { query: "fiction".into(), sort: SearchSort::Newest, title: "New arrivals".into() }]
    );

    // Sign in asks the shell for the dialog.
    state.update(cx, |state, cx| state.pop(cx));
    frame(cx, window, |window, cx| {
        window.scroll(
            "home-screen",
            gpui_kit::ScrollDelta::Pixels(gpui_kit::point(gpui_kit::px(0.), gpui_kit::px(-5_000.))),
            cx,
        )
    });
    frame(cx, window, |window, cx| window.click("home-sign-in", cx));
    // The shell took the request and opened its dialog.
    assert!(!state.update(cx, |state, _| state.take_sign_in_request()));
}

#[gpui_kit::test]
fn a_genre_tile_pushes_its_results(cx: &mut TestAppContext) {
    let backend = ScriptedBackend::new();
    backend.on_search("juvenile fiction", Ok(page(records("Kids", 2), 2, None)));
    let (state, window) = app_on(cx, backend.clone(), false);
    frame(cx, window, |window, cx| window.click("home-genre-children", cx));
    let stack = state.read_with(cx, |state, _| state.stack(Section::Home).to_vec());
    assert_eq!(
        stack,
        [Route::Results { query: "juvenile fiction".into(), sort: SearchSort::Relevance, title: "Children".into() }]
    );
    frame(cx, window, |window, _| {
        assert_eq!(window.find("results-title").label(), Some("Children"));
        assert_eq!(window.find("result-0").label(), Some("Kids 1"));
        assert_eq!(window.find("results-sort-relevance").selected(), Some(true));
    });
    assert!(backend.calls().contains(&"search:juvenile fiction:relevance".to_owned()));
}

#[gpui_kit::test]
fn one_failing_shelf_never_blocks_the_others(cx: &mut TestAppContext) {
    let backend = ScriptedBackend::new();
    backend.on_sorted("fiction", SearchSort::Newest, Err(SpydusError::Network { message: "slow".into() }));
    backend.on_featured(Ok(vec![link("Coming Soon", FeaturedKind::Search, "https://x/soon")]));
    let (_, window) = app_on(cx, backend.clone(), false);
    frame(cx, window, |window, _| {
        assert!(window.try_find("home-new-arrivals-error").is_some());
        assert!(window.try_find("home-genre-fiction").is_some());
        assert_eq!(window.find("home-featured-0").label(), Some("Coming Soon"));
    });

    // Retry loads the shelf again.
    backend.on_sorted("fiction", SearchSort::Newest, Ok(page(records("New", 2), 2, None)));
    frame(cx, window, |window, cx| window.click("home-new-arrivals-error-retry", cx));
    frame(cx, window, |window, _| assert_eq!(window.find("home-new-arrival-1").label(), Some("New 2")));
}

#[gpui_kit::test]
fn saved_covers_show_first_and_an_unrecognised_page_keeps_them(cx: &mut TestAppContext) {
    let backend = ScriptedBackend::new();
    backend.on_saved_shelf("new_arrivals", page(records("Saved", 3), 3, None));
    backend.on_saved_featured(vec![link("Saved tile", FeaturedKind::Collection, "https://x/c")]);
    backend.on_sorted("fiction", SearchSort::Newest, Ok(search_support::unrecognised()));
    backend.on_featured(Err(SpydusError::Network { message: "offline".into() }));
    let (_, window) = app_on(cx, backend.clone(), false);
    frame(cx, window, |window, _| {
        assert_eq!(window.find("home-new-arrival-0").label(), Some("Saved 1"));
        assert!(window.try_find("home-new-arrivals-error").is_none());
        assert_eq!(window.find("home-featured-0").label(), Some("Saved tile"));
    });
    assert!(!backend.calls().contains(&"save_shelf:new_arrivals".to_owned()));
}

#[gpui_kit::test]
fn signed_in_home_shows_the_demo_loans(cx: &mut TestAppContext) {
    let public = ScriptedBackend::new();
    let fixtures = DemoFixtures::bundled(ORIGIN, today()).unwrap();
    let loan_count = fixtures.loans.len();
    let demo: Arc<dyn LibraryBackend> = Arc::new(DemoBackend::new(public, fixtures, DemoDelays::NONE, false));
    let (state, window) = app_on(cx, demo, true);
    frame(cx, window, |_, _| {});
    frame(cx, window, |window, _| {
        assert_eq!(window.find("home-greeting").label(), Some("Hello Test!"));
        assert!(window.try_find("home-my-card").is_some());
        assert!(window.try_find("home-sign-in-card").is_none());
        assert!(window.try_find("home-loan-0").is_some());
        assert!(window.try_find(gpui_kit::SharedString::from(format!("home-loan-{}", loan_count - 1))).is_some());
    });
    assert!(state.read_with(cx, |state, _| state.is_signed_in()));

    // View All on "Your loans" opens Your Books.
    frame(cx, window, |window, cx| {
        window.scroll(
            "home-screen",
            gpui_kit::ScrollDelta::Pixels(gpui_kit::point(gpui_kit::px(0.), gpui_kit::px(-5_000.))),
            cx,
        )
    });
    frame(cx, window, |window, cx| window.click("home-loans-view-all", cx));
    assert_eq!(state.read_with(cx, |state, _| state.section()), Section::YourBooks);
}
