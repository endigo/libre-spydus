//! Headless UI tests of scrolling on the book page: each scroll area keeps to its own axis (a
//! vertical wheel over a carousel scrolls the page, a sideways swipe scrolls only the carousel),
//! and in a wide window the panel stays put while the sheet scrolls.

mod detail_support;

use detail_support::*;
use gpui_kit::{Bounds, Pixels, ScrollDelta, TestAppContext, WindowHandle, point, px};
use librespydus_core::{RecordRelated, RelatedList};
use librespydus_desktop::state::BookRef;

/// A book with three full carousels, so both the page and the carousels can scroll.
fn open_long_book(cx: &mut TestAppContext) -> WindowHandle<gpui_kit::base::Root> {
    let backend = ScriptedBackend::new();
    let url = format!("{ORIGIN}/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&IRN=7");
    let links = RecordRelated {
        similar: Some("https://x/similar".into()),
        author: Some("https://x/author".into()),
        series: Some("https://x/series".into()),
        versions: None,
    };
    backend.record(&url, record("7", "77", "Long page", links));
    for (list, prefix) in [("https://x/similar", "s"), ("https://x/author", "a"), ("https://x/series", "r")] {
        let items = (0..12).map(|n| related_item(&format!("{prefix}{n}"), &format!("Book {prefix}{n}"))).collect();
        backend.related(list, RelatedList { items });
    }
    let _state = signed_out_app(cx, backend);
    let (window, _) = open_detail(cx, BookRef::from_url(&url, ""));
    window
}

fn bounds(cx: &mut TestAppContext, window: WindowHandle<gpui_kit::base::Root>, id: &str) -> Bounds<Pixels> {
    let id = id.to_owned();
    with_frame(cx, window, move |window, _| gpui_kit::test::TestWindowExt::find(window, id).bounds())
}

fn scroll(cx: &mut TestAppContext, window: WindowHandle<gpui_kit::base::Root>, id: &str, x: f32, y: f32) {
    let id = id.to_owned();
    with_frame(cx, window, move |window, cx| {
        gpui_kit::test::TestWindowExt::scroll(window, id, ScrollDelta::Pixels(point(px(x), px(y))), cx)
    });
}

#[gpui_kit::test]
fn a_vertical_wheel_over_a_carousel_scrolls_the_page_not_the_carousel(cx: &mut TestAppContext) {
    let window = open_long_book(cx);
    let before = bounds(cx, window, "detail-similar-0");
    scroll(cx, window, "detail-carousel-similar", 0., -200.);
    let after = bounds(cx, window, "detail-similar-0");
    assert_eq!(after.origin.x, before.origin.x, "the carousel didn't move sideways");
    assert!(after.origin.y < before.origin.y, "the page scrolled up");
}

#[gpui_kit::test]
fn a_sideways_swipe_over_a_carousel_scrolls_only_the_carousel(cx: &mut TestAppContext) {
    let window = open_long_book(cx);
    let before = bounds(cx, window, "detail-similar-0");
    scroll(cx, window, "detail-carousel-similar", -200., 0.);
    let after = bounds(cx, window, "detail-similar-0");
    assert!(after.origin.x < before.origin.x, "the carousel scrolled");
    assert_eq!(after.origin.y, before.origin.y, "the page stayed put");
}

#[gpui_kit::test]
fn in_a_wide_window_the_panel_stays_while_the_sheet_scrolls(cx: &mut TestAppContext) {
    let window = open_long_book(cx);
    let title = bounds(cx, window, "detail-title");
    let synopsis = bounds(cx, window, "detail-synopsis-title");
    scroll(cx, window, "detail-scroll", 0., -200.);
    assert_eq!(bounds(cx, window, "detail-title").origin, title.origin, "the panel didn't move");
    assert!(bounds(cx, window, "detail-synopsis-title").origin.y < synopsis.origin.y, "the sheet scrolled");
}
