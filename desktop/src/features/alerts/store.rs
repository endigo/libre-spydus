//! `alerts.json` in the data dir (docs/desktop-app.md §11.3): the two switches, and per library
//! (by sha256(origin), like the account cache) the alerted keys and the day the due reminders last
//! showed. Demo mode keeps its own copy under `demo`, so it never changes the real settings (iOS
//! uses the `alerts.demo.` namespace the same way).
//!
//! `AlertSettings::process` is the one place a snapshot turns into notifications (iOS
//! `AlertCenter.process`), pure so it's unit-tested.

use std::collections::BTreeMap;
use std::path::PathBuf;

use chrono::{NaiveDate, NaiveDateTime};
use librespydus_core::AccountSnapshot;
use librespydus_core::app::{due_reminders, ready_alerts};
use serde::{Deserialize, Serialize};

use super::content::{self, AlertNote, DUE_DAYS_BEFORE};
use crate::state::paths::{origin_hash, write_atomically};

/// What the switches say and what each library has been told.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AlertSettings {
    /// "Ready to collect": checked each morning and when the app opens. Off until turned on.
    pub ready_enabled: bool,
    /// "Due-date reminders": 2 days before books are due, at 7:45. Off until turned on.
    pub due_enabled: bool,
    /// By sha256(origin).
    pub libraries: BTreeMap<String, LibraryAlerts>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LibraryAlerts {
    /// The ready reservations already alerted (`patron_item_key`s); `None` when none are recorded,
    /// which isn't the same as an empty list: the next snapshot is only remembered.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alerted_keys: Option<Vec<String>>,
    /// The day the due reminders last showed, and which (`DueReminder.key`) they were: each shows
    /// at most once a day.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_reminder_date: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub reminded_keys: Vec<String>,
}

impl AlertSettings {
    pub fn any_enabled(&self) -> bool {
        self.ready_enabled || self.due_enabled
    }

    /// The alerted keys for `origin`; `None` when none are recorded.
    pub fn alerted_keys(&self, origin: &str) -> Option<&[String]> {
        self.libraries.get(&origin_hash(origin))?.alerted_keys.as_deref()
    }

    pub fn library(&self, origin: &str) -> Option<&LibraryAlerts> {
        self.libraries.get(&origin_hash(origin))
    }

    /// The "Ready to collect" switch (permission already granted). On: remembers what is already
    /// waiting in `saved` without a notification (`ready_alerts(saved, [])`); with no saved account
    /// the first fetched one is remembered instead. Off: forgets every library's alerted keys.
    pub fn set_ready(&mut self, on: bool, origin: Option<&str>, saved: Option<&AccountSnapshot>) {
        if on {
            if self.ready_enabled {
                return;
            }
            self.ready_enabled = true;
            if let Some(origin) = origin {
                let entry = self.libraries.entry(origin_hash(origin)).or_default();
                entry.alerted_keys = saved.map(|saved| ready_alerts(saved.clone(), Vec::new()).alerted_keys);
            }
        } else {
            self.ready_enabled = false;
            for entry in self.libraries.values_mut() {
                entry.alerted_keys = None;
            }
            self.prune();
        }
    }

    /// The "Due-date reminders" switch (permission already granted).
    pub fn set_due(&mut self, on: bool) {
        self.due_enabled = on;
    }

    /// Signed out of `origin`: its alerted keys and reminder days go; other libraries keep theirs.
    pub fn forget(&mut self, origin: &str) {
        self.libraries.remove(&origin_hash(origin));
    }

    /// A snapshot of `origin`'s account, at local time `now`: what to post, as the switches say.
    ///
    /// - Ready: one notification per reservation newly ready (`ready_alerts`), and the alerted keys
    ///   replaced. A library with no saved keys (signed in again, or switched on without a saved
    ///   account) is only remembered, so what is waiting now is never announced as new.
    /// - Due: from 07:45, the reminders whose `remind_on` is today (`due_reminders`, 2 days ahead),
    ///   each at most once that day.
    pub fn process(&mut self, snapshot: &AccountSnapshot, origin: &str, now: NaiveDateTime) -> Vec<AlertNote> {
        let today = now.date();
        let mut notes = Vec::new();
        if self.ready_enabled {
            let entry = self.libraries.entry(origin_hash(origin)).or_default();
            let saved = entry.alerted_keys.take();
            let result = ready_alerts(snapshot.clone(), saved.clone().unwrap_or_default());
            entry.alerted_keys = Some(result.alerted_keys);
            if saved.is_some() {
                notes.extend(result.new_items.iter().map(|item| content::ready_note(item, origin, today)));
            }
        }
        if self.due_enabled && content::reminders_may_show(now) {
            let entry = self.libraries.entry(origin_hash(origin)).or_default();
            if entry.last_reminder_date != Some(today) {
                entry.last_reminder_date = Some(today);
                entry.reminded_keys.clear();
            }
            for reminder in due_reminders(snapshot.clone(), content::simple_date(today), DUE_DAYS_BEFORE) {
                if content::naive_date(reminder.remind_on) != Some(today) || entry.reminded_keys.contains(&reminder.key)
                {
                    continue;
                }
                entry.reminded_keys.push(reminder.key.clone());
                notes.push(content::due_note(&reminder, origin));
            }
        }
        notes
    }

    fn prune(&mut self) {
        self.libraries.retain(|_, entry| *entry != LibraryAlerts::default());
    }
}

/// The file: the real settings and demo mode's.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
struct AlertsFile {
    app: AlertSettings,
    demo: AlertSettings,
}

/// `alerts.json`, or memory only (tests: `Paths::memory()`).
pub struct AlertStore {
    path: Option<PathBuf>,
    demo: bool,
    file: AlertsFile,
}

impl AlertStore {
    /// Loads `path` (an unreadable file starts afresh); `demo` uses demo mode's settings.
    pub fn load(path: Option<PathBuf>, demo: bool) -> Self {
        let file = path
            .as_ref()
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        Self { path, demo, file }
    }

    pub fn settings(&self) -> &AlertSettings {
        if self.demo { &self.file.demo } else { &self.file.app }
    }

    /// Changes the settings and saves the file.
    pub fn update<R>(&mut self, change: impl FnOnce(&mut AlertSettings) -> R) -> R {
        let settings = if self.demo { &mut self.file.demo } else { &mut self.file.app };
        let result = change(settings);
        self.save();
        result
    }

    fn save(&self) {
        let Some(path) = &self.path else { return };
        // Losing it only means a switch turns off or an alert repeats; nothing to show the reader.
        let written = serde_json::to_vec_pretty(&self.file)
            .map_err(std::io::Error::other)
            .and_then(|bytes| write_atomically(path, &bytes));
        if let Err(error) = written {
            log::warn!("couldn't save the alert settings: {error}");
        }
    }
}
