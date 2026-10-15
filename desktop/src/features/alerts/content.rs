//! What the alerts say, their identifiers, what a click opens and when the morning check runs
//! (docs/native-apps.md §5.8; a port of iOS `AlertContent.swift`). Pure, so the wording and the
//! timing are unit-tested.

use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime};
use librespydus_core::app::{DueReminder, SimpleDate, patron_item_key};
use librespydus_core::{AccountSnapshot, PatronItem};

use crate::copy::Copy;
use crate::features::your_books::rows::{LoanRow, ReservationKind, ReservationRow, ReservationSections};
use crate::state::formatting::non_empty;
use crate::state::paths::origin_hash;
use crate::state::{BookRef, YourBooksSegment};

/// The morning alert time, local: the morning check runs then and due reminders show from then.
pub const ALERT_HOUR: u32 = 7;
pub const ALERT_MINUTE: u32 = 45;
/// Due reminders come this many days before the due date.
pub const DUE_DAYS_BEFORE: u32 = 2;

pub fn alert_time() -> NaiveTime {
    NaiveTime::from_hms_opt(ALERT_HOUR, ALERT_MINUTE, 0).expect("07:45 is a time")
}

/// A notification's kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlertKind {
    Ready,
    Due,
}

/// What a click on a notification opens: its kind, its library and the `patron_item_key`s it
/// names (§5.8 "Tap").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlertTap {
    pub kind: AlertKind,
    pub origin: String,
    pub keys: Vec<String>,
}

impl AlertTap {
    /// Your Books' list for this kind.
    pub fn segment(&self) -> YourBooksSegment {
        match self.kind {
            AlertKind::Ready => YourBooksSegment::Reservations,
            AlertKind::Due => YourBooksSegment::Loans,
        }
    }
}

/// One notification: shown as a macOS notification, or as an in-app toast when the app can't post.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlertNote {
    /// `ready:<originHash>:<key>` or `due:<originHash>:<YYYY-MM-DD>`: a repeat replaces the earlier one.
    pub id: String,
    pub title: String,
    pub body: String,
    pub tap: AlertTap,
}

// ---- Ready to collect ----

/// "<Title> · <Branch>", the branch as Your Books shows it for the row (the pickup location, else
/// the branch in Canning's status text, title-cased when shouted); just the title when the
/// branch is unknown; "A reservation" without a title.
pub fn ready_body(item: &PatronItem, today: NaiveDate) -> String {
    let title = non_empty(item.title.as_deref()).unwrap_or_else(|| Copy::Alerts::untitledReservation.to_owned());
    match ReservationRow::new(item, ReservationKind::Ready, today).pickup {
        Some(branch) => Copy::Alerts::readyBody(&title, &branch),
        None => title,
    }
}

/// The "Ready to collect" notification for `item`, a newly ready reservation of `origin`'s account.
pub fn ready_note(item: &PatronItem, origin: &str, today: NaiveDate) -> AlertNote {
    let key = patron_item_key(item);
    AlertNote {
        id: ready_id(&origin_hash(origin), &key),
        title: Copy::Alerts::readyTitle.to_owned(),
        body: ready_body(item, today),
        tap: AlertTap { kind: AlertKind::Ready, origin: origin.to_owned(), keys: vec![key] },
    }
}

// ---- Due-date reminders ----

/// "Due Thursday 16 Oct"; "Due today" when shown on the due date, "Due tomorrow" the day before.
pub fn due_title(due: SimpleDate, shown_on: SimpleDate) -> String {
    let (Some(due_day), Some(shown)) = (naive_date(due), naive_date(shown_on)) else {
        // Not a calendar date: its day and month, in the same "16 Oct" style.
        let month = MONTHS[usize::from(due.month.clamp(1, 12)) - 1];
        return Copy::Loans::dueOn(&format!("{} {month}", due.day));
    };
    match (due_day - shown).num_days() {
        0 => Copy::Loans::dueToday.to_owned(),
        1 => Copy::Loans::dueTomorrow.to_owned(),
        _ => Copy::Loans::dueOn(&due_day.format("%A %-d %b").to_string()),
    }
}

// copy-exempt: English short month names, as chrono's `%b` writes them
const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// One loan: its title; two: "<A> and <B>"; more: "<A> and N more". A loan without a title is
/// "A loan".
pub fn due_body(items: &[PatronItem]) -> String {
    let titles: Vec<String> = items
        .iter()
        .map(|item| non_empty(item.title.as_deref()).unwrap_or_else(|| Copy::Alerts::untitledLoan.to_owned()))
        .collect();
    match titles.as_slice() {
        [] => String::new(),
        [one] => one.clone(),
        [first, second] => Copy::Alerts::dueTwo(first, second),
        [first, rest @ ..] => Copy::Alerts::dueMore(rest.len() as i64, first),
    }
}

/// The reminder for `reminder`'s loans of `origin`'s account. It names every loan due that day.
pub fn due_note(reminder: &DueReminder, origin: &str) -> AlertNote {
    AlertNote {
        id: due_id(&origin_hash(origin), reminder.due),
        title: due_title(reminder.due, reminder.remind_on),
        body: due_body(&reminder.items),
        tap: AlertTap {
            kind: AlertKind::Due,
            origin: origin.to_owned(),
            keys: reminder.items.iter().map(patron_item_key).collect(),
        },
    }
}

// ---- Identifiers ----

/// `ready:<originHash>:<patronItemKey>`.
pub fn ready_id(origin_hash: &str, key: &str) -> String {
    format!("ready:{origin_hash}:{key}") // copy-exempt: a notification identifier
}

/// `due:<originHash>:<YYYY-MM-DD>`.
pub fn due_id(origin_hash: &str, due: SimpleDate) -> String {
    format!("due:{origin_hash}:{:04}-{:02}-{:02}", due.year, due.month, due.day) // copy-exempt: an identifier
}

// ---- Tap ----

/// The book page a click on a notification opens over its Your Books list: the one reservation or
/// loan it names, found by `patron_item_key` among the rows Your Books shows for `snapshot` and
/// opened with that row's own `BookRef`, as clicking the row does. `None` (just the list) when it
/// names several loans or none, or the item is no longer in the account (collected, returned).
pub fn book_for_tap(tap: &AlertTap, snapshot: &AccountSnapshot, today: NaiveDate) -> Option<BookRef> {
    let [key] = tap.keys.as_slice() else { return None };
    if key.is_empty() {
        return None;
    }
    match tap.kind {
        AlertKind::Ready => {
            let sections = ReservationSections::new(&snapshot.reservations, today);
            sections
                .ready
                .iter()
                .chain(&sections.pending)
                .find(|row| patron_item_key(&row.item) == *key)
                .map(|row| row.book.clone())
        }
        AlertKind::Due => LoanRow::rows(&snapshot.loans.items, today)
            .into_iter()
            .find(|row| patron_item_key(&row.item) == *key)
            .map(|row| row.book),
    }
}

// ---- Time ----

/// The next 07:45 strictly after `now` (local wall-clock time): today's while it's still ahead,
/// else tomorrow's.
pub fn next_alert_time(now: NaiveDateTime) -> NaiveDateTime {
    let today = now.date().and_time(alert_time());
    if today > now { today } else { today + Duration::days(1) }
}

/// The morning check is due once the wall clock reaches `scheduled`. The timer that wakes the app
/// runs on a clock that stops while the Mac sleeps, so the app also compares the wall clock
/// regularly: after a night asleep the check runs at once, once, and the next one is the
/// following 07:45 (`next_alert_time(now)`).
pub fn morning_check_due(now: NaiveDateTime, scheduled: NaiveDateTime) -> bool {
    now >= scheduled
}

/// Whether due reminders may show at `now`: from 07:45 on, as on the phones.
pub fn reminders_may_show(now: NaiveDateTime) -> bool {
    now.time() >= alert_time()
}

pub fn simple_date(date: NaiveDate) -> SimpleDate {
    use chrono::Datelike;
    SimpleDate { year: date.year(), month: date.month() as u8, day: date.day() as u8 }
}

pub fn naive_date(date: SimpleDate) -> Option<NaiveDate> {
    NaiveDate::from_ymd_opt(date.year, u32::from(date.month), u32::from(date.day))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(date: &str, time: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(&format!("{date} {time}"), "%Y-%m-%d %H:%M").unwrap()
    }

    fn day(y: i32, m: u8, d: u8) -> SimpleDate {
        SimpleDate { year: y, month: m, day: d }
    }

    #[test]
    fn the_next_alert_is_the_coming_0745() {
        assert_eq!(next_alert_time(at("2026-10-15", "06:00")), at("2026-10-15", "07:45"));
        assert_eq!(next_alert_time(at("2026-10-15", "07:45")), at("2026-10-16", "07:45"));
        assert_eq!(next_alert_time(at("2026-10-15", "21:30")), at("2026-10-16", "07:45"));
        assert_eq!(next_alert_time(at("2026-12-31", "08:00")), at("2027-01-01", "07:45"));
    }

    #[test]
    fn due_titles_say_today_tomorrow_or_the_weekday() {
        assert_eq!(due_title(day(2026, 10, 15), day(2026, 10, 15)), "Due today");
        assert_eq!(due_title(day(2026, 10, 16), day(2026, 10, 15)), "Due tomorrow");
        assert_eq!(due_title(day(2026, 10, 17), day(2026, 10, 15)), "Due Saturday 17 Oct");
        assert_eq!(due_title(day(2026, 2, 30), day(2026, 2, 28)), "Due 30 Feb");
    }

    #[test]
    fn due_bodies_name_one_two_or_the_first_and_a_count() {
        let loan = |title: Option<&str>| PatronItem { title: title.map(str::to_owned), ..Default::default() };
        assert_eq!(due_body(&[loan(Some("A"))]), "A");
        assert_eq!(due_body(&[loan(Some("A")), loan(None)]), "A and A loan");
        assert_eq!(due_body(&[loan(Some("A")), loan(Some("B")), loan(Some("C"))]), "A and 2 more");
    }

    #[test]
    fn ids_name_the_library_by_hash() {
        let hash = origin_hash("https://canning.spydus.com");
        assert_eq!(ready_id(&hash, "19000003"), format!("ready:{hash}:19000003"));
        assert_eq!(due_id(&hash, day(2026, 1, 5)), format!("due:{hash}:2026-01-05"));
    }
}
