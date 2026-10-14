//! A loan's or reservation's action links (iOS `BorrowedRows.swift`): the labels vary by library
//! ("Cancel reservation", "Edit", "Edit reservation", "Renew loan").

use librespydus_core::PatronItem;

use crate::state::formatting::non_empty;

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
