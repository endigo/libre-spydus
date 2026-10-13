//! Port of `middleware/src/parse/record.ts`: the BIBENQ bib-record page → [`RecordDetail`] (the
//! OPAC "Record details" block, the cover, the annotation and the lazy related-title panel URLs),
//! plus the brief cards inside a related-titles fragment (similar / by-author / in-series).
//!
//! Like the TS `HTMLRewriter`, the parsers stream the HTML through `lol_html` and keep their
//! drafts in `Rc<RefCell<…>>` state shared with the (`'static`) end-tag handlers. Handlers are
//! registered in the same order as the TS `.on(…)` calls, because that order decides which
//! handler sees an element first.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::LazyLock;

use lol_html::html_content::Element;
use lol_html::{RewriteStrSettings, element, end_tag, rewrite_str, text};
use regex::Regex;

use crate::parse::decode::{JS_WS, absolute, collapse, decode, js_trim, parse_covers};
use crate::types::{DetailRow, DetailValue, RecordDetail, RecordRelated, RelatedItem};

// record.ts:36 — `/href="([^"]*\/BIBENQ\/\d+\?[^"]*IRN[=(](\d+)[^"]*)"/`.
static BIB_LINK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"href="([^"]*/BIBENQ/[0-9]+\?[^"]*IRN[=(]([0-9]+)[^"]*)""#).unwrap()
});
// record.ts:72 — `/:\s*$/` (first match only).
static LABEL_COLON: LazyLock<Regex> = LazyLock::new(|| Regex::new(&format!(":{JS_WS}*$")).unwrap());
// record.ts:148 — `/\d{9,13}[Xx]?/`.
static ISBN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[0-9]{9,13}[Xx]?").unwrap());
// record.ts:151,203 — `/BRN=(\d+)/`.
static BRN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"BRN=([0-9]+)").unwrap());
// record.ts:203 — `/IRN[=(](\d+)/`.
static IRN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"IRN[=(]([0-9]+)").unwrap());
// record.ts:154 — `/^(loading\.?\.?\.?)?$/i`.
static LOADING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?i-u)(?:loading\.?\.?\.?)?$").unwrap());
// record.ts:200 — `/^\d{4}(\s*[-–]\s*\d{4})?$/`.
static YEAR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!("^[0-9]{{4}}(?:{JS_WS}*[-–]{JS_WS}*[0-9]{{4}})?$")).unwrap()
});

/// The concrete bib record a format fragment links to (`bibLinkFromFragment`).
#[derive(serde::Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BibLink {
    pub bib_url: String,
    pub bib_irn: String,
}

/// `bibLinkFromFragment(html, origin)` — regex over the entity-decoded fragment HTML.
/// record.ts:32-40.
pub fn bib_link_from_fragment(html: &str, origin: &str) -> Option<BibLink> {
    let decoded = decode(html);
    let caps = BIB_LINK.captures(&decoded)?;
    let bib_url = absolute(origin, Some(&caps[1]))?;
    Some(BibLink { bib_url, bib_irn: caps[2].to_owned() })
}

/// `lol_html` settings matching Bun's `HTMLRewriter` (non-strict, no ESI tags).
fn settings<'h, 's>() -> RewriteStrSettings<'h, 's> {
    RewriteStrSettings::new().with_strict(false).with_enable_esi_tags(false)
}

/// `el.onEndTag(handler)`. Void elements can't have one; their handler is dropped.
fn on_end_tag(el: &mut Element<'_, '_>, mut handler: impl FnMut() + 'static) {
    if let Some(handlers) = el.end_tag_handlers() {
        handlers.push(end_tag!(move |_| {
            handler();
            Ok(())
        }));
    }
}

// ---------------------------------------------------------------------------
// Full bib record → RecordDetail
// ---------------------------------------------------------------------------

/// One labelled row being collected (record.ts:46-51).
#[derive(Default)]
struct RowDraft {
    label: String,
    values: Vec<DetailValue>,
    val_buf: Option<String>,
    val_href: Option<String>,
}

impl RowDraft {
    /// Push the buffered value line, if it has text, with its first link.
    fn flush_value(&mut self, origin: &str) {
        if let Some(buf) = &self.val_buf {
            let text = collapse(buf);
            if !text.is_empty() {
                let url = absolute(origin, self.val_href.as_deref());
                self.values.push(DetailValue { text, url });
            }
        }
    }
}

#[derive(Default)]
struct DetailState {
    details: Vec<DetailRow>,
    /// The open row; a nested `.row` replaces it and clears it on its end tag.
    row: Option<Rc<RefCell<RowDraft>>>,
    covers: Vec<String>,
    isbn_from_cover: Option<String>,
    related: HashMap<String, String>,
    summary_buf: Option<String>,
}

impl DetailState {
    fn row(&self) -> Option<Rc<RefCell<RowDraft>>> {
        self.row.clone()
    }
}

/// `parseRecordDetail(html, origin, bibIrn)`. record.ts:53-173.
pub fn parse_record_detail(html: &str, origin: &str, bib_irn: Option<String>) -> RecordDetail {
    let state = Rc::new(RefCell::new(DetailState::default()));
    let origin_rc: Rc<str> = Rc::from(origin);

    let rewrite = settings()
        // One labelled row of the Record-details block.
        .append_element_content_handler(element!("#divtabRECDETAILS .row", |el| {
            let draft = Rc::new(RefCell::new(RowDraft::default()));
            state.borrow_mut().row = Some(Rc::clone(&draft));
            let (state, origin) = (Rc::clone(&state), Rc::clone(&origin_rc));
            on_end_tag(el, move || {
                let mut d = draft.borrow_mut();
                d.flush_value(&origin);
                let label = LABEL_COLON.replace(&collapse(&d.label), "").into_owned();
                let mut st = state.borrow_mut();
                if !label.is_empty() && !d.values.is_empty() {
                    st.details.push(DetailRow { label, values: std::mem::take(&mut d.values) });
                }
                if st.row.as_ref().is_some_and(|r| Rc::ptr_eq(r, &draft)) {
                    st.row = None;
                }
            });
            Ok(())
        }))
        .append_element_content_handler(text!("#divtabRECDETAILS .fd-caption", |t| {
            if let Some(row) = state.borrow().row() {
                row.borrow_mut().label.push_str(t.as_str());
            }
            Ok(())
        }))
        // Each value line is a <span class="d-block">; collect its text + first link. Flushing on
        // open AND on end keeps both sibling and nested spans as separate values.
        .append_element_content_handler(element!("#divtabRECDETAILS .col span.d-block", |el| {
            let Some(row) = state.borrow().row() else { return Ok(()) };
            {
                let mut r = row.borrow_mut();
                r.flush_value(origin);
                r.val_buf = Some(String::new());
                r.val_href = None;
            }
            let (state, origin) = (Rc::clone(&state), Rc::clone(&origin_rc));
            on_end_tag(el, move || {
                // The row open at the end tag, which need not be the one open at the start tag.
                let Some(row) = state.borrow().row() else { return };
                let mut r = row.borrow_mut();
                if r.val_buf.is_some() {
                    r.flush_value(&origin);
                    r.val_buf = None;
                    r.val_href = None;
                }
            });
            Ok(())
        }))
        .append_element_content_handler(text!("#divtabRECDETAILS .col span.d-block", |t| {
            if let Some(row) = state.borrow().row()
                && let Some(buf) = &mut row.borrow_mut().val_buf
            {
                buf.push_str(t.as_str());
            }
            Ok(())
        }))
        .append_element_content_handler(element!(
            "#divtabRECDETAILS .col span.d-block a[href]",
            |el| {
                if let Some(row) = state.borrow().row() {
                    let mut r = row.borrow_mut();
                    // `!row.valHref`: an empty href is falsy, so a later link replaces it.
                    if r.val_buf.is_some() && r.val_href.as_deref().is_none_or(str::is_empty) {
                        r.val_href = el.get_attribute("href");
                    }
                }
                Ok(())
            }
        ))
        // Main cover: the FIRST imgsc carrying real cover URLs wins.
        .append_element_content_handler(element!("img.imgsc", |el| {
            let mut st = state.borrow_mut();
            if !st.covers.is_empty() {
                return Ok(());
            }
            let c = parse_covers(el.get_attribute("data-imgurls").as_deref());
            if !c.covers.is_empty() {
                st.covers = c.covers;
                if c.isbn.is_some() {
                    st.isbn_from_cover = c.isbn;
                }
            }
            Ok(())
        }))
        // Lazy related-panel launch URLs: <a id="urltabSIMILAR" href="...">.
        .append_element_content_handler(element!("a[id]", |el| {
            let id = el.get_attribute("id").unwrap_or_default();
            if let Some(rest) = id.strip_prefix("urltab")
                && let Some(href) = el.get_attribute("href").filter(|h| !h.is_empty())
            {
                state.borrow_mut().related.entry(rest.to_lowercase()).or_insert(href);
            }
            Ok(())
        }))
        // Inline annotation/summary, when the page renders it (not lazy).
        .append_element_content_handler(element!("#divtabSUMMARY", |_| {
            state.borrow_mut().summary_buf.get_or_insert_with(String::new);
            Ok(())
        }))
        .append_element_content_handler(text!("#divtabSUMMARY", |t| {
            if let Some(buf) = &mut state.borrow_mut().summary_buf {
                buf.push_str(t.as_str());
            }
            Ok(())
        }));
    // The output is discarded. Handlers never fail and non-strict parsing has no ambiguity
    // errors, so keep whatever was collected even if lol_html reports one.
    let _ = rewrite_str(html, rewrite);

    let st = state.take();
    let find = |matches: fn(&str) -> bool| st.details.iter().find(|d| matches(&d.label));
    let first_text =
        |matches: fn(&str) -> bool| find(matches).and_then(|d| d.values.first()).map(|v| &v.text);

    let title = first_text(|l| l.eq_ignore_ascii_case("main title"))
        .map(|t| js_trim(t.split(" / ").next().unwrap_or_default()).to_owned());
    let series_title = first_text(|l| starts_with_ignore_ascii_case(l, "series")).cloned();
    let isbn_row = first_text(|l| l.eq_ignore_ascii_case("isbn"))
        .and_then(|t| ISBN.find(t))
        .map(|m| m.as_str().to_owned());
    let brn_row = first_text(|l| l.eq_ignore_ascii_case("brn")).cloned();
    let bookmark_row = find(|l| starts_with_ignore_ascii_case(l, "bookmark"))
        .and_then(|d| d.values.first())
        .and_then(|v| v.url.clone());
    let brn = brn_row.or_else(|| {
        let url = bookmark_row.as_deref()?;
        Some(BRN.captures(url)?[1].to_owned())
    });

    let mut summary = st.summary_buf.as_deref().map(collapse);
    // `summary && /^(loading…)?$/i.test(summary)`: an empty summary is falsy and stays "".
    if summary.as_deref().is_some_and(|s| !s.is_empty() && LOADING.is_match(s)) {
        summary = None;
    }

    let synthesized = brn
        .as_deref()
        .filter(|b| !b.is_empty())
        .map(|b| format!("/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&BRN={b}"));
    let bookmark_url = absolute(origin, bookmark_row.or(synthesized).as_deref());
    let related = |key: &str| absolute(origin, st.related.get(key).map(String::as_str));

    RecordDetail {
        bib_irn,
        brn,
        title,
        series_title,
        isbn: isbn_row.or(st.isbn_from_cover),
        cover_urls: st.covers,
        summary,
        bookmark_url,
        related: RecordRelated {
            similar: related("similar"),
            author: related("author"),
            series: related("series"),
            // The panel id is `urltabVERSION` (singular).
            versions: related("version"),
        },
        details: st.details,
    }
}

/// JS `/^prefix/i` for an ASCII `prefix`.
fn starts_with_ignore_ascii_case(s: &str, prefix: &str) -> bool {
    s.as_bytes()
        .get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix.as_bytes()))
}

// ---------------------------------------------------------------------------
// Related-titles fragment → brief cards
// ---------------------------------------------------------------------------

/// One brief card being collected (record.ts:179-190).
#[derive(Default)]
struct CardDraft {
    bib_url: Option<String>,
    title_buf: String,
    cover_urls: Vec<String>,
    isbn: Option<String>,
    line_buf: Option<String>,
    detail_lines: Vec<String>,
}

/// `finalizeCard`: a card needs a title and a bib URL. record.ts:192-205.
fn finalize_card(d: &CardDraft, origin: &str) -> Option<RelatedItem> {
    let title = collapse(&d.title_buf);
    let bib_url = d.bib_url.as_deref().filter(|u| !u.is_empty())?;
    if title.is_empty() {
        return None;
    }
    let mut author = None;
    let mut year = None;
    for raw in &d.detail_lines {
        let line = collapse(raw);
        if line.is_empty() {
            continue;
        }
        if year.is_none() && YEAR.is_match(&line) {
            year = Some(line);
        } else if author.is_none() {
            author = Some(line);
        }
    }
    let brn = BRN.captures(bib_url).or_else(|| IRN.captures(bib_url)).map(|c| c[1].to_owned());
    Some(RelatedItem {
        title: Some(title),
        bib_url: absolute(origin, Some(bib_url)),
        brn,
        author,
        year,
        isbn: d.isbn.clone(),
        cover_urls: d.cover_urls.clone(),
    })
}

#[derive(Default)]
struct CardsState {
    items: Vec<RelatedItem>,
    current: Option<CardDraft>,
}

impl CardsState {
    fn flush(&mut self, origin: &str) {
        if let Some(item) = self.current.as_ref().and_then(|c| finalize_card(c, origin)) {
            self.items.push(item);
        }
    }
}

/// `parseBriefCards(html, origin)`. record.ts:207-265.
pub fn parse_brief_cards(html: &str, origin: &str) -> Vec<RelatedItem> {
    let state = Rc::new(RefCell::new(CardsState::default()));

    let rewrite = settings()
        // Each card opens with its cover link; use it as the card boundary.
        .append_element_content_handler(element!("a.thumblink", |el| {
            let mut st = state.borrow_mut();
            st.flush(origin);
            st.current =
                Some(CardDraft { bib_url: el.get_attribute("href"), ..CardDraft::default() });
            Ok(())
        }))
        .append_element_content_handler(element!("span.card-title a", |el| {
            let href = el.get_attribute("href");
            let mut st = state.borrow_mut();
            match st.current.as_mut() {
                None => st.current = Some(CardDraft { bib_url: href, ..CardDraft::default() }),
                Some(card) if card.bib_url.as_deref().is_none_or(str::is_empty) => {
                    card.bib_url = href;
                }
                Some(_) => {}
            }
            Ok(())
        }))
        .append_element_content_handler(text!("span.card-title a", |t| {
            if let Some(card) = &mut state.borrow_mut().current {
                card.title_buf.push_str(t.as_str());
            }
            Ok(())
        }))
        // Unlike the record page, the LAST imgsc with covers wins within a card.
        .append_element_content_handler(element!("img.imgsc", |el| {
            if let Some(card) = &mut state.borrow_mut().current {
                let c = parse_covers(el.get_attribute("data-imgurls").as_deref());
                if !c.covers.is_empty() {
                    card.cover_urls = c.covers;
                }
                if c.isbn.is_some() {
                    card.isbn = c.isbn;
                }
            }
            Ok(())
        }))
        .append_element_content_handler(element!(".card-text.recdetails span.d-block", |el| {
            if let Some(card) = &mut state.borrow_mut().current {
                card.line_buf = Some(String::new());
            }
            let state = Rc::clone(&state);
            on_end_tag(el, move || {
                if let Some(card) = &mut state.borrow_mut().current
                    && let Some(line) = card.line_buf.take()
                {
                    card.detail_lines.push(line);
                }
            });
            Ok(())
        }))
        .append_element_content_handler(text!(".card-text.recdetails span.d-block", |t| {
            if let Some(card) = &mut state.borrow_mut().current
                && let Some(buf) = &mut card.line_buf
            {
                buf.push_str(t.as_str());
            }
            Ok(())
        }));
    // See parse_record_detail: keep what was collected even on a rewriter error.
    let _ = rewrite_str(html, rewrite);

    let mut st = state.take();
    st.flush(origin);
    st.items
}
