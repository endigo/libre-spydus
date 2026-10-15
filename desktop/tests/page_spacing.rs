//! Headless UI tests of the page frame (docs/desktop-app.md §6): every section's page starts at
//! the same place under the toolbar, a pushed page's title starts there too, and Back lines up
//! with it, in a window wide enough to centre the content column.

mod support;

use std::sync::Arc;

use gpui_kit::base::Root;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{AppContext, Bounds, Pixels, TestAppContext, WindowHandle};
use librespydus_desktop::backend::LibraryBackend;
use librespydus_desktop::backend::demo::{DemoBackend, DemoDelays, DemoFixtures};
use librespydus_desktop::backend::fake::FakeBackend;
use librespydus_desktop::state::formatting::today;
use librespydus_desktop::state::library::canning;
use librespydus_desktop::state::{AppState, BookRef, Dependencies, Route, Section};
use librespydus_desktop::ui::PAGE_TOP;

const ORIGIN: &str = "https://canning.spydus.com";

/// Signed in to the demo account, in a 1600-wide window.
fn demo_app(cx: &mut TestAppContext) -> (gpui_kit::Entity<AppState>, WindowHandle<Root>) {
    let fixtures = DemoFixtures::bundled(ORIGIN, today()).unwrap();
    let demo = DemoBackend::new(FakeBackend::new(ORIGIN), fixtures, DemoDelays::NONE, false);
    let mut deps = Dependencies::in_memory(Arc::new(move |_| -> Arc<dyn LibraryBackend> { Arc::new(demo.clone()) }));
    deps.is_demo = true;
    let state = support::app_state(cx, deps);
    state.update(cx, |state, cx| state.select_library(canning(), cx));
    let window = support::open_root_sized(cx, 1600., 1000.);
    cx.run_until_parked();
    (state, window)
}

fn bounds(cx: &mut TestAppContext, window: WindowHandle<Root>, id: &'static str) -> Bounds<Pixels> {
    cx.run_until_parked();
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        window.find(id).bounds()
    })
    .unwrap()
}

#[gpui_kit::test]
fn every_section_starts_at_the_same_place(cx: &mut TestAppContext) {
    let (state, window) = demo_app(cx);
    let toolbar = bounds(cx, window, "toolbar");
    let mut starts = Vec::new();
    for (section, first) in [
        (Section::Home, "home-header"),
        (Section::YourBooks, "your-books-title"),
        (Section::Search, "search-title"),
        (Section::MyPicks, "my-picks-title"),
        (Section::Profile, "profile-title"),
    ] {
        state.update(cx, |state, cx| state.select(section, cx));
        starts.push((first, bounds(cx, window, first).origin));
    }
    let (_, home) = starts[0];
    assert_eq!(f32::from(home.y - (toolbar.origin.y + toolbar.size.height)), PAGE_TOP, "PAGE_TOP under the toolbar");
    for (id, origin) in &starts {
        assert_eq!(*origin, home, "{id} starts where Home does");
    }
}

#[gpui_kit::test]
fn a_pushed_page_starts_there_too_and_back_lines_up_with_it(cx: &mut TestAppContext) {
    let (state, window) = demo_app(cx);
    let section_title = bounds(cx, window, "home-header");
    let query = "fiction".to_owned();
    state.update(cx, |state, cx| {
        state.push(
            Route::Results { query: query.clone(), sort: librespydus_core::SearchSort::Relevance, title: query },
            cx,
        )
    });
    // The header's row (inside its `PAGE_TOP`) starts where a section's page does; its title
    // starts at the same edge.
    let header = bounds(cx, window, "results-header");
    let title = bounds(cx, window, "results-title");
    assert_eq!(f32::from(header.origin.y) + PAGE_TOP, f32::from(section_title.origin.y), "the header's row");
    assert_eq!(title.origin.x, section_title.origin.x, "and its title at the same edge");
    let back = bounds(cx, window, "toolbar-back");
    // The button's padding (8) is outside the column, so its chevron starts at the title's edge.
    assert_eq!(f32::from(back.origin.x) + 8., f32::from(title.origin.x), "Back lines up with the title");
    assert!(f32::from(back.size.width) < 120., "Back is as wide as its label ({:?})", back.size.width);

    // The book page's panel starts at the same edge.
    state.update(cx, |state, cx| state.push(Route::Detail(BookRef::from_url(&format!("{ORIGIN}/x"), "A book")), cx));
    let detail = bounds(cx, window, "detail-scroll");
    assert_eq!(f32::from(detail.origin.x), f32::from(title.origin.x), "the book page's left edge");
}
