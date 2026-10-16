//! A book the app can open, list or keep in My Picks (docs/native-apps.md §5.3, §5.5; iOS
//! `BookRef.swift`): enough to draw a cover card and to load the record again. Serialized
//! camelCase, the same shape as the phones' saved picks.

use chrono::{DateTime, Utc};
use librespydus_core::app::{is_digital_format, patron_item_record_url};
use librespydus_core::{PatronItem, RecordDetail, RecordFormat, RelatedItem, SearchRecord};
use serde::{Deserialize, Serialize};

use crate::copy::Copy;
use crate::state::formatting::non_empty;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    /// Search records only; a ref opened from a loan or a pick may find them again by ISBN/title.
    #[serde(default)]
    pub formats: Vec<FormatRef>,
    /// Set by the local My Picks store when saved.
    #[serde(default)]
    pub saved_at: Option<DateTime<Utc>>,
}

impl BookRef {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            author: None,
            cover_url: None,
            url: None,
            isbn: None,
            year: None,
            formats: Vec::new(),
            saved_at: None,
        }
    }

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

    /// A loan or reservation. Its own `bib_url` uses a set from the patron's OPAC session and loads
    /// an empty record without it, so the ref keeps the session-free link
    /// (`patron_item_record_url`). The id `bib:<IRN>` remembers which edition the patron has.
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

    /// A "Similar books" / "More by this author" / "In this series" item.
    pub fn from_related_item(item: &RelatedItem) -> Self {
        Self {
            id: key(Some(("brn", item.brn.as_deref())), item.bib_url.as_deref(), item.title.as_deref()),
            title: title(item.title.as_deref()),
            author: non_empty(item.author.as_deref()),
            cover_url: item.cover_urls.first().cloned(),
            url: item.bib_url.clone(),
            isbn: non_empty(item.isbn.as_deref()),
            year: non_empty(item.year.as_deref()),
            formats: Vec::new(),
            saved_at: None,
        }
    }

    /// Just a URL (`--route detail:<url>`, links).
    pub fn from_url(url: &str, title: &str) -> Self {
        let mut book = Self::new(key(None, Some(url), Some(title)), title);
        book.url = Some(url.to_owned());
        book
    }

    /// The bib IRN of a ref made from a loan or reservation (id `bib:<IRN>`): the edition the
    /// patron has, which the book page selects and checks against the account.
    pub fn bib_irn(&self) -> Option<&str> {
        self.id.strip_prefix("bib:").filter(|irn| !irn.trim().is_empty())
    }

    /// What its book page is saved under on this computer (§5.13): the id when it is an IRN or BRN
    /// (`work:`, `bib:`, `brn:`). `None` for a `url:` or `title:` id: a URL carries a search's set
    /// id, so the same book would be saved again under every search.
    pub fn saved_page_key(&self) -> Option<&str> {
        ["work:", "bib:", "brn:"].iter().any(|prefix| self.id.starts_with(prefix)).then_some(self.id.as_str())
    }

    /// This ref updated from its loaded record: the stable bookmark URL, a better cover, ISBN or
    /// title. Keeps `id`, so a pick saved before and after loading stays the same entry.
    pub fn updated_with(&self, detail: &RecordDetail) -> Self {
        let mut copy = self.clone();
        if let Some(bookmark) = non_empty(detail.bookmark_url.as_deref()) {
            copy.url = Some(bookmark);
        }
        if copy.title.is_empty()
            && let Some(title) = non_empty(detail.title.as_deref())
        {
            copy.title = title;
        }
        if copy.cover_url.is_none() {
            copy.cover_url = detail.cover_urls.first().cloned();
        }
        if copy.isbn.is_none() {
            copy.isbn = non_empty(detail.isbn.as_deref());
        }
        copy
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
        None => format!("title:{}", unique_id()),
    }
}

fn unique_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let nanos = Utc::now().timestamp_nanos_opt().unwrap_or_default();
    format!("{nanos:x}-{}", NEXT.fetch_add(1, Ordering::Relaxed))
}

fn title(title: Option<&str>) -> String {
    non_empty(title).unwrap_or_else(|| Copy::Common::untitled.to_owned())
}

/// `RecordFormat` (code, label, reserve and availability URLs) in a serializable form.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

    /// The label, else the code.
    pub fn display_label(&self) -> String {
        non_empty(self.label.as_deref()).unwrap_or_else(|| self.code.clone())
    }

    /// Ebook / eaudio / streaming (the core's `is_digital_format`). Whether it can be reserved is
    /// `reserve_url` alone: some libraries take holds on ebooks.
    pub fn is_digital(&self) -> bool {
        is_digital_format(self.record())
    }

    /// The one format the core gives an edition-level item (a Featured list's card, §5.10): no
    /// code and no label.
    pub fn is_edition_level(&self) -> bool {
        self.code.trim().is_empty() && non_empty(self.label.as_deref()).is_none()
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

/// An edition-level item: exactly one format, without a code. No format chip or picker.
pub fn is_edition_level(formats: &[FormatRef]) -> bool {
    formats.len() == 1 && formats[0].is_edition_level()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(work_irn: Option<&str>, url: Option<&str>, title: Option<&str>) -> SearchRecord {
        SearchRecord {
            work_irn: work_irn.map(Into::into),
            detail_url: url.map(Into::into),
            title: title.map(Into::into),
            author: Some(" Haig, Matt, author ".into()),
            cover_urls: vec!["https://example.test/c.gif".into()],
            formats: vec![RecordFormat { code: "BK".into(), label: Some("Book".into()), ..Default::default() }],
            ..Default::default()
        }
    }

    #[test]
    fn ids_follow_the_ios_keys() {
        let work =
            BookRef::from_search_record(&record(Some("15642035"), Some("https://x/a"), Some("The Midnight Library")));
        assert_eq!(work.id, "work:15642035");
        assert_eq!(work.author.as_deref(), Some("Haig, Matt, author"));
        assert_eq!(work.formats.len(), 1);
        let by_url = BookRef::from_search_record(&record(Some(" "), Some("https://x/a"), Some("T")));
        assert_eq!(by_url.id, "url:https://x/a");
        let by_title = BookRef::from_search_record(&record(None, None, Some("Whistler")));
        assert_eq!(by_title.id, "title:whistler");
        let untitled = BookRef::from_search_record(&record(None, None, None));
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
        assert_eq!(loan_ref.bib_irn(), Some("19244682"));
        assert_eq!(
            loan_ref.url.as_deref(),
            Some("https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&IRN=19244682")
        );

        let related = RelatedItem { brn: Some("467086".into()), title: Some("x".into()), ..Default::default() };
        assert_eq!(BookRef::from_related_item(&related).id, "brn:467086");
    }

    #[test]
    fn saved_page_key_only_for_irns_and_brns() {
        for id in ["work:1", "bib:2", "brn:3"] {
            assert_eq!(BookRef::new(id, "t").saved_page_key(), Some(id));
        }
        assert_eq!(BookRef::new("url:https://x/a", "t").saved_page_key(), None);
        assert_eq!(BookRef::new("title:t", "t").saved_page_key(), None);
        assert_eq!(BookRef::from_url("https://x/y", "").saved_page_key(), None);
    }

    #[test]
    fn round_trips_as_camel_case_json() {
        let mut book = BookRef::from_search_record(&record(Some("1"), Some("https://x/a"), Some("T")));
        book.saved_at = Some(Utc::now());
        let json = serde_json::to_string(&book).unwrap();
        assert!(json.contains("\"coverUrl\"") && json.contains("\"savedAt\"") && json.contains("\"availabilityUrl\""));
        assert_eq!(serde_json::from_str::<BookRef>(&json).unwrap(), book);
    }

    #[test]
    fn updated_with_a_record_keeps_the_id() {
        let book = BookRef::from_url("https://x/search-set", "");
        let detail = RecordDetail {
            bookmark_url: Some("https://x/stable".into()),
            title: Some("Found".into()),
            isbn: Some("123".into()),
            ..Default::default()
        };
        let updated = book.updated_with(&detail);
        assert_eq!(updated.id, book.id);
        assert_eq!(updated.url.as_deref(), Some("https://x/stable"));
        assert_eq!(updated.title, "Found");
        assert_eq!(updated.isbn.as_deref(), Some("123"));
    }
}
