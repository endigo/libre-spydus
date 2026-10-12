//! JSON/FFI shapes, mirroring `middleware/src/types.ts` key-for-key.
//!
//! TS `x | null` → `Option<T>` (serializes `null`), arrays → `Vec<T>`, JS numbers → `i64`,
//! `Record<string, …>` → `HashMap`. Every record is a UniFFI record and serializes camelCase.

use std::collections::HashMap;

/// "Showing 1 - 10 of 46" → `{from: 1, to: 10}`.
#[derive(uniffi::Record, serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Showing {
    pub from: i64,
    pub to: i64,
}

#[derive(uniffi::Record, serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SearchResults {
    pub query: Option<String>,
    pub set_id: Option<String>,
    pub total_results: Option<i64>,
    pub showing: Option<Showing>,
    /// Absolute URL of the next page of results, or null on the last page.
    pub next_page_url: Option<String>,
    /// Absolute URL of the previous page, or null on the first page.
    pub prev_page_url: Option<String>,
    /// True only for the OPAC's genuine "no records found" page ("Sorry, your search for … did
    /// not find any records."); `records` is then empty. Any other page without records (an
    /// unrecognised page) stays false.
    pub no_results: bool,
    pub records: Vec<SearchRecord>,
}

/// Result order for `SpydusClient::search_sorted`.
#[derive(
    uniffi::Enum,
    serde::Serialize,
    serde::Deserialize,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Default,
)]
#[serde(rename_all = "camelCase")]
pub enum SearchSort {
    /// The OPAC's relevance ranking (`SORTS=SQL_REL_GENWRK`), what `search` uses.
    #[default]
    Relevance,
    /// Most recently added to the catalogue first ("new arrivals"):
    /// `SORTS=MAIN.CREATED_DATE.DESC]MAIN.CREATED_TIME.DESC`, the order of the OPAC's own RSS feed.
    Newest,
}

#[derive(uniffi::Record, serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SearchRecord {
    pub rank: Option<i64>,
    /// Work IRN (the SVL checkbox value); identifies the grouped work.
    pub work_irn: Option<String>,
    pub title: Option<String>,
    pub detail_url: Option<String>,
    pub author: Option<String>,
    pub author_role: Option<String>,
    pub author_url: Option<String>,
    pub year: Option<String>,
    pub language: Option<String>,
    pub summary: Option<String>,
    pub isbn: Option<String>,
    pub cover_urls: Vec<String>,
    pub formats: Vec<RecordFormat>,
}

#[derive(uniffi::Record, serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RecordFormat {
    /// e.g. BK, EBK, AB — parsed from the tab element id.
    pub code: String,
    pub label: Option<String>,
    /// Reservation launch URL (/PGM/OPAC/CCOPT/...).
    pub reserve_url: Option<String>,
    /// Lazy per-format availability fragment (…RECFMT={code}); feed to `parse_availability`.
    pub availability_url: Option<String>,
}

#[derive(uniffi::Record, serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Availability {
    /// The concrete bib-record IRN this format resolves to (differs from workIrn).
    pub bib_record_irn: Option<String>,
    pub title: Option<String>,
    pub publisher: Option<String>,
    /// All label:value availability pairs, lower-cased keys (reserved, copies, ...).
    pub counts: HashMap<String, i64>,
    pub reserved: Option<i64>,
    pub copies: Option<i64>,
    /// Branch-level holdings modal (/XHLD/OPAC/BIBENQ/{setId}/{bibIrn}).
    pub holdings_url: Option<String>,
    /// Full record fragment (/XFULL/OPAC/BIBENQ/{setId}/{bibIrn}).
    pub record_url: Option<String>,
}

/// One value within a record-detail row — text plus an optional OPAC link.
#[derive(uniffi::Record, serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DetailValue {
    pub text: String,
    pub url: Option<String>,
}

/// One labelled row of the BIBENQ "Record details" block.
#[derive(uniffi::Record, serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DetailRow {
    pub label: String,
    pub values: Vec<DetailValue>,
}

/// Lazy related-title panel URLs (`<a id="urltab{NAME}">`).
#[derive(uniffi::Record, serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RecordRelated {
    pub similar: Option<String>,
    pub author: Option<String>,
    pub series: Option<String>,
    pub versions: Option<String>,
}

/// The rich bib-record page (BIBENQ), mirroring the OPAC "Record details".
#[derive(uniffi::Record, serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RecordDetail {
    pub bib_irn: Option<String>,
    pub brn: Option<String>,
    pub title: Option<String>,
    pub series_title: Option<String>,
    pub isbn: Option<String>,
    pub cover_urls: Vec<String>,
    pub summary: Option<String>,
    pub details: Vec<DetailRow>,
    pub bookmark_url: Option<String>,
    pub related: RecordRelated,
}

/// A brief card from a related-titles fragment (similar / author / series).
#[derive(uniffi::Record, serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RelatedItem {
    pub title: Option<String>,
    pub bib_url: Option<String>,
    pub brn: Option<String>,
    pub author: Option<String>,
    pub year: Option<String>,
    pub isbn: Option<String>,
    pub cover_urls: Vec<String>,
}

/// `/related` response body: `{items: [...]}`.
#[derive(uniffi::Record, serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RelatedList {
    pub items: Vec<RelatedItem>,
}

/// A row action link (Renew loan, Cancel/Edit reservation).
#[derive(uniffi::Record, serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct PatronAction {
    pub label: String,
    pub href: String,
}

/// One row in a patron list (a current loan or a reservation).
#[derive(uniffi::Record, serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct PatronItem {
    /// The SVL value — loan IRN or reservation IRN.
    pub irn: Option<String>,
    /// The bib-record IRN (parsed from bibUrl) — matches a RecordDetail.bibIrn.
    pub bib_irn: Option<String>,
    pub title: Option<String>,
    pub bib_url: Option<String>,
    pub author: Option<String>,
    pub year: Option<String>,
    pub cover_urls: Vec<String>,
    pub isbn: Option<String>,
    /// Columnar cells keyed by their `data-caption` (e.g. {Due: "27 Oct 2026"}).
    pub fields: HashMap<String, String>,
    pub actions: Vec<PatronAction>,
}

#[derive(uniffi::Record, serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct PatronList {
    pub total: Option<i64>,
    pub items: Vec<PatronItem>,
}

/// `/account/reservations` response: ready-for-pickup + pending lists.
#[derive(uniffi::Record, serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Reservations {
    pub ready: PatronList,
    pub pending: PatronList,
}

/// Links extracted from the authenticated account (BRWENQ) page.
#[derive(uniffi::Record, serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct AccountLinks {
    pub name: Option<String>,
    pub loans_url: Option<String>,
    pub reservations_ready_url: Option<String>,
    pub reservations_pending_url: Option<String>,
}

/// A "Not required after" choice (`XDAYS` option).
#[derive(uniffi::Record, serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExpiryChoice {
    pub days: String,
    pub label: String,
}

/// What the app shows before confirming a reservation.
#[derive(uniffi::Record, serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReserveOptions {
    pub can_reserve: bool,
    pub pickup_location: Option<String>,
    pub available_summary: Option<String>,
    pub expiry_choices: Vec<ExpiryChoice>,
    pub message: Option<String>,
}

/// Outcome of a place / cancel / edit reservation attempt.
#[derive(uniffi::Record, serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReserveResult {
    pub ok: bool,
    pub message: Option<String>,
}

/// A pickup-location choice from the edit-reservation form.
#[derive(uniffi::Record, serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct PickupChoice {
    pub code: String,
    pub name: String,
}

/// What the app shows before editing a reservation.
#[derive(uniffi::Record, serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct EditOptions {
    pub can_edit: bool,
    pub pickup_choices: Vec<PickupChoice>,
    pub pickup_selected: Option<String>,
    pub delay_until: Option<String>,
    pub not_required_after: Option<String>,
    pub message: Option<String>,
}

/// Caller-supplied changes for `edit_reservation` (auth.ts `EditOverrides`).
#[derive(uniffi::Record, serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct EditOverrides {
    pub pickup: Option<String>,
    pub delay_until: Option<String>,
    pub not_required_after: Option<String>,
    pub clear_delay: bool,
    pub clear_expiry: bool,
}

/// One library from the public Spydus directory (`directory::list_libraries`), for a library
/// picker. `origin` is what `SpydusClient::new` takes.
#[derive(uniffi::Record, serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct LibraryInfo {
    pub id: String,
    pub name: String,
    /// Normalized `scheme://host[:port]`: no path or trailing slash, host lower-cased,
    /// a missing scheme colon (`https//x`) repaired.
    pub origin: String,
    pub logo_url: Option<String>,
    /// Display name: "Australia", "United Kingdom", "United States", "New Zealand",
    /// "Singapore", else the directory's raw value (trimmed).
    pub country: Option<String>,
    /// "AU" | "UK" | "US" | "NZ" | "SG" | "OTHER".
    pub region: String,
    /// A Civica test/UAT/training/demo/QA instance rather than a real library.
    pub is_test: bool,
}
