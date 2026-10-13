//! `parse::search::parse_search_results` — golden parity with `parseSearchResults` (MANIFEST rows
//! `search.json`, `search-midnight.json`, `multi/*`, `synthetic/search-*`) plus focused cases for
//! the trickier TS quirks, the Wave 2 author / setId rules and the Phase 2 `noResults` flag.

mod common;

use librespydus_core::config::ORIGIN;
use librespydus_core::parse::search::parse_search_results;
use librespydus_core::types::{SearchRecord, Showing};

#[test]
fn golden_search() {
    let html = common::fixture("search.html");
    let actual = parse_search_results(&html, ORIGIN, Some("The baby-sitters club graphic novel"));
    common::assert_golden(&actual, "search.json");
}

#[test]
fn golden_search_midnight() {
    let html = common::fixture("search-midnight.html");
    let actual = parse_search_results(&html, ORIGIN, Some("the midnight library"));
    common::assert_golden(&actual, "search-midnight.json");
}

// ---- Wave 2: other libraries' work-level results, each parsed with its own origin ----

fn golden_multi(label: &str, origin: &str) {
    let html = common::fixture(&format!("multi/{label}-harry-potter.html"));
    let actual = parse_search_results(&html, origin, Some("harry potter"));
    common::assert_golden(&actual, &format!("multi/{label}-harry-potter.json"));
}

#[test]
fn golden_multi_camden() {
    golden_multi("camden", "https://camden.spydus.com");
}

#[test]
fn golden_multi_cumberlandlibraries() {
    golden_multi(
        "cumberlandlibraries",
        "https://cumberlandlibraries.spydus.com",
    );
}

#[test]
fn golden_multi_esperance() {
    golden_multi("esperance", "https://esperance.spydus.com");
}

#[test]
fn golden_multi_porirua() {
    golden_multi("porirua", "https://porirua.spydus.co.nz");
}

#[test]
fn golden_multi_hutt() {
    golden_multi("hutt", "https://hutt.spydus.co.nz");
}

#[test]
fn golden_multi_niagara() {
    golden_multi("niagara", "https://niagara.spydus.com");
}

// ---- Phase 2: the OPAC's "did not find any records" page, and another error page ----

fn golden_no_results(label: &str, origin: &str) {
    let html = common::fixture(&format!("multi/{label}-no-results.html"));
    let actual = parse_search_results(&html, origin, Some("zzqxjv"));
    assert!(actual.no_results && actual.records.is_empty(), "{label}");
    common::assert_golden(&actual, &format!("multi/{label}-no-results.json"));
}

#[test]
fn golden_no_results_canning() {
    golden_no_results("canning", "https://canning.spydus.com");
}

#[test]
fn golden_no_results_cumberlandlibraries() {
    golden_no_results(
        "cumberlandlibraries",
        "https://cumberlandlibraries.spydus.com",
    );
}

#[test]
fn golden_no_results_hutt() {
    golden_no_results("hutt", "https://hutt.spydus.co.nz");
}

#[test]
fn golden_no_results_niagara() {
    golden_no_results("niagara", "https://niagara.spydus.com");
}

#[test]
fn golden_no_results_mybcls() {
    golden_no_results("mybcls", "https://mybcls.spydus.com");
}

#[test]
fn golden_empty_search_is_not_no_results() {
    let html = common::fixture("multi/canning-empty-search.html");
    let actual = parse_search_results(&html, ORIGIN, Some("*"));
    assert!(!actual.no_results && actual.records.is_empty());
    common::assert_golden(&actual, "multi/canning-empty-search.json");
}

#[test]
fn golden_synthetic_author_lines() {
    let html = common::fixture("synthetic/search-author-lines.html");
    let actual = parse_search_results(&html, ORIGIN, None);
    common::assert_golden(&actual, "synthetic/search-author-lines.json");
}

#[test]
fn golden_synthetic_allenq() {
    let html = common::fixture("synthetic/search-allenq.html");
    let mut actual = parse_search_results(&html, ORIGIN, None);
    // A tab-less edition-level card gets one format for its bib (new; the TS parser has none).
    let format = actual.records[0].formats.pop().expect("an edition-level format");
    assert_eq!((format.code.as_str(), format.label.as_deref(), format.reserve_url.as_deref()), ("", None, None));
    assert_eq!(
        format.availability_url.as_deref(),
        Some("https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&IRN=1001&FMT=RECFMT")
    );
    common::assert_golden(&actual, "synthetic/search-allenq.json");
}

/// One results card whose `div.recdetails` holds `lines` as `span.d-block` children.
fn card_with_lines(lines: &[&str]) -> String {
    let spans: String = lines
        .iter()
        .map(|l| format!(r#"<span class="d-block">{l}</span>"#))
        .collect();
    format!(
        r#"<fieldset class="card card-list"><div class="card-text recdetails">{spans}</div></fieldset>"#
    )
}

fn only_record(html: &str) -> SearchRecord {
    let mut results = parse_search_results(html, ORIGIN, None);
    assert_eq!(results.records.len(), 1, "expected exactly one record");
    results.records.remove(0)
}

#[test]
fn line_classification_author_year_language() {
    let r = only_record(&card_with_lines(&[
        r#"<a href="/cgi-bin/spydus.exe/NAV/OPAC/AUTHENQ/1?QRY=IRN(2)"><span>Haig, Matt</span></a>, author"#,
        "2020",
        "English",
    ]));
    assert_eq!(r.author.as_deref(), Some("Haig, Matt"));
    assert_eq!(r.author_role.as_deref(), Some("author"));
    assert_eq!(
        r.author_url.as_deref(),
        Some("https://canning.spydus.com/cgi-bin/spydus.exe/NAV/OPAC/AUTHENQ/1?QRY=IRN(2)")
    );
    assert_eq!(r.year.as_deref(), Some("2020"));
    assert_eq!(r.language.as_deref(), Some("English"));
}

#[test]
fn line_classification_role_variants() {
    // Plural role, trailing period, any case → lower-cased singular role; role suffix stripped.
    let r = only_record(&card_with_lines(&["Smith, Jane,  Illustrators."]));
    assert_eq!(r.author.as_deref(), Some("Smith, Jane"));
    assert_eq!(r.author_role.as_deref(), Some("illustrator"));

    // Only the first role line becomes the author; a later one is neither year nor language.
    let r = only_record(&card_with_lines(&[
        "A, B, editor",
        "C, D, translator",
        "French",
    ]));
    assert_eq!(r.author.as_deref(), Some("A, B"));
    assert_eq!(r.author_role.as_deref(), Some("editor"));
    assert_eq!(r.language.as_deref(), Some("French"));

    // A bare role leaves an empty author (`|| null`) but still records the role.
    let r = only_record(&card_with_lines(&[", narrator"]));
    assert_eq!(r.author, None);
    assert_eq!(r.author_role.as_deref(), Some("narrator"));

    // JS `/i` folds ASCII only: a long s (U+017F) is not an `s`, so this is not a role line.
    let r = only_record(&card_with_lines(&["Doe, J, illu\u{17F}trator"]));
    assert_eq!(r.author, None);
    assert_eq!(r.author_role, None);
}

/// `(author, author_role)` of a one-card page with these detail lines.
fn author_of(lines: &[&str]) -> (Option<String>, Option<String>) {
    let r = only_record(&card_with_lines(lines));
    (r.author, r.author_role)
}

fn named(author: &str, role: Option<&str>) -> (Option<String>, Option<String>) {
    (Some(author.to_owned()), role.map(str::to_owned))
}

const LINKED_HAIG: &str =
    r#"<a href="/cgi-bin/spydus.exe/NAV/OPAC/AUTHENQ/1?QRY=IRN(2)"><span>Haig, Matt</span></a>"#;

#[test]
fn author_life_dates_before_role() {
    // A bare space after the life dates separates the role; the dates stay in the author.
    assert_eq!(
        author_of(&["Rowling, J. K., 1965- author."]),
        named("Rowling, J. K., 1965-", Some("author"))
    );
    assert_eq!(
        author_of(&["Dickens, Charles, 1812-1870 editor"]),
        named("Dickens, Charles, 1812-1870", Some("editor"))
    );
    // The comma form is unchanged (Canning: "Haig, Matt, 1975-, author").
    assert_eq!(
        author_of(&["Haig, Matt, 1975-, author"]),
        named("Haig, Matt, 1975-", Some("author"))
    );
    assert_eq!(
        author_of(&["Rowling, J. K. 1965-, author"]),
        named("Rowling, J. K. 1965-", Some("author"))
    );
    // A space without life dates before it is not a separator.
    assert_eq!(author_of(&["Smith, Jane author"]), (None, None));
}

#[test]
fn author_qualified_and_new_roles() {
    assert_eq!(
        author_of(&["Yates, David, 1963-, film director"]),
        named("Yates, David, 1963-", Some("film director"))
    );
    assert_eq!(
        author_of(&["Heyman, David, Executive Producer"]),
        named("Heyman, David", Some("executive producer"))
    );
    assert_eq!(
        author_of(&["Smith, Jane, co-author"]),
        named("Smith, Jane", Some("co-author"))
    );
    for role in [
        "director",
        "producer",
        "screenwriter",
        "performer",
        "actor",
        "photographer",
        "compiler",
        "cartographer",
        "lyricist",
        "presenter",
        "speaker",
        "interviewer",
    ] {
        assert_eq!(
            author_of(&[&format!("Doe, Jo, {role}s.")]),
            named("Doe, Jo", Some(role)),
            "{role}"
        );
    }
}

#[test]
fn author_role_chain_takes_first_role() {
    assert_eq!(
        author_of(&["Gaiman, Neil, author, narrator."]),
        named("Gaiman, Neil", Some("author"))
    );
    assert_eq!(
        author_of(&["Spielberg, Steven, 1946-, director, producer"]),
        named("Spielberg, Steven, 1946-", Some("director"))
    );
}

#[test]
fn never_an_author() {
    // Publisher / imprint / distributor / edition / format / year / language lines, first or not.
    for line in [
        "Bloomsbury Children's Books, publisher",
        "Warner Home Video, distributor",
        "Monash University, issuing body",
        "London : Bloomsbury, 2014",
        "London : Bloomsbury, 1997-",
        "2nd edition",
        "Large print",
        "Book",
        "2023",
        "2016 - 2018",
        "English, French",
    ] {
        assert_eq!(author_of(&[line]), (None, None), "{line}");
        assert_eq!(
            author_of(&["2020", line]),
            (None, None),
            "{line} (not first)"
        );
    }
}

#[test]
fn roleless_first_line_fallback() {
    // An author-search link in the first line: the author, no role; later lines still classify.
    let r = only_record(&card_with_lines(&[LINKED_HAIG, "2020", "English"]));
    assert_eq!(
        (r.author.as_deref(), r.author_role),
        (Some("Haig, Matt"), None)
    );
    assert_eq!(
        (r.year.as_deref(), r.language.as_deref()),
        (Some("2020"), Some("English"))
    );

    // A personal name with life dates, no link.
    assert_eq!(
        author_of(&["Haig, Matt, 1975-"]),
        named("Haig, Matt, 1975-", None)
    );
    assert_eq!(
        author_of(&["Rowling, J. K. 1965-"]),
        named("Rowling, J. K. 1965-", None)
    );
    assert_eq!(
        author_of(&["Dickens, Charles, 1812-1870."]),
        named("Dickens, Charles, 1812-1870.", None)
    );

    // Unlinked and dateless: indistinguishable from a language list, so not an author.
    let r = only_record(&card_with_lines(&["Haig, Matt", "2020"]));
    assert_eq!(
        (r.author, r.language.as_deref()),
        (None, Some("Haig, Matt"))
    );

    // Only the first non-empty line qualifies.
    let r = only_record(&card_with_lines(&["2020", LINKED_HAIG]));
    assert_eq!(
        (r.author, r.language.as_deref()),
        (None, Some("Haig, Matt"))
    );
    assert_eq!(author_of(&["2020", "Haig, Matt, 1975-"]), (None, None));
    assert_eq!(author_of(&["  ", LINKED_HAIG]), named("Haig, Matt", None));

    // Only an AUTHENQ link counts, and a linked year is still a year.
    let subject = r#"<a href="/cgi-bin/spydus.exe/ENQ/OPAC/SUBENQ/1?QRY=X">Wizards</a>"#;
    assert_eq!(author_of(&[subject]), (None, None));
    let r = only_record(&card_with_lines(&[
        r#"<a href="/cgi-bin/spydus.exe/NAV/OPAC/AUTHENQ/1">2020</a>"#,
    ]));
    assert_eq!((r.author, r.year.as_deref()), (None, Some("2020")));

    // A role line always wins over the fallback (first line or not).
    assert_eq!(
        author_of(&[LINKED_HAIG, "Smith, Jane, illustrator"]),
        named("Haig, Matt", None)
    );
    assert_eq!(
        author_of(&["Haig, Matt", "Smith, Jane, illustrator"]),
        named("Smith, Jane", Some("illustrator"))
    );
}

#[test]
fn line_classification_year_ranges_and_languages() {
    let r = only_record(&card_with_lines(&["2022 - 2023", "1999"]));
    assert_eq!(
        r.year.as_deref(),
        Some("2022 - 2023"),
        "first year line wins"
    );

    let r = only_record(&card_with_lines(&["2019\u{2013}2021"]));
    assert_eq!(r.year.as_deref(), Some("2019\u{2013}2021"), "en dash range");

    let r = only_record(&card_with_lines(&["20201"]));
    assert_eq!(r.year, None);

    let r = only_record(&card_with_lines(&["English, French", "German"]));
    assert_eq!(r.language.as_deref(), Some("English, French"));

    // Lines are collapsed (entities decoded, whitespace runs folded) before classification.
    let r = only_record(&card_with_lines(&[
        "  English&nbsp;\n",
        "&#50;&#48;&#50;&#53;",
    ]));
    assert_eq!(r.language.as_deref(), Some("English"));
    assert_eq!(r.year.as_deref(), Some("2025"));

    let r = only_record(&card_with_lines(&["english", "ENGLISH", "Large print"]));
    assert_eq!(r.language, None, "language needs Capitalised words only");
}

#[test]
fn detail_lines_must_be_direct_children() {
    // `div.recdetails > span.d-block` only: a nested span is not a line of its own.
    let html = r#"<fieldset class="card card-list"><div class="card-text recdetails"><div><span class="d-block">2020</span></div><span class="d-block">English</span></div></fieldset>"#;
    let r = only_record(html);
    assert_eq!(r.year, None);
    assert_eq!(r.language.as_deref(), Some("English"));
}

#[test]
fn format_codes_from_tab_ids_reserve_links_and_lazy_panes() {
    let html = r#"<fieldset class="card card-list">
      <div class="card-body-tabs">
        <button class="nav-link" id="bk-tab-1"><span class="fa"></span> Book </button>
        <button class="nav-link" id="ebk-tab-1">eBook</button>
        <button class="nav-link" id="weird">Ignored label</button>
        <div class="tab-pane" id="bk-pane-1">
          <a href="/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/1/0/V/0/R/9/ALLWRKENQ?SVL=1&amp;recfmt=bk">Reserve</a>
          <a href="/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/1?SVL=1">no RECFMT, ignored</a>
          <div class="tab-pane-url" id="BK-pane-1-url" data-tab-href="/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/1?RECFMT=BK"></div>
        </div>
        <div class="tab-pane" id="dvd-pane-1">
          <a href="/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/1?SVL=1&RECFMT=DVD">Reserve</a>
          <div class="tab-pane-url" id="DVD-pane-1-url" data-tab-href=""></div>
          <div class="tab-pane-url" id="no-code-here" data-tab-href="/x"></div>
        </div>
      </div>
    </fieldset>"#;
    let r = only_record(html);
    let summary: Vec<_> = r
        .formats
        .iter()
        .map(|f| {
            (
                f.code.as_str(),
                f.label.as_deref(),
                f.reserve_url.is_some(),
                f.availability_url.is_some(),
            )
        })
        .collect();
    assert_eq!(
        summary,
        [
            // Codes are upper-cased; labels collapsed; insertion order kept.
            ("BK", Some("Book"), true, true),
            ("EBK", Some("eBook"), false, false),
            // A format first seen through its reserve link has the draft label "" → null.
            ("DVD", None, true, false),
        ]
    );
    assert_eq!(
        r.formats[0].reserve_url.as_deref(),
        Some(
            "https://canning.spydus.com/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/1/0/V/0/R/9/ALLWRKENQ?SVL=1&recfmt=bk"
        )
    );
    assert_eq!(
        r.formats[0].availability_url.as_deref(),
        Some("https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/1?RECFMT=BK")
    );
}

#[test]
fn record_boundary_and_first_wins_fields() {
    let html = r#"
      <fieldset class="card card-list">
        <input name="SVL" value=""><input name="SVL" value="111"><input name="SVL" value="222">
        <span class="item-num"> </span><span class="item-num">1,234</span><span class="item-num">9</span>
        <h3 class="card-title"><a href="/cgi-bin/spydus.exe/FULL/OPAC/ALLWRKENQ/5/111,1">The <b>Title</b></a></h3>
        <img class="imgsc" data-imgurls="https://x/isbn=9780000000001/a.jpg**https://x/isbn=9780000000001/a.jpg">
        <img class="imgsc" data-imgurls="">
      </fieldset>
      <span class="item-num">99</span>
      <fieldset class="card card-list"><h3 class="card-title"><a href="x">Second</a></h3></fieldset>"#;
    let results = parse_search_results(html, ORIGIN, Some(""));
    assert_eq!(results.query.as_deref(), Some(""));
    assert_eq!(results.records.len(), 2);
    let first = &results.records[0];
    assert_eq!(
        first.work_irn.as_deref(),
        Some("111"),
        "empty SVL value is falsy, next wins"
    );
    assert_eq!(
        first.rank,
        Some(1234),
        "first chunk with an integer sets rank"
    );
    assert_eq!(first.title.as_deref(), Some("The Title"));
    assert_eq!(
        first.cover_urls,
        ["https://x/isbn=9780000000001/a.jpg"],
        "empty imgurls keeps covers"
    );
    assert_eq!(first.isbn.as_deref(), Some("9780000000001"));
    let second = &results.records[1];
    assert_eq!(
        second.rank, None,
        "the stray item-num between cards is outside any fieldset"
    );
    assert_eq!(
        second.detail_url.as_deref(),
        Some("https://canning.spydus.com/x")
    );
    // No "Showing" header: total falls back to the record count.
    assert_eq!(results.total_results, Some(2));
    assert_eq!(results.showing, None);
}

#[test]
fn no_records_and_no_header_gives_null_total() {
    let results = parse_search_results("<p>No results</p>", ORIGIN, None);
    assert_eq!(results.total_results, None);
    assert!(results.records.is_empty());
    assert_eq!(results.query, None);
}

fn header(text: &str) -> String {
    format!(r#"<div class="result-header result-header-brief"><div>{text}</div></div>"#)
}

#[test]
fn showing_regex() {
    let results = parse_search_results(&header("Showing 11 - 20 of 1,046"), ORIGIN, None);
    assert_eq!(results.showing, Some(Showing { from: 11, to: 20 }));
    assert_eq!(results.total_results, Some(1046));

    // Case-insensitive, whitespace collapsed, text split across descendants and entities.
    let results = parse_search_results(
        &header("<b>sHoWiNg</b>\n 1&nbsp;-<i>10</i>   OF\t46 results"),
        ORIGIN,
        None,
    );
    assert_eq!(results.showing, Some(Showing { from: 1, to: 10 }));
    assert_eq!(results.total_results, Some(46));

    // No space before "of" → no match → total falls back to the record count (none → null).
    let results = parse_search_results(&header("Showing 1 - 10of 46"), ORIGIN, None);
    assert_eq!(results.showing, None);
    assert_eq!(results.total_results, None);

    // Long s (U+017F) is not an ASCII `s` under JS `/i`.
    let results = parse_search_results(&header("\u{17F}howing 1 - 10 of 46"), ORIGIN, None);
    assert_eq!(results.showing, None);
}

#[test]
fn pagination_set_id_first_wins() {
    let html = r#"
      <nav class="result-pages-prvnxt">
        <span class="prv"><a href="">empty is falsy</a><a href="/cgi-bin/spydus.exe/SET/OPAC/ALLWRKENQ/77/1?PREV=1">Prev</a></span>
        <span class="nxt"><a href="/cgi-bin/spydus.exe/SET/OPAC/ALLWRKENQ/78/2?NREC=10">Next</a><a href="/other">x</a></span>
      </nav>"#;
    let results = parse_search_results(html, ORIGIN, None);
    assert_eq!(
        results.prev_page_url.as_deref(),
        Some("https://canning.spydus.com/cgi-bin/spydus.exe/SET/OPAC/ALLWRKENQ/77/1?PREV=1")
    );
    assert_eq!(
        results.next_page_url.as_deref(),
        Some("https://canning.spydus.com/cgi-bin/spydus.exe/SET/OPAC/ALLWRKENQ/78/2?NREC=10")
    );
    assert_eq!(
        results.set_id.as_deref(),
        Some("77"),
        "first ALLWRKENQ link in document order"
    );
}

#[test]
fn set_id_accepts_edition_level_allenq() {
    let page = |links: &str| parse_search_results(links, ORIGIN, None).set_id;
    assert_eq!(
        page(r#"<a href="/cgi-bin/spydus.exe/ENQ/OPAC/ALLENQ/4321?SORTS=SQL_REL_BIB">x</a>"#)
            .as_deref(),
        Some("4321")
    );
    // Links without a set number (or another ALL… enquiry) are skipped; the first match wins.
    let links = r#"<a href="/cgi-bin/spydus.exe/ENQ/OPAC/ALLENQ?ENTRY=x">a</a>
      <a href="/cgi-bin/spydus.exe/ENQ/OPAC/ALLBIBENQ/5">b</a>
      <a href="/cgi-bin/spydus.exe/ENQ/OPAC/ALLENQ/6/1">c</a>
      <a href="/cgi-bin/spydus.exe/ENQ/OPAC/ALLWRKENQ/7">d</a>"#;
    assert_eq!(page(links).as_deref(), Some("6"));
}

#[test]
fn no_results_needs_the_wording_in_an_alert_and_no_records() {
    let alert = |text: &str| format!(r#"<div class="alert alert-info" role="alert">{text}</div>"#);
    let no_results = |html: &str| parse_search_results(html, ORIGIN, None).no_results;

    // Nested markup, entities, line breaks and case are all fine (the text is collapsed first).
    assert!(no_results(&alert(
        "Sorry, your search for <b>Anywhere: x</b> did&nbsp;not\n  find any RECORDS."
    )));
    // NZ wording.
    assert!(no_results(&alert(
        "Sorry, your search for Anywhere: x (Keywords) in the local catalogue did not find any records."
    )));
    // A results page whose alert happens to say it still has records → false.
    let with_record = format!(
        r#"{}<fieldset class="card card-list"><h3 class="card-title"><a href="/b">T</a></h3></fieldset>"#,
        alert("did not find any records")
    );
    assert!(!no_results(&with_record));
    // The wording outside an alert, other alerts, and empty or foreign pages → false.
    assert!(!no_results(
        "<p>Sorry, your search did not find any records.</p>"
    ));
    assert!(!no_results(&alert(
        "Please enter something in the search form."
    )));
    assert!(!no_results(&alert(
        "To protect your privacy please logout when you have finished."
    )));
    assert!(!no_results(""));
    assert!(!no_results(
        "<html><title>Human Verification</title></html>"
    ));
}
