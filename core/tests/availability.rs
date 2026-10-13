//! `parse::availability::parse_availability` — golden parity with `parseAvailability` (MANIFEST
//! rows `availability.json`, `availability-midnight.json`) plus focused cases for the TS quirks.

mod common;

use librespydus_core::config::ORIGIN;
use librespydus_core::parse::availability::parse_availability;
use std::collections::HashMap;

#[test]
fn golden_availability() {
    let html = common::fixture("availability.html");
    common::assert_golden(&parse_availability(&html, ORIGIN), "availability.json");
}

#[test]
fn golden_availability_midnight() {
    let html = common::fixture("bk-recfmt.html");
    common::assert_golden(
        &parse_availability(&html, ORIGIN),
        "availability-midnight.json",
    );
}

fn pair(label: &str, val: &str) -> String {
    format!(
        r#"<span class="availability-text">{label}</span><span class="availability-val">{val}</span>"#
    )
}

#[test]
fn counts_pair_each_label_with_the_next_value() {
    let html = [
        pair("Reserved: ", "1"),
        pair(" On&nbsp;order:", "1,203"),
        // A value with no integer drops its label; a label with no value is overwritten.
        pair("Copies:", "none"),
        r#"<span class="availability-text">In transit:</span>"#.to_owned(),
        pair("Copies:", "<b>3</b> copies"),
        // A value with no pending label is ignored; an empty label is no label.
        r#"<span class="availability-val">7</span>"#.to_owned(),
        pair(":", "8"),
        // Only one trailing colon is stripped.
        pair("Odd::", "2"),
    ]
    .concat();
    let a = parse_availability(&html, ORIGIN);
    let expected: HashMap<String, i64> = [
        ("reserved", 1),
        ("on order", 1203),
        ("copies", 3),
        ("odd:", 2),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v))
    .collect();
    assert_eq!(a.counts, expected);
    assert_eq!(a.reserved, Some(1));
    assert_eq!(a.copies, Some(3));
}

#[test]
fn title_latches_after_first_non_blank_cell() {
    let html = r#"<table><tr>
      <td data-caption="Title"><a href="/t"> </a></td>
      <td data-caption="Title"><a href="/t">The <i>Midnight</i> Library</a></td>
      <td data-caption="Title"><a href="/t">Second edition</a></td>
    </tr></table>"#;
    let a = parse_availability(html, ORIGIN);
    assert_eq!(a.title.as_deref(), Some("The Midnight Library"));

    assert_eq!(parse_availability("<p>nothing</p>", ORIGIN).title, None);
}

fn imprint_block(spans: &[&str]) -> String {
    let cols: String = spans
        .iter()
        .map(|s| format!(r#"<div class="col"><span>{s}</span></div>"#))
        .collect();
    format!(r#"<div class="recfmt-container">{cols}</div>"#)
}

#[test]
fn publisher_prefers_separator_and_year() {
    let a = parse_availability(
        &imprint_block(&["English", "London : Faber", "New York : Viking, 2021."]),
        ORIGIN,
    );
    assert_eq!(a.publisher.as_deref(), Some("New York : Viking, 2021."));

    // No candidate with a year → first with " : ".
    let a = parse_availability(
        &imprint_block(&["2021", "London : Faber", "Paris : X"]),
        ORIGIN,
    );
    assert_eq!(a.publisher.as_deref(), Some("London : Faber"));

    // A year alone is not enough; ":" needs whitespace on both sides.
    let a = parse_availability(&imprint_block(&["2021", "London:Faber, 2020"]), ORIGIN);
    assert_eq!(a.publisher, None);

    // Only direct `span` children of a `.col` inside `.recfmt-container` count.
    let outside = r#"<div class="col"><span>London : Faber, 2020</span></div>"#;
    let nested = r#"<div class="recfmt-container"><div class="col"><p><span>London : Faber, 2020</span></p></div></div>"#;
    assert_eq!(parse_availability(outside, ORIGIN).publisher, None);
    assert_eq!(parse_availability(nested, ORIGIN).publisher, None);
}

#[test]
fn xhld_irn_overrides_an_earlier_xfull_irn() {
    let html = r#"
      <a href="/cgi-bin/spydus.exe/XFULL/OPAC/BIBENQ/78/111?FMT=REC">record</a>
      <a href="/cgi-bin/spydus.exe/XFULL/OPAC/BIBENQ/78/999?FMT=REC">second record link ignored</a>
      <a href="/cgi-bin/spydus.exe/XHLD/OPAC/BIBENQ/78/222?RECDISP=REC">holdings</a>
      <a href="/cgi-bin/spydus.exe/XHLD/OPAC/BIBENQ/78/333?RECDISP=REC">second holdings link ignored</a>"#;
    let a = parse_availability(html, ORIGIN);
    assert_eq!(a.bib_record_irn.as_deref(), Some("222"));
    assert_eq!(
        a.record_url.as_deref(),
        Some("https://canning.spydus.com/cgi-bin/spydus.exe/XFULL/OPAC/BIBENQ/78/111?FMT=REC")
    );
    assert_eq!(
        a.holdings_url.as_deref(),
        Some("https://canning.spydus.com/cgi-bin/spydus.exe/XHLD/OPAC/BIBENQ/78/222?RECDISP=REC")
    );
}

#[test]
fn xfull_irn_is_only_a_fallback() {
    // XHLD first: its IRN stands; XFULL does not replace it.
    let html = r#"
      <a href="/cgi-bin/spydus.exe/XHLD/OPAC/BIBENQ/78/222">holdings</a>
      <a href="/cgi-bin/spydus.exe/XFULL/OPAC/BIBENQ/78/111">record</a>"#;
    assert_eq!(
        parse_availability(html, ORIGIN).bib_record_irn.as_deref(),
        Some("222")
    );

    // XHLD link without the /{set}/{irn} shape: URL kept, IRN falls back to XFULL.
    let html = r#"
      <a href="/cgi-bin/spydus.exe/XHLD/OPAC/BIBENQ?IRN=5">holdings</a>
      <a href="/cgi-bin/spydus.exe/XFULL/OPAC/BIBENQ/78/111">record</a>"#;
    let a = parse_availability(html, ORIGIN);
    assert_eq!(a.bib_record_irn.as_deref(), Some("111"));
    assert!(a.holdings_url.is_some());
}
