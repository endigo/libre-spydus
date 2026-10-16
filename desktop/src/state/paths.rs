//! Where the app keeps its files (docs/desktop-app.md §5.4).
//!
//! - `data/` = `~/Library/Application Support/LibreSpydus/` (or `<--data-dir>/LibreSpydus/`):
//!   `settings.json`, `AccountCache/<sha256(origin)>.json`, `MyPicks/<sha256(origin)>.json`;
//! - `saved_pages/` = `~/Library/Caches/LibreSpydus/SavedPages` (or
//!   `<--data-dir>/Caches/LibreSpydus/SavedPages`), through the core (§5.13).

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    /// `…/LibreSpydus`; `None` keeps everything in memory (tests).
    pub data: Option<PathBuf>,
    /// The core's saved pages; `None` saves none.
    pub saved_pages: Option<PathBuf>,
}

impl Paths {
    /// The real folders, or everything under `data_dir` (`--data-dir`).
    pub fn new(data_dir: Option<&Path>) -> Self {
        match data_dir {
            Some(dir) => Self {
                data: Some(dir.join("LibreSpydus")),
                saved_pages: Some(dir.join("Caches").join("LibreSpydus").join("SavedPages")),
            },
            None => Self {
                data: dirs::data_dir().map(|dir| dir.join("LibreSpydus")),
                saved_pages: dirs::cache_dir().map(|dir| dir.join("LibreSpydus").join("SavedPages")),
            },
        }
    }

    /// Nothing on disk (tests).
    pub fn memory() -> Self {
        Self { data: None, saved_pages: None }
    }

    pub fn settings_file(&self) -> Option<PathBuf> {
        self.data.as_ref().map(|dir| dir.join("settings.json"))
    }

    pub fn account_cache_dir(&self) -> Option<PathBuf> {
        self.data.as_ref().map(|dir| dir.join("AccountCache"))
    }

    pub fn my_picks_dir(&self) -> Option<PathBuf> {
        self.data.as_ref().map(|dir| dir.join("MyPicks"))
    }

    /// `--reset`: deletes every file the app keeps (secrets are the caller's).
    pub fn delete_all(&self) {
        for dir in [&self.data, &self.saved_pages].into_iter().flatten() {
            if dir.exists()
                && let Err(error) = std::fs::remove_dir_all(dir)
            {
                log::warn!("reset: couldn't delete {}: {error}", dir.display());
            }
        }
    }
}

/// The hex SHA-256 of `origin`: file names that say nothing about which library they are.
pub fn origin_hash(origin: &str) -> String {
    Sha256::digest(origin.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Writes `bytes` to `path` through a temporary file, so a crash never leaves half a file.
pub fn write_atomically(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
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
        assert_eq!(paths.settings_file().unwrap(), Path::new("/tmp/x/LibreSpydus/settings.json"));
        assert_eq!(paths.saved_pages.unwrap(), Path::new("/tmp/x/Caches/LibreSpydus/SavedPages"));
        assert_eq!(origin_hash("abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }
}
