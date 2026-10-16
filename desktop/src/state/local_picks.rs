//! My Picks on this computer (signed out; docs/native-apps.md §5.3, iOS `FavouritesStore.swift`):
//! one list per library origin, newest first, in `MyPicks/<sha256(origin)>.json` (a JSON array of
//! `BookRef`). Without a folder (demo mode never needs one, tests) the list lives in memory.

use std::path::PathBuf;

use chrono::{DateTime, Utc};

use crate::copy::Copy;
use crate::state::book_ref::BookRef;
use crate::state::paths::{origin_hash, write_atomically};

#[derive(Debug, Clone, Default)]
pub struct LocalPicksStore {
    directory: Option<PathBuf>,
    origin: Option<String>,
    /// Saved books, newest first.
    items: Vec<BookRef>,
    /// The last load/save failure (`picks.loadFailed` / `saveFailed`); the list still works in memory.
    last_error: Option<&'static str>,
}

impl LocalPicksStore {
    pub fn new(directory: Option<PathBuf>) -> Self {
        Self { directory, ..Default::default() }
    }

    pub fn origin(&self) -> Option<&str> {
        self.origin.as_deref()
    }

    pub fn items(&self) -> &[BookRef] {
        &self.items
    }

    pub fn last_error(&self) -> Option<&'static str> {
        self.last_error
    }

    /// Switches to `origin`'s list (`None`: empty, nothing saved).
    pub fn load(&mut self, origin: Option<&str>) {
        self.origin = origin.map(str::to_owned);
        self.last_error = None;
        self.items.clear();
        let Some(file) = self.file() else { return };
        let Ok(bytes) = std::fs::read(&file) else { return };
        match serde_json::from_slice::<Vec<BookRef>>(&bytes) {
            Ok(items) => self.items = sorted(items),
            Err(error) => {
                log::warn!("My Picks file unreadable: {error}");
                self.last_error = Some(Copy::Picks::loadFailed);
            }
        }
    }

    pub fn contains(&self, id: &str) -> bool {
        self.items.iter().any(|item| item.id == id)
    }

    /// Saves `book` (stamped `saved_at = now`, moved to the top), replacing the saved copy.
    pub fn add(&mut self, book: &BookRef, now: DateTime<Utc>) {
        let mut saved = book.clone();
        saved.saved_at = Some(now);
        self.items.retain(|item| item.id != book.id);
        self.items.insert(0, saved);
        self.save();
    }

    pub fn remove(&mut self, id: &str) {
        let before = self.items.len();
        self.items.retain(|item| item.id != id);
        if self.items.len() != before {
            self.save();
        }
    }

    /// Adds or removes `book`; returns whether it's saved afterwards.
    pub fn toggle(&mut self, book: &BookRef, now: DateTime<Utc>) -> bool {
        if self.contains(&book.id) {
            self.remove(&book.id);
            false
        } else {
            self.add(book, now);
            true
        }
    }

    fn file(&self) -> Option<PathBuf> {
        let origin = self.origin.as_deref().filter(|origin| !origin.is_empty())?;
        Some(self.directory.as_ref()?.join(format!("{}.json", origin_hash(origin))))
    }

    fn save(&mut self) {
        let Some(file) = self.file() else { return };
        let result = serde_json::to_vec_pretty(&self.items)
            .map_err(std::io::Error::other)
            .and_then(|bytes| write_atomically(&file, &bytes));
        self.last_error = match result {
            Ok(()) => None,
            Err(error) => {
                log::warn!("couldn't save My Picks: {error}");
                Some(Copy::Picks::saveFailed)
            }
        };
    }
}

fn sorted(mut items: Vec<BookRef>) -> Vec<BookRef> {
    items.sort_by_key(|item| std::cmp::Reverse(item.saved_at));
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn picks_persist_per_library_newest_first() {
        let dir = tempfile::tempdir().unwrap();
        let picks_dir = dir.path().join("MyPicks");
        let now = Utc::now();
        let mut store = LocalPicksStore::new(Some(picks_dir.clone()));
        store.load(Some("https://canning.spydus.com"));
        assert!(store.items().is_empty());
        let a = BookRef::new("work:1", "A");
        let b = BookRef::new("bib:2", "B");
        store.add(&a, now - Duration::minutes(5));
        store.add(&b, now);
        assert!(store.toggle(&BookRef::new("brn:3", "C"), now + Duration::minutes(1)));
        assert!(!store.toggle(&BookRef::new("brn:3", "C"), now + Duration::minutes(2)));
        assert_eq!(store.last_error(), None);

        let file = picks_dir.join(format!("{}.json", origin_hash("https://canning.spydus.com")));
        assert!(file.exists());
        let mut reloaded = LocalPicksStore::new(Some(picks_dir.clone()));
        reloaded.load(Some("https://canning.spydus.com"));
        let ids: Vec<_> = reloaded.items().iter().map(|item| item.id.as_str()).collect();
        assert_eq!(ids, ["bib:2", "work:1"]);
        assert!(reloaded.items()[0].saved_at.is_some());

        // Another library has its own (empty) list; a library's list survives switching back.
        reloaded.load(Some("https://cumberland.spydus.com"));
        assert!(reloaded.items().is_empty());
        reloaded.load(Some("https://canning.spydus.com"));
        assert_eq!(reloaded.items().len(), 2);

        std::fs::write(&file, b"[{]").unwrap();
        reloaded.load(Some("https://canning.spydus.com"));
        assert!(reloaded.items().is_empty());
        assert_eq!(reloaded.last_error(), Some(Copy::Picks::loadFailed));
    }
}
