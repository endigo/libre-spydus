//! Headless UI tests of when the book page lists its copies (docs/native-apps.md §5.10; iOS
//! `BookDetailModel.copiesSource`): an edition-level item (a Featured list's card, one format with
//! no code) lists them; a book with format chips doesn't, even for a physical format whose
//! availability has a holdings link.

mod detail_support;

use detail_support::*;
use gpui_kit::TestAppContext;
use librespydus_core::{Holding, RecordFormat, RecordRelated, SearchRecord};
use librespydus_desktop::state::BookRef;

const HOLDINGS_URL: &str = "https://canning.spydus.com/cgi-bin/spydus.exe/XHLD/OPAC/BIBENQ/1?IRN=100";

fn holdings() -> Vec<Holding> {
    vec![Holding {
        location: Some("RIVERTON LIBRARY".into()),
        status: Some("Available".into()),
        call_number: Some("823.92 HAI".into()),
        available: true,
        ..Default::default()
    }]
}

/// A card with one format: `code`/`label` as given, availability at `url`.
fn card(code: &str, label: Option<&str>, url: &str) -> SearchRecord {
    SearchRecord {
        work_irn: Some("15974354".into()),
        title: Some("Board game: Ticket to Ride".into()),
        detail_url: Some(format!("{ORIGIN}/cgi-bin/spydus.exe/FULL/OPAC/ALLWRKENQ/1/15974354,1")),
        formats: vec![RecordFormat {
            code: code.into(),
            label: label.map(str::to_owned),
            reserve_url: Some(format!("{ORIGIN}/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/1/0/V/0/R/9/ALLWRKENQ?SVL=1")),
            availability_url: Some(url.to_owned()),
        }],
        ..Default::default()
    }
}

fn open(
    cx: &mut TestAppContext,
    code: &str,
    label: Option<&str>,
) -> (gpui_kit::WindowHandle<gpui_kit::base::Root>, bool) {
    let backend = ScriptedBackend::new();
    let url = format!("{ORIGIN}/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/1?FMT=RECFMT&QRY={code}");
    backend.record(&url, record("100", "1000", "Board game: Ticket to Ride", RecordRelated::default()));
    let mut availability = counts(1, 1, "100");
    availability.holdings_url = Some(HOLDINGS_URL.into());
    backend.availability(&url, availability);
    backend.holdings(HOLDINGS_URL, holdings());
    let _state = signed_out_app(cx, backend.clone());
    let (window, _) = open_detail(cx, BookRef::from_search_record(&card(code, label, &url)));
    let listed = exists(cx, window, "detail-copies");
    (window, listed)
}

#[gpui_kit::test]
fn an_edition_level_item_lists_its_copies(cx: &mut TestAppContext) {
    let (window, listed) = open(cx, "", None);
    assert!(listed, "the Copies list");
    let row = label(cx, window, "detail-copy-0").expect("one row per copy");
    assert!(row.contains("Riverton Library"), "{row}");
}

#[gpui_kit::test]
fn a_book_with_a_format_lists_no_copies(cx: &mut TestAppContext) {
    let (window, listed) = open(cx, "BK", Some("Book"));
    assert!(!listed, "no Copies list for a physical format");
    assert!(exists(cx, window, "detail-summary"), "the rest of the page is there");
}
