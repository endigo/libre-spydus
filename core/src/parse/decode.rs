//! Port of `middleware/src/parse/decode.ts` plus the shared `parseCovers` helper
//! (search-results.ts:77-95; the copies in record.ts:9-25 and account.ts:100-116 are equivalent).
//!
//! JS-regex parity notes for every parser port:
//! - JS `\s` / `String.prototype.trim` ≠ Rust `\s` / `str::trim`: JS includes U+FEFF and excludes
//!   U+0085. Use [`JS_WS`] in patterns and [`js_trim`] for trimming.
//! - JS `\d` and `\w` are ASCII-only; Rust's are Unicode. Write `[0-9]` / `[A-Za-z0-9_]`.
//! - JS `/i` without `/u` never folds non-ASCII into ASCII; Rust `(?i)` does (`ſ` matches `s`,
//!   `K` (Kelvin) matches `k`). Prefer explicit ASCII classes or `(?i-u)` for ASCII-only patterns.

use regex::{Captures, Regex};
use std::sync::LazyLock;

/// Regex character class equivalent to JS `\s` (WhiteSpace + LineTerminator).
pub const JS_WS: &str = r"[\t\n\x0B\x0C\r \x{A0}\x{1680}\x{2000}-\x{200A}\x{2028}\x{2029}\x{202F}\x{205F}\x{3000}\x{FEFF}]";

/// `true` for exactly the characters JS `\s` and `String.prototype.trim` treat as whitespace.
pub fn is_js_whitespace(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n'
            | '\u{0B}'
            | '\u{0C}'
            | '\r'
            | ' '
            | '\u{A0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200A}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202F}'
            | '\u{205F}'
            | '\u{3000}'
            | '\u{FEFF}'
    )
}

/// JS `String.prototype.trim`.
pub fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_whitespace)
}

// decode.ts:27 — `/&(#x?[0-9a-f]+|[a-z][a-z0-9]*);/gi`, spelled with ASCII classes.
static ENTITY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"&(#[xX]?[0-9a-fA-F]+|[a-zA-Z][a-zA-Z0-9]*);").unwrap());
static WS_RUN: LazyLock<Regex> = LazyLock::new(|| Regex::new(&format!("{JS_WS}+")).unwrap());
static INT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"-?[0-9]+").unwrap());
static HTTP_SCHEME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(?i-u)https?://").unwrap());
static SYNDETICS_PLACEHOLDER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i-u)syndetics\.com/index\.aspx\?isbn=/mc\.gif").unwrap());
static COVER_ISBN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"isbn=([0-9]{9,13}[Xx]?)").unwrap());

/// decode.ts:6-22. Note `nbsp` maps to a plain U+0020 space, not U+00A0.
fn named(name: &str) -> Option<&'static str> {
    Some(match name {
        "amp" => "&",
        "lt" => "<",
        "gt" => ">",
        "quot" => "\"",
        "apos" => "'",
        "nbsp" => " ",
        "copy" => "©",
        "reg" => "®",
        "hellip" => "…",
        "mdash" => "—",
        "ndash" => "–",
        "rsquo" => "’",
        "lsquo" => "‘",
        "ldquo" => "“",
        "rdquo" => "”",
        _ => return None,
    })
}

/// `&#…;` body (without `&`/`;`) → replacement, replicating `parseInt` + `String.fromCodePoint`.
fn numeric(body: &str, whole: &str) -> String {
    let (digits, radix) = match body.as_bytes().get(1) {
        Some(b'x' | b'X') => (&body[2..], 16),
        _ => (&body[1..], 10),
    };
    // parseInt reads the longest valid-digit prefix; no digits → NaN → keep the entity.
    let mut value = 0f64;
    let mut seen = false;
    for c in digits.chars() {
        let Some(d) = c.to_digit(radix) else { break };
        value = value * f64::from(radix) + f64::from(d);
        seen = true;
    }
    if !seen || !value.is_finite() {
        return whole.to_owned();
    }
    // fromCodePoint throws RangeError above U+10FFFF → "".
    if value > f64::from(0x10FFFF) {
        return String::new();
    }
    // JS keeps a lone surrogate (U+D800-U+DFFF); a Rust String can't, so emit U+FFFD.
    char::from_u32(value as u32).map_or_else(|| "\u{FFFD}".to_owned(), String::from)
}

/// Decode HTML entities (named + numeric, decimal and hex). decode.ts:25-38.
pub fn decode(input: &str) -> String {
    if !input.contains('&') {
        return input.to_owned();
    }
    ENTITY
        .replace_all(input, |caps: &Captures| {
            let whole = &caps[0];
            let body = &caps[1];
            if body.starts_with('#') {
                numeric(body, whole)
            } else {
                named(&body.to_ascii_lowercase()).map_or_else(|| whole.to_owned(), str::to_owned)
            }
        })
        .into_owned()
}

/// Decode, collapse runs of whitespace to single spaces, and trim. decode.ts:49-51.
pub fn collapse(input: &str) -> String {
    let decoded = decode(input);
    js_trim(&WS_RUN.replace_all(&decoded, " ")).to_owned()
}

/// First integer found in a string (commas ignored), or `None`. decode.ts:54-58.
///
/// Beyond `i64` the TS returns an imprecise float; this returns `None`.
pub fn to_int(input: &str) -> Option<i64> {
    if input.is_empty() {
        return None;
    }
    let stripped = input.replace(',', "");
    INT.find(&stripped)?.as_str().parse().ok()
}

/// Normalise a relative Spydus path to an absolute URL on `origin`. decode.ts:61-67.
pub fn absolute(origin: &str, href: Option<&str>) -> Option<String> {
    let href = href.filter(|h| !h.is_empty())?;
    let decoded = decode(href);
    let clean = js_trim(&decoded);
    if clean.is_empty() {
        return None;
    }
    if HTTP_SCHEME.is_match(clean) {
        return Some(clean.to_owned());
    }
    let base = origin.strip_suffix('/').unwrap_or(origin);
    Some(if clean.starts_with('/') {
        format!("{base}{clean}")
    } else {
        format!("{base}/{clean}")
    })
}

/// Cover URLs + ISBN split out of an `img.imgsc[data-imgurls]` value.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Covers {
    pub covers: Vec<String>,
    pub isbn: Option<String>,
}

/// `data-imgurls` ("**"-joined) → de-duplicated cover URLs + first ISBN. search-results.ts:77-95.
pub fn parse_covers(data_imgurls: Option<&str>) -> Covers {
    let Some(raw) = data_imgurls.filter(|s| !s.is_empty()) else {
        return Covers::default();
    };
    let decoded = decode(raw);
    let parts: Vec<&str> = decoded
        .split("**")
        .map(js_trim)
        .filter(|s| !s.is_empty())
        // drop Syndetics placeholders with no ISBN (…index.aspx?isbn=/mc.gif)
        .filter(|u| !SYNDETICS_PLACEHOLDER.is_match(u))
        .collect();
    let isbn = parts
        .iter()
        .find_map(|u| COVER_ISBN.captures(u).map(|c| c[1].to_owned()));
    let mut covers: Vec<String> = Vec::with_capacity(parts.len());
    for part in parts {
        if !covers.iter().any(|c| c == part) {
            covers.push(part.to_owned());
        }
    }
    Covers { covers, isbn }
}
