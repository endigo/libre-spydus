//! Headless UI tests of the alerts (docs/native-apps.md §5.8): Profile's Notifications section
//! (switches, permission asked, the blocked state, hidden when signed out), a reservation that
//! becomes ready raising one alert, a clicked alert opening the book over Your Books, sign-out
//! forgetting the library, and the morning check after the Mac slept.

mod support;

use std::rc::Rc;
use std::sync::Arc;

use chrono::NaiveDateTime;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{AppContext, Entity, ScrollDelta, TestAppContext, WindowHandle, point, px};
use librespydus_desktop::backend::LibraryBackend;
use librespydus_desktop::backend::demo::{DemoBackend, DemoDelays, DemoFixtures};
use librespydus_desktop::backend::fake::FakeBackend;
use librespydus_desktop::features::alerts::debug::NewReadyBackend;
use librespydus_desktop::features::alerts::{self, AlertKind, FakeSystem, Permission};
use librespydus_desktop::state::library::canning;
use librespydus_desktop::state::{AppState, Dependencies, Route, Section};

const CANNING: &str = "https://canning.spydus.com";

/// Demo mode at Canning, whose second account snapshot has "Hollow bones" ready, on Profile.
fn demo_app(cx: &mut TestAppContext, system: Rc<FakeSystem>) -> (Entity<AppState>, WindowHandle<gpui_kit::base::Root>) {
    let demo = DemoBackend::new(
        FakeBackend::new(CANNING),
        DemoFixtures::bundled(CANNING, chrono::Local::now().date_naive()).unwrap(),
        DemoDelays::NONE,
        false,
    );
    let backend: Arc<dyn LibraryBackend> = Arc::new(NewReadyBackend::new(Arc::new(demo), 0));
    let mut deps = Dependencies::in_memory(Arc::new(move |_| backend.clone()));
    deps.is_demo = true;
    let state = support::app_state(cx, deps);
    cx.update(|cx| {
        alerts::center(cx).update(cx, |center, _| {
            center.set_system(system);
            center.set_allow_demo(true);
            // After 07:45, so due reminders may show.
            center.set_clock(Rc::new(|| at("2026-10-15 09:00")));
        })
    });
    state.update(cx, |state, cx| state.select_library(canning(), cx));
    let window = support::open_root(cx);
    state.update(cx, |state, cx| state.select(Section::Profile, cx));
    cx.run_until_parked();
    (state, window)
}

fn at(text: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M").unwrap()
}

fn show_section(window: WindowHandle<gpui_kit::base::Root>, cx: &mut TestAppContext) {
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        window.scroll("profile-screen", ScrollDelta::Pixels(point(px(0.), px(-400.))), cx);
    })
    .unwrap();
}

fn click(window: WindowHandle<gpui_kit::base::Root>, id: &'static str, cx: &mut TestAppContext) {
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        window.click(id, cx);
    })
    .unwrap();
    cx.run_until_parked();
}

fn checked(window: WindowHandle<gpui_kit::base::Root>, id: &'static str, cx: &mut TestAppContext) -> Option<bool> {
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        window.find(id).checked()
    })
    .unwrap()
}

#[gpui_kit::test]
fn the_switches_ask_for_permission_and_turn_on(cx: &mut TestAppContext) {
    let system = FakeSystem::new(Permission::NotDetermined, true);
    let (_state, window) = demo_app(cx, system.clone());
    show_section(window, cx);
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("alerts-header").is_some());
        assert_eq!(window.find("alerts-ready-switch").label(), Some("Ready to collect"));
        assert!(window.try_find("alerts-blocked").is_none());
    })
    .unwrap();
    assert_eq!(checked(window, "alerts-ready-switch", cx), Some(false));
    assert_eq!(checked(window, "alerts-due-switch", cx), Some(false));

    click(window, "alerts-ready-switch", cx);
    assert_eq!(system.requests.get(), 1);
    assert_eq!(checked(window, "alerts-ready-switch", cx), Some(true));
    click(window, "alerts-due-switch", cx);
    assert_eq!(checked(window, "alerts-due-switch", cx), Some(true));
    let (ready, due, keys) = cx.update(|cx| {
        let center = alerts::center(cx);
        let settings = center.read(cx).settings();
        (settings.ready_enabled, settings.due_enabled, settings.alerted_keys(CANNING).map(<[String]>::to_vec))
    });
    assert!(ready && due);
    // What was already waiting is remembered, not announced.
    assert_eq!(keys, Some(vec!["19000001".to_owned()]));
    let posted = cx.update(|cx| alerts::center(cx).read(cx).posted().to_vec());
    assert!(posted.iter().all(|posted| posted.note.tap.kind == AlertKind::Due));

    // Off again.
    click(window, "alerts-ready-switch", cx);
    assert_eq!(checked(window, "alerts-ready-switch", cx), Some(false));
}

#[gpui_kit::test]
fn refused_permission_keeps_the_switch_off_and_says_so(cx: &mut TestAppContext) {
    let system = FakeSystem::new(Permission::NotDetermined, false);
    let (_state, window) = demo_app(cx, system);
    show_section(window, cx);
    click(window, "alerts-ready-switch", cx);
    assert_eq!(checked(window, "alerts-ready-switch", cx), Some(false));
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find("alerts-blocked").label(), Some("Notifications are turned off for LibreSpydus."));
        assert_eq!(window.find("alerts-open-settings").label(), Some("Open System Settings"));
    })
    .unwrap();
    assert!(cx.update(|cx| !alerts::center(cx).read(cx).settings().ready_enabled));
}

#[gpui_kit::test]
fn the_section_shows_only_while_signed_in(cx: &mut TestAppContext) {
    let fake = FakeBackend::new(CANNING);
    let (state, window) = support::signed_out_app(cx, &fake);
    state.update(cx, |state, cx| state.select(Section::Profile, cx));
    cx.run_until_parked();
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("profile-signed-out").is_some());
        assert!(window.try_find("alerts-section").is_none());
    })
    .unwrap();
}

#[gpui_kit::test]
fn a_newly_ready_reservation_raises_one_alert_that_opens_the_book(cx: &mut TestAppContext) {
    // No notification center (an unbundled build): the alert is an in-app toast.
    let system = FakeSystem::new(Permission::Unavailable, true);
    let (state, window) = demo_app(cx, system);
    show_section(window, cx);
    click(window, "alerts-ready-switch", cx);
    assert_eq!(checked(window, "alerts-ready-switch", cx), Some(true));

    // The next refresh: "Hollow bones" is ready.
    state.update(cx, |state, cx| state.refresh_account(true, cx)).detach();
    cx.run_until_parked();
    let posted = cx.update(|cx| alerts::center(cx).read(cx).posted().to_vec());
    let ready: Vec<_> = posted.iter().filter(|posted| posted.note.tap.kind == AlertKind::Ready).collect();
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].note.body, "Hollow bones · Riverton Library");
    assert!(!ready[0].to_system);
    // Again: nothing new.
    state.update(cx, |state, cx| state.refresh_account(true, cx)).detach();
    cx.run_until_parked();
    let again = cx.update(|cx| alerts::center(cx).read(cx).posted().len());
    assert_eq!(again, posted.len());

    // Clicked: Your Books › Reservations › the book.
    let tap = ready[0].note.tap.clone();
    cx.update(|cx| alerts::open_alert(&tap, cx));
    cx.run_until_parked();
    state.read_with(cx, |state, _| {
        assert_eq!(state.section(), Section::YourBooks);
        match state.stack(Section::YourBooks) {
            [Route::Detail(book)] => assert_eq!(book.title, "Hollow bones"),
            other => panic!("expected the book page, got {other:?}"),
        }
    });
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
    })
    .unwrap();

    // A click from another library only opens the list.
    let mut elsewhere = tap.clone();
    elsewhere.origin = "https://hutt.spydus.co.nz".into();
    state.update(cx, |state, cx| state.select(Section::Home, cx));
    cx.update(|cx| alerts::open_alert(&elsewhere, cx));
    state.read_with(cx, |state, _| {
        assert_eq!(state.section(), Section::YourBooks);
        assert!(state.stack(Section::YourBooks).is_empty());
    });
}

#[gpui_kit::test]
fn sign_out_forgets_the_library(cx: &mut TestAppContext) {
    let system = FakeSystem::new(Permission::Allowed, true);
    let (state, window) = demo_app(cx, system);
    show_section(window, cx);
    click(window, "alerts-ready-switch", cx);
    assert!(cx.update(|cx| alerts::center(cx).read(cx).settings().alerted_keys(CANNING).is_some()));
    state.update(cx, |state, cx| state.sign_out(cx));
    cx.run_until_parked();
    let (enabled, entry) = cx.update(|cx| {
        let center = alerts::center(cx);
        let settings = center.read(cx).settings();
        (settings.ready_enabled, settings.library(CANNING).cloned())
    });
    assert!(enabled, "the switches are app-wide");
    assert_eq!(entry, None);
}

#[gpui_kit::test]
fn the_morning_check_runs_once_after_the_mac_slept(cx: &mut TestAppContext) {
    let system = FakeSystem::new(Permission::Allowed, true);
    let (_state, _window) = demo_app(cx, system);
    let now = Rc::new(std::cell::Cell::new(at("2026-10-14 23:00")));
    let clock = now.clone();
    cx.update(|cx| alerts::center(cx).update(cx, |center, _| center.set_clock(Rc::new(move || clock.get()))));
    let take = |cx: &mut TestAppContext| {
        cx.update(|cx| alerts::center(cx).update(cx, |center, _| center.take_morning_check()))
    };
    assert_eq!(cx.update(|cx| alerts::center(cx).read(cx).next_check()), at("2026-10-15 07:45"));
    assert!(!take(cx));
    // Asleep until Friday 09:10: once, then tomorrow's.
    now.set(at("2026-10-16 09:10"));
    assert!(take(cx));
    assert!(!take(cx));
    assert_eq!(cx.update(|cx| alerts::center(cx).read(cx).next_check()), at("2026-10-17 07:45"));
}
