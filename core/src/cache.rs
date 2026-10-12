//! Thread-safe in-memory TTL cache (the middleware's `cached()` map, server.ts:40-50). Owner: task E.
//!
//! An entry is live while its expiry is strictly after "now" (`hit.exp > now`); expired entries
//! read as absent and are dropped lazily. Values are cloned out, so `V` is usually cheap to clone
//! (an `Arc` or a small record).

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

pub struct TtlCache<V: Clone + Send> {
    /// key → (expiry, value). `None` expiry = a TTL too large for `Instant` (never expires).
    entries: Mutex<HashMap<String, (Option<Instant>, V)>>,
}

impl<V: Clone + Send> TtlCache<V> {
    pub fn new() -> Self {
        Self { entries: Mutex::new(HashMap::new()) }
    }

    /// The live value for `key`, or `None` if absent or expired (an expired entry is removed).
    pub fn get(&self, key: &str) -> Option<V> {
        let now = Instant::now();
        let mut entries = self.lock();
        match entries.get(key) {
            Some((expiry, value)) if is_live(*expiry, now) => Some(value.clone()),
            Some(_) => {
                entries.remove(key);
                None
            }
            None => None,
        }
    }

    /// Store `value` under `key` for `ttl` (a zero TTL is never readable), replacing any entry.
    pub fn insert(&self, key: String, value: V, ttl: Duration) {
        let now = Instant::now();
        let expiry = now.checked_add(ttl);
        let mut entries = self.lock();
        // Drop dead entries so keys that are never read again (e.g. old sessions) don't pile up.
        entries.retain(|_, (exp, _)| is_live(*exp, now));
        entries.insert(key, (expiry, value));
    }

    pub fn remove(&self, key: &str) {
        self.lock().remove(key);
    }

    /// A panic while holding the lock can't leave the map half-updated, so recover from poison
    /// instead of propagating the panic (it would abort across the FFI boundary).
    fn lock(&self) -> MutexGuard<'_, HashMap<String, (Option<Instant>, V)>> {
        self.entries.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl<V: Clone + Send> Default for TtlCache<V> {
    fn default() -> Self {
        Self::new()
    }
}

fn is_live(expiry: Option<Instant>, now: Instant) -> bool {
    expiry.is_none_or(|exp| exp > now)
}
