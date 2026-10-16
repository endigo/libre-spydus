//! The public catalogue data of every demo loan and reservation
//! (`ios/LibreSpydus/Resources/DemoData/demo-catalogue.json`), so their book pages work offline in
//! demo mode. Ported unchanged from the desktop app's `backend/demo_catalogue.rs` (iOS
//! `DemoCatalogue.swift`). `DemoBackend` answers from it the way the OPAC does:
//! - `search` for a query that names a work (every word of its main title) → that work and its
//!   formats;
//! - `availability(<a format's availability URL>)` → the counts and the edition's `bib_record_irn`;
//! - `record` of the session-free `…/BIBENQ?SETLVL=&IRN=<irn>`, the edition's bookmark link or a
//!   format's availability URL → the edition's full record;
//! - `record` of a loan's or reservation's own `…/BIBENQ/<set>?…QRY=IRN(<irn>)` link → an empty
//!   record: that set lives in the patron's OPAC session.

use std::collections::{HashMap, HashSet};

use librespydus_core::{Availability, RecordDetail, RecordFormat, SearchRecord, SearchResults};
use serde::Deserialize;

/// The search set every generated URL uses (the demo fixtures' own).
pub const SET_ID: &str = "78500000";

#[derive(Debug, Clone)]
pub struct Edition {
    pub format: RecordFormat,
    pub bib_irn: String,
    pub availability: Availability,
    pub record: RecordDetail,
}

#[derive(Debug, Clone)]
pub struct Work {
    /// As a search result, with one format per edition.
    pub record: SearchRecord,
    pub editions: Vec<Edition>,
    /// `normalized_title` words: a query holding all of them finds this work.
    pub title_words: HashSet<String>,
}

#[derive(Debug, Clone, Default)]
pub struct DemoCatalogue {
    pub works: Vec<Work>,
    records_by_url: HashMap<String, RecordDetail>,
    editions_by_irn: HashMap<String, Edition>,
    editions_by_availability_url: HashMap<String, Edition>,
}

impl DemoCatalogue {
    pub fn new(works: Vec<Work>) -> Self {
        let mut catalogue = Self { works, ..Default::default() };
        for edition in catalogue.works.iter().flat_map(|work| work.editions.iter()) {
            catalogue.editions_by_irn.insert(edition.bib_irn.clone(), edition.clone());
            if let Some(url) = &edition.format.availability_url {
                catalogue.editions_by_availability_url.insert(url.clone(), edition.clone());
                catalogue.records_by_url.insert(url.clone(), edition.record.clone());
            }
            if let Some(bookmark) = &edition.record.bookmark_url {
                catalogue.records_by_url.insert(bookmark.clone(), edition.record.clone());
            }
        }
        catalogue
    }

    /// From the bundled JSON (already moved to `origin`).
    pub fn from_json(json: &str, origin: &str) -> serde_json::Result<Self> {
        let dto: CatalogueDto = serde_json::from_str(json)?;
        let works = dto
            .works
            .into_iter()
            .map(|work| {
                let editions: Vec<Edition> = work
                    .formats
                    .into_iter()
                    .map(|format| Edition {
                        format: RecordFormat {
                            code: format.code.clone(),
                            label: format.label.clone(),
                            reserve_url: Some(reserve_url(origin, &work.work_irn, &format.code)),
                            availability_url: Some(availability_url(origin, &work.work_irn, &format.code)),
                        },
                        bib_irn: format.bib_irn.clone(),
                        availability: Availability {
                            bib_record_irn: Some(format.bib_irn.clone()),
                            title: format.record.title.clone(),
                            publisher: format.publisher.clone(),
                            reserved: format.counts.get("reserved").copied(),
                            copies: format.counts.get("copies").copied(),
                            counts: format.counts,
                            holdings_url: None,
                            record_url: None,
                        },
                        record: format.record,
                    })
                    .collect();
                let record = SearchRecord {
                    rank: Some(1),
                    work_irn: Some(work.work_irn.clone()),
                    title: Some(work.title.clone()),
                    detail_url: Some(format!(
                        "{origin}/cgi-bin/spydus.exe/FULL/OPAC/ALLWRKENQ/{SET_ID}/{},1",
                        work.work_irn
                    )),
                    author: work.author,
                    author_role: work.author_role,
                    author_url: None,
                    year: work.year,
                    language: work.language,
                    summary: None,
                    isbn: work.isbn,
                    cover_urls: work.cover_urls,
                    formats: editions.iter().map(|edition| edition.format.clone()).collect(),
                };
                Work { record, editions, title_words: words(&normalized_title(&work.title)) }
            })
            .collect();
        Ok(Self::new(works))
    }

    /// The works `query` names, as a results page; `None` when it names none (ask the library).
    pub fn search(&self, query: &str) -> Option<SearchResults> {
        let query_words = words(&normalized_title(query));
        let hits: Vec<SearchRecord> = self
            .works
            .iter()
            .filter(|work| !work.title_words.is_empty() && work.title_words.is_subset(&query_words))
            .map(|work| work.record.clone())
            .collect();
        if hits.is_empty() {
            return None;
        }
        let records: Vec<SearchRecord> = hits
            .into_iter()
            .enumerate()
            .map(|(index, mut record)| {
                record.rank = Some(index as i64 + 1);
                record
            })
            .collect();
        Some(SearchResults {
            query: Some(query.to_owned()),
            set_id: Some(SET_ID.into()),
            total_results: Some(records.len() as i64),
            showing: None,
            next_page_url: None,
            prev_page_url: None,
            no_results: false,
            records,
        })
    }

    /// A demo format's availability (an edition-level card's `…/BIBENQ?SETLVL=&IRN=<irn>&FMT=RECFMT`
    /// too); `None` for any other URL.
    pub fn availability(&self, url: &str) -> Option<Availability> {
        if let Some(edition) = self.editions_by_availability_url.get(url) {
            return Some(edition.availability.clone());
        }
        if !url.to_ascii_uppercase().contains("FMT=RECFMT") {
            return None;
        }
        session_free_irn(url).and_then(|irn| self.editions_by_irn.get(&irn)).map(|edition| edition.availability.clone())
    }

    /// A demo edition's record, an empty record for a patron row's session-set link, `None` otherwise.
    pub fn record(&self, url: &str) -> Option<RecordDetail> {
        if let Some(record) = self.records_by_url.get(url) {
            return Some(record.clone());
        }
        if is_session_set_link(url) {
            return Some(RecordDetail::default());
        }
        session_free_irn(url).and_then(|irn| self.editions_by_irn.get(&irn)).map(|edition| edition.record.clone())
    }

    /// The bib IRN a demo format's availability URL resolves to.
    pub fn bib_irn(&self, availability_url: &str) -> Option<String> {
        self.editions_by_availability_url.get(availability_url).map(|edition| edition.bib_irn.clone())
    }
}

// ---- URLs ----

/// A loan's or reservation's own link, as the OPAC writes it: a set in the patron's session.
pub fn session_set_url(origin: &str, irn: &str) -> String {
    format!("{origin}/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/{SET_ID}?SETLVL=&QRY=IRN({irn})&NAVLVL=SET")
}

pub fn availability_url(origin: &str, work: &str, code: &str) -> String {
    format!(
        "{origin}/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/{SET_ID}?SETLVL=SET&FMT=RECFMT&XSLT=XRecFmtDisplay.xsl&SETID={SET_ID}&QRY=WRK01%5C{work}\\{code}"
    )
}

pub fn reserve_url(origin: &str, work: &str, code: &str) -> String {
    format!("{origin}/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/{SET_ID}/0/V/0/R/9/ALLWRKENQ?SVL={work}&RECFMT={code}")
}

/// `…/BIBENQ/<set>?…QRY=IRN(…`: a bib in a search set (the patron's, for account rows).
pub fn is_session_set_link(url: &str) -> bool {
    let upper = url.to_ascii_uppercase();
    let Some(at) = upper.find("/BIBENQ/") else {
        return false;
    };
    let rest = &upper[at + "/BIBENQ/".len()..];
    let Some(question) = rest.find('?') else {
        return false;
    };
    !rest[..question].is_empty() && !rest[..question].contains('/') && rest[question..].contains("QRY=IRN(")
}

/// The IRN of a set-less `…/BIBENQ?…IRN=<irn>` link (`patron_item_record_url`).
pub fn session_free_irn(url: &str) -> Option<String> {
    let upper = url.to_ascii_uppercase();
    let at = upper.find("/BIBENQ?")?;
    let query = &url[at + "/BIBENQ?".len()..];
    query.split('&').find_map(|pair| {
        let (name, value) = pair.split_once('=')?;
        (name.eq_ignore_ascii_case("IRN") && !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()))
            .then(|| value.to_owned())
    })
}

// ---- Titles (iOS `FormatRefinder.normalizedTitle`) ----

/// Folded, punctuation-free words of the main title, without a leading article.
pub fn normalized_title(title: &str) -> String {
    let folded: String = main_title(title)
        .chars()
        .flat_map(char::to_lowercase)
        .map(fold_diacritic)
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    let words: Vec<&str> = folded.split_whitespace().collect();
    let trimmed = if words.len() > 1 && ["the", "a", "an"].contains(&words[0]) { &words[1..] } else { &words[..] };
    trimmed.join(" ")
}

/// The title without bracketed notes, subtitle or statement of responsibility.
pub fn main_title(title: &str) -> String {
    let mut text = String::new();
    let mut depth = 0;
    for c in title.chars() {
        match c {
            '[' => depth += 1,
            ']' if depth > 0 => depth -= 1,
            _ if depth == 0 => text.push(c),
            _ => {}
        }
    }
    for separator in [" / ", " : ", ": "] {
        if let Some(at) = text.find(separator) {
            text.truncate(at);
        }
    }
    text.trim_matches(|c: char| c.is_whitespace() || ".:/;,".contains(c)).to_owned()
}

fn fold_diacritic(c: char) -> char {
    match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' => 'a',
        'ç' | 'č' => 'c',
        'è' | 'é' | 'ê' | 'ë' | 'ē' => 'e',
        'ì' | 'í' | 'î' | 'ï' | 'ī' => 'i',
        'ñ' => 'n',
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' => 'o',
        'ù' | 'ú' | 'û' | 'ü' | 'ū' => 'u',
        'ý' | 'ÿ' => 'y',
        other => other,
    }
}

fn words(text: &str) -> HashSet<String> {
    text.split_whitespace().map(str::to_owned).collect()
}

// ---- JSON shapes ----

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogueDto {
    works: Vec<WorkDto>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkDto {
    work_irn: String,
    title: String,
    author: Option<String>,
    author_role: Option<String>,
    year: Option<String>,
    language: Option<String>,
    isbn: Option<String>,
    cover_urls: Vec<String>,
    formats: Vec<FormatDto>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FormatDto {
    code: String,
    label: Option<String>,
    bib_irn: String,
    publisher: Option<String>,
    counts: HashMap<String, i64>,
    record: RecordDetail,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_and_links() {
        assert_eq!(normalized_title("The Midnight Library / Matt Haig."), "midnight library");
        assert_eq!(normalized_title("Whistler : a novel [large print]"), "whistler");
        assert!(is_session_set_link(&session_set_url("https://x", "19244682")));
        assert!(!is_session_set_link("https://x/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&IRN=19244682"));
        assert_eq!(
            session_free_irn("https://x/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&IRN=19244682&FMT=RECFMT").as_deref(),
            Some("19244682")
        );
    }
}
