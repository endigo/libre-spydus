//! A renewal's words (docs/native-apps.md §5.12; iOS `RenewText` in `LoanRenewal.swift`), shared
//! by Your Books and the book page's "On loan" pill: the confirmation's message and the result
//! toast. Every piece comes from `Copy::YourBooks`, dates as `Copy::Loans::dueOn` writes them.

use chrono::NaiveDate;
use librespydus_core::{PatronItem, RenewResult};

use crate::copy::Copy;
use crate::state::BookRef;
use crate::state::formatting::{DueStatus, due_date, non_empty, short_date};

// copy-exempt: the loans list's field name (a key)
const DUE: &str = "Due";

/// The confirmation's message: "“Whistler” is due 15 Oct. Renewing asks the library for a later
/// due date.", or without the date when the loan has none.
pub fn confirm_message(loan: &PatronItem, today: NaiveDate) -> String {
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

/// The success toast: "Renewed · Due 3 Nov" from `RenewResult.due` as the library writes it
/// ("3 Nov 2026"; kept as written when it can't be read), else "Renewed".
pub fn renewed(due: Option<&str>, today: NaiveDate) -> String {
    let Some(due) = non_empty(due) else { return Copy::YourBooks::renewedNoDate.to_owned() };
    let date = due_date::parse(&due).map_or(due, |date| short_date(date, today));
    Copy::YourBooks::renewed(&date)
}

/// The error toast when the library didn't renew: its reason (or the core's), else `renewFailed`.
pub fn refused(result: &RenewResult) -> String {
    non_empty(result.message.as_deref()).unwrap_or_else(|| Copy::YourBooks::renewFailed.to_owned())
}

/// The result toast's text: `renewed` when the library renewed the loan, else `refused`.
pub fn result_text(result: &RenewResult, today: NaiveDate) -> String {
    if result.ok { renewed(result.due.as_deref(), today) } else { refused(result) }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn loan(title: Option<&str>, due: Option<&str>) -> PatronItem {
        PatronItem {
            title: title.map(str::to_owned),
            fields: due.map(|due| ("Due".to_owned(), due.to_owned())).into_iter().collect::<HashMap<_, _>>(),
            ..Default::default()
        }
    }

    #[test]
    fn confirmation() {
        let today = day(2026, 10, 14);
        assert_eq!(
            confirm_message(&loan(Some("Whistler"), Some("16 Oct 2026")), today),
            "“Whistler” is due 16 Oct. Renewing asks the library for a later due date."
        );
        assert_eq!(
            confirm_message(&loan(Some("Whistler"), Some("3 Jan 2027")), today),
            "“Whistler” is due 3 Jan 2027. Renewing asks the library for a later due date."
        );
        assert_eq!(
            confirm_message(&loan(Some("Whistler"), None), today),
            "Renewing “Whistler” asks the library for a later due date."
        );
        // An unreadable date is kept as written, unless it already says "due".
        assert_eq!(
            confirm_message(&loan(Some("Whistler"), Some("soon")), today),
            "“Whistler” is due soon. Renewing asks the library for a later due date."
        );
        assert_eq!(
            confirm_message(&loan(Some("Whistler"), Some("Due back soon")), today),
            "Renewing “Whistler” asks the library for a later due date."
        );
        assert_eq!(
            confirm_message(&loan(None, Some("  ")), today),
            "Renewing “Untitled” asks the library for a later due date."
        );
    }

    #[test]
    fn results() {
        let today = day(2026, 10, 14);
        assert_eq!(renewed(Some("6 Nov 2026"), today), "Renewed · Due 6 Nov");
        assert_eq!(renewed(Some("later"), today), "Renewed · Due later");
        assert_eq!(renewed(None, today), "Renewed");
        let renewed_result = RenewResult { ok: true, message: None, due: Some("6 Nov 2026".into()) };
        assert_eq!(result_text(&renewed_result, today), "Renewed · Due 6 Nov");
        assert_eq!(result_text(&RenewResult { ok: true, message: None, due: None }, today), "Renewed");
        let refused_result =
            RenewResult { ok: false, message: Some("Reserved by another borrower.".into()), due: None };
        assert_eq!(refused(&refused_result), "Reserved by another borrower.");
        assert_eq!(result_text(&refused_result, today), "Reserved by another borrower.");
        assert_eq!(result_text(&RenewResult::default(), today), "Couldn’t renew this loan");
    }
}
