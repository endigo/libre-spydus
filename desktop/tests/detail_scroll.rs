//! Headless UI tests of the book page's layout and scrolling: in a wide window the details (left)
//! and the other books (right, as grids) scroll separately; narrow, the other books follow the
//! details in one column.

mod detail_support;

use detail_support::*;
use gpui_kit::{Bounds, Pixels, ScrollDelta, TestAppContext, WindowHandle, point, px};
use librespydus_core::{DetailRow, DetailValue, RecordRelated, RelatedList};
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
    let mut long = record("7", "77", "Long page", links);
    // Eight detail rows of several lines each, so the details column can scroll too.
    long.details = (0..8)
        .map(|n| DetailRow {
            label: format!("Fact {n}"),
            values: (0..3).map(|line| DetailValue { text: format!("Line {line} of fact {n}"), url: None }).collect(),
        })
        .collect();
    backend.record(&url, long);
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
fn the_other_books_scroll_on_their_own_beside_the_details(cx: &mut TestAppContext) {
    let window = open_long_book(cx);
    let title = bounds(cx, window, "detail-title");
    let similar = bounds(cx, window, "detail-similar-0");
    assert!(similar.origin.x > title.origin.x + title.size.width, "the other books are on the right");
    scroll(cx, window, "detail-related-scroll", 0., -200.);
    assert!(bounds(cx, window, "detail-similar-0").origin.y < similar.origin.y, "the other books scrolled");
    assert_eq!(bounds(cx, window, "detail-title").origin, title.origin, "the details stayed put");
}

#[gpui_kit::test]
fn the_details_scroll_on_their_own_beside_the_other_books(cx: &mut TestAppContext) {
    let window = open_long_book(cx);
    let title = bounds(cx, window, "detail-title");
    let similar = bounds(cx, window, "detail-similar-0");
    scroll(cx, window, "detail-scroll", 0., -200.);
    assert!(bounds(cx, window, "detail-title").origin.y < title.origin.y, "the details scrolled");
    assert_eq!(bounds(cx, window, "detail-similar-0").origin, similar.origin, "the other books stayed put");
}

#[gpui_kit::test]
fn a_sideways_swipe_moves_nothing(cx: &mut TestAppContext) {
    let window = open_long_book(cx);
    let similar = bounds(cx, window, "detail-similar-0");
    scroll(cx, window, "detail-related-similar", -200., 0.);
    assert_eq!(bounds(cx, window, "detail-similar-0").origin, similar.origin, "a grid doesn't scroll sideways");
}

#[gpui_kit::test]
fn a_narrow_window_puts_the_other_books_below_the_details(cx: &mut TestAppContext) {
    let backend = ScriptedBackend::new();
    let url = format!("{ORIGIN}/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&IRN=7");
    let links = RecordRelated { similar: Some("https://x/similar".into()), ..Default::default() };
    backend.record(&url, record("7", "77", "Narrow", links));
    let items = (0..6).map(|n| related_item(&format!("s{n}"), &format!("Book s{n}"))).collect();
    backend.related("https://x/similar", RelatedList { items });
    let _state = signed_out_app(cx, backend);
    let (window, _) = open_detail_sized(cx, BookRef::from_url(&url, ""), 960., 640.);
    let synopsis = bounds(cx, window, "detail-synopsis-title");
    scroll(cx, window, "detail-scroll", 0., -2_000.);
    let similar = bounds(cx, window, "detail-similar-0");
    let synopsis_after = bounds(cx, window, "detail-synopsis-title");
    assert!(synopsis_after.origin.y < synopsis.origin.y, "one column scrolls");
    assert!(similar.origin.y > synopsis_after.origin.y, "the other books come after the details");
}

#[gpui_kit::test]
fn a_wide_window_opens_in_two_columns_while_the_book_loads(cx: &mut TestAppContext) {
    let backend = ScriptedBackend::new();
    let url = format!("{ORIGIN}/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&IRN=7");
    let links = RecordRelated { similar: Some("https://x/similar".into()), ..Default::default() };
    backend.record(&url, record("7", "77", "Loading", links));
    let items = (0..6).map(|n| related_item(&format!("s{n}"), &format!("Book s{n}"))).collect();
    backend.related("https://x/similar", RelatedList { items });
    backend.hold_records(true);
    let _state = signed_out_app(cx, backend.clone());
    let (window, _) = open_detail(cx, BookRef::from_url(&url, "Loading"));

    let details = bounds(cx, window, "detail-scroll");
    let placeholder = bounds(cx, window, "detail-related-placeholder");
    assert!(placeholder.origin.x > details.origin.x + details.size.width, "placeholders on the right");

    // The book's other books take the placeholders' place, in the same column.
    assert_eq!(backend.release_records(), 1);
    let similar = bounds(cx, window, "detail-similar-0");
    assert!(!exists(cx, window, "detail-related-placeholder"));
    assert_eq!(bounds(cx, window, "detail-scroll").size.width, details.size.width, "the details kept their width");
    assert!(similar.origin.x >= placeholder.origin.x);
}
