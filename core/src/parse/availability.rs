//! Port of `middleware/src/parse/availability.ts` (RECFMT fragment). Owner: task B.
//!
//! This is the per-record-per-format fragment the results WebView lazy-loads
//! (`/ENQ/OPAC/BIBENQ/{setId}?...&RECFMT=BK`). It carries the live availability counts, the
//! publisher line, and the links to the branch-level holdings modal. Same single streaming
//! `lol_html` pass as the TS `HTMLRewriter`, with handlers registered in the TS order.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::LazyLock;

use lol_html::html_content::{Element, EndTag};
use lol_html::{RewriteStrSettings, element, rewrite_str, text};
use regex::Regex;

use crate::parse::decode::{JS_WS, absolute, collapse, to_int};
use crate::types::Availability;

// availability.ts:88 / :99. JS `\w` and `\d` are ASCII-only.
static XHLD_IRN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"/XHLD/OPAC/[A-Za-z0-9_]+/[0-9]+/([0-9]+)").unwrap());
static XFULL_IRN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"/XFULL/OPAC/[A-Za-z0-9_]+/[0-9]+/([0-9]+)").unwrap());
// availability.ts:111 — an imprint's " : " (place : publisher) separator, and a 4-digit year.
static IMPRINT_SEP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!("{JS_WS}:{JS_WS}")).unwrap());
static YEAR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[0-9]{4}").unwrap());

#[derive(Default)]
struct State {
    counts: HashMap<String, i64>,
    pending_label: Option<String>,
    label_buf: String,
    val_buf: String,

    title_buf: String,
    title_done: bool,
    span_candidates: Vec<String>,
    span_buf: Option<String>,

    holdings_url: Option<String>,
    record_url: Option<String>,
    bib_record_irn: Option<String>,
}

/// JS falsiness of `string | null`: null or "".
fn is_blank(v: &Option<String>) -> bool {
    v.as_deref().is_none_or(str::is_empty)
}

/// `el.onEndTag(cb)`. A void element can't take an end-tag handler; the callback is then
/// dropped, like an end tag that never arrives.
fn on_end_tag(el: &mut Element<'_, '_>, f: impl FnOnce() + 'static) {
    let _ = el.on_end_tag(Box::new(move |_: &mut EndTag<'_>| {
        f();
        Ok(())
    }));
}

/// `parseAvailability(html, { origin })`.
pub fn parse_availability(html: &str, origin: &str) -> Availability {
    let st = Rc::new(RefCell::new(State::default()));

    let settings = RewriteStrSettings::new()
        // Keep parsing through ambiguous markup instead of failing the whole fragment.
        .with_strict(false)
        // availability label → value pairs (Reserved: 1, Copies: 3, ...)
        .append_element_content_handler(element!("span.availability-text", |el| {
            st.borrow_mut().label_buf.clear();
            let st = Rc::clone(&st);
            on_end_tag(el, move || {
                let mut s = st.borrow_mut();
                let label = collapse(&s.label_buf);
                let label = label.strip_suffix(':').unwrap_or(&label).to_lowercase();
                s.pending_label = (!label.is_empty()).then_some(label);
            });
            Ok(())
        }))
        .append_element_content_handler(text!("span.availability-text", |t| {
            st.borrow_mut().label_buf.push_str(t.as_str());
            Ok(())
        }))
        .append_element_content_handler(element!("span.availability-val", |el| {
            st.borrow_mut().val_buf.clear();
            let st = Rc::clone(&st);
            on_end_tag(el, move || {
                let mut s = st.borrow_mut();
                let n = to_int(&s.val_buf);
                if let Some(label) = s.pending_label.take()
                    && let Some(n) = n
                {
                    s.counts.insert(label, n);
                }
            });
            Ok(())
        }))
        .append_element_content_handler(text!("span.availability-val", |t| {
            st.borrow_mut().val_buf.push_str(t.as_str());
            Ok(())
        }))
        // record title — first title cell of the stacked table
        .append_element_content_handler(element!(r#"td[data-caption="Title"] a"#, |el| {
            let st = Rc::clone(&st);
            on_end_tag(el, move || {
                let mut s = st.borrow_mut();
                if !collapse(&s.title_buf).is_empty() {
                    s.title_done = true;
                }
            });
            Ok(())
        }))
        .append_element_content_handler(text!(r#"td[data-caption="Title"] a"#, |t| {
            let mut s = st.borrow_mut();
            if !s.title_done {
                s.title_buf.push_str(t.as_str());
            }
            Ok(())
        }))
        // imprint candidates — bare spans inside a column; pick the publisher line after
        .append_element_content_handler(element!(".recfmt-container .col > span", |el| {
            st.borrow_mut().span_buf = Some(String::new());
            let st = Rc::clone(&st);
            on_end_tag(el, move || {
                let mut s = st.borrow_mut();
                let candidate = collapse(s.span_buf.as_deref().unwrap_or_default());
                if !candidate.is_empty() {
                    s.span_candidates.push(candidate);
                }
                s.span_buf = None;
            });
            Ok(())
        }))
        .append_element_content_handler(text!(".recfmt-container .col > span", |t| {
            if let Some(buf) = st.borrow_mut().span_buf.as_mut() {
                buf.push_str(t.as_str());
            }
            Ok(())
        }))
        // branch-level holdings modal (XHLD) and full record (XFULL)
        .append_element_content_handler(element!(r#"a[href*="/XHLD/OPAC/"]"#, |el| {
            let mut s = st.borrow_mut();
            if let Some(href) = el.get_attribute("href").filter(|h| !h.is_empty())
                && is_blank(&s.holdings_url)
            {
                // Unconditional, as in TS: replaces an IRN an earlier XFULL link supplied.
                if let Some(m) = XHLD_IRN.captures(&href) {
                    s.bib_record_irn = Some(m[1].to_owned());
                }
                s.holdings_url = Some(href);
            }
            Ok(())
        }))
        .append_element_content_handler(element!(r#"a[href*="/XFULL/OPAC/"]"#, |el| {
            let mut s = st.borrow_mut();
            if let Some(href) = el.get_attribute("href").filter(|h| !h.is_empty())
                && is_blank(&s.record_url)
            {
                if is_blank(&s.bib_record_irn)
                    && let Some(m) = XFULL_IRN.captures(&href)
                {
                    s.bib_record_irn = Some(m[1].to_owned());
                }
                s.record_url = Some(href);
            }
            Ok(())
        }));

    // No handler returns an error and non-strict mode never bails out, so this can't fail;
    // the rewritten output itself is unused.
    let _ = rewrite_str(html, settings);
    let s = st.take();

    // The publisher line is the span that looks like an imprint: a " : " separator
    // (place : publisher) and/or a 4-digit year. Pick the first such candidate.
    let publisher = s
        .span_candidates
        .iter()
        .find(|c| IMPRINT_SEP.is_match(c) && YEAR.is_match(c))
        .or_else(|| s.span_candidates.iter().find(|c| IMPRINT_SEP.is_match(c)))
        .cloned();

    Availability {
        bib_record_irn: s.bib_record_irn,
        title: Some(collapse(&s.title_buf)).filter(|t| !t.is_empty()),
        publisher,
        reserved: s.counts.get("reserved").copied(),
        copies: s.counts.get("copies").copied(),
        counts: s.counts,
        holdings_url: absolute(origin, s.holdings_url.as_deref()),
        record_url: absolute(origin, s.record_url.as_deref()),
    }
}
