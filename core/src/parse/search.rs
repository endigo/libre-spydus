//! Port of `middleware/src/parse/search-results.ts` (ALLWRKENQ results page; also the setId of
//! edition-level ALLENQ pages). Owner: task B; Wave 2 author/setId rules: task G1; Phase 2
//! `noResults`: task P.
//!
//! Engine: `lol_html` (what Bun's `HTMLRewriter` wraps) in a single streaming pass. There is no
//! DOM and handlers fire in document order, so descendant handlers write into the `current`
//! record draft, opened on each `<fieldset class="card card-list">` and sealed on its end tag.
//! Handlers are registered in the TS order so lol_html dispatches them in the same sequence.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::LazyLock;

use lol_html::html_content::{Element, EndTag};
use lol_html::{RewriteStrSettings, element, rewrite_str, text};
use regex::Regex;

use crate::parse::decode::{JS_WS, absolute, collapse, decode, js_trim, parse_covers, to_int};
use crate::types::{RecordFormat, SearchRecord, SearchResults, Showing};

// search-results.ts `ROLE_WORDS`: relator terms (RDA) that end a creator line. Deliberately no
// publisher / distributor / issuing body: an imprint is never an author.
const ROLE_WORDS: &str = "author|illustrator|editor|translator|adapter|artist|writer|narrator|\
composer|contributor|creator|director|producer|screenwriter|performer|actor|photographer|compiler|\
cartographer|lyricist|presenter|speaker|interviewer|interviewee|host|librettist|arranger|conductor|\
singer|musician|instrumentalist|animator|cinematographer|choreographer|abridger|reader";
// search-results.ts `AUTHOR_ROLE`: ", <role>[, <role>…]" at the end of the line, or a bare space after
// life dates ("Rowling, J. K., 1965- author."). Group 1 = those life dates (kept in the author),
// group 2 = the first role, optionally qualified ("film director", "co-author"). JS `\s` → `JS_WS`;
// JS `/i` folds ASCII only → `(?i-u:…)` (kept off `JS_WS`, whose class is non-ASCII).
static AUTHOR_ROLE: LazyLock<Regex> = LazyLock::new(|| {
    let role = format!(
        r"(?:(?i-u:film|television|executive|voice|cover|joint){JS_WS}+)?(?i-u:(?:co-?)?(?:{ROLE_WORDS}))"
    );
    Regex::new(&format!(
        r"(?:,{JS_WS}*|([0-9]{{4}}-(?:[0-9]{{4}})?\.?){JS_WS}+)({role})(?i-u:s?)(?:{JS_WS}*,{JS_WS}*{role}(?i-u:s?))*\.?$"
    ))
    .unwrap()
});
// search-results.ts `NAME_WITH_DATES`: role-less personal name with life dates.
static NAME_WITH_DATES: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"^[^0-9,:;/()\[\]]+,[^0-9,:;/()\[\]]+(?:,{JS_WS}*|{JS_WS}+)[0-9]{{4}}-(?:[0-9]{{4}})?\.?$"
    ))
    .unwrap()
});
// search-results.ts `YEAR` — a single year "2025" or a range "2022 - 2023".
static YEAR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(r"^[0-9]{{4}}(?:{JS_WS}*[-–]{JS_WS}*[0-9]{{4}})?$")).unwrap()
});
// search-results.ts:117 (case-sensitive).
static LANGUAGE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"^[A-Z][a-z]+(?:,{JS_WS}*[A-Z][a-z]+)*$")).unwrap());
// search-results.ts:66 — `BK-tab-15974354` / `BK-pane-15974354-url`.
static CODE_FROM_ID: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?i-u)([A-Z0-9]+)-(?:tab|pane)-").unwrap());
// search-results.ts:73 — `…&RECFMT=BK`.
static CODE_FROM_QUERY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i-u)[?&]RECFMT=([A-Z0-9]+)").unwrap());
// search-results.ts setId: work-level ALLWRKENQ or edition-level ALLENQ.
static SET_ID: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"/OPAC/ALL(?:WRK)?ENQ/([0-9]+)").unwrap());
// search-results.ts `NO_RECORDS` — the OPAC's empty result, in a `div.alert` (same wording on every
// AU/NZ/US library surveyed): "Sorry, your search for Anywhere: zzqxjv (Keywords) [in the local
// catalogue] did not find any records." Tested on the collapsed text, so spaces are single.
static NO_RECORDS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i-u:did not find any records)").unwrap());
// search-results.ts:293 — "Showing 1 - 10 of 46".
static SHOWING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?i-u:showing){JS_WS}+([0-9,]+){JS_WS}*-{JS_WS}*([0-9,]+){JS_WS}+(?i-u:of){JS_WS}+([0-9,]+)"
    ))
    .unwrap()
});

#[derive(Default)]
struct FormatDraft {
    code: String,
    label: String,
    reserve_url: Option<String>,
    availability_url: Option<String>,
}

#[derive(Default)]
struct Draft {
    rank: Option<i64>,
    work_irn: Option<String>,
    title_buf: String,
    detail_url: Option<String>,
    author_url: Option<String>,
    detail_lines: Vec<String>,
    /// Per detail line: did it contain an author-search (AUTHENQ) link?
    detail_linked: Vec<bool>,
    line_buf: Option<String>,
    line_linked: bool,
    summary_buf: String,
    isbn: Option<String>,
    cover_urls: Vec<String>,
    /// Insertion-ordered, like the TS `Map`.
    formats: Vec<FormatDraft>,
    fmt_label_code: Option<String>,
    fmt_label_buf: String,
}

impl Draft {
    /// `fmtOf`: the format draft for `code`, created with `label: ""` on first use.
    fn fmt_of(&mut self, code: &str) -> &mut FormatDraft {
        let i = match self.formats.iter().position(|f| f.code == code) {
            Some(i) => i,
            None => {
                self.formats.push(FormatDraft {
                    code: code.to_owned(),
                    ..FormatDraft::default()
                });
                self.formats.len() - 1
            }
        };
        &mut self.formats[i]
    }
}

#[derive(Default)]
struct State {
    origin: String,
    /// Every draft opened so far; the index is the draft's identity (TS `current === d`).
    drafts: Vec<Draft>,
    current: Option<usize>,
    records: Vec<SearchRecord>,
    set_id: Option<String>,
    header_buf: String,
    /// Text of every `.alert` element (the no-records message lives in one).
    alert_buf: String,
    next_page_url: Option<String>,
    prev_page_url: Option<String>,
}

impl State {
    fn current(&mut self) -> Option<&mut Draft> {
        let i = self.current?;
        self.drafts.get_mut(i)
    }
}

/// JS falsiness of `string | null`: null or "".
fn is_blank(v: &Option<String>) -> bool {
    v.as_deref().is_none_or(str::is_empty)
}

/// JS `s || null`.
fn non_empty(s: String) -> Option<String> {
    (!s.is_empty()).then_some(s)
}

/// `el.onEndTag(cb)`. A void element can't take an end-tag handler; the callback is then
/// dropped, like an end tag that never arrives.
fn on_end_tag(el: &mut Element<'_, '_>, f: impl FnOnce() + 'static) {
    let _ = el.on_end_tag(Box::new(move |_: &mut EndTag<'_>| {
        f();
        Ok(())
    }));
}

/// `codeFromId`: `id="BK-pane-15974354-url"` / `id="BK-tab-15974354"` → `BK`.
fn code_from_id(id: Option<&str>) -> Option<String> {
    CODE_FROM_ID
        .captures(id?)
        .map(|c| c[1].to_ascii_uppercase())
}

/// `codeFromQuery`: `...?...&RECFMT=BK` (entity-decoded first) → `BK`.
fn code_from_query(href: Option<&str>) -> Option<String> {
    CODE_FROM_QUERY
        .captures(&decode(href?))
        .map(|c| c[1].to_ascii_uppercase())
}

/// search-results.ts `finalize` — derive author / year / language from the detail lines.
fn finalize(d: Draft, origin: &str) -> SearchRecord {
    let mut author = None;
    let mut author_role = None;
    let mut year = None;
    let mut language = None;

    let mut first = true;
    for (i, raw) in d.detail_lines.iter().enumerate() {
        let line = collapse(raw);
        if line.is_empty() {
            continue;
        }
        let is_first = std::mem::replace(&mut first, false);
        let role = AUTHOR_ROLE.captures(&line);
        if let Some(c) = &role
            && author.is_none()
        {
            author_role = Some(c[2].to_ascii_lowercase());
            let whole = c.get(0).expect("group 0");
            let dates = c.get(1).map_or("", |m| m.as_str());
            author = non_empty(js_trim(&format!("{}{dates}", &line[..whole.start()])).to_owned());
            continue;
        }
        // Guarded fallback: a role-less first line is the author only with structural evidence —
        // an author-search link, or a personal name with life dates. A bare "Haig, Matt" without a
        // link is indistinguishable from a language list ("English, French") and stays unclassified.
        if is_first
            && role.is_none()
            && !YEAR.is_match(&line)
            && (d.detail_linked.get(i).copied().unwrap_or(false) || NAME_WITH_DATES.is_match(&line))
        {
            author = Some(line);
            continue;
        }
        if year.is_none() && YEAR.is_match(&line) {
            year = Some(line);
            continue;
        }
        if language.is_none() && LANGUAGE.is_match(&line) && role.is_none() {
            language = Some(line);
        }
    }

    let formats = d
        .formats
        .into_iter()
        .map(|f| RecordFormat {
            label: non_empty(collapse(&f.label)),
            reserve_url: absolute(origin, f.reserve_url.as_deref()),
            availability_url: absolute(origin, f.availability_url.as_deref()),
            code: f.code,
        })
        .collect();

    SearchRecord {
        rank: d.rank,
        work_irn: d.work_irn,
        title: non_empty(collapse(&d.title_buf)),
        detail_url: absolute(origin, d.detail_url.as_deref()),
        author,
        author_role,
        author_url: absolute(origin, d.author_url.as_deref()),
        year,
        language,
        summary: non_empty(collapse(&d.summary_buf)),
        isbn: d.isbn,
        cover_urls: d.cover_urls,
        formats,
    }
}

/// `parseSearchResults(html, { origin, query })`.
pub fn parse_search_results(html: &str, origin: &str, query: Option<&str>) -> SearchResults {
    let st = Rc::new(RefCell::new(State {
        origin: origin.to_owned(),
        ..State::default()
    }));

    let settings = RewriteStrSettings::new()
        // Keep parsing through ambiguous markup instead of failing the whole page.
        .with_strict(false)
        // ---- pagination: Next / Previous links ----
        .append_element_content_handler(element!(".result-pages-prvnxt .nxt a", |el| {
            let mut s = st.borrow_mut();
            if is_blank(&s.next_page_url) {
                s.next_page_url = el.get_attribute("href");
            }
            Ok(())
        }))
        .append_element_content_handler(element!(".result-pages-prvnxt .prv a", |el| {
            let mut s = st.borrow_mut();
            if is_blank(&s.prev_page_url) {
                s.prev_page_url = el.get_attribute("href");
            }
            Ok(())
        }))
        // ---- page-level: result count header ("Showing 1 - 10 of 46") ----
        .append_element_content_handler(text!(".result-header-brief", |t| {
            st.borrow_mut().header_buf.push_str(t.as_str());
            Ok(())
        }))
        // ---- page-level: setId from the first ALLWRKENQ (work) or ALLENQ (edition) link ----
        .append_element_content_handler(element!(r#"a[href*="/OPAC/ALL"]"#, |el| {
            let mut s = st.borrow_mut();
            if s.set_id.is_none() {
                let href = decode(&el.get_attribute("href").unwrap_or_default());
                s.set_id = SET_ID.captures(&href).map(|c| c[1].to_owned());
            }
            Ok(())
        }))
        // ---- record boundary ----
        .append_element_content_handler(element!("fieldset.card-list", |el| {
            let i = {
                let mut s = st.borrow_mut();
                s.drafts.push(Draft::default());
                let i = s.drafts.len() - 1;
                s.current = Some(i);
                i
            };
            let st = Rc::clone(&st);
            on_end_tag(el, move || {
                let mut s = st.borrow_mut();
                // Never written again: `current` only ever moves on to newer drafts.
                let d = std::mem::take(&mut s.drafts[i]);
                let record = finalize(d, &s.origin);
                s.records.push(record);
                if s.current == Some(i) {
                    s.current = None;
                }
            });
            Ok(())
        }))
        .append_element_content_handler(text!("fieldset.card-list span.item-num", |t| {
            if let Some(d) = st.borrow_mut().current()
                && d.rank.is_none()
            {
                d.rank = to_int(t.as_str());
            }
            Ok(())
        }))
        .append_element_content_handler(element!(r#"fieldset.card-list input[name="SVL"]"#, |el| {
            if let Some(d) = st.borrow_mut().current()
                && is_blank(&d.work_irn)
            {
                d.work_irn = el.get_attribute("value");
            }
            Ok(())
        }))
        // ---- title ----
        .append_element_content_handler(element!("fieldset.card-list h3.card-title a", |el| {
            if let Some(d) = st.borrow_mut().current()
                && is_blank(&d.detail_url)
            {
                d.detail_url = el.get_attribute("href");
            }
            Ok(())
        }))
        .append_element_content_handler(text!("fieldset.card-list h3.card-title a", |t| {
            if let Some(d) = st.borrow_mut().current() {
                d.title_buf.push_str(t.as_str());
            }
            Ok(())
        }))
        // ---- author / year / language lines ----
        .append_element_content_handler(element!("fieldset.card-list div.recdetails a", |el| {
            if let Some(d) = st.borrow_mut().current()
                && is_blank(&d.author_url)
            {
                d.author_url = el.get_attribute("href");
            }
            Ok(())
        }))
        .append_element_content_handler(element!(
            "fieldset.card-list div.recdetails > span.d-block",
            |el| {
                if let Some(d) = st.borrow_mut().current() {
                    d.line_buf = Some(String::new());
                    d.line_linked = false;
                }
                let st = Rc::clone(&st);
                on_end_tag(el, move || {
                    if let Some(d) = st.borrow_mut().current()
                        && let Some(line) = d.line_buf.take()
                    {
                        d.detail_lines.push(line);
                        d.detail_linked.push(d.line_linked);
                    }
                });
                Ok(())
            }
        ))
        .append_element_content_handler(text!(
            "fieldset.card-list div.recdetails > span.d-block",
            |t| {
                if let Some(d) = st.borrow_mut().current()
                    && let Some(buf) = d.line_buf.as_mut()
                {
                    buf.push_str(t.as_str());
                }
                Ok(())
            }
        ))
        .append_element_content_handler(element!(
            r#"fieldset.card-list div.recdetails > span.d-block a[href*="/AUTHENQ/"]"#,
            |_el| {
                if let Some(d) = st.borrow_mut().current()
                    && d.line_buf.is_some()
                {
                    d.line_linked = true;
                }
                Ok(())
            }
        ))
        // ---- summary / blurb ----
        .append_element_content_handler(text!("fieldset.card-list div.summary .text-html", |t| {
            if let Some(d) = st.borrow_mut().current() {
                d.summary_buf.push_str(t.as_str());
            }
            Ok(())
        }))
        // ---- cover image + ISBN ----
        .append_element_content_handler(element!("fieldset.card-list img.imgsc", |el| {
            if let Some(d) = st.borrow_mut().current() {
                let covers = parse_covers(el.get_attribute("data-imgurls").as_deref());
                if !covers.covers.is_empty() {
                    d.cover_urls = covers.covers;
                }
                if covers.isbn.is_some() {
                    d.isbn = covers.isbn;
                }
            }
            Ok(())
        }))
        // ---- formats: tab labels ----
        .append_element_content_handler(element!(
            "fieldset.card-list .card-body-tabs .nav-link",
            |el| {
                let mut s = st.borrow_mut();
                let Some(d) = s.current() else { return Ok(()) };
                d.fmt_label_code = code_from_id(el.get_attribute("id").as_deref());
                d.fmt_label_buf.clear();
                let st = Rc::clone(&st);
                on_end_tag(el, move || {
                    if let Some(d) = st.borrow_mut().current()
                        && let Some(code) = d.fmt_label_code.take()
                    {
                        let label = d.fmt_label_buf.clone();
                        d.fmt_of(&code).label = label;
                    }
                });
                Ok(())
            }
        ))
        .append_element_content_handler(text!(
            "fieldset.card-list .card-body-tabs .nav-link",
            |t| {
                if let Some(d) = st.borrow_mut().current()
                    && d.fmt_label_code.is_some()
                {
                    d.fmt_label_buf.push_str(t.as_str());
                }
                Ok(())
            }
        ))
        // ---- formats: reservation link ----
        .append_element_content_handler(element!(
            r#"fieldset.card-list .tab-pane a[href*="/PGM/OPAC/CCOPT/"]"#,
            |el| {
                if let Some(d) = st.borrow_mut().current() {
                    let href = el.get_attribute("href");
                    if let Some(code) = code_from_query(href.as_deref()) {
                        d.fmt_of(&code).reserve_url = href;
                    }
                }
                Ok(())
            }
        ))
        // ---- formats: lazy availability fragment url ----
        .append_element_content_handler(element!("fieldset.card-list .tab-pane-url", |el| {
            if let Some(d) = st.borrow_mut().current()
                && let Some(code) = code_from_id(el.get_attribute("id").as_deref())
                && let Some(href) = el.get_attribute("data-tab-href").filter(|h| !h.is_empty())
            {
                d.fmt_of(&code).availability_url = Some(href);
            }
            Ok(())
        }))
        // ---- page-level: alert boxes (the "did not find any records" message lives in one) ----
        .append_element_content_handler(text!(".alert", |t| {
            st.borrow_mut().alert_buf.push_str(t.as_str());
            Ok(())
        }));

    // No handler returns an error and non-strict mode never bails out, so this can't fail;
    // the rewritten output itself is unused.
    let _ = rewrite_str(html, settings);
    let s = st.take();

    let header = collapse(&s.header_buf);
    let showing_match = SHOWING.captures(&header);
    // TS asserts both groups parse (`toInt(...)!`); a group of bare commas would give it
    // `{from: null}`, which `Showing` can't hold, so that case yields no `showing` at all.
    let showing = showing_match.as_ref().and_then(|m| {
        Some(Showing {
            from: to_int(&m[1])?,
            to: to_int(&m[2])?,
        })
    });
    let total_results = match &showing_match {
        Some(m) => to_int(&m[3]),
        None => i64::try_from(s.records.len()).ok().filter(|&n| n > 0),
    };

    SearchResults {
        query: query.map(str::to_owned),
        set_id: s.set_id,
        total_results,
        showing,
        next_page_url: absolute(origin, s.next_page_url.as_deref()),
        prev_page_url: absolute(origin, s.prev_page_url.as_deref()),
        no_results: s.records.is_empty() && NO_RECORDS.is_match(&collapse(&s.alert_buf)),
        records: s.records,
    }
}
