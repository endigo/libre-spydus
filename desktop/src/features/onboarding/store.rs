//! The Civica library directory (iOS `LibraryPickerModel.swift`): fetched once per launch with
//! `list_libraries(false)` (one public GET, on the core's runtime) and shared by the onboarding
//! screen and the Change library dialog, so the dialog opens at once after onboarding.
//!
//! On failure with nothing to show it's the error state (Retry); a failed refresh keeps the list
//! on screen with a banner. Tests swap the fetch with `set_directory_fetch`.

use std::sync::Arc;

use futures::FutureExt;
use futures::future::BoxFuture;
use gpui_kit::{App, AppContext, Context, Entity, Global};
use librespydus_core::{LibraryInfo, SpydusError};

use crate::copy::Copy;
use crate::runtime::spawn_core;
use crate::state::{ErrorKind, UserFacingError};

/// Fetches the directory: `list_libraries(false)` by default.
pub type DirectoryFetch = Arc<dyn Fn() -> BoxFuture<'static, Result<Vec<LibraryInfo>, SpydusError>> + Send + Sync>;

pub struct DirectoryStore {
    fetch: DirectoryFetch,
    libraries: Vec<LibraryInfo>,
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
/// already fetched.
pub fn set_directory_fetch(fetch: DirectoryFetch, cx: &mut App) {
    let store = cx.new(|_| DirectoryStore::new(fetch));
    cx.set_global(GlobalDirectory(store));
}

impl DirectoryStore {
    fn new(fetch: DirectoryFetch) -> Self {
        Self {
            fetch,
            libraries: Vec::new(),
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
        let store = cx.new(|_| DirectoryStore::new(default_fetch()));
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

    /// A picker appeared: fetches unless a list is already here (an earlier failure tries again).
    pub fn load(&mut self, cx: &mut Context<Self>) {
        if self.libraries.is_empty() && !self.fetching {
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
