//! The Civica library directory (iOS `LibraryPickerModel.swift`): fetched with
//! `list_libraries(false)` (one public GET, on the core's runtime) and shared by the onboarding
//! screen and the Change library dialog, so the dialog opens at once after onboarding.
//!
//! The list is also kept on disk for 7 days (iOS `LibraryDirectoryCache`), in
//! `<data>/library-directory.json`: a picker shows the saved list at once and fetches again only
//! when it's stale (or on Retry). On failure with nothing to show it's the error state (Retry); a
//! failed refresh keeps the list on screen with a banner. Tests swap the fetch and the cache with
//! `set_directory_fetch` / `set_directory`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, TimeDelta, Utc};
use futures::FutureExt;
use futures::future::BoxFuture;
use gpui_kit::{App, AppContext, Context, Entity, Global};
use librespydus_core::{LibraryInfo, SpydusError};
use serde::{Deserialize, Serialize};

use crate::copy::Copy;
use crate::runtime::spawn_core;
use crate::state::paths::write_atomically;
use crate::state::{AppState, ErrorKind, Paths, UserFacingError};

/// Fetches the directory: `list_libraries(false)` by default.
pub type DirectoryFetch = Arc<dyn Fn() -> BoxFuture<'static, Result<Vec<LibraryInfo>, SpydusError>> + Send + Sync>;

/// The clock the cache's age is measured with.
pub type Clock = Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>;

pub struct DirectoryStore {
    fetch: DirectoryFetch,
    /// The list on disk; `None` keeps it in memory only (tests, screenshots of the error states).
    cache: Option<DirectoryCache>,
    now: Clock,
    libraries: Vec<LibraryInfo>,
    /// When `libraries` came from the library (now, or the saved list's date).
    fetched_at: Option<DateTime<Utc>>,
    /// Bumped whenever `libraries` changes.
    generation: u64,
    /// A fetch came back (even an empty one): the empty state, not loading.
    loaded: bool,
    fetching: bool,
    /// The list couldn't be fetched and there's nothing to show.
    load_error: Option<UserFacingError>,
    /// A refresh failed while an older list is on screen (shown as a banner).
    refresh_error: Option<UserFacingError>,
}

struct GlobalDirectory(Entity<DirectoryStore>);

impl Global for GlobalDirectory {}

/// Replaces how the directory is fetched (tests: canned libraries, a failure), dropping any list
/// already fetched. Nothing is kept on disk.
pub fn set_directory_fetch(fetch: DirectoryFetch, cx: &mut App) {
    set_directory(fetch, None, Arc::new(Utc::now), cx);
}

/// Replaces how the directory is fetched, where it's kept (`cache_file`) and the clock (tests).
pub fn set_directory(fetch: DirectoryFetch, cache_file: Option<PathBuf>, now: Clock, cx: &mut App) {
    let store = cx.new(|_| DirectoryStore::new(fetch, cache_file.map(DirectoryCache::new), now));
    cx.set_global(GlobalDirectory(store));
}

impl DirectoryStore {
    fn new(fetch: DirectoryFetch, cache: Option<DirectoryCache>, now: Clock) -> Self {
        Self {
            fetch,
            cache,
            now,
            libraries: Vec::new(),
            fetched_at: None,
            generation: 0,
            loaded: false,
            fetching: false,
            load_error: None,
            refresh_error: None,
        }
    }

    /// The app's directory, made on first use.
    pub fn global(cx: &mut App) -> Entity<DirectoryStore> {
        if let Some(global) = cx.try_global::<GlobalDirectory>() {
            return global.0.clone();
        }
        let deps = &AppState::global(cx).read(cx).deps;
        let (cache, now) = (default_cache(&deps.paths), deps.now.clone());
        let store = cx.new(|_| DirectoryStore::new(default_fetch(), cache, now));
        cx.set_global(GlobalDirectory(store.clone()));
        store
    }

    pub fn libraries(&self) -> &[LibraryInfo] {
        &self.libraries
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Nothing to show yet and a fetch is running (or about to).
    pub fn is_loading(&self) -> bool {
        self.libraries.is_empty() && self.load_error.is_none() && (self.fetching || !self.loaded)
    }

    pub fn is_fetching(&self) -> bool {
        self.fetching
    }

    pub fn load_error(&self) -> Option<&UserFacingError> {
        self.load_error.as_ref()
    }

    pub fn refresh_error(&self) -> Option<&UserFacingError> {
        self.refresh_error.as_ref()
    }

    /// A picker appeared: shows the list on disk at once, and fetches unless a fresh list is
    /// already here (an earlier failure tries again).
    pub fn load(&mut self, cx: &mut Context<Self>) {
        if self.fetching {
            return;
        }
        if self.libraries.is_empty()
            && let Some(saved) = self.cache.as_ref().and_then(DirectoryCache::load)
            && !saved.libraries.is_empty()
        {
            self.libraries = saved.libraries;
            self.fetched_at = Some(saved.fetched_at);
            self.generation += 1;
            self.loaded = true;
            cx.notify();
        }
        let fresh = self.fetched_at.is_some_and(|at| DirectoryCache::is_fresh(at, (self.now)()));
        if self.libraries.is_empty() || !fresh {
            self.refresh(cx);
        }
    }

    /// Retry: fetches again; on failure whatever is on screen stays.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        if self.fetching {
            return;
        }
        self.fetching = true;
        if self.libraries.is_empty() {
            self.load_error = None;
        }
        cx.notify();
        let fetch = (self.fetch)();
        cx.spawn(async move |this, cx| {
            let result = fetch.await;
            let _ = this.update(cx, |this, cx| {
                this.fetching = false;
                match result {
                    Ok(libraries) => {
                        let fetched_at = (this.now)();
                        if let Some(cache) = &this.cache
                            && let Err(error) = cache.save(fetched_at, &libraries)
                        {
                            log::warn!("library directory: couldn't save it: {error}");
                        }
                        this.fetched_at = Some(fetched_at);
                        this.libraries = libraries;
                        this.generation += 1;
                        this.loaded = true;
                        this.load_error = None;
                        this.refresh_error = None;
                    }
                    Err(error) => {
                        let shown = user_facing_error(&error);
                        if this.libraries.is_empty() {
                            this.load_error = Some(shown);
                        } else {
                            this.refresh_error = Some(shown);
                        }
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }
}

/// The directory isn't a library, so the §5.7 messages name it instead.
pub fn user_facing_error(error: &SpydusError) -> UserFacingError {
    let detail = Some(error.to_string());
    match error {
        SpydusError::Network { .. } => {
            UserFacingError::new(ErrorKind::Network, Copy::Onboarding::directoryNetwork, detail)
        }
        _ => UserFacingError::new(ErrorKind::Unexpected, Copy::Onboarding::directoryUnexpected, detail),
    }
}

/// `<data>/library-directory.json`; none in memory-only runs, nor for the screenshots of the
/// directory's own error, empty and loading states.
fn default_cache(paths: &Paths) -> Option<DirectoryCache> {
    #[cfg(feature = "evidence")]
    if super::evidence_step("onboarding").is_some_and(|step| step.starts_with("directory-")) {
        return None;
    }
    paths.data.as_ref().map(|dir| DirectoryCache::new(dir.join("library-directory.json")))
}

fn default_fetch() -> DirectoryFetch {
    Arc::new(|| {
        #[cfg(feature = "evidence")]
        if let Some(fetch) = evidence_fetch() {
            return fetch;
        }
        spawn_core(librespydus_core::list_libraries(false)).boxed()
    })
}

/// Screenshots of the error, empty and loading states (`onboarding::evidence_step`): a failed
/// fetch, an empty directory, or one that never answers. Nothing reaches the network.
#[cfg(feature = "evidence")]
fn evidence_fetch() -> Option<BoxFuture<'static, Result<Vec<LibraryInfo>, SpydusError>>> {
    match super::evidence_step("onboarding")?.as_str() {
        "directory-error" => Some(
            futures::future::ready(Err(SpydusError::Network { message: "evidence: directory-error".into() })).boxed(), // copy-exempt: evidence-only error detail
        ),
        "directory-empty" => Some(futures::future::ready(Ok(Vec::new())).boxed()),
        "directory-loading" => Some(futures::future::pending().boxed()),
        _ => None,
    }
}

/// The directory on disk (iOS `LibraryDirectoryCache`), so the picker opens at once and offline
/// after the first fetch.
#[derive(Debug, Clone)]
pub struct DirectoryCache {
    file: PathBuf,
}

/// What `DirectoryCache::load` read back.
#[derive(Debug, Clone, PartialEq)]
pub struct SavedDirectory {
    pub fetched_at: DateTime<Utc>,
    pub libraries: Vec<LibraryInfo>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredDirectory {
    version: u32,
    fetched_at: DateTime<Utc>,
    libraries: Vec<LibraryInfo>,
}

impl DirectoryCache {
    /// Kept for a week, as on iOS.
    pub const MAX_AGE: TimeDelta = TimeDelta::days(7);
    const VERSION: u32 = 1;

    pub fn new(file: PathBuf) -> Self {
        Self { file }
    }

    pub fn file(&self) -> &Path {
        &self.file
    }

    /// Fresh when fetched less than `MAX_AGE` ago. A date in the future (the clock moved back) is
    /// stale, so a wrong clock can't pin an old list forever.
    pub fn is_fresh(fetched_at: DateTime<Utc>, now: DateTime<Utc>) -> bool {
        let age = now - fetched_at;
        age >= TimeDelta::zero() && age < Self::MAX_AGE
    }

    /// The saved directory, or `None` when there's none or it can't be read.
    pub fn load(&self) -> Option<SavedDirectory> {
        let bytes = std::fs::read(&self.file).ok()?;
        let stored: StoredDirectory = serde_json::from_slice(&bytes).ok()?;
        (stored.version == Self::VERSION)
            .then_some(SavedDirectory { fetched_at: stored.fetched_at, libraries: stored.libraries })
    }

    pub fn save(&self, fetched_at: DateTime<Utc>, libraries: &[LibraryInfo]) -> std::io::Result<()> {
        let stored = StoredDirectory { version: Self::VERSION, fetched_at, libraries: libraries.to_vec() };
        write_atomically(&self.file, &serde_json::to_vec(&stored)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(day: u32, hour: u32) -> DateTime<Utc> {
        chrono::NaiveDate::from_ymd_opt(2026, 10, day).unwrap().and_hms_opt(hour, 0, 0).unwrap().and_utc()
    }

    #[test]
    fn a_week_old_list_is_stale_and_so_is_one_from_the_future() {
        let fetched = at(1, 12);
        assert!(DirectoryCache::is_fresh(fetched, at(1, 12)));
        assert!(DirectoryCache::is_fresh(fetched, at(8, 11)));
        assert!(!DirectoryCache::is_fresh(fetched, at(8, 12)));
        assert!(!DirectoryCache::is_fresh(fetched, at(1, 11)));
    }

    #[test]
    fn saves_and_loads_the_list() {
        let dir = std::env::temp_dir().join(format!("ls-directory-cache-{}", std::process::id()));
        let cache = DirectoryCache::new(dir.join("library-directory.json"));
        assert_eq!(cache.load(), None);
        let libraries = vec![LibraryInfo {
            id: "canning".into(),
            name: "City of Canning".into(),
            origin: "https://canning.spydus.com".into(),
            logo_url: Some("https://x/logo.png".into()),
            country: Some("Australia".into()),
            region: "AU".into(),
            is_test: false,
        }];
        cache.save(at(3, 9), &libraries).unwrap();
        assert_eq!(cache.load(), Some(SavedDirectory { fetched_at: at(3, 9), libraries }));
        // Another version, or a broken file, reads as nothing.
        std::fs::write(cache.file(), b"{\"version\":2,\"fetchedAt\":\"2026-10-03T09:00:00Z\",\"libraries\":[]}")
            .unwrap();
        assert_eq!(cache.load(), None);
        std::fs::write(cache.file(), b"not json").unwrap();
        assert_eq!(cache.load(), None);
        let _ = std::fs::remove_dir_all(dir);
    }
}
