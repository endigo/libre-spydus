//! Where a library's sign-in details are kept (docs/native-apps.md §5.14, docs/desktop-app.md
//! §11.4; iOS `BiometricVault.swift`). The vault only stores: every read goes through
//! `Biometrics::read`, which runs the Touch ID check first (`check.rs`).
//!
//! **Why not a data-protection Keychain item:** iOS keeps the details in an item with
//! `SecAccessControl(.biometryCurrentSet)`, which the Keychain itself refuses to read without the
//! check. On macOS that item lives in the data-protection keychain, which needs an app signed
//! with a team and a `keychain-access-groups` entitlement; a dev or ad-hoc build has neither, and
//! `SecItemAdd` fails with `errSecMissingEntitlement`. So v1 keeps `{username, pin}` in the login
//! keychain through `keyring` (the same protection as the session token) and gates every use with
//! `LAContext`. A signed build can add a `DataProtectionVault` behind this same trait (and pass
//! the passed `LAContext` as `kSecUseAuthenticationContext`) without touching the flows.

use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::copy::Copy;

/// The login keychain service of the details (account = origin), and of the plain username
/// beside them (`<service>.username`).
pub const SIGN_IN_SERVICE: &str = "com.librespydus.desktop.signin";
/// Demo mode with `LS_DEMO_KEYCHAIN=1` (debug builds).
pub const DEMO_SIGN_IN_SERVICE: &str = "com.librespydus.desktop.signin.demo";

/// The computer's biometric check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Biometry {
    TouchId,
    FaceId,
    OpticId,
}

impl Biometry {
    /// `{method}` in `Copy::Biometrics`: "Touch ID" on a Mac.
    pub fn name(self) -> &'static str {
        match self {
            Biometry::TouchId => Copy::Biometrics::touchId,
            Biometry::FaceId => Copy::Biometrics::faceId,
            Biometry::OpticId => Copy::Biometrics::opticId,
        }
    }
}

/// The username / card number and PIN of a typed sign-in that succeeded. Its `Debug` never
/// prints the PIN.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignInDetails {
    pub username: String,
    pub pin: String,
}

impl SignInDetails {
    pub fn new(username: impl Into<String>, pin: impl Into<String>) -> Self {
        Self { username: username.into(), pin: pin.into() }
    }
}

impl std::fmt::Debug for SignInDetails {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SignInDetails").field("username", &self.username).finish_non_exhaustive()
    }
}

/// Why a check didn't save or hand back the details.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BiometricFailure {
    /// Cancelled, "Enter PIN" chosen, or the finger wasn't recognised.
    Declined,
    /// The check can't run now (no sensor, nothing enrolled, locked out), or the store failed.
    Unavailable,
    /// Nothing readable for that library: deleted.
    Missing,
}

/// A library's sign-in details per origin. Never asks for the check itself: `Biometrics` runs
/// it before `load` and `store`.
pub trait BiometricVault: Send + Sync {
    /// The username of `origin`'s details, `None` when it has none.
    fn username(&self, origin: &str) -> Option<String>;
    /// `origin`'s details (only after a passed check). Unreadable ones are deleted (`Missing`).
    fn load(&self, origin: &str) -> Result<SignInDetails, BiometricFailure>;
    fn store(&self, origin: &str, details: &SignInDetails) -> Result<(), BiometricFailure>;
    fn delete(&self, origin: &str);
    fn delete_all(&self);
}

/// The login keychain through `keyring`: the JSON `{username, pin}` under `service`, the plain
/// username under `<service>.username`, account = origin.
pub struct KeychainVault {
    service: &'static str,
    username_service: String,
}

impl KeychainVault {
    pub fn new(service: &'static str) -> Self {
        Self { service, username_service: format!("{service}.username") }
    }

    fn entry(service: &str, origin: &str) -> Option<keyring::Entry> {
        keyring::Entry::new(service, origin).map_err(|error| log::warn!("keychain {service}: {error}")).ok()
    }

    fn read(service: &str, origin: &str) -> Result<Option<String>, ()> {
        let Some(entry) = Self::entry(service, origin) else { return Err(()) };
        match entry.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => {
                log::warn!("keychain {service} read: {error}");
                Err(())
            }
        }
    }

    fn remove(service: &str, origin: &str) {
        let Some(entry) = Self::entry(service, origin) else { return };
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => {}
            Err(error) => log::warn!("keychain {service} delete: {error}"),
        }
    }
}

impl BiometricVault for KeychainVault {
    fn username(&self, origin: &str) -> Option<String> {
        Self::read(&self.username_service, origin).ok().flatten()
    }

    fn load(&self, origin: &str) -> Result<SignInDetails, BiometricFailure> {
        match Self::read(self.service, origin) {
            Ok(Some(json)) => serde_json::from_str(&json).map_err(|_| {
                self.delete(origin);
                BiometricFailure::Missing
            }),
            Ok(None) => {
                self.delete(origin);
                Err(BiometricFailure::Missing)
            }
            Err(()) => Err(BiometricFailure::Unavailable),
        }
    }

    fn store(&self, origin: &str, details: &SignInDetails) -> Result<(), BiometricFailure> {
        let json = serde_json::to_string(details).map_err(|_| BiometricFailure::Unavailable)?;
        let entry = Self::entry(self.service, origin).ok_or(BiometricFailure::Unavailable)?;
        entry.set_password(&json).map_err(|error| {
            log::warn!("keychain {} write: {error}", self.service);
            BiometricFailure::Unavailable
        })?;
        let saved = Self::entry(&self.username_service, origin).map(|entry| entry.set_password(&details.username));
        if !matches!(saved, Some(Ok(()))) {
            Self::remove(self.service, origin);
            return Err(BiometricFailure::Unavailable);
        }
        Ok(())
    }

    fn delete(&self, origin: &str) {
        Self::remove(self.service, origin);
        Self::remove(&self.username_service, origin);
    }

    /// `keyring` can't list a service's accounts, so this asks the Security framework to delete
    /// every generic password of both services.
    fn delete_all(&self) {
        #[cfg(target_os = "macos")]
        for service in [self.service, self.username_service.as_str()] {
            use security_framework::item::{ItemClass, ItemSearchOptions};
            // Each call may delete one match or all of them, depending on the keychain.
            for _ in 0..256 {
                if ItemSearchOptions::new().class(ItemClass::generic_password()).service(service).delete().is_err() {
                    break;
                }
            }
        }
    }
}

/// In memory: demo mode, and whenever secrets stay in memory (`--data-dir`). The real check
/// still runs.
#[derive(Default)]
pub struct MemoryVault {
    details: Mutex<HashMap<String, SignInDetails>>,
}

impl MemoryVault {
    pub fn new() -> Self {
        Self::default()
    }

    fn map(&self) -> std::sync::MutexGuard<'_, HashMap<String, SignInDetails>> {
        self.details.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl BiometricVault for MemoryVault {
    fn username(&self, origin: &str) -> Option<String> {
        self.map().get(origin).map(|details| details.username.clone())
    }

    fn load(&self, origin: &str) -> Result<SignInDetails, BiometricFailure> {
        self.map().get(origin).cloned().ok_or(BiometricFailure::Missing)
    }

    fn store(&self, origin: &str, details: &SignInDetails) -> Result<(), BiometricFailure> {
        self.map().insert(origin.to_owned(), details.clone());
        Ok(())
    }

    fn delete(&self, origin: &str) {
        self.map().remove(origin);
    }

    fn delete_all(&self) {
        self.map().clear();
    }
}

/// Tests (iOS `FakeBiometricVault`): details in memory, and `load` can be made to fail.
#[cfg(debug_assertions)]
#[derive(Default)]
pub struct FakeVault {
    details: Mutex<HashMap<String, SignInDetails>>,
    load_failure: Mutex<Option<BiometricFailure>>,
}

#[cfg(debug_assertions)]
impl FakeVault {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(origin: &str, details: SignInDetails) -> Self {
        let vault = Self::default();
        vault.map().insert(origin.to_owned(), details);
        vault
    }

    pub fn details(&self) -> HashMap<String, SignInDetails> {
        self.map().clone()
    }

    /// What the next `load`s do: `Missing` deletes the details as an unreadable item would.
    pub fn set_load_failure(&self, failure: Option<BiometricFailure>) {
        *self.load_failure.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = failure;
    }

    fn map(&self) -> std::sync::MutexGuard<'_, HashMap<String, SignInDetails>> {
        self.details.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[cfg(debug_assertions)]
impl BiometricVault for FakeVault {
    fn username(&self, origin: &str) -> Option<String> {
        self.map().get(origin).map(|details| details.username.clone())
    }

    fn load(&self, origin: &str) -> Result<SignInDetails, BiometricFailure> {
        let failure = *self.load_failure.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        match failure {
            Some(BiometricFailure::Missing) => {
                self.delete(origin);
                Err(BiometricFailure::Missing)
            }
            Some(failure) => Err(failure),
            None => self.map().get(origin).cloned().ok_or(BiometricFailure::Missing),
        }
    }

    fn store(&self, origin: &str, details: &SignInDetails) -> Result<(), BiometricFailure> {
        self.map().insert(origin.to_owned(), details.clone());
        Ok(())
    }

    fn delete(&self, origin: &str) {
        self.map().remove(origin);
    }

    fn delete_all(&self) {
        self.map().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn details_debug_hides_the_pin() {
        let shown = format!("{:?}", SignInDetails::new("card", "1234"));
        assert!(shown.contains("card"));
        assert!(!shown.contains("1234"));
    }

    #[test]
    fn details_are_the_json_the_phones_keep() {
        let json = serde_json::to_string(&SignInDetails::new("card", "1234")).unwrap();
        assert_eq!(json, r#"{"username":"card","pin":"1234"}"#);
    }

    #[test]
    fn memory_vault_keeps_each_origin() {
        let vault = MemoryVault::new();
        vault.store("a", &SignInDetails::new("one", "1")).unwrap();
        vault.store("b", &SignInDetails::new("two", "2")).unwrap();
        assert_eq!(vault.username("a").as_deref(), Some("one"));
        vault.delete("a");
        assert_eq!(vault.username("a"), None);
        assert_eq!(vault.load("b").unwrap().pin, "2");
        vault.delete_all();
        assert_eq!(vault.load("b"), Err(BiometricFailure::Missing));
    }
}
