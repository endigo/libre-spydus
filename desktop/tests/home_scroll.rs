//! Headless UI tests of Home's horizontal rows: a sideways swipe scrolls New arrivals and Featured
//! (their covers lay out on the scroller itself), and a vertical wheel over a row scrolls the page.

mod search_support;
mod support;

use std::sync::Arc;

use gpui_kit::base::Root;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{AppContext, Bounds, Pixels, ScrollDelta, TestAppContext, WindowHandle, point, px};
use librespydus_core::{FeaturedKind, FeaturedLink, SearchSort};
use librespydus_desktop::backend::LibraryBackend;
use librespydus_desktop::state::Dependencies;
use librespydus_desktop::state::library::canning;
use search_support::{ScriptedBackend, page, records};

fn home(cx: &mut TestAppContext) -> WindowHandle<Root> {
    let backend = ScriptedBackend::new();
    backend.on_sorted("fiction", SearchSort::Newest, Ok(page(records("New", 12), 500, None)));
    let links = (0..10)
        .map(|n| FeaturedLink {
            title: format!("Collection {n}"),
            description: None,
            icon: Some("book".into()),
            kind: FeaturedKind::Collection,
            url: format!("https://x/c{n}"),
        })
        .collect();
    backend.on_featured(Ok(links));
    let backend: Arc<dyn LibraryBackend> = backend;
    let deps = Dependencies::in_memory(Arc::new(move |_| -> Arc<dyn LibraryBackend> { backend.clone() }));
    let state = support::app_state(cx, deps);
    state.update(cx, |state, cx| state.select_library(canning(), cx));
    support::open_root(cx)
}

fn bounds(cx: &mut TestAppContext, window: WindowHandle<Root>, id: &str) -> Bounds<Pixels> {
    cx.run_until_parked();
    let id = id.to_owned();
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        window.find(id).bounds()
    })
    .unwrap()
}

fn scroll(cx: &mut TestAppContext, window: WindowHandle<Root>, id: &str, x: f32, y: f32) {
    cx.run_until_parked();
    let id = id.to_owned();
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        window.scroll(id, ScrollDelta::Pixels(point(px(x), px(y))), cx);
    })
    .unwrap();
    cx.run_until_parked();
}

#[gpui_kit::test]
fn a_sideways_swipe_scrolls_new_arrivals(cx: &mut TestAppContext) {
    let window = home(cx);
    let before = bounds(cx, window, "home-new-arrival-0");
    scroll(cx, window, "home-new-arrival", -200., 0.);
    let after = bounds(cx, window, "home-new-arrival-0");
    assert!(after.origin.x < before.origin.x, "the shelf scrolled");
    assert_eq!(after.origin.y, before.origin.y, "the page stayed put");
}

#[gpui_kit::test]
fn a_sideways_swipe_scrolls_featured(cx: &mut TestAppContext) {
    let window = home(cx);
    let before = bounds(cx, window, "home-featured-0");
    scroll(cx, window, "home-featured-row", -200., 0.);
    assert!(bounds(cx, window, "home-featured-0").origin.x < before.origin.x, "the row scrolled");
}

#[gpui_kit::test]
fn a_vertical_wheel_over_a_shelf_scrolls_the_page(cx: &mut TestAppContext) {
    let window = home(cx);
    let before = bounds(cx, window, "home-new-arrival-0");
    scroll(cx, window, "home-new-arrival", 0., -200.);
    let after = bounds(cx, window, "home-new-arrival-0");
    assert_eq!(after.origin.x, before.origin.x, "the shelf didn't move sideways");
    assert!(after.origin.y < before.origin.y, "the page scrolled");
}
