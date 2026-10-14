//! `settings.json` (docs/desktop-app.md §5.4): the selected library, recent searches (at most 8,
//! newest first), the libraries whose local picks were offered to the account (§5.11), and the
//! origins that have keychain items (so `--reset` can delete them).

use std::path::PathBuf;

use librespydus_core::LibraryInfo;
use serde::{Deserialize, Serialize};

use crate::state::paths::{origin_hash, write_atomically};

/// Recent searches kept (iOS `RecentSearchStore.limit`).
pub const RECENT_SEARCH_LIMIT: usize = 8;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SettingsData {
    pub library: Option<LibraryInfo>,
    pub recent_searches: Vec<String>,
    /// `sha256(origin)` of every library whose local picks were offered (either answer).
    pub picks_offered: Vec<String>,
    /// The same, for demo mode (which never changes the real answers).
    pub demo_picks_offered: Vec<String>,
    /// Origins with keychain items.
    pub secret_origins: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Settings {
    path: Option<PathBuf>,
    pub data: SettingsData,
}

impl Settings {
    /// Reads `path` (`None`: in memory). A file that doesn't read starts afresh.
    pub fn load(path: Option<PathBuf>) -> Self {
        let data = path
            .as_ref()
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|bytes| match serde_json::from_slice(&bytes) {
                Ok(data) => Some(data),
                Err(error) => {
                    log::warn!("settings.json unreadable, starting afresh: {error}");
                    None
                }
            })
            .unwrap_or_default();
        Self { path, data }
    }

    pub fn save(&self) {
        let Some(path) = &self.path else { return };
        match serde_json::to_vec_pretty(&self.data) {
            Ok(bytes) => {
                if let Err(error) = write_atomically(path, &bytes) {
                    log::warn!("couldn't save settings.json: {error}");
                }
            }
            Err(error) => log::warn!("couldn't encode settings: {error}"),
        }
    }

    pub fn set_library(&mut self, info: &LibraryInfo) {
        self.data.library = Some(info.clone());
        self.save();
    }

    // ---- Recent searches (iOS `RecentSearchStore`) ----

    /// Moves `query` (trimmed, inner whitespace collapsed) to the top; a case-insensitive repeat
    /// replaces the older entry. Blank queries are ignored.
    pub fn add_recent_search(&mut self, query: &str) {
        let cleaned = query.split_whitespace().collect::<Vec<_>>().join(" ");
        if cleaned.is_empty() {
            return;
        }
        let lower = cleaned.to_lowercase();
        self.data.recent_searches.retain(|item| item.to_lowercase() != lower);
        self.data.recent_searches.insert(0, cleaned);
        self.data.recent_searches.truncate(RECENT_SEARCH_LIMIT);
        self.save();
    }

    pub fn remove_recent_search(&mut self, query: &str) {
        self.data.recent_searches.retain(|item| item != query);
        self.save();
    }

    pub fn clear_recent_searches(&mut self) {
        self.data.recent_searches.clear();
        self.save();
    }

    // ---- My Picks' copy-once offer ----

    pub fn was_picks_offered(&self, origin: &str, demo: bool) -> bool {
        let hash = origin_hash(origin);
        self.offered(demo).contains(&hash)
    }

    pub fn mark_picks_offered(&mut self, origin: &str, demo: bool) {
        let hash = origin_hash(origin);
        let list = if demo { &mut self.data.demo_picks_offered } else { &mut self.data.picks_offered };
        if !list.contains(&hash) {
            list.push(hash);
            self.save();
        }
    }

    fn offered(&self, demo: bool) -> &[String] {
        if demo { &self.data.demo_picks_offered } else { &self.data.picks_offered }
    }

    /// Remembers that `origin` has keychain items.
    pub fn note_secret_origin(&mut self, origin: &str) {
        if !self.data.secret_origins.iter().any(|known| known == origin) {
            self.data.secret_origins.push(origin.to_owned());
            self.save();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip_through_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("LibreSpydus").join("settings.json");
        let mut settings = Settings::load(Some(path.clone()));
        assert_eq!(settings.data, SettingsData::default());
        let library = LibraryInfo {
            id: "x".into(),
            name: "City of Canning".into(),
            origin: "https://canning.spydus.com".into(),
            logo_url: None,
            country: Some("Australia".into()),
            region: "AU".into(),
            is_test: false,
        };
        settings.set_library(&library);
        for query in ["harry potter", "  Harry   Potter ", "dune", "", "a", "b", "c", "d", "e", "f"] {
            settings.add_recent_search(query);
        }
        settings.mark_picks_offered("https://canning.spydus.com", false);
        settings.note_secret_origin("https://canning.spydus.com");

        let reloaded = Settings::load(Some(path.clone()));
        assert_eq!(reloaded.data, settings.data);
        assert_eq!(reloaded.data.library.as_ref().unwrap().name, "City of Canning");
        assert_eq!(reloaded.data.recent_searches.len(), RECENT_SEARCH_LIMIT);
        assert_eq!(reloaded.data.recent_searches[0], "f");
        assert!(reloaded.data.recent_searches.contains(&"Harry Potter".to_owned()));
        assert!(!reloaded.data.recent_searches.contains(&"harry potter".to_owned()));
        assert!(reloaded.was_picks_offered("https://canning.spydus.com", false));
        assert!(!reloaded.was_picks_offered("https://canning.spydus.com", true));

        std::fs::write(&path, b"{not json").unwrap();
        assert_eq!(Settings::load(Some(path)).data, SettingsData::default());
    }
}
