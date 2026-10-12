//! Port of `middleware/src/parse/reserve.ts`: the CCOPT `#mainForm` wizards (place reservation,
//! cancel confirm, edit reservation) → a submittable field set, and the result pages → ok/message.
//!
//! The TS is regex over raw HTML, so this is too: same patterns, spelled for JS semantics —
//! `(?-u:\b)` is JS's ASCII word boundary, `(?i-u)` is JS's ASCII-only `/i`, and `JS_WS` stands in
//! for JS `\s`. Known residual difference: JS counts `{m,n}` repetitions in UTF-16 code units,
//! Rust in code points (only differs for astral characters inside those spans).

use crate::parse::decode::{JS_WS, absolute, collapse, decode, js_trim};
use crate::types::{EditOptions, ExpiryChoice, PickupChoice, ReserveOptions, ReserveResult};
use indexmap::IndexMap;
use regex::Regex;
use std::sync::LazyLock;

/// The serialized `#mainForm` (place reservation / cancel confirm).
#[derive(serde::Serialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReserveForm {
    /// Absolute POST target.
    pub action: String,
    /// Pre-filled fields to submit, in form order (hidden inputs, defaults, csrf, submit).
    pub body: IndexMap<String, String>,
    pub pickup_location: Option<String>,
    pub available_summary: Option<String>,
    pub expiry_choices: Vec<ExpiryChoice>,
}

/// The serialized edit-reservation `#mainForm` plus its pickup choices.
#[derive(serde::Serialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct EditForm {
    pub action: String,
    pub body: IndexMap<String, String>,
    pub pickup_choices: Vec<PickupChoice>,
    pub pickup_selected: Option<String>,
    pub delay_until: Option<String>,
    pub not_required_after: Option<String>,
}

/// Which manage wizard produced a result page (`"cancel" | "edit"` in TS).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManageKind {
    Cancel,
    Edit,
}

fn re(pattern: &str) -> Regex {
    Regex::new(pattern).expect("valid regex")
}

// ---- form serialization (reserve.ts:22-74, 114-138) ----
static ACTION: LazyLock<Regex> = LazyLock::new(|| re(r#"action="([^"]*)""#));
static INPUT_TAG: LazyLock<Regex> = LazyLock::new(|| re(r"<input(?-u:\b)[^>]*>"));
static ATTR_NAME: LazyLock<Regex> = LazyLock::new(|| re(r#"(?-u:\b)name="([^"]+)""#));
static ATTR_TYPE: LazyLock<Regex> = LazyLock::new(|| re(r#"(?-u:\b)type="([^"]+)""#));
static ATTR_CHECKED: LazyLock<Regex> = LazyLock::new(|| re(r"(?-u:\b)checked(?-u:\b)"));
static ATTR_VALUE: LazyLock<Regex> = LazyLock::new(|| re(r#"(?-u:\b)value="([^"]*)""#));
static SELECT: LazyLock<Regex> =
    LazyLock::new(|| re(r#"<select(?-u:\b)[^>]*(?-u:\b)name="([^"]+)"[^>]*>([\s\S]*?)</select>"#));
static OPTION_SELECTED_THEN_VALUE: LazyLock<Regex> =
    LazyLock::new(|| re(r#"<option[^>]*(?-u:\b)selected(?-u:\b)[^>]*(?-u:\b)value="([^"]*)""#));
static OPTION_VALUE_THEN_SELECTED: LazyLock<Regex> =
    LazyLock::new(|| re(r#"<option[^>]*(?-u:\b)value="([^"]*)"[^>]*(?-u:\b)selected(?-u:\b)"#));
static OPTION_VALUE: LazyLock<Regex> =
    LazyLock::new(|| re(r#"<option[^>]*(?-u:\b)value="([^"]*)""#));
static SUBMIT_BUTTON: LazyLock<Regex> = LazyLock::new(|| {
    re(r#"<button[^>]*type="submit"[^>]*(?-u:\b)name="([^"]+)"[^>]*>([\s\S]*?)</button>"#)
});
static TAG: LazyLock<Regex> = LazyLock::new(|| re(r"<[^>]+>"));
static PICKUP_LOCATION: LazyLock<Regex> =
    LazyLock::new(|| re(r#"id="pickuplocs"[\s\S]*?form-static-text">([^<]+)<"#));
static STATIC_TEXT: LazyLock<Regex> = LazyLock::new(|| re(r#"form-static-text">([^<]+)<"#));
static RESERVABLE: LazyLock<Regex> = LazyLock::new(|| re(r"(?i-u)reservable|available"));
static XDAYS_SELECT: LazyLock<Regex> =
    LazyLock::new(|| re(r#"<select(?-u:\b)[^>]*(?-u:\b)name="XDAYS"[^>]*>([\s\S]*?)</select>"#));
static PICKUP_SELECT: LazyLock<Regex> =
    LazyLock::new(|| re(r#"<select(?-u:\b)[^>]*(?-u:\b)name="PICKUP"[^>]*>([\s\S]*?)</select>"#));
static OPTION_LABEL: LazyLock<Regex> = LazyLock::new(|| {
    re(&format!(
        r#"<option[^>]*(?-u:\b)value="([^"]*)"[^>]*>{JS_WS}*([^<]*)"#
    ))
});

// ---- result classification (reserve.ts:162-207) ----
static MANAGE_VALIDATION: LazyLock<Regex> =
    LazyLock::new(|| re(r#"id="formValidationMsg"[^>]*>([^<]{2,160})<"#));
static RESERVE_VALIDATION: LazyLock<Regex> =
    LazyLock::new(|| re(r#"id="formValidationMsg"[^>]*>([^<]{2,120})<"#));
static CANCELLED: LazyLock<Regex> = LazyLock::new(|| re(r"(?i-u)cancell?ed"));
static RESERVATION_WORD: LazyLock<Regex> = LazyLock::new(|| re(r"(?i-u)reservation|request|hold"));
static EDIT_UPDATED: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i-u:reservation[s]?)[^.]{0,40}(?i-u:(updated|changed|amended|saved))"));
static EDIT_SUCCESSFULLY: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i-u)successfully (updated|changed|amended)"));
static MANAGE_FAIL: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i-u:(cannot|could not|unable to|not allowed|error))[^.]{0,80}"));
static RESERVE_PLACED: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i-u:(reservation|request))[^.]{0,40}(?i-u:(placed|successful|added|submitted|received|confirmed))",
    )
});
static RESERVE_SUCCESSFULLY: LazyLock<Regex> = LazyLock::new(|| re(r"(?i-u)successfully reserved"));
static RESERVE_YOU_HAVE: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i-u)you have (successfully )?reserved"));
static RESERVE_FAIL: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i-u:(you already have|already reserved|cannot be reserved|not allowed|unable to|maximum|exceeded|too many))[^.]{0,80}",
    )
});

// reserve.ts `CANCEL_REFUSAL`: an explicit cancel refusal ("This reservation cannot be cancelled.",
// "can't be cancelled", "unable to cancel", …). The negation must sit right before the cancel verb
// (only be/now/currently/yet between). The apostrophe alternatives stay outside `(?i-u:…)`.
const CANCEL_REFUSAL: &str = r"(?-u:\b)(?:(?i-u:cannot|can not)|(?i-u:can)(?:'|\x{2019})?(?i-u:t)|(?i-u:could not)|(?i-u:couldn)(?:'|\x{2019})?(?i-u:t)|(?i-u:can no longer|unable to|not able to|not allowed to|not permitted to))";
static CANCEL_REFUSED: LazyLock<Regex> = LazyLock::new(|| {
    re(&format!(
        r"{CANCEL_REFUSAL}(?:{JS_WS}+(?i-u:be|now|currently|yet))*{JS_WS}+(?i-u:cancel)"
    ))
});
// reserve.ts `CANCEL_REFUSAL_SENTENCE`: the refusal's sentence, bounded by . ! ? or a block-element
// edge (NUL), at most 160 chars each side.
static CANCEL_REFUSAL_SENTENCE: LazyLock<Regex> = LazyLock::new(|| {
    re(&format!(
        r"[^.!?\x00]{{0,160}}{CANCEL_REFUSAL}(?:{JS_WS}+(?i-u:be|now|currently|yet))*{JS_WS}+(?i-u:cancel)[^.!?\x00]{{0,160}}[.!?]?"
    ))
});
static SCRIPT_STYLE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"<(?i-u:script)(?-u:\b)[^>]*>[\s\S]*?</(?i-u:script)>|<(?i-u:style)(?-u:\b)[^>]*>[\s\S]*?</(?i-u:style)>",
    )
});
static BLOCK_TAG: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"</?(?i-u:address|article|aside|blockquote|body|br|button|caption|dd|details|div|dl|dt|fieldset|figcaption|figure|footer|form|h[1-6]|header|hr|html|label|legend|li|main|nav|ol|option|p|section|select|summary|table|tbody|td|textarea|tfoot|th|thead|title|tr|ul)(?-u:\b)[^>]*>",
    )
});
static WS_RUN: LazyLock<Regex> = LazyLock::new(|| re(&format!("{JS_WS}+")));

const MAIN_FORM: &str = r#"id="mainForm""#;

fn group<'h>(re: &Regex, haystack: &'h str) -> Option<&'h str> {
    re.captures(haystack)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str())
}

/// `<option … selected … value="X">` or `<option … value="X" … selected>` (reserve.ts:52-53).
fn selected_option(inner: &str) -> Option<&str> {
    group(&OPTION_SELECTED_THEN_VALUE, inner).or_else(|| group(&OPTION_VALUE_THEN_SELECTED, inner))
}

/// `<option value>` → (value, collapsed label) pairs, raw value (reserve.ts:69, 120).
fn option_pairs(inner: &str) -> impl Iterator<Item = (&str, String)> {
    OPTION_LABEL
        .captures_iter(inner)
        .map(|c| (c.get(1).map_or("", |m| m.as_str()), collapse(&c[2])))
}

/// Visible text of an HTML snippet: tags → spaces, then collapse.
fn strip_tags(html: &str) -> String {
    collapse(&TAG.replace_all(html, " "))
}

/// Serialize the `#mainForm` reservation-request form. `None` if absent. reserve.ts:22-74.
pub fn parse_reserve_form(html: &str, origin: &str) -> Option<ReserveForm> {
    let idx = html.find(MAIN_FORM)?;
    // Byte offsets stand in for JS string indices: every needle is ASCII.
    let open_start = html[..idx].rfind("<form");
    let open_end = html[idx..].find('>').map(|i| idx + i);
    let close_from = open_end.unwrap_or(0);
    let close_idx = html[close_from..].find("</form>").map(|i| close_from + i);
    // JS `slice(start, end)` with start -1 / end 0 yields "" here; the action regex can't match then.
    let open_tag = match (open_start, open_end) {
        (Some(s), Some(e)) => &html[s..=e],
        _ => "",
    };
    let region_start = open_end.map_or(0, |e| e + 1);
    let region = &html[region_start..close_idx.unwrap_or(html.len())];

    let action_raw = decode(group(&ACTION, open_tag).unwrap_or(""));
    let action = absolute(origin, Some(&action_raw)).unwrap_or_default();
    let mut body: IndexMap<String, String> = IndexMap::new();

    // Named inputs (hidden/text/date). Checkboxes/radios only when checked.
    for tag in INPUT_TAG.find_iter(region).map(|m| m.as_str()) {
        let Some(name) = group(&ATTR_NAME, tag) else {
            continue;
        };
        let kind = group(&ATTR_TYPE, tag).unwrap_or("text").to_lowercase();
        if kind == "checkbox" || kind == "radio" {
            if ATTR_CHECKED.is_match(tag) {
                body.insert(
                    name.to_owned(),
                    decode(group(&ATTR_VALUE, tag).unwrap_or("on")),
                );
            }
            continue;
        }
        body.insert(
            name.to_owned(),
            decode(group(&ATTR_VALUE, tag).unwrap_or("")),
        );
    }

    // Selects → selected option, else first option.
    for sm in SELECT.captures_iter(region) {
        let inner = &sm[2];
        let first = group(&OPTION_VALUE, inner).unwrap_or("");
        body.insert(
            sm[1].to_owned(),
            decode(selected_option(inner).unwrap_or(first)),
        );
    }

    // Primary submit button (name=value) — Spydus expects submitButton=Continue. Only
    // `type="submit"` qualifies, so the JS-nav `cancelButton` (type=button) is never sent.
    if let Some(sb) = SUBMIT_BUTTON.captures(region) {
        body.insert(sb[1].to_owned(), strip_tags(&sb[2]));
    }

    let pickup_location = group(&PICKUP_LOCATION, region).map(|s| js_trim(s).to_owned());
    let available_summary = STATIC_TEXT
        .captures_iter(region)
        .map(|c| collapse(&c[1]))
        .find(|s| RESERVABLE.is_match(s));

    let xdays = group(&XDAYS_SELECT, region).unwrap_or("");
    let expiry_choices = option_pairs(xdays)
        .map(|(days, label)| ExpiryChoice {
            days: days.to_owned(),
            label,
        })
        .collect();

    Some(ReserveForm {
        action,
        body,
        pickup_location,
        available_summary,
        expiry_choices,
    })
}

/// reserve.ts:76-93.
pub fn to_reserve_options(form: Option<&ReserveForm>) -> ReserveOptions {
    match form {
        None => ReserveOptions {
            can_reserve: false,
            pickup_location: None,
            available_summary: None,
            expiry_choices: Vec::new(),
            message: Some("This title can't be reserved right now.".to_owned()),
        },
        Some(f) => ReserveOptions {
            can_reserve: true,
            pickup_location: f.pickup_location.clone(),
            available_summary: f.available_summary.clone(),
            expiry_choices: f.expiry_choices.clone(),
            message: None,
        },
    }
}

/// Serialize the edit-reservation `#mainForm` plus its pickup-location choices. reserve.ts:114-138.
pub fn parse_edit_form(html: &str, origin: &str) -> Option<EditForm> {
    let base = parse_reserve_form(html, origin)?;

    // The PICKUP select is searched in the whole page, not just the form region.
    let sel = group(&PICKUP_SELECT, html).unwrap_or("");
    let pickup_choices = option_pairs(sel)
        .map(|(code, name)| PickupChoice {
            code: decode(code),
            name,
        })
        .filter(|c| !c.code.is_empty() && !c.name.is_empty())
        .collect();
    // Raw (undecoded) like the TS; "" → None.
    let pickup_selected = selected_option(sel)
        .filter(|s| !s.is_empty())
        .map(str::to_owned);
    let non_empty = |key: &str| base.body.get(key).filter(|v| !v.is_empty()).cloned();

    Some(EditForm {
        delay_until: non_empty("SDTE"),
        not_required_after: non_empty("XDTE"),
        action: base.action,
        body: base.body,
        pickup_choices,
        pickup_selected,
    })
}

/// reserve.ts:140-159.
pub fn to_edit_options(form: Option<&EditForm>) -> EditOptions {
    match form {
        None => EditOptions {
            can_edit: false,
            pickup_choices: Vec::new(),
            pickup_selected: None,
            delay_until: None,
            not_required_after: None,
            message: Some("This reservation can't be edited right now.".to_owned()),
        },
        Some(f) => EditOptions {
            can_edit: true,
            pickup_choices: f.pickup_choices.clone(),
            pickup_selected: f.pickup_selected.clone(),
            delay_until: f.delay_until.clone(),
            not_required_after: f.not_required_after.clone(),
            message: None,
        },
    }
}

/// A populated `formValidationMsg` (not the template's `Error` placeholder), collapsed.
fn validation_error(re: &Regex, html: &str) -> Option<String> {
    let err = js_trim(group(re, html)?);
    (!err.is_empty() && !err.eq_ignore_ascii_case("error")).then(|| collapse(err))
}

/// reserve.ts `cancelRefusal`: the sentence that refuses a cancel, whitespace-collapsed.
fn cancel_refusal(html: &str) -> Option<String> {
    // Visible-ish text: scripts/styles dropped, block edges kept as NUL so a sentence never runs
    // across elements, inline tags → spaces, entities decoded.
    let text = SCRIPT_STYLE.replace_all(html, " ");
    let text = BLOCK_TAG.replace_all(&text, "\0");
    let text = decode(&TAG.replace_all(&text, " "));
    if !CANCEL_REFUSED.is_match(&text) {
        return None;
    }
    let sentence = CANCEL_REFUSAL_SENTENCE.find(&text)?;
    Some(js_trim(&WS_RUN.replace_all(sentence.as_str(), " ")).to_owned())
}

fn result(ok: bool, message: impl Into<String>) -> ReserveResult {
    ReserveResult {
        ok,
        message: Some(message.into()),
    }
}

/// Classify the page returned after committing a cancel / edit. reserve.ts `parseManageResult`.
///
/// For `Cancel`, an explicit refusal ("This reservation cannot be cancelled.") is checked before
/// the success wording, since a refusal also says "reservation" and "cancelled" (Wave 2, G1).
pub fn parse_manage_result(html: &str, kind: ManageKind) -> ReserveResult {
    let text = strip_tags(html);
    if let Some(err) = validation_error(&MANAGE_VALIDATION, html) {
        return result(false, err);
    }
    if kind == ManageKind::Cancel
        && let Some(refusal) = cancel_refusal(html)
    {
        return result(false, refusal);
    }

    let success = match kind {
        ManageKind::Cancel => CANCELLED.is_match(&text) && RESERVATION_WORD.is_match(&text),
        ManageKind::Edit => EDIT_UPDATED.is_match(&text) || EDIT_SUCCESSFULLY.is_match(&text),
    };
    let (ok_message, default_failure) = match kind {
        ManageKind::Cancel => ("Reservation cancelled.", "Couldn't cancel the reservation."),
        ManageKind::Edit => ("Reservation updated.", "Couldn't update the reservation."),
    };
    if success {
        return result(true, ok_message);
    }

    let fail = MANAGE_FAIL.find(&text).map(|m| collapse(m.as_str()));
    // Left the wizard (no mainForm) → Spydus redirected back to the list: success.
    if !html.contains(MAIN_FORM) {
        return result(true, ok_message);
    }
    result(false, fail.unwrap_or_else(|| default_failure.to_owned()))
}

/// Classify the page returned after POSTing the reservation. reserve.ts:188-207.
pub fn parse_reserve_result(html: &str) -> ReserveResult {
    let text = strip_tags(html);
    // A validation error re-renders the form with a populated message.
    if let Some(err) = validation_error(&RESERVE_VALIDATION, html) {
        return result(false, err);
    }

    if RESERVE_PLACED.is_match(&text)
        || RESERVE_SUCCESSFULLY.is_match(&text)
        || RESERVE_YOU_HAVE.is_match(&text)
    {
        return result(true, "Reservation placed.");
    }
    if let Some(fail) = RESERVE_FAIL.find(&text) {
        return result(false, collapse(fail.as_str()));
    }

    // Still on the form → an extra step is required we didn't complete.
    if html.contains(MAIN_FORM) {
        return result(false, "Reservation could not be completed.");
    }
    result(true, "Reservation placed.")
}
