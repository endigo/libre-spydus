//! Where things are kept (docs/cli.md §5): the config (`cli.json`: the chosen library), the last
//! numbered list, My Picks on this computer (one file per library, named by the SHA-256 of its
//! origin), and secrets (the session and the verification cookie) in the OS keychain through
//! `keyring`, or in `<data-dir>/secrets.json` with `--data-dir`. Nothing here touches the network.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use librespydus_core::SearchRecord;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::book_ref::BookRef;
use crate::error::Fail;
use crate::text;

/// Keychain services (account = the library origin).
pub const SESSION_SERVICE: &str = "com.librespydus.cli.session";
pub const VERIFICATION_SERVICE: &str = "com.librespydus.cli.verification";

/// A list older than this can't be numbered from: the OPAC's result sets don't last.
pub const LIST_MAX_AGE_HOURS: i64 = 24;

/// The folders and files the CLI reads and writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    /// `cli.json`; `None` keeps the config in memory.
    pub config_file: Option<PathBuf>,
    /// The last list and My Picks on this computer; `None` keeps them in memory.
    pub lists_dir: Option<PathBuf>,
    /// `secrets.json` (`--data-dir` only); `None` means the keychain.
    pub secrets_file: Option<PathBuf>,
}

impl Paths {
    /// The user's folders, or everything under `data_dir` (`--data-dir`).
    pub fn new(data_dir: Option<&Path>) -> Self {
        match data_dir {
            Some(dir) => Self {
                config_file: Some(dir.join("cli.json")),
                lists_dir: Some(dir.join("lists")),
                secrets_file: Some(dir.join("secrets.json")),
            },
            None => Self {
                config_file: dirs::config_dir().map(|dir| dir.join("LibreSpydus").join("cli.json")),
                lists_dir: dirs::cache_dir().map(|dir| dir.join("LibreSpydus").join("cli")),
                secrets_file: None,
            },
        }
    }

    /// Nothing on disk (tests).
    pub fn memory() -> Self {
        Self { config_file: None, lists_dir: None, secrets_file: None }
    }

    /// The last list's file; demo mode has its own, so it never mixes with the reader's.
    fn last_list_file(&self, demo: bool) -> Option<PathBuf> {
        let dir = self.lists_dir.as_ref()?;
        Some(if demo { dir.join("demo").join("last-list.json") } else { dir.join("last-list.json") })
    }

    fn picks_file(&self, origin: &str) -> Option<PathBuf> {
        Some(self.lists_dir.as_ref()?.join("my-picks").join(format!("{}.json", origin_hash(origin))))
    }
}

// ---- Config ----

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedLibrary {
    pub origin: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub library: Option<SavedLibrary>,
}

impl Config {
    pub fn load(paths: &Paths) -> Result<Self, Fail> {
        read_json(paths.config_file.as_deref()).map(Option::unwrap_or_default)
    }

    pub fn save(&self, paths: &Paths) -> Result<(), Fail> {
        write_json(paths.config_file.as_deref(), self)
    }
}

// ---- The last numbered list ----

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ListKind {
    Search,
    Picks,
}

/// One numbered row: the results card, and the local pick behind it when the list is My Picks
/// on this computer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListRow {
    pub number: usize,
    pub record: SearchRecord,
    #[serde(default)]
    pub book: Option<BookRef>,
}

/// The list the reader saw last, with its library and query, so numbers mean the same thing on
/// the next run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LastList {
    pub kind: ListKind,
    pub origin: String,
    pub library: String,
    /// The search query (`Search`), or what the picks list is called.
    pub query: String,
    pub saved_at: DateTime<Utc>,
    /// My Picks on this computer rather than on the account.
    #[serde(default)]
    pub local: bool,
    pub rows: Vec<ListRow>,
}

impl LastList {
    pub fn load(paths: &Paths, demo: bool) -> Result<Option<Self>, Fail> {
        read_json(paths.last_list_file(demo).as_deref())
    }

    pub fn save(&self, paths: &Paths, demo: bool) -> Result<(), Fail> {
        write_json(paths.last_list_file(demo).as_deref(), self)
    }

    /// The row numbered `number`, if this list is `origin`'s and still fresh.
    pub fn row(&self, number: usize, origin: &str, now: DateTime<Utc>) -> Result<&ListRow, Fail> {
        if self.origin != origin {
            return Err(Fail::List(text::list_for_other_library(&self.library)));
        }
        if (now - self.saved_at).num_hours() >= LIST_MAX_AGE_HOURS {
            return Err(Fail::List(text::list_stale(&self.query)));
        }
        let (Some(first), Some(last)) = (self.rows.first(), self.rows.last()) else {
            return Err(Fail::List(text::list_empty(&self.query)));
        };
        self.rows
            .iter()
            .find(|row| row.number == number)
            .ok_or_else(|| Fail::List(text::no_such_number_in(number, first.number, last.number)))
    }
}

// ---- My Picks on this computer ----

/// One library's local picks, newest first (the desktop app's `state/local_picks.rs`).
#[derive(Debug, Clone, Default)]
pub struct LocalPicks {
    pub items: Vec<BookRef>,
}

impl LocalPicks {
    pub fn load(paths: &Paths, origin: &str) -> Result<Self, Fail> {
        let mut items: Vec<BookRef> = read_json(paths.picks_file(origin).as_deref())?.unwrap_or_default();
        items.sort_by_key(|item| std::cmp::Reverse(item.saved_at));
        Ok(Self { items })
    }

    pub fn save(&self, paths: &Paths, origin: &str) -> Result<(), Fail> {
        write_json(paths.picks_file(origin).as_deref(), &self.items)
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
    }

    /// Removes the pick with `id`; whether there was one.
    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.items.len();
        self.items.retain(|item| item.id != id);
        self.items.len() != before
    }
}

// ---- Secrets ----

pub trait SecretStore: Send + Sync {
    fn get(&self, service: &str, origin: &str) -> Result<Option<String>, Fail>;
    /// `None` deletes.
    fn set(&self, service: &str, origin: &str, value: Option<&str>) -> Result<(), Fail>;
}

/// Generic-password items in the OS keychain: service = `SESSION_SERVICE` or
/// `VERIFICATION_SERVICE`, account = the library origin. Never used with `--data-dir`.
pub struct KeychainSecretStore;

impl SecretStore for KeychainSecretStore {
    fn get(&self, service: &str, origin: &str) -> Result<Option<String>, Fail> {
        let entry =
            keyring::Entry::new(service, origin).map_err(|e| Fail::Io(text::keychain_failed(&e.to_string())))?;
        match entry.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(Fail::Io(text::keychain_failed(&error.to_string()))),
        }
    }

    fn set(&self, service: &str, origin: &str, value: Option<&str>) -> Result<(), Fail> {
        let entry =
            keyring::Entry::new(service, origin).map_err(|e| Fail::Io(text::keychain_failed(&e.to_string())))?;
        let result = match value {
            Some(value) => entry.set_password(value),
            None => match entry.delete_credential() {
                Err(keyring::Error::NoEntry) => Ok(()),
                other => other,
            },
        };
        result.map_err(|error| Fail::Io(text::keychain_failed(&error.to_string())))
    }
}

/// `<data-dir>/secrets.json`: `{"<service>": {"<origin>": "<value>"}}`, plain text, for tests.
pub struct FileSecretStore {
    path: PathBuf,
}

impl FileSecretStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    fn read(&self) -> Result<HashMap<String, HashMap<String, String>>, Fail> {
        read_json(Some(&self.path)).map(Option::unwrap_or_default)
    }
}

impl SecretStore for FileSecretStore {
    fn get(&self, service: &str, origin: &str) -> Result<Option<String>, Fail> {
        Ok(self.read()?.get(service).and_then(|accounts| accounts.get(origin)).cloned())
    }

    fn set(&self, service: &str, origin: &str, value: Option<&str>) -> Result<(), Fail> {
        let mut all = self.read()?;
        let accounts = all.entry(service.to_owned()).or_default();
        match value {
            Some(value) => {
                accounts.insert(origin.to_owned(), value.to_owned());
            }
            None => {
                accounts.remove(origin);
            }
        }
        write_json(Some(&self.path), &all)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&self.path, std::fs::Permissions::from_mode(0o600));
        }
        Ok(())
    }
}

/// In memory: demo mode and tests. Nothing reaches the keychain or a file.
#[derive(Default)]
pub struct MemorySecretStore {
    values: Mutex<HashMap<(String, String), String>>,
}

impl SecretStore for MemorySecretStore {
    fn get(&self, service: &str, origin: &str) -> Result<Option<String>, Fail> {
        let values = self.values.lock().unwrap_or_else(|p| p.into_inner());
        Ok(values.get(&(service.to_owned(), origin.to_owned())).cloned())
    }

    fn set(&self, service: &str, origin: &str, value: Option<&str>) -> Result<(), Fail> {
        let mut values = self.values.lock().unwrap_or_else(|p| p.into_inner());
        let key = (service.to_owned(), origin.to_owned());
        match value {
            Some(value) => values.insert(key, value.to_owned()),
            None => values.remove(&key),
        };
        Ok(())
    }
}

// ---- Files ----

/// The hex SHA-256 of `origin`: file names that say nothing about which library they are.
pub fn origin_hash(origin: &str) -> String {
    Sha256::digest(origin.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

/// `None` when there is no path or no file; `Io` when the file can't be read or isn't JSON.
fn read_json<T: serde::de::DeserializeOwned>(path: Option<&Path>) -> Result<Option<T>, Fail> {
    let Some(path) = path else { return Ok(None) };
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(Fail::Io(text::couldnt_read(&path.display().to_string(), &error.to_string())));
        }
    };
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|error| Fail::Io(text::couldnt_read(&path.display().to_string(), &error.to_string())))
}

/// Writes `value` as pretty JSON through a temporary file; nothing without a path.
fn write_json<T: Serialize>(path: Option<&Path>, value: &T) -> Result<(), Fail> {
    let Some(path) = path else { return Ok(()) };
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| Fail::Io(text::couldnt_write(&path.display().to_string(), &error.to_string())))?;
    write_atomically(path, &bytes)
        .map_err(|error| Fail::Io(text::couldnt_write(&path.display().to_string(), &error.to_string())))
}

fn write_atomically(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("tmp");
    std::fs::write(&temporary, bytes)?;
    std::fs::rename(&temporary, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_dir_moves_every_file() {
        let paths = Paths::new(Some(Path::new("/tmp/x")));
        assert_eq!(paths.config_file.as_deref(), Some(Path::new("/tmp/x/cli.json")));
        assert_eq!(paths.secrets_file.as_deref(), Some(Path::new("/tmp/x/secrets.json")));
        assert_eq!(paths.last_list_file(false).unwrap(), Path::new("/tmp/x/lists/last-list.json"));
        assert_eq!(paths.last_list_file(true).unwrap(), Path::new("/tmp/x/lists/demo/last-list.json"));
        assert_eq!(origin_hash("abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert!(Paths::memory().last_list_file(false).is_none());
    }

    #[test]
    fn config_lists_picks_and_secrets_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::new(Some(dir.path()));
        assert_eq!(Config::load(&paths).unwrap(), Config::default());
        let config = Config {
            library: Some(SavedLibrary { origin: "https://canning.spydus.com".into(), name: "City of Canning".into() }),
        };
        config.save(&paths).unwrap();
        assert_eq!(Config::load(&paths).unwrap(), config);

        let now = Utc::now();
        let list = LastList {
            kind: ListKind::Search,
            origin: "https://canning.spydus.com".into(),
            library: "City of Canning".into(),
            query: "midnight".into(),
            saved_at: now,
            local: false,
            rows: vec![ListRow {
                number: 1,
                record: SearchRecord { title: Some("A".into()), ..Default::default() },
                book: None,
            }],
        };
        assert!(LastList::load(&paths, false).unwrap().is_none());
        list.save(&paths, false).unwrap();
        let loaded = LastList::load(&paths, false).unwrap().unwrap();
        assert_eq!(loaded, list);
        assert!(LastList::load(&paths, true).unwrap().is_none(), "demo mode has its own list");
        assert_eq!(loaded.row(1, "https://canning.spydus.com", now).unwrap().record.title.as_deref(), Some("A"));
        assert!(matches!(loaded.row(2, "https://canning.spydus.com", now), Err(Fail::List(_))));
        assert!(matches!(loaded.row(1, "https://other.spydus.com", now), Err(Fail::List(_))));
        let later = now + chrono::Duration::hours(LIST_MAX_AGE_HOURS);
        assert!(matches!(loaded.row(1, "https://canning.spydus.com", later), Err(Fail::List(_))));

        let mut picks = LocalPicks::load(&paths, "https://canning.spydus.com").unwrap();
        assert!(picks.items.is_empty());
        let a = BookRef::from_search_record(&SearchRecord {
            work_irn: Some("1".into()),
            title: Some("A".into()),
            ..Default::default()
        });
        let b = BookRef::from_search_record(&SearchRecord {
            work_irn: Some("2".into()),
            title: Some("B".into()),
            ..Default::default()
        });
        picks.add(&a, now - chrono::Duration::minutes(5));
        picks.add(&b, now);
        picks.save(&paths, "https://canning.spydus.com").unwrap();
        let reloaded = LocalPicks::load(&paths, "https://canning.spydus.com").unwrap();
        let ids: Vec<_> = reloaded.items.iter().map(|item| item.id.as_str()).collect();
        assert_eq!(ids, ["work:2", "work:1"]);
        assert!(reloaded.contains("work:1"));
        assert!(LocalPicks::load(&paths, "https://other.spydus.com").unwrap().items.is_empty());

        let secrets = FileSecretStore::new(paths.secrets_file.clone().unwrap());
        assert_eq!(secrets.get(SESSION_SERVICE, "https://canning.spydus.com").unwrap(), None);
        secrets.set(SESSION_SERVICE, "https://canning.spydus.com", Some("token")).unwrap();
        secrets.set(VERIFICATION_SERVICE, "https://canning.spydus.com", Some("cookie")).unwrap();
        assert_eq!(secrets.get(SESSION_SERVICE, "https://canning.spydus.com").unwrap().as_deref(), Some("token"));
        secrets.set(SESSION_SERVICE, "https://canning.spydus.com", None).unwrap();
        assert_eq!(secrets.get(SESSION_SERVICE, "https://canning.spydus.com").unwrap(), None);
        assert_eq!(secrets.get(VERIFICATION_SERVICE, "https://canning.spydus.com").unwrap().as_deref(), Some("cookie"));
        let memory = MemorySecretStore::default();
        memory.set(SESSION_SERVICE, "o", Some("v")).unwrap();
        assert_eq!(memory.get(SESSION_SERVICE, "o").unwrap().as_deref(), Some("v"));
    }
}
