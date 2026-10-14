//! The saved account, one file per library origin (docs/native-apps.md §5.2a, iOS
//! `AccountSnapshotCache.swift`): `AccountCache/<sha256(origin)>.json`, holding the core's
//! `encode_account_snapshot` JSON, written atomically. A file that doesn't decode (corrupt, or
//! another snapshot version) is deleted and treated as missing.

use std::path::PathBuf;

use librespydus_core::AccountSnapshot;
use librespydus_core::app::{decode_account_snapshot, encode_account_snapshot};

use crate::state::paths::{origin_hash, write_atomically};

#[derive(Debug, Clone)]
pub struct AccountSnapshotCache {
    directory: PathBuf,
}

impl AccountSnapshotCache {
    pub fn new(directory: PathBuf) -> Self {
        Self { directory }
    }

    /// `origin`'s saved snapshot, or `None` when there's none or it can't be read.
    pub fn load(&self, origin: &str) -> Option<AccountSnapshot> {
        let file = self.file(origin);
        let text = std::fs::read_to_string(&file).ok()?;
        match decode_account_snapshot(text) {
            Ok(snapshot) => Some(snapshot),
            Err(_) => {
                // Malformed or another snapshot version: fetch a new one instead.
                let _ = std::fs::remove_file(&file);
                None
            }
        }
    }

    pub fn save(&self, snapshot: &AccountSnapshot, origin: &str) -> std::io::Result<()> {
        write_atomically(&self.file(origin), encode_account_snapshot(snapshot.clone()).as_bytes())
    }

    pub fn remove(&self, origin: &str) {
        let _ = std::fs::remove_file(self.file(origin));
    }

    /// The origin is hashed so the file name says nothing about which library it is.
    pub fn file(&self, origin: &str) -> PathBuf {
        self.directory.join(format!("{}.json", origin_hash(origin)))
    }
}
