//! Golden + unit tests for `parse::record` (port of `middleware/src/parse/record.ts`).
//! Unit-test expectations were produced by running the TS parsers on the same snippets.

mod common;

use librespydus_core::config::ORIGIN;
use librespydus_core::parse::record::{
    bib_link_from_fragment, parse_brief_cards, parse_record_detail,
};
use librespydus_core::types::RecordDetail;
use serde::Serialize;
use serde_json::{Value, json};

// ---------------------------------------------------------------------------
// Goldens (rows of tests/golden/MANIFEST.md)
// ---------------------------------------------------------------------------

#[test]
fn golden_biblink_bsc() {
    let html = common::fixture("availability.html");
    common::assert_golden(&bib_link_from_fragment(&html, ORIGIN), "biblink-bsc.json");
}

#[test]
fn golden_biblink() {
    let html = common::fixture("bk-recfmt.html");
    common::assert_golden(&bib_link_from_fragment(&html, ORIGIN), "biblink.json");
}

#[test]
fn golden_record() {
    let html = common::fixture("record.html");
    common::assert_golden(&parse_record_detail(&html, ORIGIN, None), "record.json");
}

#[test]
fn golden_record_bib() {
    let html = common::fixture("record-bib.html");
    let actual = parse_record_detail(&html, ORIGIN, Some("8753080".into()));
    common::assert_golden(&actual, "record-bib.json");
}

#[test]
fn golden_related() {
    let html = common::fixture("related.html");
    common::assert_golden(&parse_brief_cards(&html, ORIGIN), "related.json");
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

fn to_json(actual: &impl Serialize) -> Value {
    serde_json::to_value(actual).expect("serialize")
}

fn url(path: &str) -> String {
    format!("{ORIGIN}{path}")
}

/// `#divtabRECDETAILS` holding the given rows.
fn rec_details(rows: &str) -> String {
    format!(r#"<div id="divtabRECDETAILS">{rows}</div>"#)
}

/// One Record-details row with a caption and a value column.
fn row(label: &str, col: &str) -> String {
    format!(
        r#"<div class="row"><div class="fd-caption">{label}</div><div class="col">{col}</div></div>"#
    )
}

fn detail(rows: &str) -> RecordDetail {
    parse_record_detail(&rec_details(rows), ORIGIN, None)
}

#[test]
fn sibling_spans_are_separate_values_with_first_link() {
    let d = detail(&row(
        "Subjects:",
        r#"<span class="d-block"><a href="/s/1">One</a> <a href="/s/2">x</a></span><span class="d-block">Two</span><span class="d-block">  </span>"#,
    ));
    assert_eq!(
        to_json(&d.details),
        json!([{ "label": "Subjects", "values": [
            { "text": "One x", "url": url("/s/1") },
            { "text": "Two", "url": null },
        ]}])
    );
}

#[test]
fn nested_span_flushes_outer_on_open_and_drops_trailing_text() {
    // Outer opens (buf=""), "Outer " buffered; inner open flushes "Outer" and resets; inner end
    // flushes "Inner" and clears the buffer, so the outer's " tail" is dropped.
    let d = detail(&row(
        "Notes",
        r#"<span class="d-block">Outer <span class="d-block"><a href="/in">Inner</a></span> tail</span>"#,
    ));
    assert_eq!(
        to_json(&d.details),
        json!([{ "label": "Notes", "values": [
            { "text": "Outer", "url": null },
            { "text": "Inner", "url": url("/in") },
        ]}])
    );
}

#[test]
fn empty_href_is_replaced_by_a_later_link() {
    let d = detail(&row(
        "Link",
        r#"<span class="d-block"><a href="">a</a><a href="/second">b</a></span>"#,
    ));
    assert_eq!(d.details[0].values[0].url.as_deref(), Some(url("/second").as_str()));
}

#[test]
fn rows_need_label_and_values_and_strip_one_trailing_colon() {
    let d = detail(&format!(
        "{}{}{}",
        row("", r#"<span class="d-block">no label</span>"#),
        row("Empty:", r#"<span class="d-block"> </span>"#),
        row("Main title :", r#"<span class="d-block">Spaced / x</span>"#),
    ));
    // `/:\s*$/` leaves the space before the colon, so "Main title " is not the main title.
    assert_eq!(
        to_json(&d.details),
        json!([{ "label": "Main title ", "values": [{ "text": "Spaced / x", "url": null }] }])
    );
    assert_eq!(d.title, None);
}

#[test]
fn nested_row_takes_over_and_closes_the_outer_row() {
    let d = detail(concat!(
        r#"<div class="row"><div class="fd-caption">Outer</div><div class="col"><span class="d-block">o1</span></div>"#,
        r#"<div class="row"><div class="fd-caption">Inner</div><div class="col"><span class="d-block">i1</span></div></div>"#,
        r#"<div class="col"><span class="d-block">o2</span></div></div>"#,
    ));
    assert_eq!(
        to_json(&d.details),
        json!([
            { "label": "Inner", "values": [{ "text": "i1", "url": null }] },
            { "label": "Outer", "values": [{ "text": "o1", "url": null }] },
        ])
    );
}

#[test]
fn related_versions_reads_the_singular_version_key() {
    let d = parse_record_detail(
        concat!(
            r#"<a id="urltabSIMILAR" href="/s1">a</a><a id="urltabSIMILAR" href="/s2">b</a>"#,
            r#"<a id="urltabVERSIONS" href="/wrong">c</a><a id="urltabVersion" href="/v">d</a>"#,
            r#"<a id="urltabAUTHOR" href="">e</a><a id="urltabAUTHOR" href="/au">f</a>"#,
            r#"<a id="xurltabSERIES" href="/no">g</a>"#,
        ),
        ORIGIN,
        None,
    );
    assert_eq!(
        to_json(&d.related),
        json!({
            "similar": url("/s1"),
            "author": url("/au"),
            "series": null,
            "versions": url("/v"),
        })
    );
}

#[test]
fn loading_summary_is_nulled_but_empty_summary_stays_empty() {
    let summary = |inner: &str| {
        let html = format!(r#"<div id="divtabSUMMARY">{inner}</div>"#);
        parse_record_detail(&html, ORIGIN, None).summary
    };
    assert_eq!(summary(" Loading... "), None);
    assert_eq!(summary("LOADING"), None);
    assert_eq!(summary("Loading."), None);
    assert_eq!(summary("Loading...."), Some("Loading....".into()));
    // TS: `summary && /^(loading…)?$/i.test(summary)`: "" is falsy, so it is kept as "".
    assert_eq!(summary("  "), Some(String::new()));
    assert_eq!(summary("<p>A &amp; B</p>\n <p>c</p>"), Some("A & B c".into()));
    assert_eq!(parse_record_detail("<p>none</p>", ORIGIN, None).summary, None);
}

#[test]
fn bookmark_is_synthesized_from_the_brn_row() {
    let html = rec_details(&row("BRN:", r#"<span class="d-block">12345</span>"#));
    let d = parse_record_detail(&html, ORIGIN, Some("999".into()));
    assert_eq!(d.bib_irn.as_deref(), Some("999"));
    assert_eq!(d.brn.as_deref(), Some("12345"));
    assert_eq!(d.bookmark_url, Some(url("/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&BRN=12345")));
}

#[test]
fn bookmark_row_url_wins_and_supplies_the_brn() {
    // The row url is absolutized once when collected and again when returned (double decode).
    let d = detail(&row(
        "Bookmark link",
        r#"<span class="d-block"><a href="/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&amp;amp;BRN=678">link</a></span>"#,
    ));
    assert_eq!(
        d.details[0].values[0].url,
        Some(url("/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&amp;BRN=678"))
    );
    assert_eq!(d.brn.as_deref(), Some("678"));
    assert_eq!(d.bookmark_url, Some(url("/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&BRN=678")));

    let d = detail(&row(
        "Bookmark",
        r#"<span class="d-block"><a href="https://other.example/x">link</a></span>"#,
    ));
    assert_eq!(d.brn, None);
    assert_eq!(d.bookmark_url.as_deref(), Some("https://other.example/x"));
}

#[test]
fn derived_title_series_isbn_without_brn_or_bookmark() {
    let d = detail(&format!(
        "{}{}{}",
        row("Main title:", r#"<span class="d-block">Alpha / Beta / Gamma</span>"#),
        row("Series title", r#"<span class="d-block">S1</span>"#),
        row("ISBN", r#"<span class="d-block">pbk. 978014303943X (x)</span>"#),
    ));
    assert_eq!(d.title.as_deref(), Some("Alpha"));
    assert_eq!(d.series_title.as_deref(), Some("S1"));
    assert_eq!(d.isbn.as_deref(), Some("978014303943X"));
    assert_eq!(d.brn, None);
    assert_eq!(d.bookmark_url, None);
}

#[test]
fn first_real_cover_wins_and_isbn_row_beats_cover_isbn() {
    let d = parse_record_detail(
        concat!(
            r#"<img class="imgsc" data-imgurls="https://secure.syndetics.com/index.aspx?isbn=/mc.gif">"#,
            r#"<img class="imgsc" data-imgurls="https://a/isbn=9780000000001/lc.gif**https://b">"#,
            r#"<img class="imgsc" data-imgurls="https://c/isbn=9780000000002">"#,
        ),
        ORIGIN,
        None,
    );
    assert_eq!(d.cover_urls, ["https://a/isbn=9780000000001/lc.gif", "https://b"]);
    assert_eq!(d.isbn.as_deref(), Some("9780000000001"));

    let html = rec_details(&row("ISBN", r#"<span class="d-block">9781111111111</span>"#))
        + r#"<img class="imgsc" data-imgurls="https://a/isbn=9780000000001/lc.gif">"#;
    let d = parse_record_detail(&html, ORIGIN, None);
    assert_eq!(d.isbn.as_deref(), Some("9781111111111"));
    assert_eq!(d.cover_urls, ["https://a/isbn=9780000000001/lc.gif"]);
}

#[test]
fn brief_cards_boundaries_lines_and_finalize_rules() {
    let html = r#"
        <span class="card-title"><a href="/orphan?BRN=1">Orphan first</a></span>
        <a class="thumblink" href="/b?BRN=10&amp;SETLVL="><img class="imgsc" data-imgurls="https://x/isbn=9780000000010/lc.gif"></a>
        <span class="card-title"><a href="/ignored">Ten</a></span>
        <img class="imgsc" data-imgurls="https://y/second.gif">
        <div class="card-text recdetails"><span class="d-block">2001</span><span class="d-block"> Smith, J. </span><span class="d-block">1999 - 2000</span><span class="d-block">Other</span></div>
        <a class="thumblink" href=""></a><span class="card-title"><a href="/c?IRN(55)">Fifty five</a></span>
        <div class="card-text recdetails"><span class="d-block">Author <span class="d-block">Nested</span> lost</span></div>
        <a class="thumblink" href="/notitle?BRN=3"></a><span class="card-title"><a href="/x"> </a></span>
        <a class="thumblink" href="/last?IRN=77"></a><span class="card-title"><a href="/y">Last</a></span><div class="card-text recdetails"><span class="d-block">1999–2001</span></div>
    "#;
    assert_eq!(
        to_json(&parse_brief_cards(html, ORIGIN)),
        json!([
            { "title": "Orphan first", "bibUrl": url("/orphan?BRN=1"), "brn": "1",
              "author": null, "year": null, "isbn": null, "coverUrls": [] },
            { "title": "Ten", "bibUrl": url("/b?BRN=10&SETLVL="), "brn": "10",
              "author": "Smith, J.", "year": "2001", "isbn": "9780000000010",
              "coverUrls": ["https://y/second.gif"] },
            { "title": "Fifty five", "bibUrl": url("/c?IRN(55)"), "brn": "55",
              "author": "Nested", "year": null, "isbn": null, "coverUrls": [] },
            { "title": "Last", "bibUrl": url("/last?IRN=77"), "brn": "77",
              "author": null, "year": "1999–2001", "isbn": null, "coverUrls": [] },
        ])
    );
}

#[test]
fn bib_link_decodes_html_and_takes_the_first_match() {
    let link = bib_link_from_fragment(
        r#"<a href="/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/123?SETLVL=&amp;IRN=456&amp;NAVLVL=SET">x</a>"#,
        ORIGIN,
    )
    .expect("link");
    assert_eq!(
        link.bib_url,
        url("/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/123?SETLVL=&IRN=456&NAVLVL=SET")
    );
    assert_eq!(link.bib_irn, "456");

    let link = bib_link_from_fragment(
        r#"<a href="/x/BIBENQ/1?IRN(9)">a</a><a href="/y/BIBENQ/2?IRN=8">b</a>"#,
        ORIGIN,
    )
    .expect("link");
    assert_eq!(link.bib_url, url("/x/BIBENQ/1?IRN(9)"));
    assert_eq!(link.bib_irn, "9");

    assert_eq!(bib_link_from_fragment(r#"<a href="/x/BIBENQ/abc?IRN=9">a</a>"#, ORIGIN), None);
}
