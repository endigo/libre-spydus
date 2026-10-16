//! Small string secrets keyed by library origin (sessions, verification cookies, card numbers):
//! the OS keychain through `keyring` (macOS Keychain; services `com.librespydus.desktop.session`,
//! `.verification`, `.card`, account = origin), or memory in demo mode and with `--data-dir`
//! (docs/desktop-app.md §5.4).

use std::collections::HashMap;
use std::sync::Mutex;

pub const SESSION_SERVICE: &str = "com.librespydus.desktop.session";
pub const VERIFICATION_SERVICE: &str = "com.librespydus.desktop.verification";
pub const CARD_SERVICE: &str = "com.librespydus.desktop.card";

pub trait SecretStore: Send + Sync {
    fn value(&self, key: &str) -> Option<String>;
    /// `None` deletes.
    fn set_value(&self, key: &str, value: Option<&str>);
}

/// Generic-password items under one keychain service.
pub struct KeychainSecretStore {
    service: &'static str,
}

impl KeychainSecretStore {
    pub fn new(service: &'static str) -> Self {
        Self { service }
    }

    fn entry(&self, key: &str) -> Option<keyring::Entry> {
        match keyring::Entry::new(self.service, key) {
            Ok(entry) => Some(entry),
            Err(error) => {
                log::warn!("keychain {}: {error}", self.service);
                None
            }
        }
    }
}

impl SecretStore for KeychainSecretStore {
    fn value(&self, key: &str) -> Option<String> {
        match self.entry(key)?.get_password() {
            Ok(value) => Some(value),
            Err(keyring::Error::NoEntry) => None,
            Err(error) => {
                log::warn!("keychain {} read: {error}", self.service);
                None
            }
        }
    }

    fn set_value(&self, key: &str, value: Option<&str>) {
        let Some(entry) = self.entry(key) else { return };
        let result = match value {
            Some(value) => entry.set_password(value),
            None => match entry.delete_credential() {
                Err(keyring::Error::NoEntry) => Ok(()),
                other => other,
            },
        };
        if let Err(error) = result {
            log::warn!("keychain {} write: {error}", self.service);
        }
    }
}

/// In memory: demo mode, `--data-dir`, tests. Nothing reaches the keychain.
#[derive(Default)]
pub struct MemorySecretStore {
    values: Mutex<HashMap<String, String>>,
}

impl MemorySecretStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SecretStore for MemorySecretStore {
    fn value(&self, key: &str) -> Option<String> {
        self.values.lock().unwrap_or_else(|p| p.into_inner()).get(key).cloned()
    }

    fn set_value(&self, key: &str, value: Option<&str>) {
        let mut values = self.values.lock().unwrap_or_else(|p| p.into_inner());
        match value {
            Some(value) => values.insert(key.to_owned(), value.to_owned()),
            None => values.remove(key),
        };
    }
}
