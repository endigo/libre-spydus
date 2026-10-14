//! Headless UI tests of "Already yours" (docs/native-apps.md §6) and renewing from the book page
//! (§5.12) against the demo account: the held pills, a renewal moving the due date, the demo
//! library's refusal for IRN 18000002, and a reservation's pill opening Your Books.

mod detail_support;

use chrono::Days;
use detail_support::*;
use gpui_kit::TestAppContext;
use librespydus_core::PatronItem;
use librespydus_desktop::backend::demo::{RENEW_REFUSAL, RENEWAL_DAYS, UNRENEWABLE_LOAN_IRN};
use librespydus_desktop::state::formatting::{due_date, today};
use librespydus_desktop::state::{BookRef, Section, YourBooksSegment};

fn loan(demo: &librespydus_desktop::backend::demo::DemoBackend, irn: &str) -> PatronItem {
    demo.fixtures().loans.into_iter().find(|loan| loan.irn.as_deref() == Some(irn)).expect("demo loan")
}

fn spoken(date: chrono::NaiveDate) -> String {
    date.format("%-d %B").to_string()
}

fn notice(cx: &mut TestAppContext, state: &gpui_kit::Entity<librespydus_desktop::state::AppState>) -> Option<String> {
    state.read_with(cx, |state, _| state.notice().map(|notice| notice.message.clone()))
}

#[gpui_kit::test]
fn a_demo_loan_shows_on_loan_and_renews_from_the_pill(cx: &mut TestAppContext) {
    let (state, demo) = demo_app(cx, ScriptedBackend::new());
    // "The Love Library", due in 5 days, renewable.
    let item = loan(&demo, "18000003");
    let due = due_date::parse(item.fields.get("Due").unwrap()).unwrap();
    let (window, screen) = open_detail(cx, BookRef::from_patron_item(&item));

    assert_eq!(label(cx, window, "detail-held").as_deref(), Some(format!("On loan, due {}", spoken(due)).as_str()));
    assert!(!exists(cx, window, "detail-reserve"), "the pill takes Reserve's place");

    // The pill asks first; Not now changes nothing.
    click(cx, window, "detail-held");
    assert!(exists(cx, window, "confirm-ok"));
    click(cx, window, "confirm-cancel");
    assert!(!exists(cx, window, "confirm-ok"));
    assert_eq!(loan(&demo, "18000003").fields.get("Due"), item.fields.get("Due"));

    click(cx, window, "detail-held");
    click(cx, window, "confirm-ok");
    cx.run_until_parked();
    let renewed = due.checked_add_days(Days::new(RENEWAL_DAYS)).unwrap();
    assert_eq!(loan(&demo, "18000003").fields.get("Due").map(String::as_str), Some(due_date::format(renewed).as_str()));
    let short = librespydus_desktop::state::formatting::short_date(renewed, today());
    assert_eq!(notice(cx, &state).as_deref(), Some(format!("Renewed · Due {short}").as_str()));
    assert!(!screen.read_with(cx, |screen, _| screen.is_renewing()));
    assert_eq!(
        label(cx, window, "detail-held").as_deref(),
        Some(format!("On loan, due {}", spoken(renewed)).as_str()),
        "the pill moves to the new due date"
    );
}

#[gpui_kit::test]
fn the_demo_library_refuses_to_renew_whistler(cx: &mut TestAppContext) {
    let (state, demo) = demo_app(cx, ScriptedBackend::new());
    let item = loan(&demo, UNRENEWABLE_LOAN_IRN);
    let due = due_date::parse(item.fields.get("Due").unwrap()).unwrap();
    let (window, _) = open_detail(cx, BookRef::from_patron_item(&item));
    assert_eq!(label(cx, window, "detail-held").as_deref(), Some(format!("On loan, due {}", spoken(due)).as_str()));

    click(cx, window, "detail-held");
    click(cx, window, "confirm-ok");
    cx.run_until_parked();
    assert_eq!(notice(cx, &state).as_deref(), Some(RENEW_REFUSAL));
    assert_eq!(loan(&demo, UNRENEWABLE_LOAN_IRN).fields.get("Due"), item.fields.get("Due"));
    assert_eq!(label(cx, window, "detail-held").as_deref(), Some(format!("On loan, due {}", spoken(due)).as_str()));
}

#[gpui_kit::test]
fn an_overdue_loan_says_so(cx: &mut TestAppContext) {
    let (_state, demo) = demo_app(cx, ScriptedBackend::new());
    // "The long night", two days overdue.
    let item = loan(&demo, "18000001");
    let (window, _) = open_detail(cx, BookRef::from_patron_item(&item));
    let label = label(cx, window, "detail-held").unwrap_or_default();
    assert!(label.starts_with("On loan, overdue, was due "), "{label}");
}

#[gpui_kit::test]
fn reservation_pills_open_your_books(cx: &mut TestAppContext) {
    let (state, demo) = demo_app(cx, ScriptedBackend::new());
    let fixtures = demo.fixtures();
    // "Hollow bones": pending, to collect from Riverton Library.
    let pending = fixtures.pending.iter().find(|item| item.irn.as_deref() == Some("19000003")).unwrap().clone();
    let (window, _) = open_detail(cx, BookRef::from_patron_item(&pending));
    assert_eq!(label(cx, window, "detail-held").as_deref(), Some("Reserved, collect from Riverton Library"));
    click(cx, window, "detail-held");
    assert_eq!(state.read_with(cx, |state, _| state.section()), Section::YourBooks);
    assert_eq!(state.update(cx, |state, _| state.take_pending_segment()), Some(YourBooksSegment::Reservations));
    assert!(!exists(cx, window, "confirm-ok"), "no renewal for a reservation");

    // "The midnight library": ready to collect.
    let ready = fixtures.ready[0].clone();
    let (window, _) = open_detail(cx, BookRef::from_patron_item(&ready));
    assert_eq!(label(cx, window, "detail-held").as_deref(), Some("Ready to collect at Riverton Library"));
}
