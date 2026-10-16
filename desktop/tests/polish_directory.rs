//! Headless UI tests of the library directory's 7-day disk cache (iOS `LibraryDirectoryCache`):
//! the picker shows the saved list at once, fetches only when it's stale, and saves what it fetched.

mod support;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, TimeDelta, TimeZone, Utc};
use futures::FutureExt;
use futures::channel::oneshot;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{AppContext, TestAppContext};
use librespydus_core::{LibraryInfo, SpydusError};
use librespydus_desktop::backend::fake::FakeBackend;
use librespydus_desktop::features::onboarding::store::{DirectoryCache, DirectoryFetch, set_directory};

fn library(name: &str, host: &str) -> LibraryInfo {
    LibraryInfo {
        id: host.into(),
        name: name.into(),
        origin: format!("https://{host}"),
        logo_url: None,
        country: Some("Australia".into()),
        region: "AU".into(),
        is_test: false,
    }
}

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 15, 9, 0, 0).unwrap()
}

type Pending = Arc<Mutex<Vec<oneshot::Sender<Result<Vec<LibraryInfo>, SpydusError>>>>>;

/// A fetch that counts its calls and answers only when the test sends the list.
fn held_fetch() -> (DirectoryFetch, Arc<AtomicUsize>, Pending) {
    let (calls, pending) = (Arc::new(AtomicUsize::new(0)), Pending::default());
    let (counter, senders) = (calls.clone(), pending.clone());
    let fetch: DirectoryFetch = Arc::new(move || {
        counter.fetch_add(1, Ordering::SeqCst);
        let (sender, receiver) = oneshot::channel();
        senders.lock().unwrap().push(sender);
        receiver.map(|answer| answer.unwrap_or_else(|_| Ok(Vec::new()))).boxed()
    });
    (fetch, calls, pending)
}

fn open_picker(cx: &mut TestAppContext, fetch: DirectoryFetch, file: &std::path::Path) -> gpui_kit::AnyWindowHandle {
    let fake = FakeBackend::new("https://canning.spydus.com");
    support::app_state(cx, support::fake_deps(&fake));
    let file = file.to_owned();
    cx.update(|cx| set_directory(fetch, Some(file), Arc::new(now), cx));
    let window = support::open_root(cx);
    cx.run_until_parked();
    window.into()
}

fn shows(cx: &mut TestAppContext, window: gpui_kit::AnyWindowHandle, id: &str) -> bool {
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
        window.try_find(id.to_owned()).is_some()
    })
    .unwrap()
}

#[gpui_kit::test]
fn a_fresh_saved_list_shows_at_once_without_a_fetch(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("library-directory.json");
    let cache = DirectoryCache::new(file.clone());
    cache.save(now() - TimeDelta::days(6), &[library("City of Canning", "canning.spydus.com")]).unwrap();
    let (fetch, calls, _pending) = held_fetch();

    let window = open_picker(cx, fetch, &file);
    assert!(shows(cx, window, "library-canning.spydus.com"), "the saved list, at once");
    assert!(!shows(cx, window, "loading"));
    assert_eq!(calls.load(Ordering::SeqCst), 0, "six days old: no fetch");
}

#[gpui_kit::test]
fn a_stale_saved_list_shows_then_refreshes_and_is_saved_again(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("library-directory.json");
    let cache = DirectoryCache::new(file.clone());
    cache.save(now() - TimeDelta::days(7), &[library("City of Canning", "canning.spydus.com")]).unwrap();
    let (fetch, calls, pending) = held_fetch();

    let window = open_picker(cx, fetch, &file);
    // The old list stays on screen while the library answers.
    assert!(shows(cx, window, "library-canning.spydus.com"));
    assert_eq!(calls.load(Ordering::SeqCst), 1, "a week old: fetched again");

    let fresh = vec![library("City of Canning", "canning.spydus.com"), library("Fremantle", "fremantle.spydus.com")];
    pending.lock().unwrap().remove(0).send(Ok(fresh.clone())).unwrap();
    cx.run_until_parked();
    assert!(shows(cx, window, "library-fremantle.spydus.com"));
    let saved = cache.load().expect("saved");
    assert_eq!((saved.fetched_at, saved.libraries), (now(), fresh));
}

#[gpui_kit::test]
fn a_failed_refresh_keeps_the_saved_list(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("library-directory.json");
    let cache = DirectoryCache::new(file.clone());
    let old = vec![library("City of Canning", "canning.spydus.com")];
    cache.save(now() - TimeDelta::days(30), &old).unwrap();
    let (fetch, _calls, pending) = held_fetch();

    let window = open_picker(cx, fetch, &file);
    pending.lock().unwrap().remove(0).send(Err(SpydusError::Network { message: "offline".into() })).unwrap();
    cx.run_until_parked();
    assert!(shows(cx, window, "library-canning.spydus.com"), "still listed, with the banner");
    assert_eq!(cache.load().unwrap().libraries, old, "the file is left as it was");
}

#[gpui_kit::test]
fn the_first_fetch_is_saved(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("library-directory.json");
    let (fetch, calls, pending) = held_fetch();

    let window = open_picker(cx, fetch, &file);
    assert!(shows(cx, window, "loading"), "nothing saved yet: loading");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let fetched = vec![library("City of Canning", "canning.spydus.com")];
    pending.lock().unwrap().remove(0).send(Ok(fetched.clone())).unwrap();
    cx.run_until_parked();
    assert!(shows(cx, window, "library-canning.spydus.com"));
    assert_eq!(DirectoryCache::new(file).load().unwrap().libraries, fetched);
}
