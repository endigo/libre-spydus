//! Headless UI tests of Your Books (docs/desktop-app.md §9) on a `DemoBackend` with no delays and
//! faked public reads: segments, renew through the "…" menu (accepted and refused), cancel with its
//! confirmation, edit's pickup, the group headers' counts and the right-click menu.

mod support;

use std::sync::Arc;

use gpui_kit::base::Root;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{AppContext, Entity, TestAppContext, Window, WindowHandle};
use librespydus_desktop::backend::LibraryBackend;
use librespydus_desktop::backend::demo::{DemoBackend, DemoDelays, DemoFixtures, RENEW_REFUSAL};
use librespydus_desktop::backend::fake::FakeBackend;
use librespydus_desktop::state::formatting::{due_date, today};
use librespydus_desktop::state::library::canning;
use librespydus_desktop::state::{AppState, Dependencies, Route, Section};

const ORIGIN: &str = "https://canning.spydus.com";

/// The app signed in to the demo account, on Your Books, with the account loaded.
fn demo_app(cx: &mut TestAppContext) -> (Entity<AppState>, WindowHandle<Root>, DemoBackend) {
    let fixtures = DemoFixtures::bundled(ORIGIN, today()).unwrap();
    let demo = DemoBackend::new(FakeBackend::new(ORIGIN), fixtures, DemoDelays::NONE, false);
    let backend = demo.clone();
    let mut deps = Dependencies::in_memory(Arc::new(move |_| -> Arc<dyn LibraryBackend> { Arc::new(backend.clone()) }));
    deps.is_demo = true;
    let state = support::app_state(cx, deps);
    state.update(cx, |state, cx| state.select_library(canning(), cx));
    let window = support::open_root(cx);
    state.update(cx, |state, cx| state.select(Section::YourBooks, cx));
    cx.run_until_parked();
    assert!(state.read_with(cx, |state, _| state.account().snapshot().is_some()));
    (state, window, demo)
}

fn ui(cx: &mut TestAppContext, window: WindowHandle<Root>, f: impl FnOnce(&mut Window, &mut gpui_kit::App)) {
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        f(window, cx);
    })
    .unwrap();
    cx.run_until_parked();
}

/// Clicks the open menu's item `index`.
fn choose(window: &mut Window, index: usize, cx: &mut gpui_kit::App) {
    window.render_frame(cx);
    window.within("popup-menu").click(index, cx);
}

fn due_of(state: &Entity<AppState>, irn: &str, cx: &mut TestAppContext) -> String {
    state.read_with(cx, |state, _| {
        let loans = &state.account().snapshot().unwrap().loans.items;
        loans.iter().find(|loan| loan.irn.as_deref() == Some(irn)).unwrap().fields["Due"].clone()
    })
}

fn notice(state: &Entity<AppState>, cx: &mut TestAppContext) -> Option<String> {
    state.read_with(cx, |state, _| state.notice().map(|notice| notice.message.clone()))
}

#[gpui_kit::test]
fn segments_switch_and_follow_a_pending_segment(cx: &mut TestAppContext) {
    let (state, window, _) = demo_app(cx);
    ui(cx, window, |window, _| {
        assert_eq!(window.find("your-books-segment-loans").label(), Some("Loans (5)"));
        assert_eq!(window.find("your-books-segment-reservations").label(), Some("Reservations (4)"));
        assert_eq!(window.find("your-books-segment-loans").selected(), Some(true));
        assert!(window.try_find("loan-18000001").is_some());
        assert!(window.try_find("reservation-19000001").is_none());
    });
    ui(cx, window, |window, cx| window.click("your-books-segment-reservations", cx));
    ui(cx, window, |window, _| {
        assert_eq!(window.find("your-books-segment-reservations").selected(), Some(true));
        assert!(window.try_find("reservation-19000001").is_some());
        assert!(window.try_find("loan-18000001").is_none());
        // Ready first, highlighted; the headers count each group.
        assert_eq!(window.find("reservations-ready-title").label(), Some("Ready to collect (1)"));
        assert_eq!(window.find("reservations-pending-title").label(), Some("Pending (3)"));
        assert!(
            window.find("reservation-19000001").bounds().origin.y
                < window.find("reservation-19000003").bounds().origin.y
        );
    });
    ui(cx, window, |window, cx| window.click("your-books-segment-loans", cx));
    // A book page's pill asks for Reservations.
    state.update(cx, |state, cx| {
        state.select(Section::Home, cx);
        state.open_your_books(librespydus_desktop::state::YourBooksSegment::Reservations, cx);
    });
    cx.run_until_parked();
    ui(cx, window, |window, _| {
        assert_eq!(window.find("your-books-segment-reservations").selected(), Some(true));
    });
    // A card opens its book page on Your Books' stack.
    ui(cx, window, |window, cx| window.click("reservation-19000003", cx));
    let stack = state.read_with(cx, |state, _| state.stack(Section::YourBooks).to_vec());
    assert!(
        matches!(stack.as_slice(), [Route::Detail(book)] if book.id == "bib:19244690" || book.title == "Hollow bones")
    );
}

#[gpui_kit::test]
fn renew_through_the_menu_moves_the_date_or_shows_the_refusal(cx: &mut TestAppContext) {
    let (state, window, _) = demo_app(cx);
    let before = due_date::parse(&due_of(&state, "18000003", cx)).unwrap();

    ui(cx, window, |window, cx| window.click("loan-18000003-more", cx));
    ui(cx, window, |window, cx| {
        assert!(window.try_find("popup-menu").is_some());
        choose(window, 0, cx);
    });
    // The confirmation: Not now changes nothing…
    ui(cx, window, |window, cx| {
        assert!(window.try_find("confirm-ok").is_some());
        assert_eq!(window.find("confirm-ok").label(), Some("Renew"));
        window.click("confirm-cancel", cx);
    });
    assert_eq!(due_date::parse(&due_of(&state, "18000003", cx)), Some(before));
    // …Renew renews: the new date at once and the toast.
    ui(cx, window, |window, cx| window.click("loan-18000003-more", cx));
    ui(cx, window, |window, cx| choose(window, 0, cx));
    ui(cx, window, |window, cx| window.click("confirm-ok", cx));
    let after = due_date::parse(&due_of(&state, "18000003", cx)).unwrap();
    assert_eq!(after, before + chrono::Duration::days(21));
    let expected = format!("Renewed · Due {}", librespydus_desktop::state::formatting::short_date(after, today()));
    assert_eq!(notice(&state, cx), Some(expected));

    // The library refuses Whistler: its reason, the date unchanged.
    let whistler = due_of(&state, "18000002", cx);
    ui(cx, window, |window, cx| window.click("loan-18000002-more", cx));
    ui(cx, window, |window, cx| choose(window, 0, cx));
    ui(cx, window, |window, cx| window.click("confirm-ok", cx));
    assert_eq!(notice(&state, cx).as_deref(), Some(RENEW_REFUSAL));
    assert_eq!(due_of(&state, "18000002", cx), whistler);
}

#[gpui_kit::test]
fn right_click_opens_the_same_menu(cx: &mut TestAppContext) {
    let (_, window, _) = demo_app(cx);
    ui(cx, window, |window, cx| window.right_click("loan-18000004-row", cx));
    ui(cx, window, |window, _| {
        assert!(window.try_find("popup-menu").is_some());
        assert_eq!(window.within("popup-menu").find(0usize).label(), Some("Renew loan"));
    });
    ui(cx, window, |window, cx| window.press("escape", cx));
    ui(cx, window, |window, cx| window.click("your-books-segment-reservations", cx));
    ui(cx, window, |window, cx| window.right_click("reservation-19000003-row", cx));
    ui(cx, window, |window, _| {
        let menu = window.within("popup-menu");
        assert_eq!(menu.find(0usize).label(), Some("Edit reservation"));
        assert_eq!(menu.find(1usize).label(), Some("Cancel reservation"));
    });
}

#[gpui_kit::test]
fn cancel_asks_first_and_removes_the_row(cx: &mut TestAppContext) {
    let (state, window, demo) = demo_app(cx);
    ui(cx, window, |window, cx| window.click("your-books-segment-reservations", cx));
    ui(cx, window, |window, cx| window.click("reservation-19000003-more", cx));
    ui(cx, window, |window, cx| choose(window, 1, cx));
    // Keep reservation keeps it.
    ui(cx, window, |window, cx| {
        assert_eq!(window.find("confirm-ok").label(), Some("Cancel reservation"));
        assert_eq!(window.find("confirm-cancel").label(), Some("Keep reservation"));
        window.click("confirm-cancel", cx);
    });
    ui(cx, window, |window, _| assert!(window.try_find("reservation-19000003").is_some()));

    ui(cx, window, |window, cx| window.click("reservation-19000003-more", cx));
    ui(cx, window, |window, cx| choose(window, 1, cx));
    ui(cx, window, |window, cx| window.click("confirm-ok", cx));
    ui(cx, window, |window, _| {
        assert!(window.try_find("reservation-19000003").is_none());
        assert_eq!(window.find("reservations-pending-title").label(), Some("Pending (2)"));
        assert_eq!(window.find("your-books-notice").label(), Some("Reservation cancelled"));
    });
    assert!(demo.fixtures().pending.iter().all(|item| item.irn.as_deref() != Some("19000003")));
    assert_eq!(state.read_with(cx, |state, _| state.account().snapshot().unwrap().reservations.pending.items.len()), 2);
}

#[gpui_kit::test]
fn edit_changes_the_pickup(cx: &mut TestAppContext) {
    let (state, window, demo) = demo_app(cx);
    ui(cx, window, |window, cx| window.click("your-books-segment-reservations", cx));
    ui(cx, window, |window, cx| window.click("reservation-19000003-more", cx));
    ui(cx, window, |window, cx| choose(window, 0, cx));
    ui(cx, window, |window, cx| {
        assert!(window.try_find("edit-reservation").is_some());
        // The form starts on the reservation's branch; nothing to save yet.
        assert_eq!(window.find("edit-pickup-25363").checked(), Some(true));
        window.click("edit-pickup-25353", cx);
    });
    ui(cx, window, |window, cx| {
        assert_eq!(window.find("edit-pickup-25353").checked(), Some(true));
        window.click("edit-save", cx);
    });
    ui(cx, window, |window, _| {
        assert!(window.try_find("edit-reservation").is_none());
        assert_eq!(window.find("your-books-notice").label(), Some("Reservation updated"));
    });
    let pickup = |item: &librespydus_core::PatronItem| item.fields.get("Pickup location").cloned();
    let demo_row = demo.fixtures().pending.into_iter().find(|item| item.irn.as_deref() == Some("19000003")).unwrap();
    assert_eq!(pickup(&demo_row).as_deref(), Some("CANNINGTON LIBRARY"));
    // The account was fetched again: the card says where to collect it now.
    let shown = state.read_with(cx, |state, _| {
        state
            .account()
            .snapshot()
            .unwrap()
            .reservations
            .pending
            .items
            .iter()
            .find(|i| i.irn.as_deref() == Some("19000003"))
            .cloned()
    });
    assert_eq!(shown.as_ref().and_then(pickup).as_deref(), Some("CANNINGTON LIBRARY"));
}

#[gpui_kit::test]
fn signed_out_asks_to_sign_in(cx: &mut TestAppContext) {
    let fake = FakeBackend::new(ORIGIN);
    let (state, window) = support::signed_out_app(cx, &fake);
    state.update(cx, |state, cx| state.select(Section::YourBooks, cx));
    ui(cx, window, |window, cx| {
        assert!(window.try_find("your-books-signed-out").is_some());
        assert!(window.try_find("your-books-segment-loans").is_none());
        window.click("your-books-sign-in", cx);
    });
}

#[gpui_kit::test]
fn the_top_stays_put_while_the_cards_scroll(cx: &mut TestAppContext) {
    let (state, window, _demo) = demo_app(cx);
    // Enough loans to scroll: copies of the demo loans with their own IRNs.
    state.update(cx, |state, cx| {
        state.update_account_snapshot(
            |snapshot| {
                let originals = snapshot.loans.items.clone();
                for round in 1..5 {
                    for loan in &originals {
                        let mut copy = loan.clone();
                        copy.irn = loan.irn.as_ref().map(|irn| format!("{irn}{round}"));
                        snapshot.loans.items.push(copy);
                    }
                }
            },
            cx,
        )
    });
    let first = state.read_with(cx, |state, _| state.account().snapshot().unwrap().loans.items[0].irn.clone().unwrap());
    let card = format!("loan-{first}-row");
    let mut before = None;
    ui(cx, window, |window, _| {
        before = Some((window.find("your-books-refresh").bounds(), window.find(card.clone()).bounds()));
    });
    let (segments, card_before) = before.unwrap();
    ui(cx, window, |window, cx| {
        window.scroll(
            "your-books-scroll",
            gpui_kit::ScrollDelta::Pixels(gpui_kit::point(gpui_kit::px(0.), gpui_kit::px(-300.))),
            cx,
        )
    });
    ui(cx, window, |window, _| {
        assert_eq!(window.find("your-books-refresh").bounds().origin, segments.origin, "the top stayed");
        assert!(window.find(card.clone()).bounds().origin.y < card_before.origin.y, "the cards scrolled");
    });
}

#[gpui_kit::test]
fn the_tabs_sit_beside_the_title_and_the_update_time_beside_refresh(cx: &mut TestAppContext) {
    let (_state, window, _demo) = demo_app(cx);
    ui(cx, window, |window, _| {
        let middle = |id: &str| {
            let bounds = window.find(id.to_owned()).bounds();
            f32::from(bounds.origin.y + bounds.size.height / 2.)
        };
        let title = window.find("your-books-title").bounds();
        let segments = window.find("your-books-segments").bounds();
        let status = window.find("your-books-status").bounds();
        let refresh = window.find("your-books-refresh").bounds();
        for id in ["your-books-segments", "your-books-status", "your-books-refresh"] {
            assert!((middle(id) - middle("your-books-title")).abs() < 4., "{id} is on the title's row");
        }
        assert!(segments.origin.x > title.origin.x + title.size.width, "the tabs follow the title");
        assert!(status.origin.x > segments.origin.x + segments.size.width, "the time is past the tabs");
        let between = f32::from(refresh.origin.x - (status.origin.x + status.size.width));
        assert!((0. ..=24.).contains(&between), "the time is next to refresh ({between})");
    });
}
