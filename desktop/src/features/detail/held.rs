//! What the book page shows instead of Reserve when the patron already has the edition on screen
//! (docs/native-apps.md §6 "Already yours"; iOS `HeldStatus.swift`): "On loan · Due 14 Oct", "Ready
//! to collect · Riverton Library", "Reserved · Riverton Library". Clicking it asks to renew a loan
//! the library lets the patron renew (§5.12), and otherwise opens Your Books on `segment`. Also the
//! renewal's words (iOS `LoanRenewal.swift`).

use chrono::NaiveDate;
use librespydus_core::app::{AccountHolding, HoldingKind};
use librespydus_core::{PatronItem, RenewResult};

use crate::copy::Copy;
use crate::state::YourBooksSegment;
use crate::state::formatting::{DueStatus, due_date, non_empty, short_date};
use crate::state::patron::PatronItemExt;
use crate::ui::Lucide;

use super::logic::title_cased;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeldTone {
    Loan,
    Overdue,
    Ready,
    Reserved,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HeldStatus {
    /// One line: "On loan · Due 14 Oct".
    pub text: String,
    /// For screen readers: "On loan, due 14 October".
    pub label: String,
    pub tone: HeldTone,
    pub icon: Lucide,
    pub segment: YourBooksSegment,
    /// The loan, when it has a "Renew loan" link: clicking the pill asks to renew it.
    pub renewable_loan: Option<PatronItem>,
}

impl HeldStatus {
    pub fn new(holding: &AccountHolding, today: NaiveDate) -> Self {
        let item = &holding.item;
        match holding.kind {
            HoldingKind::Loan => {
                let mut status = Self::loan(item, today);
                if item.renew_href().is_some() {
                    status.renewable_loan = Some(item.clone());
                }
                status
            }
            HoldingKind::ReservationReady => {
                let pickup = pickup(item);
                Self {
                    text: pickup
                        .as_deref()
                        .map_or_else(|| Copy::Detail::readyToCollect.to_owned(), Copy::Detail::readyToCollectAt),
                    label: pickup
                        .as_deref()
                        .map_or_else(|| Copy::Detail::readyToCollect.to_owned(), Copy::Detail::readyToCollectAtLabel),
                    tone: HeldTone::Ready,
                    icon: Lucide::CircleCheck,
                    segment: YourBooksSegment::Reservations,
                    renewable_loan: None,
                }
            }
            HoldingKind::ReservationPending => {
                // copy-exempt: the reservations list's field name (a key)
                let rank = non_empty(item.fields.get("Rank").map(String::as_str));
                let (text, label) = if let Some(branch) = pickup(item) {
                    (Copy::Detail::reservedAt(&branch), Copy::Detail::reservedAtLabel(&branch))
                } else if let Some(rank) = rank {
                    if rank.parse::<i64>().is_ok() {
                        (Copy::Detail::reservedQueue(&rank), Copy::Detail::reservedQueueLabel(&rank))
                    } else {
                        (Copy::Detail::reservedQueueText(&rank), Copy::Detail::reservedQueueTextLabel(&rank))
                    }
                } else {
                    (Copy::Detail::reserved.to_owned(), Copy::Detail::reserved.to_owned())
                };
                Self {
                    text,
                    label,
                    tone: HeldTone::Reserved,
                    icon: Lucide::Calendar,
                    segment: YourBooksSegment::Reservations,
                    renewable_loan: None,
                }
            }
        }
    }

    /// What clicking the pill does, for screen readers.
    pub fn hint(&self) -> &'static str {
        if self.renewable_loan.is_some() { Copy::YourBooks::renewHint } else { Copy::Detail::heldHint }
    }

    /// "On loan · Due 14 Oct", "On loan · Overdue"; a due date the core can't read is shown as the
    /// library wrote it.
    fn loan(item: &PatronItem, today: NaiveDate) -> Self {
        // copy-exempt: the loans list's field name (a key)
        let due_text = non_empty(item.fields.get("Due").map(String::as_str));
        let base = |text: String, label: String, tone: HeldTone, icon: Lucide| Self {
            text,
            label,
            tone,
            icon,
            segment: YourBooksSegment::Loans,
            renewable_loan: None,
        };
        match DueStatus::from_due_text(due_text.as_deref(), today) {
            Some(due) => {
                let spoken = due.due_date.format("%-d %B").to_string();
                if due.is_overdue() {
                    base(
                        Copy::Detail::onLoanOverdue.to_owned(),
                        Copy::Detail::onLoanOverdueLabel(&spoken),
                        HeldTone::Overdue,
                        Lucide::CircleAlert,
                    )
                } else {
                    base(
                        Copy::Detail::onLoanDue(&short_date(due.due_date, today)),
                        Copy::Detail::onLoanDueLabel(&spoken),
                        HeldTone::Loan,
                        Lucide::BookOpen,
                    )
                }
            }
            None => match due_text {
                Some(text) => base(
                    Copy::Detail::onLoanDue(&text),
                    Copy::Detail::onLoanDueLabel(&text),
                    HeldTone::Loan,
                    Lucide::BookOpen,
                ),
                None => base(
                    Copy::Detail::onLoan.to_owned(),
                    Copy::Detail::onLoan.to_owned(),
                    HeldTone::Loan,
                    Lucide::BookOpen,
                ),
            },
        }
    }
}

/// A reservation's branch: `fields["Pickup location"]`, else the branch Canning writes into its
/// Status ("Requested pickup location: RIVERTON LIBRARY", "Please pickup from: RIVERTON LIBRARY by
/// 17 Oct 2026"); title-cased when the library shouts it.
pub fn pickup(item: &PatronItem) -> Option<String> {
    // copy-exempt: the reservations list's field names (keys)
    let field = non_empty(item.fields.get("Pickup location").map(String::as_str));
    let status = item.fields.get("Status").and_then(|status| status_pickup(status));
    field.or(status).map(|branch| title_cased(&branch))
}

/// The branch named in a Status like Canning's, or `None` for an ordinary status.
fn status_pickup(status: &str) -> Option<String> {
    let status = status.trim();
    let colon = status.find(':')?;
    let head: String =
        status[..colon].chars().filter(|c| !c.is_whitespace() && *c != '-').collect::<String>().to_lowercase();
    let rest = status[colon + 1..].trim();
    match head.as_str() {
        "requestedpickuplocation" => non_empty(Some(rest)),
        "pleasepickupfrom" => {
            let lower = rest.to_ascii_lowercase();
            let branch = lower.find(" by ").map_or(rest, |at| &rest[..at]);
            non_empty(Some(branch))
        }
        _ => None,
    }
}

// ---- Renewing (§5.12) ----

/// The confirmation's message: "“Whistler” is due 15 Oct. Renewing asks the library for a later due
/// date.", or without the date when the loan has none.
pub fn renew_message(item: &PatronItem, today: NaiveDate) -> String {
    let title = non_empty(item.title.as_deref()).unwrap_or_else(|| Copy::Common::untitled.to_owned());
    // copy-exempt: the loans list's field name (a key)
    let due = non_empty(item.fields.get("Due").map(String::as_str));
    match due {
        Some(text) => {
            let date = due_date::parse(&text).map_or(text, |date| short_date(date, today));
            Copy::YourBooks::renewMessage(&title, &date)
        }
        None => Copy::YourBooks::renewMessageNoDate(&title),
    }
}

/// The result toast: "Renewed · Due 3 Nov" from `RenewResult.due`, else "Renewed"; when the library
/// didn't renew, its reason (or the core's), else `renewFailed`.
pub fn renew_result_text(result: &RenewResult, today: NaiveDate) -> String {
    if !result.ok {
        return non_empty(result.message.as_deref()).unwrap_or_else(|| Copy::YourBooks::renewFailed.to_owned());
    }
    match non_empty(result.due.as_deref()) {
        Some(due) => {
            let date = due_date::parse(&due).map_or(due, |date| short_date(date, today));
            Copy::YourBooks::renewed(&date)
        }
        None => Copy::YourBooks::renewedNoDate.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use librespydus_core::PatronAction;

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn item(fields: &[(&str, &str)], renewable: bool) -> PatronItem {
        PatronItem {
            title: Some("Whistler".into()),
            fields: fields.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect::<HashMap<_, _>>(),
            actions: if renewable {
                vec![PatronAction { label: "Renew loan".into(), href: "https://x/renew".into() }]
            } else {
                Vec::new()
            },
            ..Default::default()
        }
    }

    #[test]
    fn loan_pills() {
        let today = day(2026, 10, 14);
        let loan = HeldStatus::new(
            &AccountHolding { kind: HoldingKind::Loan, item: item(&[("Due", "16 Oct 2026")], true) },
            today,
        );
        assert_eq!(loan.text, "On loan · Due 16 Oct");
        assert_eq!(loan.label, "On loan, due 16 October");
        assert!(loan.renewable_loan.is_some());
        assert_eq!(loan.hint(), "Asks the library for a later due date");
        let overdue = HeldStatus::new(
            &AccountHolding { kind: HoldingKind::Loan, item: item(&[("Due", "12 Oct 2026")], false) },
            today,
        );
        assert_eq!((overdue.text.as_str(), overdue.tone), ("On loan · Overdue", HeldTone::Overdue));
        assert_eq!(overdue.segment, YourBooksSegment::Loans);
        assert_eq!(overdue.hint(), "Opens Your Books");
        let odd =
            HeldStatus::new(&AccountHolding { kind: HoldingKind::Loan, item: item(&[("Due", "soon")], false) }, today);
        assert_eq!(odd.text, "On loan · Due soon");
        let none = HeldStatus::new(&AccountHolding { kind: HoldingKind::Loan, item: item(&[], false) }, today);
        assert_eq!(none.text, "On loan");
    }

    #[test]
    fn reservation_pills() {
        let today = day(2026, 10, 14);
        let ready = HeldStatus::new(
            &AccountHolding {
                kind: HoldingKind::ReservationReady,
                item: item(&[("Status", "Please pickup from: RIVERTON LIBRARY by 17 Oct 2026")], false),
            },
            today,
        );
        assert_eq!(ready.text, "Ready to collect · Riverton Library");
        assert_eq!(ready.label, "Ready to collect at Riverton Library");
        assert_eq!(ready.segment, YourBooksSegment::Reservations);
        let pending = |fields: &[(&str, &str)]| {
            HeldStatus::new(&AccountHolding { kind: HoldingKind::ReservationPending, item: item(fields, false) }, today)
        };
        assert_eq!(
            pending(&[("Pickup location", "CANNINGTON LIBRARY"), ("Rank", "1")]).text,
            "Reserved · Cannington Library"
        );
        assert_eq!(
            pending(&[("Status", "Requested pickup location: RIVERTON LIBRARY")]).text,
            "Reserved · Riverton Library"
        );
        assert_eq!(pending(&[("Rank", "4")]).text, "Reserved · #4 in queue");
        assert_eq!(pending(&[("Rank", "1 of 3")]).text, "Reserved · 1 of 3 in queue");
        assert_eq!(pending(&[("Status", "In transit")]).text, "Reserved");
    }

    #[test]
    fn renewal_words() {
        let today = day(2026, 10, 14);
        assert_eq!(
            renew_message(&item(&[("Due", "16 Oct 2026")], true), today),
            "“Whistler” is due 16 Oct. Renewing asks the library for a later due date."
        );
        assert_eq!(
            renew_message(&item(&[], true), today),
            "Renewing “Whistler” asks the library for a later due date."
        );
        let renewed = RenewResult { ok: true, message: None, due: Some("6 Nov 2026".into()) };
        assert_eq!(renew_result_text(&renewed, today), "Renewed · Due 6 Nov");
        assert_eq!(renew_result_text(&RenewResult { ok: true, message: None, due: None }, today), "Renewed");
        let refused = RenewResult { ok: false, message: Some("Reserved by another borrower.".into()), due: None };
        assert_eq!(renew_result_text(&refused, today), "Reserved by another borrower.");
        assert_eq!(renew_result_text(&RenewResult::default(), today), "Couldn’t renew this loan");
    }
}
