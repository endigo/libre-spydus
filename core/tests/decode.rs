//! `parse::decode` unit tests. Expected values were taken from the TS `decode.ts` / `parseCovers`
//! (run under Bun), except the two documented divergences (lone surrogate, i64 overflow).
//! Also holds self-tests for the shared golden harness in `common/`.

mod common;

use librespydus_core::config::ORIGIN;
use librespydus_core::parse::decode::{
    Covers, absolute, collapse, decode, is_js_whitespace, js_trim, parse_covers, to_int,
};

#[test]
fn decode_passthrough_and_named() {
    assert_eq!(decode(""), "");
    assert_eq!(decode("plain text"), "plain text");
    assert_eq!(decode("Tom &amp; Jerry"), "Tom & Jerry");
    // Named lookup is case-insensitive.
    assert_eq!(decode("&AMP;&Lt;&GT;&quot;&apos;"), "&<>\"'");
    assert_eq!(decode("&copy;&reg;&hellip;&mdash;&ndash;&rsquo;&lsquo;&ldquo;&rdquo;"), "©®…—–’‘“”");
    // TS maps nbsp to a plain space (U+0020), not U+00A0.
    assert_eq!(decode("a&nbsp;b"), "a b");
    // Unknown names and missing semicolons are left alone; one pass only.
    assert_eq!(decode("&unknown; &amp no-semi"), "&unknown; &amp no-semi");
    assert_eq!(decode("&amp;amp;"), "&amp;");
}

#[test]
fn decode_numeric() {
    assert_eq!(decode("&#65;&#x42;&#X43;&#0067;"), "ABCC");
    assert_eq!(decode("&#x1F600;&#128512;"), "😀😀");
    assert_eq!(decode("&#x10FFFF;"), "\u{10FFFF}");
    assert_eq!(decode("&#0;"), "\0");
    // parseInt prefix semantics: "12ab" → 12 (form feed).
    assert_eq!(decode("&#12ab;"), "\u{0C}");
    // No decimal digits → NaN → entity kept.
    assert_eq!(decode("&#abc;"), "&#abc;");
    assert_eq!(decode("&#x;"), "&#x;");
    // Above U+10FFFF → fromCodePoint RangeError → "".
    assert_eq!(decode("&#1114112;"), "");
    assert_eq!(decode("&#9999999999999999999999;"), "");
    // parseInt → Infinity → not finite → entity kept.
    let huge = format!("&#{};", "9".repeat(400));
    assert_eq!(decode(&huge), huge);
}

#[test]
fn decode_lone_surrogate_divergence() {
    // JS yields a lone UTF-16 surrogate; a Rust String cannot, so U+FFFD (documented).
    assert_eq!(decode("&#xD800;"), "\u{FFFD}");
}

#[test]
fn js_whitespace_semantics() {
    assert!(is_js_whitespace('\u{FEFF}'));
    assert!(is_js_whitespace('\u{3000}'));
    assert!(!is_js_whitespace('\u{85}'));
    assert_eq!(js_trim("\u{FEFF}\u{A0} x \u{2028}"), "x");
    assert_eq!(js_trim("\u{85}x\u{85}"), "\u{85}x\u{85}");
}

#[test]
fn collapse_whitespace() {
    assert_eq!(collapse("  a \n\t b  "), "a b");
    assert_eq!(collapse(""), "");
    assert_eq!(collapse("x&nbsp;&nbsp;y"), "x y");
    assert_eq!(collapse("&#32;&#32;"), "");
    assert_eq!(collapse("\u{3000}full\u{3000}width\u{3000}"), "full width");
    // JS \s includes U+FEFF and excludes U+0085 (NEL).
    assert_eq!(collapse("\u{FEFF} a \u{85} b \u{A0} c\u{2028}"), "a \u{85} b c");
}

#[test]
fn to_int_first_integer() {
    assert_eq!(to_int(""), None);
    assert_eq!(to_int("abc"), None);
    assert_eq!(to_int("1,234 of 5"), Some(1234));
    assert_eq!(to_int("-0"), Some(0));
    assert_eq!(to_int("x-12y"), Some(-12));
    assert_eq!(to_int("007"), Some(7));
    assert_eq!(to_int("Copies: 3"), Some(3));
    assert_eq!(to_int(" 1 2"), Some(1));
    // JS \d is ASCII-only.
    assert_eq!(to_int("٣"), None);
    // TS gives the imprecise float 1e20; Rust returns None beyond i64 (documented).
    assert_eq!(to_int("99999999999999999999"), None);
}

#[test]
fn absolute_urls() {
    let o = "https://o.com";
    assert_eq!(absolute(o, None), None);
    assert_eq!(absolute(o, Some("")), None);
    assert_eq!(absolute(o, Some("   ")), None);
    assert_eq!(absolute(o, Some("&#32;")), None);
    assert_eq!(absolute("https://o.com/", Some("x")).as_deref(), Some("https://o.com/x"));
    assert_eq!(absolute(o, Some("/x")).as_deref(), Some("https://o.com/x"));
    // Only one trailing slash is stripped from the origin.
    assert_eq!(absolute("https://o.com//", Some("/x")).as_deref(), Some("https://o.com//x"));
    assert_eq!(absolute(o, Some(" HTTP://a.b ")).as_deref(), Some("HTTP://a.b"));
    assert_eq!(absolute(o, Some("/a?b=1&amp;c=2")).as_deref(), Some("https://o.com/a?b=1&c=2"));
    assert_eq!(absolute(o, Some("https://x.y/z")).as_deref(), Some("https://x.y/z"));
    assert_eq!(absolute(o, Some("httpx://a")).as_deref(), Some("https://o.com/httpx://a"));
    assert_eq!(
        absolute(ORIGIN, Some("/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&amp;BRN=1")).as_deref(),
        Some("https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&BRN=1"),
    );
}

fn covers(urls: &[&str], isbn: Option<&str>) -> Covers {
    Covers { covers: urls.iter().map(|s| s.to_string()).collect(), isbn: isbn.map(str::to_owned) }
}

#[test]
fn parse_covers_split_filter_dedupe() {
    assert_eq!(parse_covers(None), Covers::default());
    assert_eq!(parse_covers(Some("")), Covers::default());
    assert_eq!(parse_covers(Some("****")), Covers::default());
    assert_eq!(
        parse_covers(Some(
            "https://secure.syndetics.com/index.aspx?isbn=/mc.gif**https://a/x.jpg?isbn=9780525559474&amp;c=1** https://a/x.jpg?isbn=9780525559474&c=1 **https://b/y.jpg"
        )),
        covers(&["https://a/x.jpg?isbn=9780525559474&c=1", "https://b/y.jpg"], Some("9780525559474")),
    );
    // `isbn=` is case-sensitive; first lower-case match wins.
    assert_eq!(
        parse_covers(Some("u?ISBN=1234567890**v?isbn=123456789X")),
        covers(&["u?ISBN=1234567890", "v?isbn=123456789X"], Some("123456789X")),
    );
    // The placeholder filter is case-insensitive.
    assert_eq!(parse_covers(Some("SYNDETICS.COM/INDEX.ASPX?ISBN=/MC.GIF")), Covers::default());
    assert_eq!(parse_covers(Some("a?isbn=12345678")), covers(&["a?isbn=12345678"], None));
    assert_eq!(
        parse_covers(Some("a?isbn=12345678901234x")),
        covers(&["a?isbn=12345678901234x"], Some("1234567890123")),
    );
    assert_eq!(parse_covers(Some("a***b")), covers(&["a", "*b"], None));
}

// ---- golden harness self-tests ----

#[test]
fn harness_golden_roundtrip_passes() {
    common::assert_golden(&common::golden("availability.json"), "availability.json");
}

#[test]
fn harness_reports_json_pointer() {
    let mut v = common::golden("search.json");
    v["records"][0]["title"] = serde_json::json!("changed");
    let err = std::panic::catch_unwind(|| common::assert_golden(&v, "search.json")).unwrap_err();
    let msg = err.downcast_ref::<String>().expect("panic message");
    assert!(msg.contains("/records/0/title"), "{msg}");
    // Remove the deliberately wrong actual so task B never sees a stale file.
    let written = msg.split("actual written to ").nth(1).and_then(|s| s.lines().next());
    let _ = std::fs::remove_file(written.expect("actual path"));
}

#[test]
fn harness_fixture_strips_bom() {
    assert!(common::fixture("encrypt.js").starts_with("function rsaEncrypt"));
}

/// Every fixture and golden named in MANIFEST.md exists (private ones only when staged).
#[test]
fn manifest_files_exist() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let manifest = std::fs::read_to_string(root.join("golden/MANIFEST.md")).unwrap();
    let private = common::private_available();
    let mut rows = 0;
    for line in manifest.lines().filter(|l| l.starts_with("| `")) {
        let cols: Vec<&str> = line.split(" | ").map(|c| c.trim_matches(['|', ' ', '`'])).collect();
        let (fixture, golden) = (cols[0], cols[3]);
        rows += 1;
        if fixture.starts_with("private/") && !private {
            continue;
        }
        assert!(root.join("fixtures").join(fixture).is_file(), "missing fixture {fixture}");
        assert!(root.join("golden").join(golden).is_file(), "missing golden {golden}");
        common::golden(golden); // parses
    }
    assert!(rows >= 41, "expected ≥41 manifest rows, found {rows}");
}
