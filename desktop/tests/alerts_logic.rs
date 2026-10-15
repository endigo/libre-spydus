//! The alerts' rules without a window (docs/native-apps.md §5.8, docs/desktop-app.md §11.3): the
//! 07:45 morning check and a Mac that slept through it, ready alerts deduplicated by key, due
//! reminders once a day, sign-out forgetting a library, the switches and `alerts.json`.

use chrono::{NaiveDate, NaiveDateTime};
use librespydus_core::{AccountSnapshot, PatronItem, PatronList, Reservations};
use librespydus_desktop::features::alerts::content::{self, AlertKind};
use librespydus_desktop::features::alerts::store::{AlertSettings, AlertStore};
use librespydus_desktop::state::BookRef;

const CANNING: &str = "https://canning.spydus.com";
const OTHER: &str = "https://hutt.spydus.co.nz";

fn at(text: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M").unwrap()
}

fn reservation(irn: &str, title: &str, status: &str) -> PatronItem {
    let mut item = PatronItem { irn: Some(irn.into()), title: Some(title.into()), ..Default::default() };
    item.fields.insert("Status".into(), status.into());
    item
}

fn loan(irn: &str, title: &str, due: &str) -> PatronItem {
    let mut item = PatronItem { irn: Some(irn.into()), title: Some(title.into()), ..Default::default() };
    item.fields.insert("Due".into(), due.into());
    item
}

fn snapshot(ready: Vec<PatronItem>, loans: Vec<PatronItem>) -> AccountSnapshot {
    AccountSnapshot {
        loans: PatronList { total: Some(loans.len() as i64), items: loans },
        reservations: Reservations { ready: PatronList { total: None, items: ready }, ..Default::default() },
        ..Default::default()
    }
}

fn waiting() -> PatronItem {
    reservation("1", "Waiting", "Please pickup from: RIVERTON LIBRARY by 22 Oct 2026")
}

fn new_one() -> PatronItem {
    reservation("2", "Hollow bones", "Please pickup from: RIVERTON LIBRARY by 22 Oct 2026")
}

// ---- Scheduling ----

#[test]
fn the_morning_check_runs_at_the_next_0745() {
    assert_eq!(content::next_alert_time(at("2026-10-15 07:44")), at("2026-10-15 07:45"));
    assert_eq!(content::next_alert_time(at("2026-10-15 07:45")), at("2026-10-16 07:45"));
    assert_eq!(content::next_alert_time(at("2026-10-15 23:59")), at("2026-10-16 07:45"));
    assert!(!content::morning_check_due(at("2026-10-15 07:44"), at("2026-10-15 07:45")));
    assert!(content::morning_check_due(at("2026-10-15 07:45"), at("2026-10-15 07:45")));
}

#[test]
fn after_a_night_asleep_the_check_runs_once_and_the_next_is_tomorrow() {
    // Scheduled for Thursday 07:45; the Mac slept from Wednesday night to Friday 09:10.
    let scheduled = content::next_alert_time(at("2026-10-14 23:00"));
    assert_eq!(scheduled, at("2026-10-15 07:45"));
    let woke = at("2026-10-16 09:10");
    assert!(content::morning_check_due(woke, scheduled));
    let next = content::next_alert_time(woke);
    assert_eq!(next, at("2026-10-17 07:45"));
    assert!(!content::morning_check_due(at("2026-10-16 09:11"), next));
}

// ---- Ready to collect ----

#[test]
fn ready_alerts_announce_each_reservation_once() {
    let mut settings = AlertSettings::default();
    settings.set_ready(true, Some(CANNING), Some(&snapshot(vec![waiting()], vec![])));
    // What was waiting when the switch went on is remembered, not announced.
    assert_eq!(settings.alerted_keys(CANNING), Some(&["1".to_owned()][..]));
    let now = at("2026-10-15 12:00");
    assert!(settings.process(&snapshot(vec![waiting()], vec![]), CANNING, now).is_empty());

    let notes = settings.process(&snapshot(vec![waiting(), new_one()], vec![]), CANNING, now);
    assert_eq!(notes.len(), 1);
    let note = &notes[0];
    assert_eq!(note.title, "Ready to collect");
    assert_eq!(note.body, "Hollow bones · Riverton Library");
    assert!(note.id.starts_with("ready:") && note.id.ends_with(":2"));
    assert_eq!(note.tap.kind, AlertKind::Ready);
    assert_eq!(note.tap.keys, vec!["2".to_owned()]);
    // Not again.
    assert!(settings.process(&snapshot(vec![waiting(), new_one()], vec![]), CANNING, now).is_empty());
    // Collected, then ready again later: announced again.
    assert!(settings.process(&snapshot(vec![waiting()], vec![]), CANNING, now).is_empty());
    assert_eq!(settings.process(&snapshot(vec![waiting(), new_one()], vec![]), CANNING, now).len(), 1);
}

#[test]
fn a_library_without_saved_keys_is_only_remembered() {
    let mut settings = AlertSettings::default();
    // Switched on without a saved account.
    settings.set_ready(true, Some(CANNING), None);
    assert_eq!(settings.alerted_keys(CANNING), None);
    let now = at("2026-10-15 12:00");
    assert!(settings.process(&snapshot(vec![waiting(), new_one()], vec![]), CANNING, now).is_empty());
    assert_eq!(settings.alerted_keys(CANNING).map(<[String]>::len), Some(2));
    // Another library seen for the first time: the same.
    assert!(settings.process(&snapshot(vec![waiting()], vec![]), OTHER, now).is_empty());
}

#[test]
fn a_ready_alert_without_a_branch_or_title_still_says_something() {
    let today = NaiveDate::from_ymd_opt(2026, 10, 15).unwrap();
    let plain = PatronItem { irn: Some("3".into()), title: Some("Plain".into()), ..Default::default() };
    assert_eq!(content::ready_body(&plain, today), "Plain");
    assert_eq!(content::ready_body(&PatronItem::default(), today), "A reservation");
}

// ---- Due-date reminders ----

#[test]
fn due_reminders_show_from_0745_once_a_day() {
    let mut settings = AlertSettings::default();
    settings.set_due(true);
    let account = snapshot(
        vec![],
        vec![
            loan("10", "Whistler", "17 Oct 2026"),
            loan("11", "Dune", "17 Oct 2026"),
            loan("12", "Later", "30 Oct 2026"),
            loan("13", "Tomorrow", "16 Oct 2026"),
        ],
    );
    // Before 07:45: nothing yet.
    assert!(settings.process(&account, CANNING, at("2026-10-15 07:00")).is_empty());
    let notes = settings.process(&account, CANNING, at("2026-10-15 07:45"));
    let titles: Vec<(&str, &str)> = notes.iter().map(|note| (note.title.as_str(), note.body.as_str())).collect();
    assert_eq!(titles, vec![("Due tomorrow", "Tomorrow"), ("Due Saturday 17 Oct", "Whistler and Dune")]);
    assert_eq!(notes[1].tap.kind, AlertKind::Due);
    assert_eq!(notes[1].tap.keys, vec!["10".to_owned(), "11".to_owned()]);
    // Refreshes later that day: not again.
    assert!(settings.process(&account, CANNING, at("2026-10-15 13:00")).is_empty());
    assert!(settings.process(&account, CANNING, at("2026-10-15 23:59")).is_empty());
    // The next morning: today's again ("Due today" / "Due tomorrow").
    let notes = settings.process(&account, CANNING, at("2026-10-16 08:30"));
    let titles: Vec<&str> = notes.iter().map(|note| note.title.as_str()).collect();
    assert_eq!(titles, vec!["Due today", "Due tomorrow"]);
    assert_eq!(settings.library(CANNING).unwrap().last_reminder_date, NaiveDate::from_ymd_opt(2026, 10, 16));
}

#[test]
fn nothing_shows_while_the_switches_are_off() {
    let mut settings = AlertSettings::default();
    let account = snapshot(vec![waiting()], vec![loan("10", "Whistler", "17 Oct 2026")]);
    assert!(settings.process(&account, CANNING, at("2026-10-15 09:00")).is_empty());
    assert!(settings.libraries.is_empty());
}

// ---- Switches and sign-out ----

#[test]
fn turning_ready_off_forgets_every_library_and_sign_out_forgets_one() {
    let mut settings = AlertSettings::default();
    settings.set_due(true);
    settings.set_ready(true, Some(CANNING), Some(&snapshot(vec![waiting()], vec![])));
    let now = at("2026-10-15 09:00");
    settings.process(&snapshot(vec![waiting()], vec![loan("10", "Whistler", "17 Oct 2026")]), OTHER, now);
    assert!(settings.alerted_keys(OTHER).is_some());

    // Sign-out at Canning: only Canning's entry goes.
    settings.forget(CANNING);
    assert!(settings.library(CANNING).is_none());
    assert!(settings.library(OTHER).is_some());
    assert!(settings.ready_enabled && settings.due_enabled);

    // Ready off: every library's keys go; the reminder days stay.
    settings.set_ready(false, Some(OTHER), None);
    assert!(!settings.ready_enabled);
    assert_eq!(settings.alerted_keys(OTHER), None);
    assert!(settings.library(OTHER).unwrap().last_reminder_date.is_some());
    settings.set_due(false);
    assert!(!settings.any_enabled());
}

#[test]
fn the_file_keeps_the_switches_and_demo_mode_its_own() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("alerts.json");
    let mut store = AlertStore::load(Some(path.clone()), false);
    store.update(|settings| settings.set_ready(true, Some(CANNING), Some(&snapshot(vec![waiting()], vec![]))));
    let mut demo = AlertStore::load(Some(path.clone()), true);
    assert!(!demo.settings().ready_enabled, "demo mode has its own settings");
    demo.update(|settings| settings.set_due(true));

    let again = AlertStore::load(Some(path.clone()), false);
    assert!(again.settings().ready_enabled);
    assert!(!again.settings().due_enabled);
    assert_eq!(again.settings().alerted_keys(CANNING), Some(&["1".to_owned()][..]));
    let json = std::fs::read_to_string(&path).unwrap();
    assert!(json.contains("\"readyEnabled\": true") && json.contains("\"demo\""));
    assert!(!json.contains(CANNING), "libraries are named by hash");
    // A broken file starts afresh.
    std::fs::write(&path, b"{").unwrap();
    assert_eq!(AlertStore::load(Some(path), false).settings(), &AlertSettings::default());
}

// ---- Tap ----

#[test]
fn a_tap_finds_the_book_it_names() {
    let today = NaiveDate::from_ymd_opt(2026, 10, 15).unwrap();
    let mut account = snapshot(vec![waiting(), new_one()], vec![loan("10", "Whistler", "17 Oct 2026")]);
    account.reservations.ready.items[1].bib_irn = Some("555".into());
    let tap = |kind, keys: &[&str]| content::AlertTap {
        kind,
        origin: CANNING.into(),
        keys: keys.iter().map(|key| (*key).to_owned()).collect(),
    };
    let book: Option<BookRef> = content::book_for_tap(&tap(AlertKind::Ready, &["2"]), &account, today);
    assert_eq!(book.map(|book| book.title), Some("Hollow bones".to_owned()));
    assert_eq!(
        content::book_for_tap(&tap(AlertKind::Due, &["10"]), &account, today).map(|book| book.title),
        Some("Whistler".to_owned())
    );
    // Several loans, or one no longer there: just the list.
    assert!(content::book_for_tap(&tap(AlertKind::Due, &["10", "11"]), &account, today).is_none());
    assert!(content::book_for_tap(&tap(AlertKind::Ready, &["9"]), &account, today).is_none());
}
