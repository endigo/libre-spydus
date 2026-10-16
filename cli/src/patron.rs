//! What Your Books' rows show, built from the core's `PatronItem`s: the apps' order and wording.
//! Ported from the desktop app's `state/patron.rs` (action links), `features/your_books/rows.rs`
//! (loan and reservation rows, iOS `BorrowedRows.swift`) and `state/renew.rs` (a renewal's words,
//! iOS `RenewText`). Pure, so the sorting and wording are unit-tested.

use std::collections::HashSet;

use chrono::NaiveDate;
use librespydus_core::{PatronItem, RenewResult, Reservations};

use crate::book_ref::BookRef;
use crate::copy::Copy;
use crate::formatting::{DueStatus, due_date, non_empty, short_date};

// copy-exempt: the loans list's field name (a key)
pub const DUE: &str = "Due";
// copy-exempt: the reservations list's field names (keys)
const STATUS: &str = "Status";
// copy-exempt: a field name (a key)
const RANK: &str = "Rank";
// copy-exempt: a field name (a key)
const PLACED: &str = "Placed";
// copy-exempt: a field name (a key)
pub const PICKUP_LOCATION: &str = "Pickup location";

/// A loan's or reservation's action links (the labels vary by library: "Cancel reservation",
/// "Edit", "Edit reservation", "Renew loan").
pub trait PatronItemExt {
    /// The href of the first action whose label contains `keyword` (case-insensitive).
    fn action_href(&self, keyword: &str) -> Option<String>;
    /// `/cancel/i`: the reservation's cancel link.
    fn cancel_href(&self) -> Option<String> {
        self.action_href("cancel")
    }
    /// `/edit/i`: the reservation's edit link.
    fn edit_href(&self) -> Option<String> {
        self.action_href("edit")
    }
    /// `/renew/i`: the loan's "Renew loan" link.
    fn renew_href(&self) -> Option<String> {
        self.action_href("renew")
    }
    /// Whether one of its actions is `href`.
    fn has_action(&self, href: &str) -> bool;
}

impl PatronItemExt for PatronItem {
    fn action_href(&self, keyword: &str) -> Option<String> {
        let keyword = keyword.to_lowercase();
        self.actions
            .iter()
            .find(|action| action.label.to_lowercase().contains(&keyword))
            .and_then(|action| non_empty(Some(&action.href)))
    }

    fn has_action(&self, href: &str) -> bool {
        self.actions.iter().any(|action| action.href == href)
    }
}

/// One loan row: "Due 27 Oct · 3 days left", the error colour when overdue.
#[derive(Debug, Clone, PartialEq)]
pub struct LoanRow {
    pub item: PatronItem,
    pub book: BookRef,
    /// `None` without a "Due" field or when the core can't read it as a date.
    pub due: Option<DueStatus>,
    /// `fields["Due"]` as the library wrote it.
    pub due_text: Option<String>,
    /// The "Renew loan" link; `None` when the library offers no renewal for this loan.
    pub renew_href: Option<String>,
    today: NaiveDate,
}

impl LoanRow {
    pub fn new(item: &PatronItem, today: NaiveDate) -> Self {
        let book = BookRef::from_patron_item(item);
        let due_text = non_empty(item.fields.get(DUE).map(String::as_str));
        Self {
            due: DueStatus::from_due_text(due_text.as_deref(), today),
            due_text,
            renew_href: item.renew_href(),
            item: item.clone(),
            book,
            today,
        }
    }

    /// The due line; an unreadable date is shown as the library wrote it.
    pub fn meta(&self) -> Option<String> {
        if let Some(due) = &self.due {
            return Some(due_line(due, self.today));
        }
        let text = self.due_text.as_deref()?;
        Some(if says_due(text) { text.to_owned() } else { Copy::Loans::dueOn(text) })
    }

    pub fn is_overdue(&self) -> bool {
        self.due.is_some_and(|due| due.is_overdue())
    }

    /// Renew when the library gave the loan its link.
    pub fn can_renew(&self) -> bool {
        self.renew_href.is_some()
    }

    /// Loans soonest-due first (overdue at the top); loans without a readable date go last, in
    /// the library's order. This is the order the apps show and the CLI numbers.
    pub fn rows(items: &[PatronItem], today: NaiveDate) -> Vec<LoanRow> {
        let mut rows: Vec<(usize, LoanRow)> =
            items.iter().enumerate().map(|(index, item)| (index, LoanRow::new(item, today))).collect();
        rows.sort_by(|(left_index, left), (right_index, right)| {
            match (left.due.map(|due| due.due_date), right.due.map(|due| due.due_date)) {
                (Some(a), Some(b)) if a != b => a.cmp(&b),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                _ => left_index.cmp(right_index),
            }
        });
        rows.into_iter().map(|(_, row)| row).collect()
    }
}

/// "Due 27 Oct · 3 days left", "Due tomorrow · 13 Oct", "Due today · 12 Oct", "Due 10 Oct · 2 days
/// overdue".
pub fn due_line(due: &DueStatus, today: NaiveDate) -> String {
    let date = short_date(due.due_date, today);
    match due.days_left {
        ..0 => Copy::Loans::lineDays(&date, &Copy::Loans::daysOverdue(-due.days_left)),
        0 => Copy::Loans::lineToday(&date),
        1 => Copy::Loans::lineTomorrow(&date),
        days => Copy::Loans::lineDays(&date, &Copy::Loans::daysLeft(days)),
    }
}

/// Ready to collect, or waiting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReservationKind {
    Ready,
    Pending,
}

/// One reservation row.
#[derive(Debug, Clone, PartialEq)]
pub struct ReservationRow {
    pub kind: ReservationKind,
    pub item: PatronItem,
    pub book: BookRef,
    /// The library's status ("In transit"); `None` when it only names the pickup branch.
    pub status: Option<String>,
    pub rank: Option<String>,
    pub placed: Option<String>,
    /// `fields["Pickup location"]`, or the branch named in the status; title-cased when shouted.
    pub pickup: Option<String>,
    /// Ready reservations: the last day to collect, as the library wrote it.
    pub pickup_by: Option<String>,
    pub cancel_href: Option<String>,
    pub edit_href: Option<String>,
    pub today: NaiveDate,
}

impl ReservationRow {
    pub fn new(item: &PatronItem, kind: ReservationKind, today: NaiveDate) -> Self {
        let book = BookRef::from_patron_item(item);
        let field = |name: &str| non_empty(item.fields.get(name).map(String::as_str));
        let status_text = StatusText::parse(field(STATUS).as_deref());
        Self {
            kind,
            status: status_text.status,
            rank: field(RANK),
            placed: field(PLACED),
            pickup: display_name(field(PICKUP_LOCATION).as_deref())
                .or_else(|| display_name(status_text.pickup.as_deref())),
            pickup_by: status_text.pickup_by,
            cancel_href: item.cancel_href(),
            edit_href: item.edit_href(),
            item: item.clone(),
            book,
            today,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.kind == ReservationKind::Ready
    }

    /// "Ready to collect", or the library's own pending status.
    pub fn status_label(&self) -> String {
        if self.is_ready() {
            return Copy::YourBooks::readyToCollect.to_owned();
        }
        self.status.clone().unwrap_or_else(|| Copy::YourBooks::pending.to_owned())
    }

    fn library_date(&self, text: &str) -> String {
        due_date::parse(text).map_or_else(|| text.to_owned(), |date| short_date(date, self.today))
    }

    /// The status line: "Ready to collect"; "Pending · #4 in queue · Placed 2 Oct"; "In transit".
    pub fn status_line(&self) -> String {
        let mut parts = vec![self.status_label()];
        if !self.is_ready() {
            if let Some(rank) = &self.rank {
                parts.push(if rank.parse::<i64>().is_ok() {
                    Copy::YourBooks::queuePlace(rank)
                } else {
                    Copy::YourBooks::queuePlaceText(rank)
                });
            }
            if let Some(placed) = &self.placed {
                parts.push(Copy::YourBooks::placed(&self.library_date(placed)));
            }
        }
        parts.join(Copy::Common::separator)
    }

    /// The pickup line: "Collect from Riverton Library by 17 Oct", "Collect from Riverton
    /// Library", "Collect by 17 Oct", or nothing.
    pub fn pickup_line(&self) -> Option<String> {
        let by = self.pickup_by.as_deref().map(|date| self.library_date(date));
        match (&self.pickup, by) {
            (Some(branch), Some(date)) => Some(Copy::YourBooks::collectFromBy(branch, &date)),
            (Some(branch), None) => Some(Copy::YourBooks::collectFrom(branch)),
            (None, Some(date)) => Some(Copy::YourBooks::collectBy(&date)),
            (None, None) => None,
        }
    }

    /// The cancel confirmation's message.
    pub fn cancel_message(&self) -> String {
        if self.is_ready() {
            return match &self.pickup {
                Some(branch) => Copy::YourBooks::cancelReadyMessage(&self.book.title, branch),
                None => Copy::YourBooks::cancelReadyMessageNoBranch(&self.book.title),
            };
        }
        Copy::YourBooks::cancelPendingMessage(&self.book.title)
    }
}

/// "RIVERTON LIBRARY" → "Riverton Library"; mixed case is kept.
pub fn display_name(raw: Option<&str>) -> Option<String> {
    let name = non_empty(raw)?;
    if name != name.to_uppercase() || !name.chars().any(char::is_alphabetic) {
        return Some(name);
    }
    let mut out = String::with_capacity(name.len());
    let mut start = true;
    for ch in name.chars() {
        if ch.is_alphabetic() {
            if start {
                out.extend(ch.to_uppercase());
            } else {
                out.extend(ch.to_lowercase());
            }
            start = false;
        } else {
            out.push(ch);
            start = !(ch == '\'' || ch == '’');
        }
    }
    Some(out)
}

/// A reservation's `Status` field. Some libraries (Canning) put the pickup branch there: "Requested
/// pickup location: RIVERTON LIBRARY" while pending, "Please pickup from: RIVERTON LIBRARY by 17
/// Oct 2026" once ready. Those become a branch (and collect-by date) with no status.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StatusText {
    pub status: Option<String>,
    pub pickup: Option<String>,
    pub pickup_by: Option<String>,
}

impl StatusText {
    pub fn parse(raw: Option<&str>) -> Self {
        let Some(text) = non_empty(raw) else {
            return Self::default();
        };
        if let Some((label, rest)) = text.split_once(':') {
            let label: String = label.to_lowercase().chars().filter(|ch| !ch.is_whitespace() && *ch != '-').collect();
            match label.as_str() {
                // copy-exempt: the library's status wording, matched
                "requestedpickuplocation" => {
                    return Self { pickup: non_empty(Some(rest)), ..Default::default() };
                }
                // copy-exempt: the library's status wording, matched
                "pleasepickupfrom" => {
                    let rest = rest.trim();
                    let lower = rest.to_lowercase();
                    // copy-exempt: the library's status wording, matched
                    let split = lower.match_indices(" by ").map(|(at, _)| at).find(|at| *at > 0);
                    let (branch, by) = match split {
                        Some(at) => (&rest[..at], Some(&rest[at + 4..])),
                        None => (rest, None),
                    };
                    return Self { pickup: non_empty(Some(branch)), pickup_by: non_empty(by), status: None };
                }
                _ => {}
            }
        }
        Self { status: Some(text), ..Default::default() }
    }
}

/// Reservations in the apps' order: ready to collect first, then pending. A row the library
/// lists on both pages is shown once, as ready.
pub fn reservation_rows(reservations: &Reservations, today: NaiveDate) -> Vec<ReservationRow> {
    let ready_irns: HashSet<String> =
        reservations.ready.items.iter().filter_map(|item| non_empty(item.irn.as_deref())).collect();
    let ready = reservations.ready.items.iter().map(|item| ReservationRow::new(item, ReservationKind::Ready, today));
    let pending = reservations
        .pending
        .items
        .iter()
        .filter(|item| non_empty(item.irn.as_deref()).is_none_or(|irn| !ready_irns.contains(&irn)))
        .map(|item| ReservationRow::new(item, ReservationKind::Pending, today));
    ready.chain(pending).collect()
}

// ---- A renewal's words (iOS `RenewText`) ----

/// The confirmation's message: "“Whistler” is due 15 Oct. Renewing asks the library for a later
/// due date.", or without the date when the loan has none.
pub fn confirm_renew_message(loan: &PatronItem, today: NaiveDate) -> String {
    let title = BookRef::from_patron_item(loan).title;
    match due_date_text(loan, today) {
        Some(date) => Copy::YourBooks::renewMessage(&title, &date),
        None => Copy::YourBooks::renewMessageNoDate(&title),
    }
}

/// The loan's due date as `Copy::Loans::dueOn` writes it ("27 Oct"), or as the library wrote it
/// when the core can't read it (unless that already says "due").
pub fn due_date_text(loan: &PatronItem, today: NaiveDate) -> Option<String> {
    let text = non_empty(loan.fields.get(DUE).map(String::as_str))?;
    match DueStatus::from_due_text(Some(&text), today) {
        Some(due) => Some(short_date(due.due_date, today)),
        None => (!says_due(&text)).then_some(text),
    }
}

/// Whether the library's own date text already says "due" ("Due back soon").
pub fn says_due(text: &str) -> bool {
    text.to_lowercase().contains("due") // copy-exempt: matches the library's own wording
}

/// "Renewed · Due 3 Nov" from `RenewResult.due` as the library writes it, else "Renewed".
pub fn renewed_text(due: Option<&str>, today: NaiveDate) -> String {
    let Some(due) = non_empty(due) else {
        return Copy::YourBooks::renewedNoDate.to_owned();
    };
    let date = due_date::parse(&due).map_or(due, |date| short_date(date, today));
    Copy::YourBooks::renewed(&date)
}

/// When the library didn't renew: its reason (or the core's), else `renewFailed`.
pub fn refused_text(result: &RenewResult) -> String {
    non_empty(result.message.as_deref()).unwrap_or_else(|| Copy::YourBooks::renewFailed.to_owned())
}

/// The result's text: `renewed_text` when the library renewed the loan, else `refused_text`.
pub fn renew_result_text(result: &RenewResult, today: NaiveDate) -> String {
    if result.ok { renewed_text(result.due.as_deref(), today) } else { refused_text(result) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use librespydus_core::{PatronAction, PatronList};
    use std::collections::HashMap;

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn loan(irn: &str, title: &str, due: Option<&str>, renew: bool) -> PatronItem {
        PatronItem {
            irn: Some(irn.into()),
            title: Some(title.into()),
            fields: due.map(|due| HashMap::from([("Due".to_owned(), due.to_owned())])).unwrap_or_default(),
            actions: if renew {
                vec![PatronAction { label: "Renew loan".into(), href: format!("https://x/renew/{irn}") }]
            } else {
                Vec::new()
            },
            ..Default::default()
        }
    }

    fn reservation(irn: &str, fields: &[(&str, &str)]) -> PatronItem {
        PatronItem {
            irn: Some(irn.into()),
            title: Some(format!("Book {irn}")),
            fields: fields.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect(),
            actions: vec![
                PatronAction { label: "Cancel reservation".into(), href: format!("https://x/cancel/{irn}") },
                PatronAction { label: "Edit".into(), href: format!("https://x/edit/{irn}") },
            ],
            ..Default::default()
        }
    }

    #[test]
    fn loans_sort_by_due_date_with_their_lines() {
        let today = day(2026, 10, 14);
        let rows = LoanRow::rows(
            &[
                loan("1", "Later", Some("27 Oct 2026"), true),
                loan("2", "No date", None, false),
                loan("3", "Overdue", Some("12 Oct 2026"), true),
                loan("4", "Tomorrow", Some("15 Oct 2026"), false),
                loan("5", "Today", Some("14 Oct 2026"), true),
                loan("6", "Weird", Some("weird"), false),
            ],
            today,
        );
        let titles: Vec<_> = rows.iter().map(|row| row.book.title.as_str()).collect();
        assert_eq!(titles, ["Overdue", "Today", "Tomorrow", "Later", "No date", "Weird"]);
        assert_eq!(rows[0].meta().as_deref(), Some("Due 12 Oct · 2 days overdue"));
        assert!(rows[0].is_overdue());
        assert_eq!(rows[1].meta().as_deref(), Some("Due today · 14 Oct"));
        assert_eq!(rows[2].meta().as_deref(), Some("Due tomorrow · 15 Oct"));
        assert_eq!(rows[3].meta().as_deref(), Some("Due 27 Oct · 13 days left"));
        assert_eq!(rows[4].meta(), None);
        assert_eq!(rows[5].meta().as_deref(), Some("Due weird"));
        assert!(rows[0].can_renew() && !rows[2].can_renew());
        assert_eq!(
            confirm_renew_message(&rows[3].item, today),
            "“Later” is due 27 Oct. Renewing asks the library for a later due date."
        );
        assert_eq!(
            confirm_renew_message(&rows[4].item, today),
            "Renewing “No date” asks the library for a later due date."
        );
        assert_eq!(
            confirm_renew_message(&loan("7", "Whistler", Some("Due back soon"), true), today),
            "Renewing “Whistler” asks the library for a later due date."
        );
    }

    #[test]
    fn renewal_results() {
        let today = day(2026, 10, 14);
        assert_eq!(renewed_text(Some("6 Nov 2026"), today), "Renewed · Due 6 Nov");
        assert_eq!(renewed_text(Some("later"), today), "Renewed · Due later");
        assert_eq!(renewed_text(None, today), "Renewed");
        let ok = RenewResult { ok: true, message: None, due: Some("6 Nov 2026".into()) };
        assert_eq!(renew_result_text(&ok, today), "Renewed · Due 6 Nov");
        let refused = RenewResult { ok: false, message: Some("Reserved by another borrower.".into()), due: None };
        assert_eq!(renew_result_text(&refused, today), "Reserved by another borrower.");
        assert_eq!(renew_result_text(&RenewResult::default(), today), "Couldn’t renew this loan");
    }

    #[test]
    fn status_text_reads_the_pickup_branch() {
        assert_eq!(
            StatusText::parse(Some("Requested pickup location: RIVERTON LIBRARY")),
            StatusText { pickup: Some("RIVERTON LIBRARY".into()), ..Default::default() }
        );
        assert_eq!(
            StatusText::parse(Some("Please pick-up from: RIVERTON LIBRARY by 17 Oct 2026")),
            StatusText { status: None, pickup: Some("RIVERTON LIBRARY".into()), pickup_by: Some("17 Oct 2026".into()) }
        );
        assert_eq!(StatusText::parse(Some("In transit")).status.as_deref(), Some("In transit"));
        assert_eq!(display_name(Some("WILLETTON & DISTRICT")).as_deref(), Some("Willetton & District"));
        assert_eq!(display_name(Some("Riverton library")).as_deref(), Some("Riverton library"));
    }

    #[test]
    fn reservations_ready_first_with_their_lines() {
        let today = day(2026, 10, 14);
        let reservations = Reservations {
            ready: PatronList {
                total: Some(1),
                items: vec![reservation("10", &[("Status", "Please pickup from: RIVERTON LIBRARY by 17 Oct 2026")])],
            },
            pending: PatronList {
                total: Some(3),
                items: vec![
                    reservation("10", &[("Status", "Pending")]),
                    reservation(
                        "11",
                        &[
                            ("Status", "Pending"),
                            ("Rank", "4"),
                            ("Placed", "02 Oct 2026"),
                            ("Pickup location", "RIVERTON LIBRARY"),
                        ],
                    ),
                    reservation("12", &[("Status", "In transit"), ("Rank", "n/a")]),
                ],
            },
        };
        let rows = reservation_rows(&reservations, today);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].status_line(), "Ready to collect");
        assert_eq!(rows[0].pickup_line().as_deref(), Some("Collect from Riverton Library by 17 Oct"));
        assert_eq!(
            rows[0].cancel_message(),
            "“Book 10” is waiting for you at Riverton Library. Cancelling releases it to the next person."
        );
        assert_eq!(rows[1].status_line(), "Pending · #4 in queue · Placed 2 Oct");
        assert_eq!(rows[1].pickup_line().as_deref(), Some("Collect from Riverton Library"));
        assert_eq!(
            rows[1].cancel_message(),
            "“Book 11” will be removed from your reservations and you’ll lose your place in the queue."
        );
        assert_eq!(rows[2].status_line(), "In transit · n/a in queue");
        assert_eq!(rows[2].pickup_line(), None);
        assert_eq!(rows[1].cancel_href.as_deref(), Some("https://x/cancel/11"));
        assert_eq!(rows[1].edit_href.as_deref(), Some("https://x/edit/11"));
    }
}
