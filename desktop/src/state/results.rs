//! Result rows as the lists show them (iOS `SearchResultItem` / `SearchResultsModel.merge`): shared
//! by Search, results lists and My Picks.

use librespydus_core::SearchRecord;

use crate::state::book_ref::{BookRef, FormatRef, is_edition_level};
use crate::state::error::UserFacingError;
use crate::state::formatting::non_empty;

#[derive(Debug, Clone, PartialEq)]
pub struct SearchResultItem {
    /// Unique within a list: the dedupe key, else the row's position.
    pub id: String,
    /// `dedupe_key`.
    pub key: Option<String>,
    pub book: BookRef,
    /// Format chips, one per label; none for an edition-level item.
    pub formats: Vec<FormatRef>,
}

impl SearchResultItem {
    pub fn new(record: &SearchRecord, key: Option<String>, position: usize) -> Self {
        let book = BookRef::from_search_record(record);
        let mut labels = std::collections::HashSet::new();
        let formats = if is_edition_level(&book.formats) {
            Vec::new()
        } else {
            book.formats.iter().filter(|format| labels.insert(format.display_label().to_lowercase())).cloned().collect()
        };
        Self { id: key.clone().unwrap_or_else(|| format!("row:{position}")), key, book, formats }
    }
}

/// `existing` plus `incoming`'s records not already listed (the same work on two pages).
pub fn merge(existing: &[SearchResultItem], incoming: &[SearchRecord]) -> Vec<SearchResultItem> {
    let mut merged = existing.to_vec();
    let mut seen: std::collections::HashSet<String> = existing.iter().filter_map(|item| item.key.clone()).collect();
    for record in incoming {
        let key = dedupe_key(record);
        if let Some(key) = &key
            && !seen.insert(key.clone())
        {
            continue;
        }
        let position = merged.len();
        merged.push(SearchResultItem::new(record, key, position));
    }
    merged
}

/// The same work on two pages: `work_irn`, else `detail_url` (`None`: never a duplicate).
pub fn dedupe_key(record: &SearchRecord) -> Option<String> {
    if let Some(irn) = non_empty(record.work_irn.as_deref()) {
        return Some(format!("work:{irn}"));
    }
    non_empty(record.detail_url.as_deref()).map(|url| format!("url:{url}"))
}

/// A page with no records that isn't the OPAC's "no records" page.
pub fn unrecognised_page() -> UserFacingError {
    UserFacingError::unexpected(Some(
        // copy-exempt: technical detail, shown only under the error's Details disclosure
        "The results page wasn't recognised (no records and no \"did not find any records\" message).".into(),
    ))
}
