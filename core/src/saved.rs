//! Saved pages: the book pages and Home lists the apps show at once on the next visit while a fresh
//! copy loads (`docs/native-apps.md` §5.13). Public catalogue data only — never availability, copies
//! or anything from the account.
//!
//! Files live under the folder the app passes to `SpydusClient::set_saved_pages_dir` (in its caches
//! directory), one folder per library: `<dir>/<origin>/books/<key>.json`,
//! `<dir>/<origin>/shelves/<key>.json` and `<dir>/<origin>/featured.json`, every name made safe by
//! [`file_name`]. A file holds the format version and the value; one that doesn't read back
//! (malformed, or written with another [`SAVED_PAGES_VERSION`]) is deleted and reads as absent.
//! Writes go to a temporary file renamed into place, so a reader never sees half a file.

use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// The saved-file format. Bump it when a saved record (or anything inside it) changes shape, so
/// files written by an older build are dropped instead of misread.
pub const SAVED_PAGES_VERSION: u32 = 1;

/// Books kept per library. Saving one more deletes the least recently saved.
pub const MAX_SAVED_BOOKS: usize = 300;

/// The longest file name a key may encode to; a longer key is neither saved nor read.
const MAX_NAME_LEN: usize = 200;

/// A temporary file this old is left over from a write that never finished.
const STALE_TEMP: Duration = Duration::from_secs(60 * 60);

#[derive(Serialize, Deserialize)]
struct Envelope<T> {
    version: u32,
    value: T,
}

/// One library's saved pages: where each one's file goes.
pub(crate) struct SavedPages {
    root: PathBuf,
}

impl SavedPages {
    /// `None` for a blank `dir` (saved pages off).
    pub(crate) fn new(dir: &str, origin: &str) -> Option<Self> {
        let dir = dir.trim();
        if dir.is_empty() {
            return None;
        }
        Some(Self { root: Path::new(dir).join(file_name(origin)?) })
    }

    pub(crate) fn book(&self, key: &str) -> Option<PathBuf> {
        Some(self.root.join("books").join(format!("{}.json", file_name(key)?)))
    }

    pub(crate) fn shelf(&self, key: &str) -> Option<PathBuf> {
        Some(self.root.join("shelves").join(format!("{}.json", file_name(key)?)))
    }

    pub(crate) fn featured(&self) -> PathBuf {
        self.root.join("featured.json")
    }
}

/// `text` as one file or folder name: lower-case ASCII letters, digits, `-` and `.` as they are,
/// every other byte as `_` and two lower-case hex digits. Distinct texts never share a name, even
/// on a case-insensitive file system (`work:15642035` → `work_3a15642035`). `None` when `text` is
/// blank or its name would be longer than [`MAX_NAME_LEN`].
pub fn file_name(text: &str) -> Option<String> {
    let text = text.trim();
    let mut name = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'.' {
            name.push(char::from(byte));
        } else {
            let _ = write!(name, "_{byte:02x}");
        }
    }
    let usable = !name.is_empty() && name.len() <= MAX_NAME_LEN && name != "." && name != "..";
    usable.then_some(name)
}

/// The value saved at `path`, or `None` when there is none. A file that doesn't read back is
/// deleted.
pub(crate) fn read<T: DeserializeOwned>(path: &Path) -> Option<T> {
    let text = fs::read_to_string(path).ok()?;
    match serde_json::from_str::<Envelope<T>>(&text) {
        Ok(saved) if saved.version == SAVED_PAGES_VERSION => Some(saved.value),
        _ => {
            let _ = fs::remove_file(path);
            None
        }
    }
}

/// Saves `value` at `path`, replacing what was there.
pub(crate) fn write<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    let folder = path.parent().ok_or_else(|| io::Error::other("saved page path without a folder"))?;
    fs::create_dir_all(folder)?;
    let json = serde_json::to_vec(&Envelope { version: SAVED_PAGES_VERSION, value })?;
    let name = path.file_name().map(|name| name.to_string_lossy()).unwrap_or_default();
    // A name of its own, so two saves of the same page never write into one temporary file.
    let temp = folder.join(format!(".{name}.{:016x}.tmp", rand::random::<u64>()));
    fs::write(&temp, json)?;
    fs::rename(&temp, path).inspect_err(|_| {
        let _ = fs::remove_file(&temp);
    })
}

/// Keeps the `keep` most recently saved pages in `folder`, deleting the rest, and deletes temporary
/// files left by writes that never finished.
pub(crate) fn evict(folder: &Path, keep: usize) {
    let Ok(entries) = fs::read_dir(folder) else { return };
    let now = SystemTime::now();
    let mut saved = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(modified) = entry.metadata().and_then(|metadata| metadata.modified()) else { continue };
        match path.extension().and_then(|extension| extension.to_str()) {
            Some("json") => saved.push((modified, path)),
            Some("tmp") if now.duration_since(modified).is_ok_and(|age| age > STALE_TEMP) => {
                let _ = fs::remove_file(&path);
            }
            _ => {}
        }
    }
    if saved.len() <= keep {
        return;
    }
    saved.sort_by_key(|(modified, _)| *modified);
    let excess = saved.len() - keep;
    for (_, path) in saved.into_iter().take(excess) {
        let _ = fs::remove_file(path);
    }
}
