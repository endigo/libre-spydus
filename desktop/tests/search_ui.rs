//! Headless UI tests of Search and results lists (D3; docs/desktop-app.md §9): typing a query and
//! Return, the sort switch, infinite scroll, no results, errors and Retry, recent searches.

mod search_support;
mod support;

use std::sync::Arc;

use gpui_kit::base::Root;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{AppContext, Entity, ScrollDelta, TestAppContext, WindowHandle, point, px};
use librespydus_core::{SearchSort, SpydusError};
use librespydus_desktop::backend::LibraryBackend;
use librespydus_desktop::state::library::canning;
use librespydus_desktop::state::{AppState, Dependencies, Route, Section};
use search_support::{ScriptedBackend, page, records, unrecognised};

fn app(cx: &mut TestAppContext, backend: &Arc<ScriptedBackend>) -> (Entity<AppState>, WindowHandle<Root>) {
    let backend = backend.clone();
    let deps = Dependencies::in_memory(Arc::new(move |_| -> Arc<dyn LibraryBackend> { backend.clone() }));
    let state = support::app_state(cx, deps);
    state.update(cx, |state, cx| state.select_library(canning(), cx));
    let window = support::open_root(cx);
    (state, window)
}

/// The calls after Home's own (New arrivals, Featured).
fn searches(backend: &ScriptedBackend) -> Vec<String> {
    backend
        .calls()
        .into_iter()
        .filter(|call| (call.starts_with("search:") || call.starts_with("page:")) && call != "search:fiction:newest")
        .collect()
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

/// Search section, the field focused, `query` typed and Return.
fn search(cx: &mut TestAppContext, window: WindowHandle<Root>, query: &str) {
    frame(cx, window, |window, cx| window.click(Section::Search.id(), cx));
    frame(cx, window, |window, cx| {
        assert_eq!(window.find("search-field").focused(), Some(true), "Search focuses its field");
        window.click("search-field", cx);
        window.input(query, cx);
        window.press("enter", cx);
    });
}

#[gpui_kit::test]
fn typing_a_query_and_return_shows_results(cx: &mut TestAppContext) {
    let backend = ScriptedBackend::new();
    backend.on_search("harry potter", Ok(page(records("Harry", 3), 3, None)));
    let (state, window) = app(cx, &backend);

    // Nothing searched yet: the intro.
    frame(cx, window, |window, cx| window.click(Section::Search.id(), cx));
    frame(cx, window, |window, cx| {
        assert!(window.try_find("search-intro").is_some());
        assert_eq!(window.find("search-field").focused(), Some(true), "Search focuses its field");
        window.input("harry  potter ", cx);
        window.press("enter", cx);
    });
    frame(cx, window, |window, _| {
        assert_eq!(window.find("search-field").value(), Some("harry potter"));
        assert_eq!(window.find("results-count").label(), Some("3 results"));
        assert_eq!(window.find("result-0").label(), Some("Harry 1"));
        assert!(window.try_find("result-2").is_some());
        assert!(window.try_find("results-end").is_some(), "one page only");
    });
    assert_eq!(searches(&backend), ["search:harry potter:relevance"]);
    assert_eq!(state.read_with(cx, |state, _| state.recent_searches().to_vec()), ["harry potter"]);

    // A row opens the book on Search's stack.
    frame(cx, window, |window, cx| window.click("result-1", cx));
    let stack = state.read_with(cx, |state, _| state.stack(Section::Search).to_vec());
    assert!(matches!(&stack[..], [Route::Detail(book)] if book.title == "Harry 2"));
}

#[gpui_kit::test]
fn the_sort_switch_searches_again_newest_first(cx: &mut TestAppContext) {
    let backend = ScriptedBackend::new();
    backend.on_sorted("cats", SearchSort::Relevance, Ok(page(records("Relevant", 2), 2, None)));
    backend.on_sorted("cats", SearchSort::Newest, Ok(page(records("Newest", 2), 2, None)));
    let (_, window) = app(cx, &backend);
    search(cx, window, "cats");
    frame(cx, window, |window, cx| {
        assert_eq!(window.find("result-0").label(), Some("Relevant 1"));
        assert_eq!(window.find("search-sort-relevance").selected(), Some(true));
        window.click("search-sort-newest", cx);
    });
    frame(cx, window, |window, _| {
        assert_eq!(window.find("result-0").label(), Some("Newest 1"));
        assert_eq!(window.find("search-sort-newest").selected(), Some(true));
    });
    assert_eq!(searches(&backend), ["search:cats:relevance", "search:cats:newest"]);
}

#[gpui_kit::test]
fn scrolling_to_the_end_appends_the_next_page(cx: &mut TestAppContext) {
    let backend = ScriptedBackend::new();
    backend.on_search("long", Ok(page(records("First", 20), 40, Some("https://x/p2"))));
    backend.on_page("https://x/p2", Ok(page(records("Second", 20), 40, None)));
    let (_, window) = app(cx, &backend);
    search(cx, window, "long");
    frame(cx, window, |window, _| {
        assert_eq!(window.find("results-count").label(), Some("40 results"));
        assert!(window.try_find("result-19").is_some());
        assert!(window.try_find("result-20").is_none());
    });
    // The footer is far below: page 2 waits.
    assert_eq!(backend.count("page:"), 0);

    frame(cx, window, |window, cx| window.scroll("results-list", ScrollDelta::Pixels(point(px(0.), px(-20_000.))), cx));
    frame(cx, window, |_, _| {});
    frame(cx, window, |window, _| {
        assert_eq!(window.find("result-20").label(), Some("Second 1"));
        assert!(window.try_find("result-39").is_some());
        assert!(window.try_find("results-end").is_some());
    });
    assert_eq!(searches(&backend), ["search:long:relevance", "page:https://x/p2"]);
}

#[gpui_kit::test]
fn a_failed_next_page_offers_retry(cx: &mut TestAppContext) {
    let backend = ScriptedBackend::new();
    backend.on_search("short", Ok(page(records("First", 2), 4, Some("https://x/p2"))));
    backend.on_page("https://x/p2", Err(SpydusError::Network { message: "offline".into() }));
    let (_, window) = app(cx, &backend);
    search(cx, window, "short");
    // A short first page: the footer is in view and asks for page 2 at once, which fails.
    frame(cx, window, |_, _| {});
    frame(cx, window, |window, _| assert!(window.try_find("results-load-more-retry").is_some()));
    assert_eq!(backend.count("page:"), 1, "no automatic retry after a failure");

    backend.on_page("https://x/p2", Ok(page(records("Second", 2), 4, None)));
    frame(cx, window, |window, cx| window.click("results-load-more-retry", cx));
    frame(cx, window, |window, _| {
        assert_eq!(window.find("result-3").label(), Some("Second 2"));
        assert!(window.try_find("results-load-more-retry").is_none());
    });
}

#[gpui_kit::test]
fn no_results_and_errors(cx: &mut TestAppContext) {
    let backend = ScriptedBackend::new();
    backend.on_search("offline", Err(SpydusError::Network { message: "offline".into() }));
    let (state, window) = app(cx, &backend);

    // Nothing scripted: the OPAC's "no records" page.
    search(cx, window, "zzqx");
    frame(cx, window, |window, _| assert!(window.try_find("results-no-results").is_some()));

    // Searching again from the field: a network failure, with Retry.
    frame(cx, window, |window, cx| {
        window.click("search-field", cx);
        window.press("cmd-a", cx);
        window.input("offline", cx);
        window.press("enter", cx);
    });
    frame(cx, window, |window, cx| {
        assert!(window.try_find("results-error-state").is_some());
        window.click("results-error-retry", cx);
    });
    assert_eq!(backend.count("search:offline"), 2);

    // An unrecognised page (a results list by URL): the error state, and Retry asks again.
    backend.on_page("https://x/odd", Ok(unrecognised()));
    state.update(cx, |state, cx| {
        state.push(Route::ResultsPage { url: "https://x/odd".into(), title: "Odd list".into() }, cx)
    });
    frame(cx, window, |window, cx| {
        assert_eq!(window.find("results-title").label(), Some("Odd list"));
        assert!(window.try_find("results-sort").is_none(), "a list by URL has no sort");
        assert!(window.try_find("results-error-state").is_some());
        window.click("results-error-retry", cx);
    });
    assert_eq!(backend.count("page:https://x/odd"), 2);
}

#[gpui_kit::test]
fn recent_searches_run_again_and_clear(cx: &mut TestAppContext) {
    let backend = ScriptedBackend::new();
    backend.on_search("dogs", Ok(page(records("Dog", 1), 1, None)));
    let (state, window) = app(cx, &backend);
    state.update(cx, |state, cx| {
        state.add_recent_search("dogs", cx);
        state.add_recent_search("cats", cx);
    });
    frame(cx, window, |window, cx| window.click(Section::Search.id(), cx));
    frame(cx, window, |window, cx| {
        assert_eq!(window.find("recent-search-0").label(), Some("cats"));
        window.click("recent-search-1", cx);
    });
    frame(cx, window, |window, _| {
        assert_eq!(window.find("search-field").value(), Some("dogs"));
        assert_eq!(window.find("result-0").label(), Some("Dog 1"));
    });

    // Clearing the field goes back to the recent searches; Clear empties them.
    frame(cx, window, |window, cx| {
        window.click("search-field", cx);
        window.press("cmd-a", cx);
        window.press("backspace", cx);
    });
    frame(cx, window, |window, cx| {
        assert!(window.try_find("recent-search-0").is_some());
        window.click("recent-searches-clear", cx);
    });
    frame(cx, window, |window, _| assert!(window.try_find("search-intro").is_some()));
    assert!(state.read_with(cx, |state, _| state.recent_searches().is_empty()));
}

#[gpui_kit::test]
fn home_hands_its_query_to_search(cx: &mut TestAppContext) {
    let backend = ScriptedBackend::new();
    backend.on_search("whales", Ok(page(records("Whale", 1), 1, None)));
    let (state, window) = app(cx, &backend);
    frame(cx, window, |_, _| {});
    state.update(cx, |state, cx| state.open_search(Some("whales".into()), cx));
    frame(cx, window, |_, _| {});
    frame(cx, window, |window, _| {
        assert_eq!(window.find("search-field").value(), Some("whales"));
        assert_eq!(window.find("result-0").label(), Some("Whale 1"));
    });
}
