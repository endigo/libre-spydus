//! "Not now" answers to the Touch ID offer, per library until sign-out (docs/native-apps.md
//! §5.14): `Biometrics.json` in the data dir, with demo mode's answers apart so they never change
//! the real ones (iOS keeps them in `UserDefaults` under two namespaces).

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::state::paths::{Paths, write_atomically};

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct File {
    #[serde(default)]
    offer_declined: BTreeSet<String>,
    #[serde(default)]
    demo_offer_declined: BTreeSet<String>,
}

pub struct DeclinedStore {
    /// `None`: memory only (tests).
    path: Option<PathBuf>,
    file: File,
}

impl DeclinedStore {
    /// `<data>/Biometrics.json`, or memory when the app keeps no files.
    pub fn for_paths(paths: &Paths) -> Self {
        // copy-exempt: a file name
        Self::at(paths.data.as_ref().map(|dir| dir.join("Biometrics.json")))
    }

    pub fn at(path: Option<PathBuf>) -> Self {
        let file = path
            .as_ref()
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        Self { path, file }
    }

    pub fn contains(&self, origin: &str, demo: bool) -> bool {
        self.set(demo).contains(origin)
    }

    pub fn set_declined(&mut self, origin: &str, demo: bool, declined: bool) {
        let changed =
            if declined { self.set_mut(demo).insert(origin.to_owned()) } else { self.set_mut(demo).remove(origin) };
        if changed {
            self.save();
        }
    }

    fn set(&self, demo: bool) -> &BTreeSet<String> {
        if demo { &self.file.demo_offer_declined } else { &self.file.offer_declined }
    }

    fn set_mut(&mut self, demo: bool) -> &mut BTreeSet<String> {
        if demo { &mut self.file.demo_offer_declined } else { &mut self.file.offer_declined }
    }

    fn save(&self) {
        let Some(path) = &self.path else { return };
        let result = serde_json::to_vec_pretty(&self.file)
            .map_err(std::io::Error::other)
            .and_then(|bytes| write_atomically(path, &bytes));
        if let Err(error) = result {
            log::warn!("biometrics: couldn't save {}: {error}", path.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers_survive_a_relaunch_and_demo_keeps_its_own() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Biometrics.json");
        let mut store = DeclinedStore::at(Some(path.clone()));
        store.set_declined("https://a", false, true);
        store.set_declined("https://b", true, true);
        let again = DeclinedStore::at(Some(path.clone()));
        assert!(again.contains("https://a", false));
        assert!(!again.contains("https://a", true));
        assert!(again.contains("https://b", true));
        let mut again = again;
        again.set_declined("https://a", false, false);
        assert!(!DeclinedStore::at(Some(path)).contains("https://a", false));
    }
}
