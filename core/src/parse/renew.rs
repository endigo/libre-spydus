//! Renewing a loan (new; no TS counterpart). A loan row's "Renew loan" action is
//! `…/PGM/OPAC/CCOPT/{set}/0/N/3/LOANRENQ?SVL={loanIrn}` — the step the website's "Renew
//! selections" button submits for checked loans. Whether that page asks to confirm (a `#mainForm`
//! with a submit button, like a reservation's Cancel) or renews straight away isn't known yet, so
//! `SpydusClient::renew_loan` handles both and decides the outcome from the loans list itself.

use std::sync::LazyLock;

use regex::Regex;

use crate::parse::decode::{collapse, decode};
use crate::parse::reserve::{ReserveForm, parse_reserve_form};

static MAIN_FORM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?is)<form\b[^>]*\bid="mainForm"[^>]*>(.*?)</form>"#).unwrap());
static SUBMIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)\btype="submit""#).unwrap());
static LOAN_IRN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[?&]SVL=([0-9]+)").unwrap());
static VALIDATION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"id="formValidationMsg"[^>]*>([^<]{2,200})<"#).unwrap());
static ALERT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)<div\b[^>]*class="[^"]*\balert\b[^"]*"[^>]*>(.*?)</div>"#).unwrap()
});
/// A sentence that refuses a renewal ("This item cannot be renewed as it is reserved…").
static REFUSAL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)[^.!>]{0,80}\b(?:cannot|can't|could not|unable to|not (?:be )?renewed|not allowed|maximum|limit reached|reserved by|has a reservation)\b[^.!<]{0,120}[.!]?")
        .unwrap()
});
static TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]*>").unwrap());

/// The loan IRN a "Renew loan" link renews, from its `SVL`. `None` for anything that isn't a
/// single-loan renew link (`…/LOANRENQ?…SVL=…`).
pub fn renew_link_loan(url: &str) -> Option<String> {
    let url = decode(url);
    if !url.to_ascii_uppercase().contains("/LOANRENQ") {
        return None;
    }
    LOAN_IRN.captures(&url).map(|c| c[1].to_owned())
}

/// The page asks to confirm: its `#mainForm` has a submit button. A page that only carries an
/// empty `#mainForm` (as a finished wizard's does) is not a confirm step.
pub fn confirm_form(html: &str, origin: &str) -> Option<ReserveForm> {
    let inner = MAIN_FORM.captures(html)?.get(1)?.as_str();
    if !SUBMIT.is_match(inner) {
        return None;
    }
    parse_reserve_form(html, origin)
}

/// The library's reason on a renew page: its validation message, else an alert or sentence that
/// refuses the renewal. The site-wide privacy reminder isn't a reason.
pub fn refusal(html: &str) -> Option<String> {
    if let Some(c) = VALIDATION.captures(html) {
        let message = collapse(&decode(&c[1]));
        if !message.is_empty() && !message.eq_ignore_ascii_case("error") {
            return Some(message);
        }
    }
    let alerts = ALERT
        .captures_iter(html)
        .map(|c| collapse(&decode(&TAG.replace_all(&c[1], " "))))
        .filter(|t| !t.is_empty() && !t.to_ascii_lowercase().contains("protect your privacy"));
    for alert in alerts {
        if REFUSAL.is_match(&alert) || alert.to_ascii_lowercase().contains("renew") {
            return Some(alert);
        }
    }
    let text = collapse(&decode(&TAG.replace_all(html, " ")));
    REFUSAL.find(&text).map(|m| collapse(m.as_str())).filter(|m| m.to_ascii_lowercase().contains("renew"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_single_loan_renew_link_has_a_loan() {
        let link = "https://canning.spydus.com/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/1/0/N/3/LOANRENQ?SVL=19000005&amp;SETDESC=Current%20loans";
        assert_eq!(renew_link_loan(link).as_deref(), Some("19000005"));
        assert_eq!(renew_link_loan("https://x/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/1/0/N/2/LOANRENQ"), None);
        assert_eq!(renew_link_loan("https://x/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/1/0/C/5/RSVCENQ?SVL=1"), None);
    }

    #[test]
    fn a_confirm_step_has_a_submit_button() {
        let confirm = r#"<form id="mainForm" method="post" action="/a"><input type="hidden" name="csrf" value="c"><button type="submit" name="submitButton" value="Continue">Continue</button></form>"#;
        assert!(confirm_form(confirm, "https://x").is_some());
        assert!(confirm_form(r#"<form id="mainForm" method="post" action="/a"></form>"#, "https://x").is_none());
        assert!(confirm_form("<p>Done</p>", "https://x").is_none());
    }

    #[test]
    fn the_librarys_reason_not_its_privacy_reminder() {
        let page = r#"<div class="alert alert-info">To protect your privacy please make sure you logout.</div>
            <div class="alert alert-danger">This item cannot be renewed: maximum renewals reached.</div>"#;
        assert_eq!(refusal(page).as_deref(), Some("This item cannot be renewed: maximum renewals reached."));
        assert_eq!(
            refusal(r#"<span id="formValidationMsg">Renewal limit reached</span>"#).as_deref(),
            Some("Renewal limit reached")
        );
        assert_eq!(refusal(r#"<div class="alert">To protect your privacy please log out.</div><p>Done</p>"#), None);
    }
}
