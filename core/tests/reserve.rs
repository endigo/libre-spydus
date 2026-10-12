//! `parse::reserve` — golden parity with `middleware/src/parse/reserve.ts` (rows from
//! `tests/golden/MANIFEST.md`) plus invariant tests for the regex form serializer.
//!
//! Private rows hold the user's real account data: they skip when absent and their assertions
//! never print values.

mod common;

use librespydus_core::config::ORIGIN;
use librespydus_core::parse::reserve::{
    ManageKind, parse_edit_form, parse_manage_result, parse_reserve_form, parse_reserve_result,
    to_edit_options, to_reserve_options,
};

fn manage(stem: &str, kind: ManageKind) {
    let html = common::fixture(&format!("synthetic/{stem}.html"));
    let suffix = match kind {
        ManageKind::Cancel => "cancel",
        ManageKind::Edit => "edit",
    };
    common::assert_golden(
        &parse_manage_result(&html, kind),
        &format!("synthetic/{stem}.{suffix}.json"),
    );
}

fn reserve_result(stem: &str) {
    let html = common::fixture(&format!("synthetic/{stem}.html"));
    common::assert_golden(
        &parse_reserve_result(&html),
        &format!("synthetic/{stem}.json"),
    );
}

// ---- synthetic goldens: parse_manage_result ----

#[test]
fn golden_manage_cancel_success() {
    manage("manage-cancel-success", ManageKind::Cancel);
}

#[test]
fn golden_manage_edit_success() {
    manage("manage-edit-success", ManageKind::Edit);
}

#[test]
fn golden_manage_validation_error() {
    manage("manage-validation-error", ManageKind::Edit);
}

#[test]
fn golden_manage_cannot_cancel() {
    // Cancel: the refusal sentence wins over the "cancelled" + "reservation" success wording.
    // Edit: unaffected by the refusal rule (generic failure match).
    manage("manage-cannot-cancel", ManageKind::Cancel);
    manage("manage-cannot-cancel", ManageKind::Edit);
}

#[test]
fn golden_manage_cancel_refused() {
    manage("manage-cancel-refused", ManageKind::Cancel);
}

#[test]
fn golden_manage_cancel_success_help() {
    manage("manage-cancel-success-help", ManageKind::Cancel);
}

#[test]
fn golden_manage_still_on_form() {
    manage("manage-still-on-form", ManageKind::Cancel);
    manage("manage-still-on-form", ManageKind::Edit);
}

#[test]
fn golden_manage_left_wizard() {
    manage("manage-left-wizard", ManageKind::Cancel);
    manage("manage-left-wizard", ManageKind::Edit);
}

// ---- synthetic goldens: parse_reserve_result ----

#[test]
fn golden_reserve_placed() {
    reserve_result("reserve-placed");
}

#[test]
fn golden_reserve_already_reserved() {
    reserve_result("reserve-already-reserved");
}

#[test]
fn golden_reserve_still_on_form() {
    reserve_result("reserve-still-on-form");
}

#[test]
fn golden_reserve_no_form() {
    reserve_result("reserve-no-form");
}

#[test]
fn golden_reserve_validation_error() {
    reserve_result("reserve-validation-error");
}

// ---- synthetic goldens: forms + options ----

#[test]
fn golden_synthetic_reserve_form() {
    let html = common::fixture("synthetic/reserve-form.html");
    let form = parse_reserve_form(&html, ORIGIN);
    common::assert_golden(&form, "synthetic/reserve-form.json");
    common::assert_golden(
        &to_reserve_options(form.as_ref()),
        "synthetic/reserve-options.json",
    );
}

#[test]
fn golden_synthetic_edit_form() {
    let html = common::fixture("synthetic/edit-form.html");
    let form = parse_edit_form(&html, ORIGIN);
    common::assert_golden(&form, "synthetic/edit-form.json");
    common::assert_golden(
        &to_edit_options(form.as_ref()),
        "synthetic/edit-options.json",
    );
}

#[test]
fn golden_synthetic_no_form() {
    let html = common::fixture("synthetic/reserve-no-form.html");
    let form = parse_reserve_form(&html, ORIGIN);
    common::assert_golden(&form, "synthetic/reserve-form-none.json");
    common::assert_golden(
        &to_reserve_options(form.as_ref()),
        "synthetic/reserve-options-none.json",
    );
    common::assert_golden(
        &to_edit_options(parse_edit_form(&html, ORIGIN).as_ref()),
        "synthetic/edit-options-none.json",
    );
}

// ---- private goldens (skip when the staged account data is absent) ----

#[test]
fn golden_private_reserve_form() {
    if common::skip_private("golden_private_reserve_form") {
        return;
    }
    let html = common::fixture("private/reserve-form.html");
    let form = parse_reserve_form(&html, ORIGIN);
    common::assert_golden(&form, "private/reserve-form.json");
    common::assert_golden(
        &to_reserve_options(form.as_ref()),
        "private/reserve-options.json",
    );
}

#[test]
fn golden_private_cancel_form() {
    if common::skip_private("golden_private_cancel_form") {
        return;
    }
    let html = common::fixture("private/cancel-confirm.html");
    common::assert_golden(
        &parse_reserve_form(&html, ORIGIN),
        "private/cancel-form.json",
    );
}

/// A successful cancel lands on the reservation / loan list: the refusal rule must not fire there.
#[test]
fn golden_private_cancel_lands_on_list() {
    if common::skip_private("golden_private_cancel_lands_on_list") {
        return;
    }
    for stem in ["reservations-2", "reservations-5", "loans-1"] {
        let html = common::fixture(&format!("private/{stem}.html"));
        let r = parse_manage_result(&html, ManageKind::Cancel);
        assert!(r.ok, "{stem}: a list page must read as a successful cancel");
        common::assert_golden(&r, &format!("private/{stem}.cancel.json"));
    }
}

#[test]
fn golden_private_edit_form() {
    if common::skip_private("golden_private_edit_form") {
        return;
    }
    let html = common::fixture("private/edit-form.html");
    let form = parse_edit_form(&html, ORIGIN);
    common::assert_golden(&form, "private/edit-form.json");
    common::assert_golden(&to_edit_options(form.as_ref()), "private/edit-options.json");
}

// ---- invariants ----

/// The commit button (type=submit) is serialized; the JS-nav abort (type=button) never is.
#[test]
fn manage_forms_send_submit_never_cancel_button() {
    let mut pages = vec![
        common::fixture("synthetic/reserve-form.html"),
        common::fixture("synthetic/edit-form.html"),
    ];
    if common::private_available() {
        pages.push(common::fixture("private/cancel-confirm.html"));
        pages.push(common::fixture("private/edit-form.html"));
    }
    for (i, html) in pages.iter().enumerate() {
        let form = parse_reserve_form(html, ORIGIN).expect("form present");
        assert!(
            form.body.contains_key("submitButton"),
            "page {i}: submitButton missing"
        );
        assert!(
            !form.body.contains_key("cancelButton"),
            "page {i}: cancelButton serialized"
        );
    }
}

/// Unchecked CLR* checkboxes are left out, so an edit never clears dates by accident.
#[test]
fn unchecked_clear_flags_absent() {
    let mut pages = vec![common::fixture("synthetic/edit-form.html")];
    if common::private_available() {
        pages.push(common::fixture("private/edit-form.html"));
    }
    for (i, html) in pages.iter().enumerate() {
        let form = parse_edit_form(html, ORIGIN).expect("edit form present");
        assert!(
            !form.body.contains_key("CLRSDTE"),
            "page {i}: unchecked CLRSDTE serialized"
        );
        assert!(
            !form.body.contains_key("CLRXDTE"),
            "page {i}: unchecked CLRXDTE serialized"
        );
    }
}

fn form(inner: &str) -> String {
    format!(r#"<form id="mainForm" method="post" action="/x/R/1">{inner}</form>"#)
}

#[test]
fn checkbox_without_value_is_on_and_radio_only_checked() {
    let html = form(
        r#"<input type="checkbox" name="NOTIFY" checked>
           <input type="CHECKBOX" name="UPPER" checked value="1">
           <input type="checkbox" name="OFF" value="1">
           <input type="radio" name="MODE" value="A">
           <input type="radio" name="MODE" value="B" checked>
           <input name="PLAIN">"#,
    );
    let body = parse_reserve_form(&html, ORIGIN).unwrap().body;
    assert_eq!(body.get("NOTIFY").map(String::as_str), Some("on"));
    assert_eq!(body.get("UPPER").map(String::as_str), Some("1"));
    assert_eq!(body.get("OFF"), None);
    assert_eq!(body.get("MODE").map(String::as_str), Some("B"));
    assert_eq!(body.get("PLAIN").map(String::as_str), Some(""));
}

#[test]
fn select_selected_in_either_attribute_order() {
    let html = form(
        r#"<select name="AFTER"><option value="1">One</option><option value="2" selected>Two</option></select>
           <select name="BEFORE"><option value="1">One</option><option selected value="3">Three</option></select>
           <select name="NONE"><option value="first">F</option><option value="second">S</option></select>
           <select name="EMPTY"></select>"#,
    );
    let body = parse_reserve_form(&html, ORIGIN).unwrap().body;
    assert_eq!(body.get("AFTER").map(String::as_str), Some("2"));
    assert_eq!(body.get("BEFORE").map(String::as_str), Some("3"));
    assert_eq!(body.get("NONE").map(String::as_str), Some("first"));
    assert_eq!(body.get("EMPTY").map(String::as_str), Some(""));

    let edit = parse_edit_form(
        &form(r#"<select name="PICKUP"><option value="">Pick</option><option selected="selected" value="9">NINE</option></select>"#),
        ORIGIN,
    )
    .unwrap();
    assert_eq!(edit.pickup_selected.as_deref(), Some("9"));
    assert_eq!(edit.pickup_choices.len(), 1);
}

#[test]
fn body_keeps_first_insertion_order() {
    let html = form(
        r#"<input type="hidden" name="b" value="1"><input type="hidden" name="a" value="2">
           <input type="hidden" name="b" value="3"><button type="submit" name="go">Go</button>"#,
    );
    let body = parse_reserve_form(&html, ORIGIN).unwrap().body;
    let keys: Vec<&str> = body.keys().map(String::as_str).collect();
    assert_eq!(keys, ["b", "a", "go"]);
    assert_eq!(body.get("b").map(String::as_str), Some("3"));
}

#[test]
fn action_is_absolutized_and_double_decoded() {
    let html = r#"<form method="post" id="mainForm" action="/p?A=1&amp;amp;B=2"></form>"#;
    assert_eq!(
        parse_reserve_form(html, ORIGIN).unwrap().action,
        format!("{ORIGIN}/p?A=1&B=2")
    );
    // No <form before the id: the open tag is empty, so there is no action.
    assert_eq!(
        parse_reserve_form(r#"<div id="mainForm" action="/p"></div>"#, ORIGIN)
            .unwrap()
            .action,
        ""
    );
    assert_eq!(
        parse_reserve_form("<form id=\"other\"></form>", ORIGIN),
        None
    );
}

#[test]
fn manage_left_wizard_is_success() {
    let list = "<html><body><h2>Current reservations</h2></body></html>";
    for kind in [ManageKind::Cancel, ManageKind::Edit] {
        let r = parse_manage_result(list, kind);
        assert!(r.ok, "{kind:?}: no mainForm left must be success");
    }
    // Failure wording does not override having left the wizard.
    let r = parse_manage_result("<p>Unable to do that.</p>", ManageKind::Edit);
    assert!(r.ok);
    // …but it is reported when still on the form.
    let r = parse_manage_result(
        r#"<p>Unable to do that.</p><form id="mainForm"></form>"#,
        ManageKind::Edit,
    );
    assert_eq!(
        (r.ok, r.message.as_deref()),
        (false, Some("Unable to do that"))
    );
}

#[test]
fn validation_message_rules() {
    // The template placeholder "Error" (any case) is ignored.
    let r = parse_reserve_result(
        r#"<div id="formValidationMsg">ERROR</div><form id="mainForm"></form>"#,
    );
    assert_eq!(
        r.message.as_deref(),
        Some("Reservation could not be completed.")
    );
    // Over 120 chars (reserve) / 160 chars (manage) the message regex does not match.
    let long = "x".repeat(150);
    let page = format!(r#"<div id="formValidationMsg">{long}</div><form id="mainForm"></form>"#);
    assert_eq!(
        parse_reserve_result(&page).message.as_deref(),
        Some("Reservation could not be completed.")
    );
    assert_eq!(
        parse_manage_result(&page, ManageKind::Edit)
            .message
            .as_deref(),
        Some(long.as_str())
    );
}

#[test]
fn reserve_success_phrasings() {
    for text in [
        "Successfully reserved.",
        "You have reserved this item.",
        "Request was submitted.",
    ] {
        let r = parse_reserve_result(&format!(r#"<p>{text}</p><form id="mainForm"></form>"#));
        assert!(r.ok, "{text}");
    }
    let r = parse_manage_result(
        r#"<p>Successfully amended</p><form id="mainForm"></form>"#,
        ManageKind::Edit,
    );
    assert!(r.ok);
}

#[test]
fn ascii_only_case_folding() {
    // JS `/i` without `/u` never folds ſ (U+017F) to s; neither do these patterns.
    let r = parse_reserve_result(r#"<p>ſucceſſfully reſerved</p><form id="mainForm"></form>"#);
    assert!(!r.ok);
}

// ---- Wave 2: explicit cancel refusal ----

fn cancel(html: &str) -> (bool, Option<String>) {
    let r = parse_manage_result(html, ManageKind::Cancel);
    (r.ok, r.message)
}

fn refused(sentence: &str) -> (bool, Option<String>) {
    (false, Some(sentence.to_owned()))
}

#[test]
fn cancel_refusal_phrasings() {
    for (page, sentence) in [
        (
            "<p>This reservation cannot be cancelled.</p>",
            "This reservation cannot be cancelled.",
        ),
        (
            "<p>This reservation can not be cancelled</p>",
            "This reservation can not be cancelled",
        ),
        (
            "<p>Sorry, it can&#39;t be cancelled now!</p>",
            "Sorry, it can't be cancelled now!",
        ),
        (
            "<p>It can&rsquo;t be canceled.</p>",
            "It can\u{2019}t be canceled.",
        ),
        (
            "<p>We could not cancel your request.</p>",
            "We could not cancel your request.",
        ),
        ("<p>We couldn't cancel it?</p>", "We couldn't cancel it?"),
        (
            "<p>It can no longer be cancelled.</p>",
            "It can no longer be cancelled.",
        ),
        (
            "<p>UNABLE TO CANCEL RESERVATION</p>",
            "UNABLE TO CANCEL RESERVATION",
        ),
        (
            "<p>You are not able to cancel this hold.</p>",
            "You are not able to cancel this hold.",
        ),
        (
            "<p>You are not allowed to cancel it.</p>",
            "You are not allowed to cancel it.",
        ),
        (
            "<p>Holds are not permitted to be cancelled.</p>",
            "Holds are not permitted to be cancelled.",
        ),
        (
            "<p>It cannot currently be cancelled.</p>",
            "It cannot currently be cancelled.",
        ),
    ] {
        assert_eq!(cancel(page), refused(sentence), "{page}");
        // Same verdict whether or not the wizard form is still on the page.
        let with_form = format!(r#"{page}<form id="mainForm"></form>"#);
        assert_eq!(cancel(&with_form), refused(sentence), "{page} + mainForm");
    }
}

#[test]
fn cancel_refusal_sentence_bounds() {
    // Block edges bound the sentence; inline tags become spaces and are collapsed away.
    assert_eq!(
        cancel(
            "<h2>Reservations</h2><div>Sorry, this <strong>item</strong>\n cannot be cancelled</div><p>Next</p>"
        ),
        refused("Sorry, this item cannot be cancelled")
    );
    // The preceding sentence is cut at its terminator; the next one is not included.
    assert_eq!(
        cancel("<p>Thanks. This hold cannot be cancelled because it is in transit. Ask staff.</p>"),
        refused("This hold cannot be cancelled because it is in transit.")
    );
    // At most 160 characters before the negation and after "cancel".
    let pre = "a".repeat(200);
    let post = "b".repeat(200);
    let (ok, msg) = cancel(&format!("<p>{pre} cannot be cancelled {post}</p>"));
    assert!(!ok);
    assert_eq!(
        msg,
        Some(format!(
            "{} cannot be cancelled {}",
            "a".repeat(159),
            "b".repeat(156)
        ))
    );
}

#[test]
fn cancel_refusal_needs_the_cancel_verb_next() {
    // Negations near other verbs, or "cancel" further away, are help text, not a refusal.
    for page in [
        "<p>If you cannot collect a reservation, you can cancel it below.</p>",
        "<p>Items in transit cannot be renewed. To cancel, use the menu.</p>",
        "<p>Unable to display cover image.</p><p>Your reservation has been cancelled.</p>",
        "<p>You cannot</p><p>cancel</p>",
        "<p>Significant cancellations ahead.</p>",
    ] {
        assert_eq!(
            cancel(page),
            (true, Some("Reservation cancelled.".to_owned())),
            "{page}"
        );
    }
}

#[test]
fn cancel_refusal_ignores_script_and_style() {
    let page = r#"<script type="text/javascript">var e = "Unable to cancel request.";</script>
        <STYLE>.x::after { content: "cannot cancel"; }</STYLE>
        <p>Your reservation has been cancelled.</p>"#;
    assert_eq!(
        cancel(page),
        (true, Some("Reservation cancelled.".to_owned()))
    );
}

#[test]
fn cancel_refusal_order_and_kind() {
    // A populated validation message still comes first.
    let page = r#"<div id="formValidationMsg">Pick a branch</div><p>It cannot be cancelled.</p>"#;
    assert_eq!(cancel(page), (false, Some("Pick a branch".to_owned())));
    // Edit results never look for a cancel refusal.
    let page = r#"<p>It cannot be cancelled.</p>"#;
    let r = parse_manage_result(page, ManageKind::Edit);
    assert_eq!(
        (r.ok, r.message.as_deref()),
        (true, Some("Reservation updated."))
    );
}
