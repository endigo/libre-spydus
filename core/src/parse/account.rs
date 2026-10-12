//! Port of `middleware/src/parse/account.ts`: authenticated OPAC account pages → typed records.
//!   `parse_account_links` — the BRWENQ "My details" page → loans/reservations links
//!   `parse_patron_items`  — a LOANRENQ / RSVCENQ page → list of loans/reservations
//!
//! Loans and reservations share one table-row layout, so a single parser handles both; callers
//! interpret `fields` ("Due" for loans, "Status" for reservations) and `actions`.
//! Engine: `lol_html` (what Bun's HTMLRewriter wraps). Handlers run in document order and share
//! state through `Rc<RefCell<…>>`; end-tag handlers must be `'static`, so they own `Rc` clones.

use crate::parse::decode::{JS_WS, absolute, collapse, decode, parse_covers, to_int};
use crate::types::{AccountLinks, PatronAction, PatronItem, PatronList};
use lol_html::{
    ElementContentHandlers, RewriteStrSettings, Selector, element, end_tag, rewrite_str, text,
};
use regex::Regex;
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::LazyLock;

static FMT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[?&]FMT=([A-Z]+)").unwrap());
static YEAR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(r"^[0-9]{{4}}({JS_WS}*[-–]{JS_WS}*[0-9]{{4}})?$")).unwrap()
});
static BIB_IRN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"IRN[=(]([0-9]+)").unwrap());
static HEADER_TOTAL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"(?i-u:of){JS_WS}+([0-9,]+)")).unwrap());

/// `RewriteStrSettings` with `handlers` appended in order (the order Bun's `.on()` calls ran).
fn settings<'h>(
    handlers: Vec<(Cow<'static, Selector>, ElementContentHandlers<'h>)>,
) -> RewriteStrSettings<'h, 'static> {
    handlers
        .into_iter()
        .fold(RewriteStrSettings::new(), |s, h| {
            s.append_element_content_handler(h)
        })
}

const NAME_SELECTOR: &str = ".navbar .dropdown-toggle-account, .account-name, #accountDropdown";

// ---------------------------------------------------------------------------
// Account page → navigation links (account.ts:16-60)
// ---------------------------------------------------------------------------

#[derive(Default)]
struct Links {
    loans: Option<String>,
    ready: Option<String>,
    pending: Option<String>,
}

pub fn parse_account_links(html: &str, origin: &str) -> AccountLinks {
    let links = RefCell::new(Links::default());
    // `name` is only assigned after the rewrite in TS, so every matching element resets the buffer.
    let name_buf: RefCell<Option<String>> = RefCell::new(None);

    let _ = rewrite_str(
        html,
        settings(vec![
            element!("a[href]", |el| {
                let href = decode(&el.get_attribute("href").unwrap_or_default());
                if !href.contains("/OPAC/") {
                    return Ok(());
                }
                let fmt = FMT.captures(&href).map(|c| c[1].to_owned());
                let fmt = fmt.as_deref();
                let mut l = links.borrow_mut();
                if href.contains("/LOANRENQ/") && fmt == Some("CL") && l.loans.is_none() {
                    l.loans = Some(href);
                } else if href.contains("/RSVCENQ/") && fmt == Some("AR") && l.ready.is_none() {
                    l.ready = Some(href);
                } else if href.contains("/RSVCENQ/") && fmt == Some("WR") && l.pending.is_none() {
                    l.pending = Some(href);
                }
                Ok(())
            }),
            // Logged-in borrower name — the navbar account dropdown toggle.
            element!(NAME_SELECTOR, |_el| {
                *name_buf.borrow_mut() = Some(String::new());
                Ok(())
            }),
            text!(NAME_SELECTOR, |t| {
                if let Some(buf) = name_buf.borrow_mut().as_mut() {
                    buf.push_str(t.as_str());
                }
                Ok(())
            }),
        ]),
    );

    let name = name_buf
        .into_inner()
        .map(|b| collapse(&b))
        .filter(|n| !n.is_empty());
    let l = links.into_inner();
    AccountLinks {
        name,
        loans_url: absolute(origin, l.loans.as_deref()),
        reservations_ready_url: absolute(origin, l.ready.as_deref()),
        reservations_pending_url: absolute(origin, l.pending.as_deref()),
    }
}

// ---------------------------------------------------------------------------
// Loan / reservation list → items (account.ts:66-247)
// ---------------------------------------------------------------------------

#[derive(Default)]
struct Draft {
    irn: Option<String>,
    title_buf: String,
    bib_url: Option<String>,
    detail_lines: Vec<String>,
    line_buf: Option<String>,
    cover_urls: Vec<String>,
    isbn: Option<String>,
    fields: HashMap<String, String>,
    cap_caption: Option<String>,
    cap_buf: String,
    actions: Vec<PatronAction>,
    act_buf: Option<String>,
    act_href: Option<String>,
}

type DraftRef = Rc<RefCell<Draft>>;

#[derive(Default)]
struct ListState {
    current: Option<DraftRef>,
    items: Vec<PatronItem>,
}

/// JS truthiness of an optional string.
fn truthy(s: &Option<String>) -> bool {
    s.as_deref().is_some_and(|v| !v.is_empty())
}

fn finalize(d: &Draft, origin: &str) -> PatronItem {
    let mut author: Option<String> = None;
    let mut year: Option<String> = None;
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
    // The bib-record IRN lives inside the title link, as QRY=IRN(nnn) or IRN=nnn.
    let bib_irn = d
        .bib_url
        .as_deref()
        .and_then(|u| BIB_IRN.captures(u))
        .map(|c| c[1].to_owned());
    let title = collapse(&d.title_buf);
    PatronItem {
        irn: d.irn.clone(),
        bib_irn,
        title: (!title.is_empty()).then_some(title),
        bib_url: absolute(origin, d.bib_url.as_deref()),
        author,
        year,
        cover_urls: d.cover_urls.clone(),
        isbn: d.isbn.clone(),
        fields: d.fields.clone(),
        actions: d.actions.clone(),
    }
}

pub fn parse_patron_items(html: &str, origin: &str) -> PatronList {
    let state: Rc<RefCell<ListState>> = Rc::default();
    let header_buf = RefCell::new(String::new());
    let current = || state.borrow().current.clone();

    let _ = rewrite_str(
        html,
        settings(vec![
            text!(".result-header-brief", |t| {
                header_buf.borrow_mut().push_str(t.as_str());
                Ok(())
            }),
            // Each data row is a <tr> carrying an SVL checkbox. Header rows (no SVL) are
            // opened as drafts too but dropped at the end tag because they have no irn.
            element!("tr", |el| {
                let d: DraftRef = Rc::default();
                state.borrow_mut().current = Some(Rc::clone(&d));
                let st = Rc::clone(&state);
                let origin = origin.to_owned();
                el.on_end_tag(end_tag!(move |_end| {
                    let mut s = st.borrow_mut();
                    {
                        let draft = d.borrow();
                        if truthy(&draft.irn) && !collapse(&draft.title_buf).is_empty() {
                            s.items.push(finalize(&draft, &origin));
                        }
                    }
                    if s.current.as_ref().is_some_and(|c| Rc::ptr_eq(c, &d)) {
                        s.current = None;
                    }
                    Ok(())
                }))
            }),
            element!(r#"tr input[name="SVL"]"#, |el| {
                if let Some(cur) = current() {
                    let mut d = cur.borrow_mut();
                    if !truthy(&d.irn) {
                        d.irn = el.get_attribute("value");
                    }
                }
                Ok(())
            }),
            element!("tr h3.card-title a", |el| {
                if let Some(cur) = current() {
                    let mut d = cur.borrow_mut();
                    if !truthy(&d.bib_url) {
                        d.bib_url = el.get_attribute("href");
                    }
                }
                Ok(())
            }),
            text!("tr h3.card-title a", |t| {
                if let Some(cur) = current() {
                    cur.borrow_mut().title_buf.push_str(t.as_str());
                }
                Ok(())
            }),
            element!("tr div.card-text > span.d-block", |el| {
                if let Some(cur) = current() {
                    cur.borrow_mut().line_buf = Some(String::new());
                }
                let st = Rc::clone(&state);
                el.on_end_tag(end_tag!(move |_end| {
                    if let Some(cur) = st.borrow().current.as_ref() {
                        let mut d = cur.borrow_mut();
                        if let Some(line) = d.line_buf.take() {
                            d.detail_lines.push(line);
                        }
                    }
                    Ok(())
                }))
            }),
            text!("tr div.card-text > span.d-block", |t| {
                if let Some(cur) = current()
                    && let Some(buf) = cur.borrow_mut().line_buf.as_mut()
                {
                    buf.push_str(t.as_str());
                }
                Ok(())
            }),
            element!("tr img.imgsc", |el| {
                let Some(cur) = current() else { return Ok(()) };
                let covers = parse_covers(el.get_attribute("data-imgurls").as_deref());
                let mut d = cur.borrow_mut();
                if !covers.covers.is_empty() {
                    d.cover_urls = covers.covers;
                }
                if covers.isbn.is_some() {
                    d.isbn = covers.isbn;
                }
                Ok(())
            }),
            // Columnar cells: <td data-caption="Due|Status">…</td>
            element!("tr td[data-caption]", |el| {
                let Some(cur) = current() else { return Ok(()) };
                {
                    let mut d = cur.borrow_mut();
                    d.cap_caption = el.get_attribute("data-caption");
                    d.cap_buf.clear();
                }
                let st = Rc::clone(&state);
                el.on_end_tag(end_tag!(move |_end| {
                    if let Some(cur) = st.borrow().current.as_ref() {
                        let mut d = cur.borrow_mut();
                        if truthy(&d.cap_caption) {
                            let caption = d.cap_caption.take().unwrap_or_default();
                            let value = collapse(&d.cap_buf);
                            d.fields.insert(caption, value);
                        }
                    }
                    Ok(())
                }))
            }),
            text!("tr td[data-caption]", |t| {
                if let Some(cur) = current() {
                    let mut d = cur.borrow_mut();
                    if d.cap_caption.is_some() {
                        d.cap_buf.push_str(t.as_str());
                    }
                }
                Ok(())
            }),
            // Row action links (Renew / Cancel / Edit).
            element!("tr a.dropdown-item", |el| {
                let Some(cur) = current() else { return Ok(()) };
                {
                    let mut d = cur.borrow_mut();
                    d.act_href = Some(decode(&el.get_attribute("href").unwrap_or_default()));
                    d.act_buf = Some(String::new());
                }
                let st = Rc::clone(&state);
                let origin = origin.to_owned();
                el.on_end_tag(end_tag!(move |_end| {
                    if let Some(cur) = st.borrow().current.as_ref() {
                        let mut d = cur.borrow_mut();
                        if truthy(&d.act_href) {
                            let label = collapse(d.act_buf.as_deref().unwrap_or(""));
                            let href = absolute(&origin, d.act_href.as_deref());
                            if let Some(href) = href
                                && !label.is_empty()
                                && !d.actions.iter().any(|a| a.href == href)
                            {
                                d.actions.push(PatronAction { label, href });
                            }
                            d.act_href = None;
                            d.act_buf = None;
                        }
                    }
                    Ok(())
                }))
            }),
            text!("tr a.dropdown-item", |t| {
                if let Some(cur) = current()
                    && let Some(buf) = cur.borrow_mut().act_buf.as_mut()
                {
                    buf.push_str(t.as_str());
                }
                Ok(())
            }),
        ]),
    );

    let items = std::mem::take(&mut state.borrow_mut().items);
    let header = collapse(&header_buf.into_inner());
    let total = match HEADER_TOTAL.captures(&header) {
        Some(c) => to_int(&c[1]),
        None => Some(items.len() as i64),
    };
    PatronList { total, items }
}
