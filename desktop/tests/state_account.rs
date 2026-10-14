//! AccountStore (docs/native-apps.md §5.2a): the saved account shows at once while a fresh one
//! loads; a failed refresh keeps it; a fetch that overlaps a change is dropped and fetched again.

mod support;

use std::sync::Arc;

use gpui_kit::TestAppContext;
use librespydus_core::SpydusError;
use librespydus_desktop::backend::fake::{FAKE_SESSION, FakeBackend, test_data};
use librespydus_desktop::state::account_cache::AccountSnapshotCache;
use librespydus_desktop::state::library::canning;
use librespydus_desktop::state::settings::Settings;
use librespydus_desktop::state::{MemorySecretStore, Paths, SecretStore};

const ORIGIN: &str = "https://canning.spydus.com";

/// A data dir with Canning selected, a saved account `saved_name` fetched an hour ago, and a
/// session in memory.
fn saved_account(
    dir: &std::path::Path,
    fake: &Arc<FakeBackend>,
    saved_name: &str,
) -> librespydus_desktop::state::Dependencies {
    let paths = Paths::new(Some(dir));
    let mut settings = Settings::load(paths.settings_file());
    settings.set_library(&canning());
    let hour_ago = chrono::Utc::now().timestamp_millis() - 60 * 60 * 1000;
    AccountSnapshotCache::new(paths.account_cache_dir().unwrap())
        .save(&test_data::snapshot_at(Some(saved_name), &["Saved loan"], &[], hour_ago), ORIGIN)
        .unwrap();
    let sessions = MemorySecretStore::new();
    sessions.set_value(ORIGIN, Some(FAKE_SESSION));
    let mut deps = support::fake_deps(fake);
    deps.paths = paths;
    deps.sessions = Arc::new(sessions);
    deps
}

#[gpui_kit::test]
fn the_saved_account_shows_at_once_and_a_fresh_one_replaces_it(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeBackend::new(ORIGIN);
    fake.hold_snapshots(true);
    let state = support::app_state(cx, saved_account(dir.path(), &fake, "Saved Name"));
    cx.run_until_parked();

    // Launch: the saved snapshot is on screen while the stale one is refreshed.
    state.read_with(cx, |state, _| {
        assert!(state.is_signed_in());
        assert_eq!(state.account_name().as_deref(), Some("Saved Name"));
        assert!(state.account().is_refreshing());
        assert!(state.account().persists());
    });
    assert_eq!(fake.snapshot_calls(), 1);
    // A second refresh joins the one running.
    let _joined = state.update(cx, |state, cx| state.refresh_account(true, cx));
    cx.run_until_parked();
    assert_eq!(fake.snapshot_calls(), 1);

    assert!(fake.release_snapshot(Ok(test_data::snapshot(Some("Fresh Name"), &["A", "B"], &[]))));
    cx.run_until_parked();
    state.read_with(cx, |state, _| {
        assert_eq!(state.account_name().as_deref(), Some("Fresh Name"));
        assert!(!state.account().is_refreshing());
        assert!(state.account().last_refresh_error().is_none());
        assert!(!state.account().needs_refresh());
    });
    // …and saved on disk for the next launch.
    let cache = AccountSnapshotCache::new(Paths::new(Some(dir.path())).account_cache_dir().unwrap());
    assert_eq!(cache.load(ORIGIN).unwrap().account_name.as_deref(), Some("Fresh Name"));

    // A failed refresh keeps the snapshot and says why.
    let refresh = state.update(cx, |state, cx| state.refresh_account(true, cx));
    cx.run_until_parked();
    fake.release_snapshot(Err(SpydusError::Network { message: "offline".into() }));
    cx.run_until_parked();
    cx.foreground_executor().block_test(refresh);
    state.read_with(cx, |state, _| {
        assert_eq!(state.account_name().as_deref(), Some("Fresh Name"));
        let error = state.account().last_refresh_error().expect("an error");
        assert_eq!(error.message, "Can’t reach City of Canning. Check your connection.");
        assert!(state.account().needs_refresh());
    });

    // NotLoggedIn signs out and deletes the saved account.
    let _refresh = state.update(cx, |state, cx| state.refresh_account(true, cx));
    cx.run_until_parked();
    fake.release_snapshot(Err(SpydusError::NotLoggedIn));
    cx.run_until_parked();
    state.read_with(cx, |state, _| {
        assert!(!state.is_signed_in());
        assert!(state.account().snapshot().is_none());
    });
    assert!(cache.load(ORIGIN).is_none());
}

#[gpui_kit::test]
fn a_fetch_overlapping_a_change_is_dropped_and_fetched_again(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeBackend::new(ORIGIN);
    fake.hold_snapshots(true);
    let state = support::app_state(cx, saved_account(dir.path(), &fake, "Saved Name"));
    cx.run_until_parked();
    assert_eq!(fake.held_snapshots(), 1);

    // A reservation is cancelled while that fetch is in flight.
    let (sender, receiver) = futures::channel::oneshot::channel::<Result<bool, SpydusError>>();
    let change = state.update(cx, |state, cx| {
        state.perform_account_change(async move { receiver.await.unwrap() }, |accepted| *accepted, cx)
    });
    // The fetch comes back while the change runs: dropped, nothing fetched yet.
    fake.release_snapshot(Ok(test_data::snapshot(Some("Before the change"), &[], &["Cancelled"])));
    cx.run_until_parked();
    assert_eq!(state.read_with(cx, |state, _| state.account_name()).as_deref(), Some("Saved Name"));
    assert_eq!(fake.held_snapshots(), 0);

    // The library accepts the change: the account is fetched again.
    sender.send(Ok(true)).unwrap();
    cx.run_until_parked();
    assert!(cx.foreground_executor().block_test(change).unwrap());
    assert_eq!(fake.held_snapshots(), 1);
    fake.release_snapshot(Ok(test_data::snapshot(Some("After the change"), &[], &[])));
    cx.run_until_parked();
    state.read_with(cx, |state, _| {
        assert_eq!(state.account_name().as_deref(), Some("After the change"));
        assert!(!state.account().is_refreshing());
    });

    // update() edits the snapshot in place until the next fetch.
    state.update(cx, |state, cx| {
        state.update_account_snapshot(|snapshot| snapshot.account_name = Some("Edited".into()), cx)
    });
    assert_eq!(state.read_with(cx, |state, _| state.account_name()).as_deref(), Some("Edited"));
    assert_eq!(fake.snapshot_calls(), 2);
}

#[gpui_kit::test]
fn signed_out_picks_stay_on_this_computer(cx: &mut TestAppContext) {
    let fake = FakeBackend::new(ORIGIN);
    let (state, _window) = support::signed_out_app(cx, &fake);
    let book = librespydus_desktop::state::BookRef::new("work:15642035", "The Midnight Library");
    let saved = state.update(cx, |state, cx| state.set_picked(book.clone(), true, cx));
    assert!(cx.foreground_executor().block_test(saved));
    assert!(state.read_with(cx, |state, _| state.is_picked(&book)));
    assert!(fake.pick_calls().is_empty());

    // Signed in, the account's list is used, and the local picks are offered once.
    let sign_in = state.update(cx, |state, cx| state.sign_in("x".into(), "ok".into(), cx));
    cx.run_until_parked();
    cx.foreground_executor().block_test(sign_in).unwrap();
    assert_eq!(state.read_with(cx, |state, _| state.picks_copy_offer().map(|offer| offer.count)), Some(1));
    assert!(!state.read_with(cx, |state, _| state.is_picked(&book)));
    let answer = state.update(cx, |state, cx| state.answer_picks_copy_offer(true, cx));
    cx.run_until_parked();
    cx.foreground_executor().block_test(answer);
    assert_eq!(fake.picks_list(), ["15642035"]);
    assert!(state.read_with(cx, |state, _| state.is_picked(&book)));
    assert!(state.read_with(cx, |state, _| state.picks_copy_offer().is_none()));

    let removed = state.update(cx, |state, cx| state.set_picked(book.clone(), false, cx));
    cx.run_until_parked();
    assert!(cx.foreground_executor().block_test(removed));
    assert!(!state.read_with(cx, |state, _| state.is_picked(&book)));
    assert_eq!(fake.pick_calls(), [("15642035".to_owned(), true), ("15642035".to_owned(), false)]);
}
