//! Headless UI tests of My Picks (docs/desktop-app.md §9): signed out, books saved on this computer
//! (added and removed, persisted in a temp data dir); signed in, the demo account's Pick list
//! (remove from the menu); and the copy-once offer after a sign-in.

mod support;

use std::sync::Arc;

use gpui_kit::base::Root;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{AppContext, Entity, TestAppContext, Window, WindowHandle};
use librespydus_desktop::backend::LibraryBackend;
use librespydus_desktop::backend::demo::{DemoBackend, DemoDelays, DemoFixtures, SEED_PICK_EDITION, SEED_PICK_WORK};
use librespydus_desktop::backend::fake::{FAKE_PASSWORD, FakeBackend};
use librespydus_desktop::state::formatting::today;
use librespydus_desktop::state::library::canning;
use librespydus_desktop::state::{AppState, BookRef, Dependencies, LocalPicksStore, Paths, Section};

const ORIGIN: &str = "https://canning.spydus.com";

fn ui(cx: &mut TestAppContext, window: WindowHandle<Root>, f: impl FnOnce(&mut Window, &mut gpui_kit::App)) {
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        f(window, cx);
    })
    .unwrap();
    cx.run_until_parked();
}

fn choose(window: &mut Window, index: usize, cx: &mut gpui_kit::App) {
    window.render_frame(cx);
    window.within("popup-menu").click(index, cx);
}

fn book(id: &str, title: &str) -> BookRef {
    let mut book = BookRef::new(id, title);
    book.author = Some("Haig, Matt, author".into());
    book
}

fn notice(state: &Entity<AppState>, cx: &mut TestAppContext) -> Option<String> {
    state.read_with(cx, |state, _| state.notice().map(|notice| notice.message.clone()))
}

#[gpui_kit::test]
fn signed_out_picks_live_on_this_computer(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeBackend::new(ORIGIN);
    let mut deps = support::fake_deps(&fake);
    deps.paths = Paths::new(Some(dir.path()));
    let state = support::app_state(cx, deps);
    state.update(cx, |state, cx| state.select_library(canning(), cx));
    let window = support::open_root(cx);
    state.update(cx, |state, cx| state.select(Section::MyPicks, cx));
    ui(cx, window, |window, _| {
        assert_eq!(window.find("my-picks-subtitle").label(), Some("Saved on this computer"));
        assert!(window.try_find("my-picks-empty").is_some());
    });

    // Saved from a book page (the bookmark): newest first, on disk.
    state.update(cx, |state, cx| {
        state.set_picked(book("work:1", "First"), true, cx).detach();
        state.set_picked(book("bib:2", "Second"), true, cx).detach();
    });
    ui(cx, window, |window, _| {
        assert!(window.try_find("my-picks-empty").is_none());
        assert!(window.find("pick-bib:2").bounds().origin.x < window.find("pick-work:1").bounds().origin.x);
    });
    let saved = || {
        let mut store = LocalPicksStore::new(Paths::new(Some(dir.path())).my_picks_dir());
        store.load(Some(ORIGIN));
        store.items().iter().map(|book| book.id.clone()).collect::<Vec<_>>()
    };
    assert_eq!(saved(), ["bib:2", "work:1"]);

    // Removed from the "…" menu (Open, Remove from My Picks).
    ui(cx, window, |window, cx| window.click("pick-work:1-more", cx));
    ui(cx, window, |window, cx| {
        assert_eq!(window.within("popup-menu").find(1usize).label(), Some("Remove from My Picks"));
        choose(window, 1, cx);
    });
    ui(cx, window, |window, _| assert!(window.try_find("pick-work:1").is_none()));
    assert_eq!(notice(&state, cx).as_deref(), Some("Removed from My Picks"));
    assert_eq!(saved(), ["bib:2"]);

    // Right-click → Open pushes the book page on My Picks' stack.
    ui(cx, window, |window, cx| window.right_click("pick-bib:2-cell", cx));
    ui(cx, window, |window, cx| choose(window, 0, cx));
    assert_eq!(state.read_with(cx, |state, _| state.stack(Section::MyPicks).len()), 1);
}

#[gpui_kit::test]
fn signed_in_shows_the_account_list_and_removes_from_it(cx: &mut TestAppContext) {
    let fixtures = DemoFixtures::bundled(ORIGIN, today()).unwrap();
    let demo = DemoBackend::new(FakeBackend::new(ORIGIN), fixtures, DemoDelays::NONE, false);
    let backend = demo.clone();
    let mut deps = Dependencies::in_memory(Arc::new(move |_| -> Arc<dyn LibraryBackend> { Arc::new(backend.clone()) }));
    deps.is_demo = true;
    let state = support::app_state(cx, deps);
    state.update(cx, |state, cx| state.select_library(canning(), cx));
    let window = support::open_root(cx);
    state.update(cx, |state, cx| state.select(Section::MyPicks, cx));
    cx.run_until_parked();
    let work = format!("pick-work:{SEED_PICK_WORK}");
    let edition = format!("pick-work:{SEED_PICK_EDITION}");
    ui(cx, window, |window, _| {
        assert_eq!(window.find("my-picks-subtitle").label(), Some("Saved on your library account"));
        assert!(window.try_find(work.clone()).is_some());
        assert!(window.try_find(edition.clone()).is_some());
    });
    ui(cx, window, |window, cx| window.right_click(format!("{work}-cell"), cx));
    ui(cx, window, |window, cx| choose(window, 1, cx));
    ui(cx, window, |window, _| {
        assert!(window.try_find(work.clone()).is_none());
        assert!(window.try_find(edition.clone()).is_some());
    });
    assert_eq!(notice(&state, cx).as_deref(), Some("Removed from My Picks"));
    assert!(state.read_with(cx, |state, _| !state.is_picked(&BookRef::new(format!("work:{SEED_PICK_WORK}"), "x"))));

    // Removing the last one leaves the account's empty state.
    ui(cx, window, |window, cx| window.click(format!("{edition}-more"), cx));
    ui(cx, window, |window, cx| choose(window, 1, cx));
    state.update(cx, |state, cx| state.refresh_picks(cx).detach());
    cx.run_until_parked();
    ui(cx, window, |window, _| assert!(window.try_find("my-picks-empty").is_some()));
}

#[gpui_kit::test]
fn the_copy_offer_follows_a_sign_in_with_local_picks(cx: &mut TestAppContext) {
    let fake = FakeBackend::new(ORIGIN);
    let (state, window) = support::signed_out_app(cx, &fake);
    state.update(cx, |state, cx| {
        state.set_picked(book("work:111", "Old"), true, cx).detach();
        state.set_picked(book("bib:222", "New"), true, cx).detach();
    });
    let sign_in = state.update(cx, |state, cx| state.sign_in("someone".into(), FAKE_PASSWORD.into(), cx));
    cx.run_until_parked();
    cx.foreground_executor().block_test(sign_in).unwrap();
    ui(cx, window, |window, cx| {
        assert!(window.try_find("picks-offer").is_some());
        assert_eq!(window.find("picks-offer-add").label(), Some("Add"));
        window.click("picks-offer-add", cx);
    });
    cx.run_until_parked();
    ui(cx, window, |window, _| assert!(window.try_find("picks-offer").is_none()));
    // Oldest first, both on the account; the local list is kept; never asked again.
    assert_eq!(fake.picks_list(), ["111", "222"]);
    assert_eq!(notice(&state, cx).as_deref(), Some("Added 2 books to My Picks"));
    state.read_with(cx, |state, _| {
        assert!(state.picks_copy_offer().is_none());
        assert_eq!(state.local_picks().items().len(), 2);
    });
    state.update(cx, |state, cx| state.sign_out(cx));
    let again = state.update(cx, |state, cx| state.sign_in("someone".into(), FAKE_PASSWORD.into(), cx));
    cx.run_until_parked();
    cx.foreground_executor().block_test(again).unwrap();
    assert!(state.read_with(cx, |state, _| state.picks_copy_offer().is_none()));
}

#[gpui_kit::test]
fn not_now_answers_the_offer_without_copying(cx: &mut TestAppContext) {
    let fake = FakeBackend::new(ORIGIN);
    let (state, window) = support::signed_out_app(cx, &fake);
    state.update(cx, |state, cx| state.set_picked(book("work:111", "Old"), true, cx).detach());
    let sign_in = state.update(cx, |state, cx| state.sign_in("someone".into(), FAKE_PASSWORD.into(), cx));
    cx.run_until_parked();
    cx.foreground_executor().block_test(sign_in).unwrap();
    ui(cx, window, |window, cx| window.click("picks-offer-not-now", cx));
    assert!(fake.picks_list().is_empty());
    assert!(state.read_with(cx, |state, _| state.picks_copy_offer().is_none()));
}
