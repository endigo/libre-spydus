//! A book the CLI can list or keep in My Picks on this computer: the desktop app's
//! `state/book_ref.rs` (iOS `BookRef.swift`), reduced to what a saved pick needs. Serialized
//! camelCase, the same shape as the phones' and the desktop's saved picks.

use chrono::{DateTime, Utc};
use librespydus_core::app::patron_item_record_url;
use librespydus_core::{PatronItem, RecordDetail, RecordFormat, SearchRecord};
use serde::{Deserialize, Serialize};

use crate::copy::Copy;
use crate::formatting::non_empty;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookRef {
    /// Stable key: `bib:<IRN>`, `brn:<BRN>`, `work:<IRN>`, else `url:<url>` / `title:<title>`.
    pub id: String,
    pub title: String,
    /// As the library gives it ("Haig, Matt, author"); show with `formatting::author_name`.
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub cover_url: Option<String>,
    /// Loads with `LibraryBackend::record`. `RecordDetail.bookmark_url` (stable) is preferred
    /// over search URLs, whose set id can expire.
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub isbn: Option<String>,
    #[serde(default)]
    pub year: Option<String>,
    /// Search records only.
    #[serde(default)]
    pub formats: Vec<FormatRef>,
    /// Set by the local My Picks store when saved.
    #[serde(default)]
    pub saved_at: Option<DateTime<Utc>>,
}

impl BookRef {
    /// A search result row (`SearchRecord.detail_url` loads with `record`).
    pub fn from_search_record(record: &SearchRecord) -> Self {
        Self {
            id: key(Some(("work", record.work_irn.as_deref())), record.detail_url.as_deref(), record.title.as_deref()),
            title: title(record.title.as_deref()),
            author: non_empty(record.author.as_deref()),
            cover_url: record.cover_urls.first().cloned(),
            url: record.detail_url.clone(),
            isbn: non_empty(record.isbn.as_deref()),
            year: non_empty(record.year.as_deref()),
            formats: record.formats.iter().map(FormatRef::from).collect(),
            saved_at: None,
        }
    }

    /// A loan or reservation, keyed `bib:<IRN>` with the session-free record link.
    pub fn from_patron_item(item: &PatronItem) -> Self {
        Self {
            id: key(Some(("bib", item.bib_irn.as_deref())), item.bib_url.as_deref(), item.title.as_deref()),
            title: title(item.title.as_deref()),
            author: non_empty(item.author.as_deref()),
            cover_url: item.cover_urls.first().cloned(),
            url: patron_item_record_url(item.clone()).or_else(|| item.bib_url.clone()),
            isbn: non_empty(item.isbn.as_deref()),
            year: non_empty(item.year.as_deref()),
            formats: Vec::new(),
            saved_at: None,
        }
    }

    /// A loaded record (`spydus picks add <url>` while signed out).
    pub fn from_record(url: &str, detail: &RecordDetail) -> Self {
        Self {
            id: key(Some(("bib", detail.bib_irn.as_deref())), Some(url), detail.title.as_deref()),
            title: title(detail.title.as_deref()),
            author: detail
                .details
                .iter()
                .find(|row| ["author", "creator"].contains(&row.label.to_lowercase().as_str()))
                .and_then(|row| row.values.first())
                .map(|value| value.text.clone()),
            cover_url: detail.cover_urls.first().cloned(),
            url: non_empty(detail.bookmark_url.as_deref()).or_else(|| Some(url.to_owned())),
            isbn: non_empty(detail.isbn.as_deref()),
            year: None,
            formats: Vec::new(),
            saved_at: None,
        }
    }

    /// This pick as a results card, so lists print the same way signed in and out.
    pub fn to_search_record(&self) -> SearchRecord {
        SearchRecord {
            rank: None,
            work_irn: self.id.strip_prefix("work:").map(str::to_owned),
            title: Some(self.title.clone()),
            detail_url: self.url.clone(),
            author: self.author.clone(),
            author_role: None,
            author_url: None,
            year: self.year.clone(),
            language: None,
            summary: None,
            isbn: self.isbn.clone(),
            cover_urls: self.cover_url.iter().cloned().collect(),
            formats: self.formats.iter().map(FormatRef::record).collect(),
        }
    }
}

/// `prefix:<value>` when the IRN/BRN is known, else `url:<url>`, else `title:<title>`.
fn key(irn: Option<(&str, Option<&str>)>, url: Option<&str>, title: Option<&str>) -> String {
    if let Some((prefix, Some(value))) = irn
        && let Some(value) = non_empty(Some(value))
    {
        return format!("{prefix}:{value}");
    }
    if let Some(url) = non_empty(url) {
        return format!("url:{url}");
    }
    match non_empty(title) {
        Some(title) => format!("title:{}", title.to_lowercase()),
        None => format!("title:{}", Utc::now().timestamp_nanos_opt().unwrap_or_default()),
    }
}

fn title(title: Option<&str>) -> String {
    non_empty(title).unwrap_or_else(|| Copy::Common::untitled.to_owned())
}

/// `RecordFormat` (code, label, reserve and availability URLs) in a serializable form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormatRef {
    pub code: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub reserve_url: Option<String>,
    #[serde(default)]
    pub availability_url: Option<String>,
}

impl FormatRef {
    pub fn record(&self) -> RecordFormat {
        RecordFormat {
            code: self.code.clone(),
            label: self.label.clone(),
            reserve_url: self.reserve_url.clone(),
            availability_url: self.availability_url.clone(),
        }
    }
}

impl From<&RecordFormat> for FormatRef {
    fn from(format: &RecordFormat) -> Self {
        Self {
            code: format.code.clone(),
            label: format.label.clone(),
            reserve_url: format.reserve_url.clone(),
            availability_url: format.availability_url.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_follow_the_apps_keys() {
        let record = SearchRecord {
            work_irn: Some("15642035".into()),
            detail_url: Some("https://x/a".into()),
            title: Some("The Midnight Library".into()),
            author: Some(" Haig, Matt, author ".into()),
            formats: vec![RecordFormat { code: "BK".into(), ..Default::default() }],
            ..Default::default()
        };
        let work = BookRef::from_search_record(&record);
        assert_eq!(work.id, "work:15642035");
        assert_eq!(work.author.as_deref(), Some("Haig, Matt, author"));
        assert_eq!(work.to_search_record().formats.len(), 1);
        let untitled = BookRef::from_search_record(&SearchRecord::default());
        assert!(untitled.id.starts_with("title:"));
        assert_eq!(untitled.title, "Untitled");
        let loan = PatronItem {
            bib_irn: Some("19244682".into()),
            bib_url: Some("https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/78500000?SETLVL=&QRY=IRN(19244682)&NAVLVL=SET".into()),
            title: Some("The long night".into()),
            ..Default::default()
        };
        let loan_ref = BookRef::from_patron_item(&loan);
        assert_eq!(loan_ref.id, "bib:19244682");
        assert_eq!(
            loan_ref.url.as_deref(),
            Some("https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&IRN=19244682")
        );
        let json = serde_json::to_string(&work).unwrap();
        assert!(json.contains("\"coverUrl\"") && json.contains("\"availabilityUrl\""));
        assert_eq!(serde_json::from_str::<BookRef>(&json).unwrap(), work);
    }
}
