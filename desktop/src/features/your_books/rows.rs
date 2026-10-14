//! What Your Books' cards show, built from the core's `PatronItem`s (iOS `BorrowedRows.swift`,
//! `LoanRenewal.swift`'s wording, `CancelConfirm.swift`). Pure, so the sorting and wording are
//! unit-tested.

use std::collections::HashSet;

use chrono::NaiveDate;
use librespydus_core::{AccountSnapshot, PatronItem, PatronList, RenewResult, Reservations};

use crate::copy::Copy;
use crate::state::BookRef;
use crate::state::formatting::{DueStatus, due_date, non_empty, short_date};
use crate::state::patron::PatronItemExt;

// copy-exempt: the loans list's field name (a key)
const DUE: &str = "Due";
// copy-exempt: the reservations list's field names (keys)
const STATUS: &str = "Status";
// copy-exempt: a field name (a key)
const RANK: &str = "Rank";
// copy-exempt: a field name (a key)
const PLACED: &str = "Placed";
// copy-exempt: a field name (a key)
const PICKUP_LOCATION: &str = "Pickup location";

/// One loan card: "Due 27 Oct · 3 days left", danger colour when due within 3 days or overdue.
#[derive(Debug, Clone, PartialEq)]
pub struct LoanRow {
    /// The loan's IRN (made unique within its list by `rows`).
    pub id: String,
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
            id: non_empty(item.irn.as_deref()).unwrap_or_else(|| book.id.clone()),
            due: DueStatus::from_due_text(due_text.as_deref(), today),
            due_text,
            renew_href: item.renew_href(),
            item: item.clone(),
            book,
            today,
        }
    }

    /// The due date as `Copy::Loans::dueOn` writes it ("27 Oct"), or as the library wrote it when
    /// the core can't read it (unless that already says "due").
    pub fn due_date_text(&self) -> Option<String> {
        if let Some(due) = &self.due {
            return Some(short_date(due.due_date, self.today));
        }
        self.due_text.clone().filter(|text| !says_due(text))
    }

    /// The meta line; an unreadable date is shown as the library wrote it.
    pub fn meta(&self) -> Option<String> {
        if let Some(due) = &self.due {
            return Some(due_line(due, self.today));
        }
        let text = self.due_text.as_deref()?;
        Some(if says_due(text) { text.to_owned() } else { Copy::Loans::dueOn(text) })
    }

    pub fn is_urgent(&self) -> bool {
        self.due.is_some_and(|due| due.is_urgent())
    }

    pub fn is_overdue(&self) -> bool {
        self.due.is_some_and(|due| due.is_overdue())
    }

    /// Renew when the library gave the loan its link.
    pub fn can_renew(&self) -> bool {
        self.renew_href.is_some()
    }

    /// Loans soonest-due first (overdue at the top); loans without a readable date go last, in the
    /// library's order.
    pub fn rows(items: &[PatronItem], today: NaiveDate) -> Vec<LoanRow> {
        let mut ids = UniqueIds::default();
        let mut rows: Vec<(usize, LoanRow)> = items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let mut row = LoanRow::new(item, today);
                row.id = ids.claim(&row.id);
                (index, row)
            })
            .collect();
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

fn says_due(text: &str) -> bool {
    text.to_lowercase().contains("due") // copy-exempt: matches the library's own wording
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

/// Renewal wording (iOS `RenewText`).
pub mod renew_text {
    use super::*;

    /// "“Whistler” is due 15 Oct. Renewing asks the library for a later due date."
    pub fn confirm_message(row: &LoanRow) -> String {
        match row.due_date_text() {
            Some(date) => Copy::YourBooks::renewMessage(&row.book.title, &date),
            None => Copy::YourBooks::renewMessageNoDate(&row.book.title),
        }
    }

    /// "Renewed · Due 3 Nov" from `RenewResult.due` as the library writes it, else "Renewed".
    pub fn renewed(due: Option<&str>, today: NaiveDate) -> String {
        let Some(due) = non_empty(due) else { return Copy::YourBooks::renewedNoDate.to_owned() };
        let date = due_date::parse(&due).map_or(due, |date| short_date(date, today));
        Copy::YourBooks::renewed(&date)
    }

    /// The library's reason for not renewing, else `renewFailed`.
    pub fn refused(result: &RenewResult) -> String {
        non_empty(result.message.as_deref()).unwrap_or_else(|| Copy::YourBooks::renewFailed.to_owned())
    }
}

/// Ready to collect, or waiting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReservationKind {
    Ready,
    Pending,
}

/// One reservation card.
#[derive(Debug, Clone, PartialEq)]
pub struct ReservationRow {
    /// The reservation's IRN, else its cancel/edit link (unique within `ReservationSections`).
    pub id: String,
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
    today: NaiveDate,
}

impl ReservationRow {
    pub fn new(item: &PatronItem, kind: ReservationKind, today: NaiveDate) -> Self {
        let book = BookRef::from_patron_item(item);
        let field = |name: &str| non_empty(item.fields.get(name).map(String::as_str));
        let status_text = StatusText::parse(field(STATUS).as_deref());
        let cancel_href = item.cancel_href();
        let edit_href = item.edit_href();
        Self {
            id: non_empty(item.irn.as_deref())
                .or_else(|| cancel_href.clone())
                .or_else(|| edit_href.clone())
                .unwrap_or_else(|| book.id.clone()),
            kind,
            status: status_text.status,
            rank: field(RANK),
            placed: field(PLACED),
            pickup: display_name(field(PICKUP_LOCATION).as_deref())
                .or_else(|| display_name(status_text.pickup.as_deref())),
            pickup_by: status_text.pickup_by,
            cancel_href,
            edit_href,
            item: item.clone(),
            book,
            today,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.kind == ReservationKind::Ready
    }

    /// The status chip: "Ready to collect", or the library's own pending status.
    pub fn status_label(&self) -> String {
        if self.is_ready() {
            return Copy::YourBooks::readyToCollect.to_owned();
        }
        self.status.clone().unwrap_or_else(|| Copy::YourBooks::pending.to_owned())
    }

    pub fn is_in_transit(&self) -> bool {
        self.status_label().to_lowercase().contains("transit") // copy-exempt: matches the library's status
    }

    /// Ready: "Collect from Riverton Library by 17 Oct". Pending: "#4 in queue · Placed 2 Oct" and
    /// "Collect from Riverton Library" on a second line.
    pub fn meta(&self) -> Option<String> {
        self.meta_lines(false).map(|lines| lines.join("\n"))
    }

    /// `meta` for a screen reader: lines joined with ", ", the queue place said in full.
    pub fn spoken_meta(&self) -> Option<String> {
        self.meta_lines(true).map(|lines| lines.join(", "))
    }

    fn library_date(&self, text: &str) -> String {
        due_date::parse(text).map_or_else(|| text.to_owned(), |date| short_date(date, self.today))
    }

    fn meta_lines(&self, spoken: bool) -> Option<Vec<String>> {
        if self.is_ready() {
            let by = self.pickup_by.as_deref().map(|date| self.library_date(date));
            return match (&self.pickup, by) {
                (Some(branch), Some(date)) => Some(vec![Copy::YourBooks::collectFromBy(branch, &date)]),
                (Some(branch), None) => Some(vec![Copy::YourBooks::collectFrom(branch)]),
                (None, Some(date)) => Some(vec![Copy::YourBooks::collectBy(&date)]),
                (None, None) => None,
            };
        }
        let mut first = Vec::new();
        if let Some(rank) = &self.rank {
            let numeric = rank.parse::<i64>().is_ok();
            first.push(match (numeric, spoken) {
                (true, false) => Copy::YourBooks::queuePlace(rank),
                (true, true) => Copy::YourBooks::queuePlaceLabel(rank),
                (false, false) => Copy::YourBooks::queuePlaceText(rank),
                (false, true) => Copy::YourBooks::queuePlaceTextLabel(rank),
            });
        }
        if let Some(placed) = &self.placed {
            first.push(Copy::YourBooks::placed(&self.library_date(placed)));
        }
        let lines: Vec<String> = [
            (!first.is_empty()).then(|| first.join(Copy::Common::separator)),
            self.pickup.as_deref().map(Copy::YourBooks::collectFrom),
        ]
        .into_iter()
        .flatten()
        .collect();
        (!lines.is_empty()).then_some(lines)
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

    pub fn has_actions(&self) -> bool {
        self.cancel_href.is_some() || self.edit_href.is_some()
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
        let Some(text) = non_empty(raw) else { return Self::default() };
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

/// Reservations split for display: ready to collect first, then pending.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ReservationSections {
    pub ready: Vec<ReservationRow>,
    pub pending: Vec<ReservationRow>,
}

impl ReservationSections {
    /// A row the library lists on both pages is shown once, as ready.
    pub fn new(reservations: &Reservations, today: NaiveDate) -> Self {
        let mut ids = UniqueIds::default();
        let mut row = |item: &PatronItem, kind| {
            let mut row = ReservationRow::new(item, kind, today);
            row.id = ids.claim(&row.id);
            row
        };
        let ready: Vec<ReservationRow> =
            reservations.ready.items.iter().map(|item| row(item, ReservationKind::Ready)).collect();
        let ready_irns: HashSet<String> =
            reservations.ready.items.iter().filter_map(|item| non_empty(item.irn.as_deref())).collect();
        let pending = reservations
            .pending
            .items
            .iter()
            .filter(|item| non_empty(item.irn.as_deref()).is_none_or(|irn| !ready_irns.contains(&irn)))
            .map(|item| row(item, ReservationKind::Pending))
            .collect();
        Self { ready, pending }
    }

    pub fn count(&self) -> usize {
        self.ready.len() + self.pending.len()
    }

    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    pub fn row(&self, id: &str) -> Option<&ReservationRow> {
        self.ready.iter().chain(&self.pending).find(|row| row.id == id)
    }

    pub fn has_actions(&self) -> bool {
        self.ready.iter().chain(&self.pending).any(ReservationRow::has_actions)
    }
}

/// The cards for one account snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct Lists {
    pub loans: Vec<LoanRow>,
    pub reservations: ReservationSections,
}

impl Lists {
    pub fn new(snapshot: &AccountSnapshot, today: NaiveDate) -> Self {
        Self {
            loans: LoanRow::rows(&snapshot.loans.items, today),
            reservations: ReservationSections::new(&snapshot.reservations, today),
        }
    }
}

/// Without the reservation whose actions include `href` (just cancelled), until the next fetch.
pub fn remove_reservation(snapshot: &mut AccountSnapshot, href: &str) {
    fn without(list: &mut PatronList, href: &str) {
        let before = list.items.len();
        list.items.retain(|item| !item.has_action(href));
        let removed = (before - list.items.len()) as i64;
        if let Some(total) = &mut list.total {
            *total = (*total - removed).max(0);
        }
    }
    without(&mut snapshot.reservations.ready, href);
    without(&mut snapshot.reservations.pending, href);
}

/// Keeps row ids unique within one list: a repeated key gets `#2`, `#3`, ….
#[derive(Default)]
struct UniqueIds {
    used: HashSet<String>,
}

impl UniqueIds {
    fn claim(&mut self, id: &str) -> String {
        let mut unique = id.to_owned();
        let mut suffix = 1;
        while self.used.contains(&unique) {
            suffix += 1;
            unique = format!("{id}#{suffix}");
        }
        self.used.insert(unique.clone());
        unique
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use librespydus_core::PatronAction;
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
                loan("1", "Duplicate id", Some("weird"), false),
            ],
            today,
        );
        let titles: Vec<_> = rows.iter().map(|row| row.book.title.as_str()).collect();
        assert_eq!(titles, ["Overdue", "Today", "Tomorrow", "Later", "No date", "Duplicate id"]);
        assert_eq!(rows[0].meta().as_deref(), Some("Due 12 Oct · 2 days overdue"));
        assert!(rows[0].is_overdue() && rows[0].is_urgent());
        assert_eq!(rows[1].meta().as_deref(), Some("Due today · 14 Oct"));
        assert_eq!(rows[2].meta().as_deref(), Some("Due tomorrow · 15 Oct"));
        assert_eq!(rows[3].meta().as_deref(), Some("Due 27 Oct · 13 days left"));
        assert!(!rows[3].is_urgent());
        assert_eq!(rows[4].meta(), None);
        assert_eq!(rows[5].meta().as_deref(), Some("Due weird"));
        assert_eq!(rows[5].id, "1#2");
        assert!(rows[0].can_renew() && !rows[2].can_renew());
        assert_eq!(
            renew_text::confirm_message(&rows[3]),
            "“Later” is due 27 Oct. Renewing asks the library for a later due date."
        );
        assert_eq!(renew_text::confirm_message(&rows[4]), "Renewing “No date” asks the library for a later due date.");
        assert_eq!(renew_text::renewed(Some("3 Nov 2026"), today), "Renewed · Due 3 Nov");
        assert_eq!(renew_text::renewed(None, today), "Renewed");
        let refused = RenewResult { ok: false, message: Some("Reserved by another borrower.".into()), due: None };
        assert_eq!(renew_text::refused(&refused), "Reserved by another borrower.");
        assert_eq!(renew_text::refused(&RenewResult::default()), "Couldn’t renew this loan");
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
        assert_eq!(StatusText::parse(None), StatusText::default());
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
        let sections = ReservationSections::new(&reservations, today);
        assert_eq!(sections.ready.len(), 1);
        assert_eq!(sections.pending.len(), 2);
        assert_eq!(sections.count(), 3);
        let ready = &sections.ready[0];
        assert_eq!(ready.status_label(), "Ready to collect");
        assert_eq!(ready.meta().as_deref(), Some("Collect from Riverton Library by 17 Oct"));
        assert_eq!(
            ready.cancel_message(),
            "“Book 10” is waiting for you at Riverton Library. Cancelling releases it to the next person."
        );
        let pending = &sections.pending[0];
        assert_eq!(pending.meta().as_deref(), Some("#4 in queue · Placed 2 Oct\nCollect from Riverton Library"));
        assert_eq!(
            pending.spoken_meta().as_deref(),
            Some("number 4 in the queue · Placed 2 Oct, Collect from Riverton Library")
        );
        assert!(sections.pending[1].is_in_transit());
        assert_eq!(sections.pending[1].meta().as_deref(), Some("n/a in queue"));
        assert!(sections.has_actions());

        let mut snapshot = crate::backend::fake::test_data::snapshot(None, &[], &[]);
        snapshot.reservations = reservations;
        remove_reservation(&mut snapshot, "https://x/cancel/11");
        assert_eq!(snapshot.reservations.pending.items.len(), 2);
        assert_eq!(snapshot.reservations.pending.total, Some(2));
    }
}
