//! Pure rules behind the book page (iOS `DetailLogic.swift`), kept apart from the view so they're
//! unit-tested: which URL loads the record, the default format, the availability line, the "Already
//! yours" IRNs, subject chips, the details list, the related carousels and re-finding formats.

use std::collections::{HashMap, HashSet};

use librespydus_core::app::{AvailabilitySummary, availability_summary, patron_item_record_url};
use librespydus_core::{
    Availability, DetailRow, Holding, PatronItem, RecordDetail, RecordRelated, RelatedList, SavedBook, SearchRecord,
};

use crate::copy::Copy;
use crate::state::formatting::non_empty;
use crate::state::{BookRef, FormatRef};

// ---- Which URL loads the record ----

/// URLs to try with `record(url)`, best first.
///
/// A search record's `detail_url` (`…/FULL/OPAC/ALLWRKENQ/…`) loads the OPAC's work-level page: a
/// summary but no subjects, imprint, ISBNs, similar titles or other editions. A format's
/// `availability_url` loads the full bib page, so it comes first for work-level refs and
/// edition-level items. Bib URLs (loans, related items, saved bookmarks) are tried first as they
/// are, except a loan's or reservation's own set link (`bib_irn` given): its session-free form
/// goes first.
pub fn candidate_urls(ref_url: Option<&str>, bib_irn: Option<&str>, format: Option<&FormatRef>) -> Vec<String> {
    let format_url = format.and_then(|format| non_empty(format.availability_url.as_deref()));
    let ref_url = non_empty(ref_url);
    let ordered = if is_work_level(ref_url.as_deref()) || format.is_some_and(FormatRef::is_edition_level) {
        vec![format_url, ref_url]
    } else {
        vec![session_free_url(ref_url.as_deref(), bib_irn), ref_url, format_url]
    };
    let mut seen = HashSet::new();
    ordered.into_iter().flatten().filter(|url| seen.insert(url.clone())).collect()
}

/// For a loan's or reservation's own link (`…/BIBENQ/<set>?…QRY=IRN(…)`, which needs the patron's
/// OPAC session), the link that loads the same bib without one (`patron_item_record_url`).
pub fn session_free_url(url: Option<&str>, bib_irn: Option<&str>) -> Option<String> {
    let url = non_empty(url)?;
    let bib_irn = non_empty(bib_irn)?;
    let at = url.to_ascii_uppercase().find("/BIBENQ/")?;
    let rest = &url[at + "/BIBENQ/".len()..];
    if rest.is_empty() || rest.starts_with(['/', '?']) {
        return None;
    }
    let item = PatronItem { bib_irn: Some(bib_irn), bib_url: Some(url.clone()), ..Default::default() };
    patron_item_record_url(item).filter(|free| *free != url)
}

/// `…/ALLWRKENQ/…`, `…/WRKENQ?…`: a work (grouped editions), not one bib record.
pub fn is_work_level(url: Option<&str>) -> bool {
    url.is_some_and(|url| url.to_ascii_uppercase().contains("WRKENQ"))
}

/// No title, summary or details: what the OPAC sends for a page it won't show (a session-set link
/// without the session). Loading it counts as a failure.
pub fn is_empty_record(record: &RecordDetail) -> bool {
    non_empty(record.title.as_deref()).is_none()
        && non_empty(record.summary.as_deref()).is_none()
        && record.details.is_empty()
}

// ---- Display ----

/// A title for display, catalogue notes in brackets dropped ("The Midnight Library [electronic
/// resource] : A Novel" → "The Midnight Library : A Novel").
pub fn display_title(title: &str) -> String {
    let cleaned = without_brackets(title, true);
    let cleaned = cleaned.trim();
    if cleaned.is_empty() { title.to_owned() } else { cleaned.to_owned() }
}

/// `text` without its `[…]` notes; with `eat_space`, also the white space before each.
fn without_brackets(text: &str, eat_space: bool) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find('[') {
        let Some(close) = rest[open..].find(']') else { break };
        let before = &rest[..open];
        out.push_str(if eat_space { before.trim_end() } else { before });
        rest = &rest[open + close + 1..];
    }
    out.push_str(rest);
    out
}

/// "RIVERTON LIBRARY" → "Riverton Library"; mixed case is kept.
pub fn title_cased(text: &str) -> String {
    if text != text.to_uppercase() || !text.chars().any(char::is_alphabetic) {
        return text.to_owned();
    }
    capitalized(text)
}

/// Every word with a capital first letter and the rest lower-case (Swift `capitalized`).
fn capitalized(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut at_start = true;
    for c in text.chars() {
        if c.is_alphanumeric() {
            if at_start {
                out.extend(c.to_uppercase());
            } else {
                out.extend(c.to_lowercase());
            }
            at_start = false;
        } else {
            out.push(c);
            at_start = c != '\'' && c != '’';
        }
    }
    out
}

// ---- Formats ----

/// The format shown first: the first physical one, else the first.
pub fn default_format_index(formats: &[FormatRef]) -> Option<usize> {
    if formats.is_empty() {
        return None;
    }
    Some(formats.iter().position(|format| !format.is_digital()).unwrap_or(0))
}

/// A format's availability as the page keeps it: the counts' summary, which edition (bib record)
/// the format is, and where its copies are listed.
#[derive(Debug, Clone, PartialEq)]
pub struct FormatAvailability {
    pub summary: AvailabilitySummary,
    /// `Availability.bib_record_irn`: the edition the patron may have on loan or reserved.
    pub bib_record_irn: Option<String>,
    /// `Availability.holdings_url`: every copy at every branch (`holdings(url)`).
    pub holdings_url: Option<String>,
}

impl From<Availability> for FormatAvailability {
    fn from(availability: Availability) -> Self {
        let bib_record_irn = non_empty(availability.bib_record_irn.as_deref());
        let holdings_url = non_empty(availability.holdings_url.as_deref());
        Self { summary: availability_summary(availability), bib_record_irn, holdings_url }
    }
}

// ---- Branch holdings ----

/// Whether the page lists the copies (`holdings(availability.holdings_url)`): only for an
/// edition-level item (a Featured list's card, docs/native-apps.md §5.10; iOS `copiesSource`).
/// A book with format chips shows its counts and no copies, physical or digital.
pub fn shows_copies(formats: &[FormatRef]) -> bool {
    crate::state::book_ref::is_edition_level(formats)
}

/// One copy at one branch: where it is, its status (green when it's on the shelf), the library's
/// note on it and its call number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoldingRow {
    /// The branch in title case ("RIVERTON LIBRARY" → "Riverton Library").
    pub location: String,
    pub status: Option<String>,
    pub available: bool,
    pub description: Option<String>,
    pub call_number: Option<String>,
}

impl From<&Holding> for HoldingRow {
    fn from(holding: &Holding) -> Self {
        let location = non_empty(holding.location.as_deref())
            .map(|location| title_cased(&location))
            .or_else(|| non_empty(holding.collection.as_deref()))
            .unwrap_or_else(|| Copy::Detail::copy.to_owned());
        Self {
            location,
            status: non_empty(holding.status.as_deref()),
            available: holding.available,
            description: non_empty(holding.description.as_deref()),
            call_number: non_empty(holding.call_number.as_deref()),
        }
    }
}

// ---- Already yours ----

/// The bib IRNs to look up in the account (`account_holding`) for the edition on screen: the
/// selected format's own (`Availability.bib_record_irn`) once known; else the record's and the
/// opened loan's or reservation's, minus any known to be another format's, so a format whose
/// availability is still loading isn't taken for its neighbour.
pub fn holding_irns(
    selected_format_irn: Option<&str>,
    record_irn: Option<&str>,
    opened_irn: Option<&str>,
    other_format_irns: &HashSet<String>,
) -> Vec<String> {
    if let Some(irn) = non_empty(selected_format_irn) {
        return vec![irn];
    }
    let mut seen = HashSet::new();
    [non_empty(record_irn), non_empty(opened_irn)]
        .into_iter()
        .flatten()
        .filter(|irn| !other_format_irns.contains(irn) && seen.insert(irn.clone()))
        .collect()
}

/// The availability line's colour and icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvailabilityTone {
    Available,
    Normal,
    Digital,
    Muted,
}

/// The availability line under the byline for the selected format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailabilityLine {
    pub text: String,
    pub tone: AvailabilityTone,
}

impl AvailabilityLine {
    /// `availability_summary(…).label`; a digital format without counts reads "Available online".
    pub fn new(summary: Option<&AvailabilitySummary>, format: &FormatRef) -> Self {
        let has_counts = summary.is_some_and(|summary| {
            summary.available.is_some() || summary.copies.is_some() || summary.reserved.is_some()
        });
        if format.is_digital() && !has_counts {
            return Self { text: Copy::Detail::availableOnline.to_owned(), tone: AvailabilityTone::Digital };
        }
        match summary {
            Some(summary) => {
                let tone = if summary.available.is_some_and(|available| available > 0) {
                    AvailabilityTone::Available
                } else if format.is_digital() {
                    AvailabilityTone::Digital
                } else {
                    AvailabilityTone::Normal
                };
                Self { text: summary.label.clone(), tone }
            }
            None => Self { text: Copy::Detail::askLibrary.to_owned(), tone: AvailabilityTone::Muted },
        }
    }
}

// ---- Subjects ----

/// A clickable subject chip: `label` on the chip, `query` for `Route::Results`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubjectChip {
    pub label: String,
    pub query: String,
}

/// At most this many subject chips.
pub const SUBJECT_LIMIT: usize = 6;

/// Chips from rows labelled like Subject(s), Genre/Form, Index terms and Audience, in that
/// priority order (genres first), deduplicated, at most `limit`.
pub fn subjects(details: &[DetailRow], limit: usize) -> Vec<SubjectChip> {
    let mut ranked: Vec<(u8, usize, &DetailRow)> = details
        .iter()
        .enumerate()
        .filter_map(|(index, row)| subject_rank(&row.label).map(|rank| (rank, index, row)))
        .collect();
    ranked.sort_by_key(|(rank, index, _)| (*rank, *index));
    let mut seen = HashSet::new();
    let mut chips = Vec::new();
    for (rank, _, row) in ranked {
        for value in &row.values {
            let Some(chip) = subject_chip(&value.text, rank == 2) else { continue };
            if !seen.insert(chip.label.to_lowercase()) {
                continue;
            }
            chips.push(chip);
            if chips.len() == limit {
                return chips;
            }
        }
    }
    chips
}

/// Whether a details row becomes chips (and so is left out of the details list).
pub fn is_subject_row(label: &str) -> bool {
    subject_rank(label).is_some()
}

/// 0 genre/form, 1 subject/topic/index terms, 2 audience; `None` for any other row.
fn subject_rank(label: &str) -> Option<u8> {
    let label = label.trim().to_lowercase();
    if label.contains("genre") || label == "form" {
        return Some(0);
    }
    if label.starts_with("subject") || label.starts_with("topic") || label.starts_with("index term") {
        return Some(1);
    }
    if label.starts_with("audience") || label.ends_with("audience") {
        return Some(2);
    }
    None
}

/// "Libraries -- Fiction." → "Libraries · Fiction" / "Libraries Fiction"; "FICTION|Literary|"
/// (BISAC) → "Fiction · Literary" (short acronyms such as "DVD" stay); "Interest age level: For
/// ages 6-11 years." → "For ages 6-11 years".
fn subject_chip(raw: &str, is_audience: bool) -> Option<SubjectChip> {
    let mut text = raw.trim();
    if is_audience && let Some(colon) = text.rfind(':') {
        text = &text[colon + 1..];
    }
    let parts: Vec<String> = text
        .replace('|', "--")
        .split("--")
        .map(|part| part.trim_matches(|c: char| c.is_whitespace() || ".;,".contains(c)).to_owned())
        .filter(|part| !part.is_empty())
        .map(|part| {
            let shouting = part.chars().count() > 3 && part == part.to_uppercase() && part != part.to_lowercase();
            if shouting { capitalized(&part) } else { part }
        })
        .collect();
    if parts.is_empty() {
        return None;
    }
    let label = parts.join(Copy::Common::separator);
    let spaced: String =
        parts.join(" ").chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '\'' { c } else { ' ' }).collect();
    let query = spaced.split_whitespace().collect::<Vec<_>>().join(" ");
    if query.is_empty() {
        return None;
    }
    Some(SubjectChip { label, query })
}

// ---- Details list ----

/// One line of the compact details list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetailFact {
    pub label: String,
    pub value: String,
}

pub const MAX_FACT_ROWS: usize = 8;
const MAX_FACT_VALUES: usize = 3;

/// The library's own row labels, matched lower-cased; never shown.
const HIDDEN_ROWS: [&str; 8] =
    ["main title", "title", "work", "bookmark link", "bookmark", "brn", "go to", "permalink"]; // copy-exempt: library labels matched against the record
const BYLINE_ROWS: [&str; 4] = ["author", "creator", "authors", "creators"];

/// The library's details rows minus what the page already shows (title, byline, subject chips)
/// and navigation-only rows (work link, bookmark, BRN, "Go to"). A single-author row is the
/// byline; several authors (illustrator, translator…) are listed.
pub fn facts(details: &[DetailRow], max_rows: usize) -> Vec<DetailFact> {
    let mut facts = Vec::new();
    for row in details {
        let key = row.label.trim().to_lowercase();
        if HIDDEN_ROWS.contains(&key.as_str()) || is_subject_row(&row.label) {
            continue;
        }
        let values: Vec<String> =
            row.values.iter().map(|value| value.text.trim().to_owned()).filter(|value| !value.is_empty()).collect();
        if values.is_empty() || (BYLINE_ROWS.contains(&key.as_str()) && values.len() < 2) {
            continue;
        }
        let mut shown: Vec<String> = values.iter().take(MAX_FACT_VALUES).cloned().collect();
        if values.len() > MAX_FACT_VALUES
            && let Some(last) = shown.last_mut()
        {
            last.push_str(" …"); // copy-exempt: the ellipsis after a truncated list of the library's values
        }
        facts.push(DetailFact { label: row.label.trim().to_owned(), value: shown.join("\n") });
        if facts.len() == max_rows {
            break;
        }
    }
    facts
}

/// The author as the details list gives it (a "Author" / "Creator" row), for a ref without one.
pub fn author_in(record: Option<&RecordDetail>) -> Option<String> {
    let row =
        record?.details.iter().find(|row| BYLINE_ROWS[..2].contains(&row.label.trim().to_lowercase().as_str()))?;
    non_empty(row.values.first().map(|value| value.text.as_str()))
}

// ---- Related carousels ----

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RelatedPanel {
    Similar,
    Author,
    Series,
    Versions,
}

impl RelatedPanel {
    pub const ALL: [RelatedPanel; 4] = [Self::Similar, Self::Author, Self::Series, Self::Versions];

    pub fn title(self) -> &'static str {
        match self {
            Self::Similar => Copy::Detail::similarBooks,
            Self::Author => Copy::Detail::moreByAuthor,
            Self::Series => Copy::Detail::inThisSeries,
            Self::Versions => Copy::Detail::otherEditions,
        }
    }

    /// For element ids: `detail-related-similar`, ….
    pub fn key(self) -> &'static str {
        match self {
            Self::Similar => "similar",
            Self::Author => "author",
            Self::Series => "series",
            Self::Versions => "versions",
        }
    }

    pub fn url(self, related: &RecordRelated) -> Option<String> {
        let url = match self {
            Self::Similar => &related.similar,
            Self::Author => &related.author,
            Self::Series => &related.series,
            Self::Versions => &related.versions,
        };
        non_empty(url.as_deref())
    }

    /// This panel's list in a page saved on this computer (§5.13).
    pub fn list(self, saved: &SavedBook) -> Option<&RelatedList> {
        match self {
            Self::Similar => saved.similar.as_ref(),
            Self::Author => saved.author.as_ref(),
            Self::Series => saved.series.as_ref(),
            Self::Versions => saved.versions.as_ref(),
        }
    }

    /// A panel's books as refs, without the book on screen (matched by BRN), each once.
    pub fn items(list: &RelatedList, excluding_brn: Option<&str>) -> Vec<BookRef> {
        let brn = non_empty(excluding_brn);
        let mut seen = HashSet::new();
        list.items
            .iter()
            .filter(|item| brn.is_none() || non_empty(item.brn.as_deref()) != brn)
            .map(BookRef::from_related_item)
            .filter(|book| seen.insert(book.id.clone()))
            .collect()
    }
}

/// A carousel's state, as `visible_panels` reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelState {
    /// Not asked for yet, or loading.
    Waiting,
    Items(usize),
    Failed,
}

/// The panels to draw, in order: the record links to them and they're loading or have items.
/// Empty and failed panels are hidden.
pub fn visible_panels(related: &RecordRelated, states: &HashMap<RelatedPanel, PanelState>) -> Vec<RelatedPanel> {
    RelatedPanel::ALL
        .into_iter()
        .filter(|panel| panel.url(related).is_some())
        .filter(|panel| match states.get(panel).copied().unwrap_or(PanelState::Waiting) {
            PanelState::Waiting => true,
            PanelState::Items(count) => count > 0,
            PanelState::Failed => false,
        })
        .collect()
}

/// A book page to save: its record and the lists its carousels loaded.
pub fn saved_book(record: RecordDetail, lists: &HashMap<RelatedPanel, RelatedList>) -> SavedBook {
    SavedBook {
        record,
        similar: lists.get(&RelatedPanel::Similar).cloned(),
        author: lists.get(&RelatedPanel::Author).cloned(),
        series: lists.get(&RelatedPanel::Series).cloned(),
        versions: lists.get(&RelatedPanel::Versions).cloned(),
    }
}

// ---- Re-finding formats ----

/// What to search for, for a ref opened without formats (a loan, a pick, a related item). ISBNs
/// aren't in the OPAC keyword index, so it's the main title plus the author's surname, falling back
/// to the ISBN only when there is no title.
pub fn format_search_query(book: &BookRef, record: Option<&RecordDetail>) -> Option<String> {
    let title = main_title(&ref_title(book, record));
    let surname = surname(book.author.clone().or_else(|| author_in(record)).as_deref());
    if !title.is_empty() {
        return Some([Some(title), surname].into_iter().flatten().collect::<Vec<_>>().join(" "));
    }
    isbns(book, record).into_iter().next()
}

/// The search result that is this book, or `None` when no result is a confident match: the same
/// work IRN, else a shared ISBN, else the only result with the same title and author (or year).
pub fn matching_record<'a>(
    book: &BookRef,
    record: Option<&RecordDetail>,
    candidates: &'a [SearchRecord],
) -> Option<&'a SearchRecord> {
    let usable: Vec<&SearchRecord> = candidates.iter().filter(|candidate| !candidate.formats.is_empty()).collect();
    let works = work_irns(book, record);
    if !works.is_empty()
        && let Some(hit) = usable
            .iter()
            .find(|candidate| non_empty(candidate.work_irn.as_deref()).is_some_and(|irn| works.contains(&irn)))
    {
        return Some(hit);
    }
    let isbns: HashSet<String> = isbns(book, record).into_iter().collect();
    if !isbns.is_empty()
        && let Some(hit) = usable.iter().find(|candidate| {
            normalized_isbns(&[candidate.isbn.clone().unwrap_or_default()]).iter().any(|isbn| isbns.contains(isbn))
        })
    {
        return Some(hit);
    }
    let title = normalized_title(&ref_title(book, record));
    if title.is_empty() {
        return None;
    }
    let surname = surname(book.author.clone().or_else(|| author_in(record)).as_deref());
    let same_title: Vec<&SearchRecord> = usable
        .iter()
        .copied()
        .filter(|candidate| {
            if normalized_title(candidate.title.as_deref().unwrap_or_default()) != title {
                return false;
            }
            match (&surname, self::surname(candidate.author.as_deref())) {
                (Some(surname), Some(other)) => *surname == other,
                _ => true,
            }
        })
        .collect();
    if same_title.len() == 1 {
        return Some(same_title[0]);
    }
    if let Some(year) = non_empty(book.year.as_deref()) {
        let same_year: Vec<&SearchRecord> = same_title
            .iter()
            .copied()
            .filter(|candidate| non_empty(candidate.year.as_deref()).as_deref() == Some(year.as_str()))
            .collect();
        if same_year.len() == 1 {
            return Some(same_year[0]);
        }
    }
    None
}

/// Work IRNs this book is known by: a `work:` ref id, the record's "Work" row link
/// (`…/WRKENQ/…?IRN=<work>`), or a work-level record's own IRN.
pub fn work_irns(book: &BookRef, record: Option<&RecordDetail>) -> HashSet<String> {
    let mut irns = HashSet::new();
    if let Some(irn) = book.id.strip_prefix("work:") {
        irns.insert(irn.to_owned());
    }
    let Some(record) = record else { return irns };
    for row in record.details.iter().filter(|row| row.label.trim().eq_ignore_ascii_case("work")) {
        for value in &row.values {
            if let Some(irn) = query_item("IRN", value.url.as_deref()) {
                irns.insert(irn);
            }
        }
    }
    if is_work_level(record.bookmark_url.as_deref()) {
        if let Some(irn) = query_item("IRN", record.bookmark_url.as_deref()) {
            irns.insert(irn);
        }
        if let Some(irn) = non_empty(record.bib_irn.as_deref()) {
            irns.insert(irn);
        }
    }
    irns
}

/// Every ISBN the ref and record mention, as ISBN-13.
pub fn isbns(book: &BookRef, record: Option<&RecordDetail>) -> Vec<String> {
    let mut texts =
        vec![book.isbn.clone().unwrap_or_default(), record.and_then(|record| record.isbn.clone()).unwrap_or_default()];
    if let Some(record) = record {
        for row in record.details.iter().filter(|row| row.label.to_lowercase().starts_with("isbn")) {
            texts.extend(row.values.iter().map(|value| value.text.clone()));
        }
    }
    normalized_isbns(&texts)
}

/// ISBN-10s and ISBN-13s found in `texts` ("9781786892720 (paperback)", "1-78689-272-3"), as
/// ISBN-13s, in order, without duplicates.
pub fn normalized_isbns(texts: &[String]) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    for text in texts {
        for token in text.split(|c: char| c.is_whitespace() || "();,".contains(c)) {
            let compact: String = token.chars().filter(|c| *c != '-').collect::<String>().to_uppercase();
            if let Some(isbn) = isbn13(&compact)
                && !result.contains(&isbn)
            {
                result.push(isbn);
            }
        }
    }
    result
}

fn isbn13(text: &str) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() == 13 && chars.iter().all(char::is_ascii_digit) {
        return Some(text.to_owned());
    }
    if chars.len() != 10
        || !chars[..9].iter().all(char::is_ascii_digit)
        || !(chars[9].is_ascii_digit() || chars[9] == 'X')
    {
        return None;
    }
    let body: String = format!("978{}", chars[..9].iter().collect::<String>());
    let sum: u32 = body
        .chars()
        .enumerate()
        .map(|(index, c)| c.to_digit(10).unwrap_or(0) * if index % 2 == 0 { 1 } else { 3 })
        .sum();
    Some(format!("{body}{}", (10 - sum % 10) % 10))
}

/// "The Midnight Library [electronic resource] : a novel / Matt Haig." → "midnight library".
pub fn normalized_title(title: &str) -> String {
    let folded: String = main_title(title)
        .chars()
        .map(fold_diacritic)
        .flat_map(char::to_lowercase)
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    let words: Vec<&str> = folded.split_whitespace().collect();
    let words = if words.len() > 1 && ["the", "a", "an"].contains(&words[0]) { &words[1..] } else { &words[..] };
    words.join(" ")
}

/// The title without bracketed notes, subtitle or statement of responsibility.
pub fn main_title(title: &str) -> String {
    let mut text = without_brackets(title, false);
    for separator in [" / ", " : ", ": "] {
        if let Some(at) = text.find(separator) {
            text.truncate(at);
        }
    }
    text.trim_matches(|c: char| c.is_whitespace() || ".:/;,".contains(c)).to_owned()
}

/// "Haig, Matt, 1975-" → "haig"; "Matt Haig" → "haig".
pub fn surname(author: Option<&str>) -> Option<String> {
    let author = non_empty(author)?;
    let name = match author.split_once(',') {
        Some((surname, _)) => surname.to_owned(),
        None => author.split_whitespace().last().unwrap_or_default().to_owned(),
    };
    let folded: String = name.chars().map(fold_diacritic).flat_map(char::to_lowercase).collect();
    let trimmed = folded.trim_matches(|c: char| !c.is_alphabetic());
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

fn ref_title(book: &BookRef, record: Option<&RecordDetail>) -> String {
    if book.title != Copy::Common::untitled && !book.title.trim().is_empty() {
        return book.title.clone();
    }
    record.and_then(|record| record.title.clone()).unwrap_or_default()
}

/// A query parameter's value, matched in the text (OPAC URLs hold characters a URL parser rejects).
fn query_item(name: &str, url: Option<&str>) -> Option<String> {
    let url = url?;
    let lower = url.to_ascii_lowercase();
    let needle = format!("{}=", name.to_ascii_lowercase());
    let mut from = 0;
    while let Some(found) = lower[from..].find(&needle) {
        let at = from + found;
        if at > 0 && matches!(lower.as_bytes()[at - 1], b'?' | b'&') {
            let value: String = url[at + needle.len()..].chars().take_while(|c| *c != '&' && *c != '#').collect();
            return (!value.is_empty()).then_some(value);
        }
        from = at + needle.len();
    }
    None
}

/// Latin letters with accents → the plain letter (enough for titles and surnames).
fn fold_diacritic(c: char) -> char {
    match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' => 'a',
        'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' | 'Ā' => 'A',
        'ç' | 'ć' | 'č' => 'c',
        'Ç' | 'Ć' | 'Č' => 'C',
        'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ę' | 'ě' => 'e',
        'È' | 'É' | 'Ê' | 'Ë' | 'Ē' | 'Ę' | 'Ě' => 'E',
        'ì' | 'í' | 'î' | 'ï' | 'ī' => 'i',
        'Ì' | 'Í' | 'Î' | 'Ï' | 'Ī' => 'I',
        'ñ' | 'ń' | 'ň' => 'n',
        'Ñ' | 'Ń' | 'Ň' => 'N',
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' => 'o',
        'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' | 'Ø' | 'Ō' => 'O',
        'ù' | 'ú' | 'û' | 'ü' | 'ū' | 'ů' => 'u',
        'Ù' | 'Ú' | 'Û' | 'Ü' | 'Ū' | 'Ů' => 'U',
        'ý' | 'ÿ' => 'y',
        'Ý' => 'Y',
        'š' | 'ś' => 's',
        'Š' | 'Ś' => 'S',
        'ž' | 'ź' | 'ż' => 'z',
        'Ž' | 'Ź' | 'Ż' => 'Z',
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use librespydus_core::{DetailValue, RecordFormat, RelatedItem};

    fn row(label: &str, values: &[&str]) -> DetailRow {
        DetailRow {
            label: label.into(),
            values: values.iter().map(|text| DetailValue { text: (*text).into(), url: None }).collect(),
        }
    }

    fn format(code: &str, label: &str) -> FormatRef {
        FormatRef { code: code.into(), label: Some(label.into()), reserve_url: None, availability_url: None }
    }

    #[test]
    fn record_urls_prefer_the_bib_page() {
        let work = "https://x/cgi-bin/spydus.exe/FULL/OPAC/ALLWRKENQ/1/2,1";
        let mut book = format("BK", "Book");
        book.availability_url = Some("https://x/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/1?FMT=RECFMT&QRY=BK".into());
        assert_eq!(
            candidate_urls(Some(work), None, Some(&book)),
            vec![book.availability_url.clone().unwrap(), work.into()]
        );
        let loan = "https://x/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/78500000?SETLVL=&QRY=IRN(19244682)&NAVLVL=SET";
        assert_eq!(
            candidate_urls(Some(loan), Some("19244682"), None),
            vec!["https://x/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&IRN=19244682".to_owned(), loan.to_owned()]
        );
        let free = "https://x/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&IRN=1";
        assert_eq!(candidate_urls(Some(free), Some("1"), None), vec![free.to_owned()]);
        assert!(candidate_urls(None, None, None).is_empty());
        assert!(is_work_level(Some(work)) && !is_work_level(Some(free)));
    }

    #[test]
    fn titles_and_names() {
        assert_eq!(
            display_title("The Midnight Library [electronic resource] : A Novel"),
            "The Midnight Library : A Novel"
        );
        assert_eq!(display_title("[x]"), "[x]");
        assert_eq!(title_cased("RIVERTON LIBRARY"), "Riverton Library");
        assert_eq!(title_cased("Riverton library"), "Riverton library");
        assert_eq!(
            main_title("The Midnight Library [electronic resource] : a novel / Matt Haig."),
            "The Midnight Library"
        );
        assert_eq!(
            normalized_title("The Midnight Library [electronic resource] : a novel / Matt Haig."),
            "midnight library"
        );
        assert_eq!(surname(Some("Haig, Matt, 1975-")).as_deref(), Some("haig"));
        assert_eq!(surname(Some("Matt Haig")).as_deref(), Some("haig"));
        assert_eq!(surname(Some("Brontë, Charlotte")).as_deref(), Some("bronte"));
        assert_eq!(surname(Some(" ")), None);
    }

    #[test]
    fn default_format_is_the_first_physical_one() {
        assert_eq!(default_format_index(&[]), None);
        assert_eq!(default_format_index(&[format("EBK", "eBook"), format("BK", "Book")]), Some(1));
        assert_eq!(default_format_index(&[format("EBK", "eBook")]), Some(0));
    }

    #[test]
    fn availability_lines() {
        let summary = |available: Option<i64>, copies: Option<i64>, label: &str| AvailabilitySummary {
            available,
            copies,
            reserved: None,
            label: label.into(),
        };
        let book = format("BK", "Book");
        let ebook = format("EBK", "eBook");
        assert_eq!(
            AvailabilityLine::new(Some(&summary(Some(2), Some(3), "2 of 3 available")), &book),
            AvailabilityLine { text: "2 of 3 available".into(), tone: AvailabilityTone::Available }
        );
        assert_eq!(
            AvailabilityLine::new(Some(&summary(Some(0), Some(3), "0 of 3")), &book).tone,
            AvailabilityTone::Normal
        );
        assert_eq!(
            AvailabilityLine::new(Some(&summary(None, None, "No copies listed")), &ebook).text,
            "Available online"
        );
        assert_eq!(AvailabilityLine::new(None, &ebook).text, "Available online");
        assert_eq!(AvailabilityLine::new(None, &book).tone, AvailabilityTone::Muted);
        assert_eq!(
            AvailabilityLine::new(Some(&summary(Some(1), Some(1), "1 of 1")), &ebook).tone,
            AvailabilityTone::Available
        );
    }

    #[test]
    fn holding_irns_prefer_the_selected_format() {
        let none = HashSet::new();
        assert_eq!(holding_irns(Some("9"), Some("1"), Some("2"), &none), vec!["9"]);
        assert_eq!(holding_irns(None, Some("1"), Some("1"), &none), vec!["1"]);
        let others = HashSet::from(["1".to_owned()]);
        assert_eq!(holding_irns(None, Some("1"), Some("2"), &others), vec!["2"]);
    }

    #[test]
    fn subject_chips_rank_and_clean() {
        let details = vec![
            row("Subjects", &["Libraries -- Fiction.", "Libraries -- Fiction."]),
            row("Genre", &["FICTION|Literary|"]),
            row("Audience", &["Interest age level: For ages 6-11 years."]),
            row("Publisher", &["Canongate"]),
        ];
        let chips = subjects(&details, SUBJECT_LIMIT);
        let labels: Vec<&str> = chips.iter().map(|chip| chip.label.as_str()).collect();
        assert_eq!(labels, vec!["Fiction · Literary", "Libraries · Fiction", "For ages 6-11 years"]);
        assert_eq!(chips[1].query, "Libraries Fiction");
        assert_eq!(chips[2].query, "For ages 6-11 years");
        assert_eq!(subjects(&details, 1).len(), 1);
        assert!(is_subject_row("Subject") && !is_subject_row("Publisher"));
        let dvd = subjects(&[row("Form", &["DVD"])], 6);
        assert_eq!(dvd[0].label, "DVD");
    }

    #[test]
    fn facts_hide_what_the_page_shows() {
        let details = vec![
            row("Main title", &["x"]),
            row("Author", &["Haig, Matt"]),
            row("Creators", &["A", "B"]),
            row("Subject", &["s"]),
            row("Publisher", &["Canongate", ""]),
            row("ISBN", &["1", "2", "3", "4"]),
            row("BRN", &["467086"]),
        ];
        let facts = facts(&details, MAX_FACT_ROWS);
        assert_eq!(
            facts,
            vec![
                DetailFact { label: "Creators".into(), value: "A\nB".into() },
                DetailFact { label: "Publisher".into(), value: "Canongate".into() },
                DetailFact { label: "ISBN".into(), value: "1\n2\n3 …".into() },
            ]
        );
        let record = RecordDetail { details: vec![row("Author", &["Haig, Matt"])], ..Default::default() };
        assert_eq!(author_in(Some(&record)).as_deref(), Some("Haig, Matt"));
    }

    #[test]
    fn related_panels_hide_empty_and_failed_ones() {
        let related = RecordRelated {
            similar: Some("s".into()),
            author: Some("a".into()),
            series: Some("x".into()),
            versions: None,
        };
        let states = HashMap::from([
            (RelatedPanel::Similar, PanelState::Items(2)),
            (RelatedPanel::Author, PanelState::Items(0)),
            (RelatedPanel::Series, PanelState::Failed),
        ]);
        assert_eq!(visible_panels(&related, &states), vec![RelatedPanel::Similar]);
        assert_eq!(visible_panels(&related, &HashMap::new()).len(), 3);
        let item = |brn: &str| RelatedItem { brn: Some(brn.into()), title: Some(brn.into()), ..Default::default() };
        let list = RelatedList { items: vec![item("1"), item("2"), item("2")] };
        let items = RelatedPanel::items(&list, Some("1"));
        assert_eq!(items.iter().map(|book| book.id.as_str()).collect::<Vec<_>>(), vec!["brn:2"]);
    }

    #[test]
    fn refinding_formats_matches_confidently() {
        let mut book = BookRef::new("bib:1", "The Midnight Library [large print]");
        book.author = Some("Haig, Matt".into());
        assert_eq!(format_search_query(&book, None).as_deref(), Some("The Midnight Library haig"));
        let candidate = |irn: &str, title: &str, author: &str, isbn: Option<&str>| SearchRecord {
            work_irn: Some(irn.into()),
            title: Some(title.into()),
            author: Some(author.into()),
            isbn: isbn.map(Into::into),
            formats: vec![RecordFormat { code: "BK".into(), ..Default::default() }],
            ..Default::default()
        };
        let candidates = vec![
            candidate("10", "Midnight sun", "Meyer, Stephenie", None),
            candidate("11", "The midnight library", "Haig, Matt, author", None),
        ];
        assert_eq!(matching_record(&book, None, &candidates).and_then(|hit| hit.work_irn.as_deref()), Some("11"));
        let twins = vec![
            candidate("11", "The midnight library", "Haig", None),
            candidate("12", "The midnight library", "Haig", None),
        ];
        assert!(matching_record(&book, None, &twins).is_none());
        book.isbn = Some("1-78689-272-3".into());
        let by_isbn = vec![twins[0].clone(), candidate("12", "x", "y", Some("9781786892720 (paperback)"))];
        assert_eq!(matching_record(&book, None, &by_isbn).and_then(|hit| hit.work_irn.as_deref()), Some("12"));
        let work = BookRef::new("work:12", "x");
        assert_eq!(matching_record(&work, None, &twins).and_then(|hit| hit.work_irn.as_deref()), Some("12"));
        assert_eq!(normalized_isbns(&["1-78689-272-3".into()]), vec!["9781786892720"]);
        assert_eq!(query_item("IRN", Some("https://x/WRKENQ?a=1&IRN=77&b")).as_deref(), Some("77"));
    }

    #[test]
    fn copies_only_for_edition_level_items() {
        let format = |code: &str, label: Option<&str>| FormatRef {
            code: code.into(),
            label: label.map(str::to_owned),
            reserve_url: None,
            availability_url: Some("https://x/availability".into()),
        };
        assert!(shows_copies(&[format("", None)]));
        assert!(!shows_copies(&[format("BK", Some("Book"))]));
        assert!(!shows_copies(&[format("BK", Some("Book")), format("LP", Some("Large Print"))]));
        assert!(!shows_copies(&[format("EBK", Some("eBook"))]));
        assert!(!shows_copies(&[]));
    }

    #[test]
    fn holdings_rows() {
        let holding = Holding {
            location: Some("RIVERTON LIBRARY".into()),
            collection: None,
            call_number: Some(" 823 HAI ".into()),
            status: Some("Available".into()),
            description: None,
            available: true,
        };
        let row = HoldingRow::from(&holding);
        assert_eq!(row.location, "Riverton Library");
        assert_eq!(row.call_number.as_deref(), Some("823 HAI"));
        assert_eq!(HoldingRow::from(&Holding::default()).location, "Copy");
    }
}
