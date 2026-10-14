//! Headless UI tests of Reserve (docs/desktop-app.md §9) against the demo account: the dialog's
//! options, Confirm adding a pending reservation, the library's refusal shown as data, and signing
//! in first when signed out.

mod detail_support;

use std::time::Duration;

use detail_support::*;
use gpui_kit::TestAppContext;
use librespydus_core::{RecordFormat, RecordRelated, SearchRecord};
use librespydus_desktop::backend::demo::{PASSWORD, USERNAME};
use librespydus_desktop::state::BookRef;

/// A book the demo account has neither on loan nor reserved.
fn reservable(backend: &ScriptedBackend) -> BookRef {
    let availability_url = format!("{ORIGIN}/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/1?FMT=RECFMT&QRY=BK");
    let search = SearchRecord {
        work_irn: Some("15974354".into()),
        title: Some("Jessi Ramsey, pet-sitter".into()),
        author: Some("Crenshaw, Ellen T.".into()),
        detail_url: Some(format!("{ORIGIN}/cgi-bin/spydus.exe/FULL/OPAC/ALLWRKENQ/1/15974354,1")),
        formats: vec![RecordFormat {
            code: "BK".into(),
            label: Some("Book".into()),
            reserve_url: Some(format!(
                "{ORIGIN}/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/1/0/V/0/R/9/ALLWRKENQ?SVL=15974354&RECFMT=BK"
            )),
            availability_url: Some(availability_url.clone()),
        }],
        ..Default::default()
    };
    backend
        .record(&availability_url, record("19300001", "300001", "Jessi Ramsey, pet-sitter", RecordRelated::default()));
    backend.availability(&availability_url, counts(0, 2, "19300001"));
    BookRef::from_search_record(&search)
}

#[gpui_kit::test]
fn reserve_places_a_reservation_and_shows_a_refusal(cx: &mut TestAppContext) {
    let public = ScriptedBackend::new();
    let book = reservable(&public);
    let (state, demo) = demo_app(cx, public);
    let pending_before = demo.fixtures().pending.len();
    let (window, _) = open_detail(cx, book);
    assert!(label(cx, window, "detail-reserve").is_some(), "Reserve once the edition is known not to be held");

    click(cx, window, "detail-reserve");
    assert!(exists(cx, window, "reserve-dialog"));
    assert_eq!(label(cx, window, "reserve-collect-from").as_deref(), Some("Collect from · Riverton Library"));
    assert_eq!(
        label(cx, window, "reserve-availability").as_deref(),
        Some("Availability · Canning - Reservable items 3, available 0")
    );
    let selected = |cx: &mut TestAppContext, id: &str| {
        let id = id.to_owned();
        with_frame(cx, window, move |window, _| {
            use gpui_kit::test::TestWindowExt;
            window.find(id).selected()
        })
    };
    assert_eq!(selected(cx, "reserve-expiry-0"), Some(true), "the first expiry choice by default");
    click(cx, window, "reserve-expiry-1");
    assert_eq!(selected(cx, "reserve-expiry-1"), Some(true));
    assert_eq!(selected(cx, "reserve-expiry-0"), Some(false));

    click(cx, window, "reserve-confirm");
    assert_eq!(label(cx, window, "reserve-result").as_deref(), Some("Reservation placed"));
    assert_eq!(demo.fixtures().pending.len(), pending_before + 1, "a pending reservation was added");
    assert!(!exists(cx, window, "reserve-confirm"), "a placed reservation can't be sent twice");
    let pending = state
        .read_with(cx, |state, _| state.account().snapshot().map(|snapshot| snapshot.reservations.pending.items.len()));
    assert_eq!(pending, Some(pending_before + 1), "the account was fetched again");
    click(cx, window, "reserve-close");
    assert!(!exists(cx, window, "reserve-dialog"));

    // The demo library already has this one: `ok: false` is shown, not thrown.
    click(cx, window, "detail-reserve");
    click(cx, window, "reserve-confirm");
    assert_eq!(label(cx, window, "reserve-result").as_deref(), Some("You already have a reservation for this title."));
    assert_eq!(demo.fixtures().pending.len(), pending_before + 1);
    assert!(exists(cx, window, "reserve-close"));
}

#[gpui_kit::test]
fn reserve_signed_out_asks_to_sign_in_then_opens_the_dialog(cx: &mut TestAppContext) {
    let public = ScriptedBackend::new();
    let book = reservable(&public);
    let (state, _demo) = demo_app(cx, public);
    state.update(cx, |state, cx| state.sign_out(cx));
    let (window, _) = open_detail(cx, book);

    click(cx, window, "detail-reserve");
    assert!(state.update(cx, |state, _| state.take_sign_in_request()), "sign-in was asked for");
    assert!(!exists(cx, window, "reserve-dialog"));

    let sign_in = state.update(cx, |state, cx| state.sign_in(USERNAME.into(), PASSWORD.into(), cx));
    cx.run_until_parked();
    cx.foreground_executor().block_test(sign_in).unwrap();
    cx.executor().advance_clock(Duration::from_secs(1));
    cx.run_until_parked();
    assert!(exists(cx, window, "reserve-dialog"), "the reservation carries on once signed in");
    assert_eq!(label(cx, window, "reserve-collect-from").as_deref(), Some("Collect from · Riverton Library"));
}
