//! Headless UI tests of the book page (docs/desktop-app.md §9): the saved page first, then the fresh
//! one; the format switch; an empty carousel hidden; the My Picks bookmark signed out and in.

mod detail_support;

use detail_support::*;
use gpui_kit::TestAppContext;
use librespydus_core::{Availability, RecordFormat, RecordRelated, RelatedList, SavedBook, SearchRecord};
use librespydus_desktop::backend::fake::{FAKE_PASSWORD, FakeBackend};
use librespydus_desktop::state::BookRef;

fn related_links(similar: &str) -> RecordRelated {
    RecordRelated { similar: Some(similar.into()), ..Default::default() }
}

#[gpui_kit::test]
fn the_saved_page_shows_first_then_the_fresh_one_replaces_it(cx: &mut TestAppContext) {
    let backend = ScriptedBackend::new();
    let url = format!("{ORIGIN}/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&BRN=467086");
    let mut book = BookRef::from_related_item(&related_item("467086", "Opened title"));
    book.url = Some(url.clone());
    let saved_list = RelatedList { items: vec![related_item("1", "Saved neighbour")] };
    backend.saved(
        "brn:467086",
        SavedBook {
            record: record("19244682", "467086", "Saved title", related_links("https://x/similar-old")),
            similar: Some(saved_list),
            author: None,
            series: None,
            versions: None,
        },
    );
    backend.record(&url, record("19244682", "467086", "Fresh title", related_links("https://x/similar-new")));
    backend.related("https://x/similar-new", RelatedList { items: vec![related_item("2", "Fresh neighbour")] });
    backend.hold_records(true);
    let _state = signed_out_app(cx, backend.clone());
    let (window, screen) = open_detail(cx, book);

    // Before the library answers: the saved record and its carousel, no loading state.
    assert_eq!(label(cx, window, "detail-title").as_deref(), Some("Opened title"));
    assert_eq!(label(cx, window, "detail-summary").as_deref(), Some("All about Saved title."));
    assert!(!exists(cx, window, "detail-loading"));
    assert_eq!(label(cx, window, "detail-similar-0").as_deref(), Some("Saved neighbour"));
    assert!(backend.saves().is_empty());

    // The fresh record replaces it, its carousel loads, and the page is saved again.
    assert_eq!(backend.release_records(), 1);
    cx.run_until_parked();
    assert_eq!(label(cx, window, "detail-summary").as_deref(), Some("All about Fresh title."));
    assert_eq!(label(cx, window, "detail-similar-0").as_deref(), Some("Fresh neighbour"));
    let saves = backend.saves();
    assert!(saves.len() >= 2, "saved after the record and after the carousel");
    let (key, last) = saves.last().unwrap();
    assert_eq!(key, "brn:467086");
    assert_eq!(last.record.title.as_deref(), Some("Fresh title"));
    assert_eq!(last.similar.as_ref().map(|list| list.items.len()), Some(1));
    // The ref keeps its own title, as on the phones; the bookmark link comes from the record.
    let shown = screen.read_with(cx, |screen, _| screen.shown_book().clone());
    assert_eq!(shown.title, "Opened title");
    assert_eq!(
        shown.url.as_deref(),
        Some(format!("{ORIGIN}/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&BRN=467086").as_str())
    );
}

#[gpui_kit::test]
fn a_failed_refresh_keeps_the_saved_page_without_an_error(cx: &mut TestAppContext) {
    let backend = ScriptedBackend::new();
    let mut book = BookRef::new("bib:5", "");
    book.url = Some(format!("{ORIGIN}/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&IRN=5"));
    backend.saved(
        "bib:5",
        SavedBook {
            record: record("5", "55", "Saved title", RecordRelated::default()),
            similar: None,
            author: None,
            series: None,
            versions: None,
        },
    );
    let _state = signed_out_app(cx, backend.clone());
    let (window, _) = open_detail(cx, book);
    assert_eq!(label(cx, window, "detail-title").as_deref(), Some("Saved title"));
    assert!(!exists(cx, window, "detail-error"));
    assert!(backend.saves().is_empty());
}

#[gpui_kit::test]
fn a_failed_record_shows_the_error_with_retry(cx: &mut TestAppContext) {
    let backend = ScriptedBackend::new();
    let url = format!("{ORIGIN}/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&IRN=9");
    let book = BookRef::from_url(&url, "");
    let _state = signed_out_app(cx, backend.clone());
    let (window, _) = open_detail(cx, book);
    assert!(exists(cx, window, "detail-error"));
    backend.record(&url, record("9", "99", "Found on retry", RecordRelated::default()));
    click(cx, window, "detail-error-retry");
    assert_eq!(label(cx, window, "detail-title").as_deref(), Some("Found on retry"));
    assert!(!exists(cx, window, "detail-error"));
}

fn two_format_record() -> (SearchRecord, String, String) {
    let book_url = format!("{ORIGIN}/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/1?FMT=RECFMT&QRY=BK");
    let ebook_url = format!("{ORIGIN}/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/1?FMT=RECFMT&QRY=EBK");
    let search = SearchRecord {
        work_irn: Some("15974354".into()),
        title: Some("The midnight library".into()),
        author: Some("Haig, Matt".into()),
        detail_url: Some(format!("{ORIGIN}/cgi-bin/spydus.exe/FULL/OPAC/ALLWRKENQ/1/15974354,1")),
        formats: vec![
            RecordFormat {
                code: "BK".into(),
                label: Some("Book".into()),
                reserve_url: Some(format!(
                    "{ORIGIN}/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/1/0/V/0/R/9/ALLWRKENQ?SVL=15974354&RECFMT=BK"
                )),
                availability_url: Some(book_url.clone()),
            },
            RecordFormat {
                code: "EBK".into(),
                label: Some("eBook".into()),
                reserve_url: None,
                availability_url: Some(ebook_url.clone()),
            },
        ],
        ..Default::default()
    };
    (search, book_url, ebook_url)
}

#[gpui_kit::test]
fn switching_format_changes_the_availability_line(cx: &mut TestAppContext) {
    let backend = ScriptedBackend::new();
    let (search, book_url, ebook_url) = two_format_record();
    backend.record(&book_url, record("100", "1000", "The midnight library", RecordRelated::default()));
    backend.availability(&book_url, counts(2, 3, "100"));
    backend.availability(&ebook_url, Availability { bib_record_irn: Some("101".into()), ..Default::default() });
    let _state = signed_out_app(cx, backend.clone());
    let (window, _) = open_detail(cx, BookRef::from_search_record(&search));

    assert_eq!(label(cx, window, "detail-availability").as_deref(), Some("Availability: 2 of 3 available"));
    assert!(exists(cx, window, "detail-reserve"), "a book with a reserve link can be reserved");
    assert_eq!(label(cx, window, "detail-format-0").as_deref(), Some("Book"));

    click(cx, window, "detail-format-1");
    assert_eq!(label(cx, window, "detail-availability").as_deref(), Some("Availability: Available online"));
    assert!(!exists(cx, window, "detail-reserve"), "this eBook has no reserve link");
    assert!(backend.availability_calls().contains(&ebook_url));

    // Back to the book: its availability isn't asked for again.
    click(cx, window, "detail-format-0");
    assert_eq!(label(cx, window, "detail-availability").as_deref(), Some("Availability: 2 of 3 available"));
    assert_eq!(backend.availability_calls().iter().filter(|url| **url == book_url).count(), 1);
}

#[gpui_kit::test]
fn an_empty_carousel_is_hidden(cx: &mut TestAppContext) {
    let backend = ScriptedBackend::new();
    let url = format!("{ORIGIN}/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&IRN=7");
    let links = RecordRelated {
        similar: Some("https://x/similar".into()),
        author: Some("https://x/author".into()),
        series: Some("https://x/series".into()),
        versions: None,
    };
    backend.record(&url, record("7", "77", "With carousels", links));
    backend.related(
        "https://x/similar",
        RelatedList { items: vec![related_item("8", "Similar one"), related_item("77", "Itself")] },
    );
    backend.related("https://x/author", RelatedList::default());
    // The series list fails: hidden too.
    let _state = signed_out_app(cx, backend.clone());
    let (window, _) = open_detail(cx, BookRef::from_url(&url, ""));

    assert!(exists(cx, window, "detail-related-similar"));
    assert_eq!(label(cx, window, "detail-similar-0").as_deref(), Some("Similar one"));
    assert!(!exists(cx, window, "detail-similar-1"), "the book itself is left out");
    assert!(!exists(cx, window, "detail-related-author"));
    assert!(!exists(cx, window, "detail-related-series"));
    assert!(!exists(cx, window, "detail-related-versions"));
}

#[gpui_kit::test]
fn the_bookmark_saves_on_this_computer_when_signed_out(cx: &mut TestAppContext) {
    let backend = ScriptedBackend::new();
    let (search, book_url, _) = two_format_record();
    backend.record(&book_url, record("100", "1000", "The midnight library", RecordRelated::default()));
    let state = signed_out_app(cx, backend);
    let book = BookRef::from_search_record(&search);
    let (window, _) = open_detail(cx, book.clone());

    assert_eq!(label(cx, window, "detail-bookmark").as_deref(), Some("Save to My Picks"));
    click(cx, window, "detail-bookmark");
    assert_eq!(label(cx, window, "detail-bookmark").as_deref(), Some("Remove from My Picks"));
    assert!(state.read_with(cx, |state, _| state.local_picks().contains(&book.id)));
    assert_eq!(
        state.read_with(cx, |state, _| state.notice().map(|notice| notice.message.clone())).as_deref(),
        Some("Saved to My Picks")
    );
    // What's kept has the formats, for My Picks to show.
    assert_eq!(state.read_with(cx, |state, _| state.local_picks().items()[0].formats.len()), 2);

    click(cx, window, "detail-bookmark");
    assert_eq!(label(cx, window, "detail-bookmark").as_deref(), Some("Save to My Picks"));
    assert!(!state.read_with(cx, |state, _| state.local_picks().contains(&book.id)));
    assert_eq!(
        state.read_with(cx, |state, _| state.notice().map(|notice| notice.message.clone())).as_deref(),
        Some("Removed from My Picks")
    );
}

#[gpui_kit::test]
fn the_bookmark_saves_to_the_account_when_signed_in(cx: &mut TestAppContext) {
    let fake = FakeBackend::new(ORIGIN);
    let state = signed_out_app(cx, fake.clone());
    let sign_in = state.update(cx, |state, cx| state.sign_in("reader".into(), FAKE_PASSWORD.into(), cx));
    cx.run_until_parked();
    cx.foreground_executor().block_test(sign_in).unwrap();
    let (search, _, _) = two_format_record();
    let (window, _) = open_detail(cx, BookRef::from_search_record(&search));

    assert_eq!(label(cx, window, "detail-bookmark").as_deref(), Some("Save to My Picks"));
    click(cx, window, "detail-bookmark");
    assert_eq!(fake.pick_calls(), vec![("15974354".to_owned(), true)]);
    assert_eq!(label(cx, window, "detail-bookmark").as_deref(), Some("Remove from My Picks"));
    assert!(state.read_with(cx, |state, _| state.picks().contains("15974354")));
    assert!(state.read_with(cx, |state, _| state.local_picks().items().is_empty()), "not on this computer");

    click(cx, window, "detail-bookmark");
    assert_eq!(fake.pick_calls().last(), Some(&("15974354".to_owned(), false)));
    assert_eq!(label(cx, window, "detail-bookmark").as_deref(), Some("Save to My Picks"));
}

#[gpui_kit::test]
fn a_narrow_window_stacks_the_sheet_under_the_panel(cx: &mut TestAppContext) {
    let backend = ScriptedBackend::new();
    let url = format!("{ORIGIN}/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&IRN=7");
    backend.record(&url, record("7", "77", "Narrow", RecordRelated::default()));
    let _state = signed_out_app(cx, backend);
    let (window, _) = open_detail_sized(cx, BookRef::from_url(&url, ""), 960., 640.);
    let (panel, sheet) = with_frame(cx, window, |window, _| {
        use gpui_kit::test::TestWindowExt;
        (window.find("detail-title").bounds(), window.find("detail-synopsis-title").bounds())
    });
    assert!(sheet.origin.y > panel.origin.y + panel.size.height, "the sheet is below the panel");
}
