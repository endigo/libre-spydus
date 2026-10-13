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
    /// Work IRN (the SVL checkbox value); identifies the grouped work. It is also the IRN
    /// `save_pick` saves: an edition-level card's (saved-search lists) is its edition's bib IRN,
    /// and a Pick list card's is the IRN that was saved (its `RSVL` checkbox).
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

/// What a featured link opens (`FeaturedLink.kind`).
#[derive(uniffi::Enum, serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FeaturedKind {
    /// A collection page: `SpydusClient::collection(url)` → its sections.
    Collection,
    /// One saved search: `SpydusClient::page(url)` → its results.
    Search,
    /// Anything else (a page of links to outside services, another site): open it on the web.
    Web,
}

/// A link in the library's "Featured" list on its OPAC home page.
#[derive(uniffi::Record, serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FeaturedLink {
    /// e.g. "Library of Things".
    pub title: String,
    /// The link's tooltip when it says more than the title, e.g. "New Library items available in
    /// the last 2 months".
    pub description: Option<String>,
    /// The Font Awesome icon the library picked, without `fa-`: "book", "balloons", "computer",
    /// "circle", "globe", "game-board", "map-location-dot", "user-circle".
    pub icon: Option<String>,
    pub kind: FeaturedKind,
    /// `Collection`: the page for `collection`; `Search`: a results list for `page`; `Web`: as is.
    pub url: String,
}

/// A featured collection page (`SpydusClient::collection`).
#[derive(uniffi::Record, serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct FeaturedCollection {
    /// The page's title, e.g. "Library of Things".
    pub title: Option<String>,
    /// Its sections in order; none for a page that isn't a set of saved searches (open it on the web).
    pub sections: Vec<FeaturedSection>,
}

/// One section of a collection page: a titled saved search.
#[derive(uniffi::Record, serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FeaturedSection {
    /// e.g. "Musical Instruments".
    pub title: String,
    /// Its results list, 20 a page: `SpydusClient::page(url)`.
    pub url: String,
}

/// One copy in a record's holdings list (the "View availability" modal, `/XHLD/OPAC/BIBENQ/…`).
#[derive(uniffi::Record, serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Holding {
    /// The branch, e.g. "RIVERTON LIBRARY".
    pub location: Option<String>,
    /// e.g. "Library of Things", "Books - Adult Fiction".
    pub collection: Option<String>,
    pub call_number: Option<String>,
    /// The copy's status, e.g. "Available", "On loan - Due: 29 Oct 2026".
    pub status: Option<String>,
    /// A note on the copy that comes before its status, e.g. "Pink body, teal handle".
    pub description: Option<String>,
    /// The status says the copy is on the shelf ("Available …").
    pub available: bool,
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

/// Everything the account screens show — name, loans, both reservation lists — as one value the
/// apps save on the device and show instantly on the next launch while a fresh one loads
/// (`SpydusClient::account_snapshot`; JSON via `encode_account_snapshot` / `decode_account_snapshot`).
#[derive(uniffi::Record, serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct AccountSnapshot {
    /// `app::ACCOUNT_SNAPSHOT_VERSION` when it was made; other versions don't decode.
    pub version: u32,
    pub account_name: Option<String>,
    pub loans: PatronList,
    pub reservations: Reservations,
    /// When it was fetched, Unix epoch milliseconds.
    pub fetched_at_ms: i64,
    /// The library card number from the Personal details page; `None` when that page has none or
    /// couldn't be loaded (the rest of the snapshot is still good).
    #[serde(default)]
    #[uniffi(default = None)]
    pub card_number: Option<String>,
    /// When the card expires, as the Personal details page writes it.
    #[serde(default)]
    #[uniffi(default = None)]
    pub card_expires: Option<String>,
}

/// Links extracted from the authenticated account (BRWENQ) page.
#[derive(uniffi::Record, serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct AccountLinks {
    pub name: Option<String>,
    pub loans_url: Option<String>,
    pub reservations_ready_url: Option<String>,
    pub reservations_pending_url: Option<String>,
    /// The "Personal details" page (`…/BRWENQ/…?FMT=PD`): card number and expiry.
    #[serde(default)]
    #[uniffi(default = None)]
    pub personal_details_url: Option<String>,
}

/// From the account's "Personal details" page (`parse_personal_details`).
#[derive(uniffi::Record, serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct PersonalDetails {
    /// The library card number, e.g. Canning's `CL114…`: the barcode on the card (the sign-in
    /// ID can be a username instead).
    pub card_number: Option<String>,
    /// When the card expires, as the page writes it ("12 Mar 2027").
    pub card_expires: Option<String>,
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

/// Outcome of renewing a loan (`SpydusClient::renew_loan`).
#[derive(uniffi::Record, serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RenewResult {
    /// The loan's due date moved: the library renewed it.
    pub ok: bool,
    /// Why it wasn't renewed: the library's own words when its page gave a reason, else the
    /// core's ("The library didn’t renew this loan."). `None` when `ok`.
    pub message: Option<String>,
    /// The loan's due date after the attempt, as the loans list writes it ("17 Nov 2026"); `None`
    /// when the loan is no longer on the list.
    pub due: Option<String>,
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
