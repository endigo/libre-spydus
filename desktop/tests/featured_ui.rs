//! Headless UI tests of Featured collections (D3; docs/native-apps.md §5.10): Home's tiles lead to a
//! collection or a results list by URL; a collection's shelves load in view, three at a time,
//! empty ones hidden, failed ones with Retry; a collection without sections offers its website.

mod search_support;
mod support;

use std::sync::Arc;

use gpui_kit::base::Root;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{AppContext, Entity, TestAppContext, WindowHandle};
use librespydus_core::{FeaturedCollection, FeaturedKind, FeaturedLink, FeaturedSection, SpydusError};
use librespydus_desktop::backend::LibraryBackend;
use librespydus_desktop::features::featured::{FeaturedDestination, featured_icon, web_url};
use librespydus_desktop::state::library::canning;
use librespydus_desktop::state::{AppState, Dependencies, Route, Section};
use librespydus_desktop::ui::Lucide;
use search_support::{ScriptedBackend, page, records};

fn app(cx: &mut TestAppContext, backend: &Arc<ScriptedBackend>) -> (Entity<AppState>, WindowHandle<Root>) {
    let backend = backend.clone();
    let deps = Dependencies::in_memory(Arc::new(move |_| -> Arc<dyn LibraryBackend> { backend.clone() }));
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
    FeaturedLink { title: title.into(), description: None, icon: Some("balloons".into()), kind, url: url.into() }
}

fn section(title: &str, url: &str) -> FeaturedSection {
    FeaturedSection { title: title.into(), url: url.into() }
}

#[test]
fn icons_and_destinations() {
    assert_eq!(featured_icon(Some("fa-globe")), Lucide::Globe);
    assert_eq!(featured_icon(Some("game-board")), Lucide::Puzzle);
    assert_eq!(featured_icon(Some("something-new")), Lucide::Sparkles);
    assert_eq!(featured_icon(None), Lucide::Sparkles);
    let soon = link("Coming Soon", FeaturedKind::Search, "https://x/soon");
    assert_eq!(
        FeaturedDestination::new(&soon),
        Some(FeaturedDestination::Route(Route::ResultsPage {
            url: "https://x/soon".into(),
            title: "Coming Soon".into()
        }))
    );
    let web = link("eLibrary", FeaturedKind::Web, "https://example.org/e");
    assert_eq!(FeaturedDestination::new(&web), Some(FeaturedDestination::Web("https://example.org/e".into())));
    assert_eq!(web_url("javascript:alert(1)"), None);
}

#[gpui_kit::test]
fn a_saved_search_tile_opens_its_results_list(cx: &mut TestAppContext) {
    let backend = ScriptedBackend::new();
    backend.on_featured(Ok(vec![link("Coming Soon", FeaturedKind::Search, "https://x/soon")]));
    backend.on_page("https://x/soon", Ok(page(records("Soon", 2), 2, None)));
    let (state, window) = app(cx, &backend);
    frame(cx, window, |window, cx| window.click("home-featured-0", cx));
    assert_eq!(
        state.read_with(cx, |state, _| state.stack(Section::Home).to_vec()),
        [Route::ResultsPage { url: "https://x/soon".into(), title: "Coming Soon".into() }]
    );
    frame(cx, window, |window, _| {
        assert_eq!(window.find("results-title").label(), Some("Coming Soon"));
        assert_eq!(window.find("result-1").label(), Some("Soon 2"));
    });
}

#[gpui_kit::test]
fn a_collection_shows_its_shelves(cx: &mut TestAppContext) {
    let backend = ScriptedBackend::new();
    let new = link("What's New", FeaturedKind::Collection, "https://x/new");
    backend.on_featured(Ok(vec![new.clone()]));
    backend.on_collection(
        "https://x/new",
        FeaturedCollection {
            title: Some("What's New".into()),
            sections: vec![
                section("Adult fiction", "https://x/s0"),
                section("Nothing yet", "https://x/s1"),
                section("Broken", "https://x/s2"),
                section("DVDs", "https://x/s3"),
                section("Games", "https://x/s4"),
            ],
        },
    );
    backend.on_page("https://x/s0", Ok(page(records("Fiction", 25), 25, None)));
    backend.on_page("https://x/s1", Ok(search_support::no_results()));
    backend.on_page("https://x/s2", Err(SpydusError::Network { message: "offline".into() }));
    backend.on_page("https://x/s3", Ok(page(records("DVD", 1), 1, None)));
    backend.on_page("https://x/s4", Ok(page(records("Game", 1), 1, None)));
    let (state, window) = app(cx, &backend);
    frame(cx, window, |window, cx| window.click("home-featured-0", cx));
    assert_eq!(state.read_with(cx, |state, _| state.stack(Section::Home).to_vec()), [Route::Collection(new)]);
    for _ in 0..4 {
        frame(cx, window, |_, _| {});
    }
    frame(cx, window, |window, _| {
        assert_eq!(window.find("collection-title").label(), Some("What's New"));
        assert_eq!(window.find("collection-shelf-0-item-0").label(), Some("Fiction 1"));
        assert!(window.try_find("collection-shelf-0-item-19").is_some());
        assert!(window.try_find("collection-shelf-0-item-20").is_none(), "twenty covers at most");
        assert!(window.try_find("collection-shelf-1").is_none(), "an empty section is hidden");
        assert!(window.try_find("collection-shelf-2-error").is_some());
        assert_eq!(window.find("collection-shelf-3-item-0").label(), Some("DVD 1"));
    });
    assert_eq!(backend.count("collection:"), 1);
    // In view: every shelf asked once (the failed one waits for Retry).
    assert_eq!(backend.count("page:https://x/s2"), 1);

    // Retry asks the failed shelf again.
    backend.on_page("https://x/s2", Ok(page(records("Fixed", 1), 1, None)));
    frame(cx, window, |window, cx| window.click("collection-shelf-2-error-retry", cx));
    frame(cx, window, |window, _| assert_eq!(window.find("collection-shelf-2-item-0").label(), Some("Fixed 1")));

    // View All: the section's results list.
    frame(cx, window, |window, cx| window.click("collection-shelf-3-view-all", cx));
    let stack = state.read_with(cx, |state, _| state.stack(Section::Home).to_vec());
    assert_eq!(stack.last(), Some(&Route::ResultsPage { url: "https://x/s3".into(), title: "DVDs".into() }));
}

#[gpui_kit::test]
fn at_most_three_shelves_load_at_once(cx: &mut TestAppContext) {
    let backend = ScriptedBackend::new();
    let sections: Vec<_> = (0..6).map(|index| section(&format!("S{index}"), &format!("https://x/s{index}"))).collect();
    backend.on_collection("https://x/c", FeaturedCollection { title: None, sections });
    for index in 0..6 {
        backend.on_page(&format!("https://x/s{index}"), Ok(page(records(&format!("B{index}"), 1), 1, None)));
    }
    backend.hold_pages(true);
    let state = {
        let backend = backend.clone();
        let deps = Dependencies::in_memory(Arc::new(move |_| -> Arc<dyn LibraryBackend> { backend.clone() }));
        support::app_state(cx, deps)
    };
    state.update(cx, |state, cx| state.select_library(canning(), cx));
    // A tall window: every shelf is in view at once.
    let window = cx.update(|cx| {
        let options = gpui_kit::WindowOptions {
            window_bounds: Some(gpui_kit::WindowBounds::Windowed(gpui_kit::Bounds {
                origin: gpui_kit::point(gpui_kit::px(0.), gpui_kit::px(0.)),
                size: gpui_kit::size(gpui_kit::px(1280.), gpui_kit::px(3000.)),
            })),
            ..Default::default()
        };
        let (window, _) = gpui_kit::open_window(options, cx, |window, cx| {
            cx.new(|cx| librespydus_desktop::shell::RootView::new(window, cx))
        })
        .unwrap();
        window.downcast::<Root>().unwrap()
    });
    state.update(cx, |state, cx| {
        state.push(Route::Collection(link("Many", FeaturedKind::Collection, "https://x/c")), cx)
    });
    for _ in 0..4 {
        frame(cx, window, |_, _| {});
    }
    assert_eq!(backend.held_pages(), 3, "three shelves load, three wait");
    backend.release_pages();
    for _ in 0..4 {
        frame(cx, window, |_, _| {});
    }
    assert_eq!(backend.held_pages(), 3, "then the other three");
    backend.release_pages();
    for _ in 0..3 {
        frame(cx, window, |_, _| {});
    }
    frame(cx, window, |window, _| {
        assert_eq!(window.find("collection-shelf-0-item-0").label(), Some("B0 1"));
        assert_eq!(window.find("collection-shelf-5-item-0").label(), Some("B5 1"));
    });
    assert_eq!(backend.count("page:"), 6);
}

#[gpui_kit::test]
fn a_collection_without_sections_offers_its_website(cx: &mut TestAppContext) {
    let backend = ScriptedBackend::new();
    backend.on_collection("https://x/e", FeaturedCollection { title: Some("eLibrary".into()), sections: Vec::new() });
    let (state, window) = app(cx, &backend);
    state.update(cx, |state, cx| {
        state.push(Route::Collection(link("eLibrary", FeaturedKind::Collection, "https://x/e")), cx)
    });
    frame(cx, window, |_, _| {});
    frame(cx, window, |window, _| {
        assert!(window.try_find("collection-website").is_some());
        assert!(window.try_find("collection-open-website").is_some());
    });
}
