//! Home's "Your loans" shelf (iOS `LoanShelfItem`): a current loan's cover with "3 days left" /
//! "Due 27 Oct" / "2 days overdue", in due order.

use std::collections::HashSet;

use chrono::NaiveDate;
use librespydus_core::PatronItem;

use crate::copy::Copy;
use crate::state::BookRef;
use crate::state::formatting::{DueStatus, non_empty};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoanTone {
    Normal,
    /// Due within `SOON_DAYS` days.
    Soon,
    Overdue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoanShelfItem {
    pub book: BookRef,
    pub caption: Option<String>,
    pub tone: LoanTone,
    pub due_date: Option<NaiveDate>,
}

/// Within this many days the caption counts down ("3 days left"); later it shows the date.
pub const COUNTDOWN_DAYS: i64 = 14;
/// Due within this many days (or today): `Soon`.
pub const SOON_DAYS: i64 = 3;

impl LoanShelfItem {
    pub fn new(item: &PatronItem, today: NaiveDate) -> Self {
        let book = BookRef::from_patron_item(item);
        // copy-exempt: the loans list's field name (a key)
        let due_text = non_empty(item.fields.get("Due").map(String::as_str));
        let Some(status) = DueStatus::from_due_text(due_text.as_deref(), today) else {
            return Self {
                book,
                caption: due_text.map(|text| Copy::Loans::dueOn(&text)),
                tone: LoanTone::Normal,
                due_date: None,
            };
        };
        let (tone, caption) = if status.is_overdue() {
            (LoanTone::Overdue, status.days_text())
        } else {
            let tone = if status.days_left <= SOON_DAYS { LoanTone::Soon } else { LoanTone::Normal };
            (tone, if status.days_left <= COUNTDOWN_DAYS { status.days_text() } else { status.due_text() })
        };
        Self { book, caption: Some(caption), tone, due_date: Some(status.due_date) }
    }

    /// Loans in due order (overdue first); loans without a readable date last, in list order.
    pub fn make(items: &[PatronItem], today: NaiveDate) -> Vec<Self> {
        let mut seen = HashSet::new();
        let mut loans: Vec<Self> =
            items.iter().map(|item| Self::new(item, today)).filter(|loan| seen.insert(loan.book.id.clone())).collect();
        // A stable sort keeps list order among equal dates and among the undated.
        loans.sort_by_key(|loan| (loan.due_date.is_none(), loan.due_date));
        loans
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loan(irn: &str, due: Option<&str>) -> PatronItem {
        let mut item =
            PatronItem { bib_irn: Some(irn.into()), title: Some(format!("Book {irn}")), ..Default::default() };
        if let Some(due) = due {
            item.fields.insert("Due".into(), due.into());
        }
        item
    }

    #[test]
    fn captions_count_down_then_show_the_date() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 14).unwrap();
        let soon = LoanShelfItem::new(&loan("1", Some("16 Oct 2026")), today);
        assert_eq!(soon.caption.as_deref(), Some("2 days left"));
        assert_eq!(soon.tone, LoanTone::Soon);
        let later = LoanShelfItem::new(&loan("2", Some("20 Oct 2026")), today);
        assert_eq!(later.caption.as_deref(), Some("6 days left"));
        assert_eq!(later.tone, LoanTone::Normal);
        let far = LoanShelfItem::new(&loan("3", Some("27 Nov 2026")), today);
        assert_eq!(far.caption.as_deref(), Some("Due 27 Nov"));
        let overdue = LoanShelfItem::new(&loan("4", Some("12 Oct 2026")), today);
        assert_eq!(overdue.caption.as_deref(), Some("2 days overdue"));
        assert_eq!(overdue.tone, LoanTone::Overdue);
        let unreadable = LoanShelfItem::new(&loan("5", Some("soon")), today);
        assert_eq!(unreadable.caption.as_deref(), Some("Due soon"));
        assert_eq!(LoanShelfItem::new(&loan("6", None), today).caption, None);
    }

    #[test]
    fn loans_come_in_due_order_with_the_undated_last() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 14).unwrap();
        let items = [
            loan("a", None),
            loan("b", Some("30 Oct 2026")),
            loan("c", Some("10 Oct 2026")),
            loan("d", Some("bad")),
            loan("b", Some("30 Oct 2026")),
        ];
        let order: Vec<String> = LoanShelfItem::make(&items, today).into_iter().map(|loan| loan.book.title).collect();
        assert_eq!(order, ["Book c", "Book b", "Book a", "Book d"]);
    }
}
