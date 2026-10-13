//! Pure helpers both native apps share, so iOS and Android show the same thing for the same
//! data: whether a format is digital, a short availability line, loan due dates, the home
//! screen's genre tiles and shelves, how a loan or reservation links to its book, and what to
//! alert about (ready to collect, due-date reminders). No I/O. Every function is UniFFI-exported; tests are in
//! `tests/app.rs`. The calibration behind each rule (surveys of live libraries, 2026-10-12) is in
//! `docs/rust-core-port.md` §11.

use std::sync::LazyLock;

use regex::Regex;

use crate::parse::decode::collapse;
use crate::SpydusError;
use crate::types::{AccountSnapshot, Availability, PatronItem, RecordFormat, SearchSort};

// ---- is_digital_format ----------------------------------------------------------------------

/// Spydus format codes for e-resources: eBook, eAudiobook, eVideo, eMagazine, eMusic, eJournal,
/// eComics. EBK, EAUD, EMAG and ECMC are the ones seen on live libraries.
const DIGITAL_CODES: &[&str] = &["EBK", "EAUD", "EVID", "EMAG", "EMUS", "EJNL", "ECMC"];

/// Physical format codes, never digital whatever the label says: book, large print, talking
/// book, audiobook, DVD/Blu-ray (PM), DVD, CD, music (MU), serial/magazine (SE, MAG), object,
/// photograph (GR), kit.
const PHYSICAL_CODES: &[&str] =
    &["BK", "LP", "TB", "AB", "PM", "DVD", "CD", "MU", "SE", "MAG", "OB", "GR", "KIT"];

/// A label that names a physical carrier (vetoes the digital words below).
static PHYSICAL_LABEL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:playaways?|cds?|cd-roms?|dvds?|blu-?rays?|vinyl|cassettes?|kits?)\b")
        .unwrap()
});

/// A label that names an e-resource: "eBook", "eAudioBook", "eAudio", "eMagazine", "eComics",
/// "E-book", "Online resource", "Streaming video", "Downloadable audio", "Digital magazine"….
static DIGITAL_LABEL: LazyLock<Regex> = LazyLock::new(|| {
    let media = r"books?|audio(?:\s?books?)?|magazines?|zines?|comics?|journals?|videos?|music|newspapers?|resources?";
    Regex::new(&format!(
        r"(?i)\b(?:e-?(?:{media}|learning)|online|streaming|streamed|downloads?|downloadable|electronic|digital\s+(?:{media}))\b"
    ))
    .unwrap()
});

/// Whether `format` is an e-resource the patron uses online (eBook, eAudiobook, eMagazine, …),
/// rather than a physical item collected from a branch. Known codes decide first (the codes in
/// `DIGITAL_CODES` are digital, those in `PHYSICAL_CODES` never are); an unknown code falls back
/// to its label ("eBook", "Online …", "Streaming …", "Downloadable …"), unless the label names a
/// physical carrier (CD, DVD, Playaway, kit, …).
///
/// Digital formats usually have no `reserveUrl` and no branch counts, so the apps show
/// "Available online". Some libraries do offer holds on licensed e-copies (Brazoria County
/// gives its EBK and EAUD formats a `reserveUrl` and `available`/`copies` counts), so check
/// `reserveUrl` rather than assume.
#[uniffi::export]
pub fn is_digital_format(format: RecordFormat) -> bool {
    let code = format.code.trim().to_ascii_uppercase();
    if DIGITAL_CODES.contains(&code.as_str()) {
        return true;
    }
    if PHYSICAL_CODES.contains(&code.as_str()) {
        return false;
    }
    let label = format.label.as_deref().unwrap_or_default();
    !PHYSICAL_LABEL.is_match(label) && DIGITAL_LABEL.is_match(label)
}

// ---- availability_summary ---------------------------------------------------------------------

/// A format's availability, reduced to the numbers the OPAC shows plus a short label for the UI.
#[derive(
    uniffi::Record, serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq, Default,
)]
#[serde(rename_all = "camelCase")]
pub struct AvailabilitySummary {
    /// Copies on the shelf now (`counts["available"]`). Only some libraries show it.
    pub available: Option<i64>,
    /// Item records for this format, whatever their status (`counts["copies"]`).
    pub copies: Option<i64>,
    /// Outstanding reservations on the title (`counts["reserved"]`). Only some libraries show
    /// it, and only when there are any.
    pub reserved: Option<i64>,
    /// "2 of 3 available", "0 of 3 available", "3 copies · 1 reserved", "1 copy", "No copies",
    /// or "No copies listed" when the OPAC shows no counts (as for most digital formats).
    pub label: String,
}

/// `availability` → its counts and a one-line label. The OPAC's availability fragment has
/// `Label: value` pairs, and the keys seen on live libraries are only these (lower-cased):
///
/// - `copies`: item records for the format, whatever their status.
/// - `available`: copies on the shelf now. Shown by libraries such as Cumberland, Wellington,
///   Niagara and Brazoria County, and present even when it's 0.
/// - `reserved`: reservations queued on the title. Shown instead of `available` by libraries
///   such as Canning and Hutt City, and only when there's at least one.
///
/// An `available` of 0 is labelled "0 of N available", not "on loan": the OPAC doesn't say
/// whether the copies are on loan, in transit, on the hold shelf or missing. Without
/// `available`, the label only states what is known ("3 copies · 1 reserved").
#[uniffi::export]
pub fn availability_summary(availability: Availability) -> AvailabilitySummary {
    let count = |key: &str| availability.counts.get(key).copied();
    let available = count("available");
    let copies = availability.copies.or_else(|| count("copies"));
    let reserved = availability.reserved.or_else(|| count("reserved"));
    AvailabilitySummary {
        available,
        copies,
        reserved,
        label: summary_label(available, copies, reserved),
    }
}

fn summary_label(available: Option<i64>, copies: Option<i64>, reserved: Option<i64>) -> String {
    let mut parts = Vec::new();
    match (available, copies) {
        (_, Some(0)) => parts.push("No copies".to_owned()),
        (Some(a), Some(c)) => parts.push(format!("{a} of {c} available")),
        (Some(0), None) => parts.push("None available".to_owned()),
        (Some(a), None) => parts.push(format!("{a} available")),
        (None, Some(1)) => parts.push("1 copy".to_owned()),
        (None, Some(c)) => parts.push(format!("{c} copies")),
        (None, None) => {}
    }
    if let Some(r) = reserved.filter(|&r| r > 0) {
        parts.push(format!("{r} reserved"));
    }
    if parts.is_empty() { "No copies listed".to_owned() } else { parts.join(" · ") }
}

// ---- parse_due_date ---------------------------------------------------------------------------

/// A calendar date without a time zone.
#[derive(
    uniffi::Record, serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default,
)]
#[serde(rename_all = "camelCase")]
pub struct SimpleDate {
    pub year: i32,
    /// 1–12.
    pub month: u8,
    /// 1–31, valid for the month and year.
    pub day: u8,
}

/// "27 Oct 2026", "27 October 2026", "27 Oct. 2026".
static DAY_MONTH_YEAR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([0-9]{1,2}) ([A-Za-z]+)\.?,? ([0-9]{4})$").unwrap());
/// "Oct 27, 2026", "October 27 2026".
static MONTH_DAY_YEAR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([A-Za-z]+)\.? ([0-9]{1,2}),? ([0-9]{4})$").unwrap());
/// "27/10/2026", "27-10-2026", "27.10.2026" — day first, one separator throughout.
static NUMERIC_DMY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([0-9]{1,2})([/.-])([0-9]{1,2})([/.-])([0-9]{4})$").unwrap());
/// "2026-10-27" (ISO 8601 calendar date).
static ISO: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([0-9]{4})-([0-9]{1,2})-([0-9]{1,2})$").unwrap());

const MONTHS: [&str; 12] = [
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];

/// A loan's due date (`PatronItem.fields["Due"]`, e.g. "27 Oct 2026") → its date. Accepts
/// "27 Oct 2026", "27 October 2026", day-first "27/10/2026" (also with `-` or `.`), ISO
/// "2026-10-27" and "Oct 27, 2026"; month names in any case, as full names, three-letter
/// abbreviations or "Sept". Whitespace is collapsed and entities decoded first. Anything else,
/// or a date that doesn't exist (31 Feb), is `None`.
#[uniffi::export]
pub fn parse_due_date(text: String) -> Option<SimpleDate> {
    let text = collapse(&text);
    let num = |s: &str| s.parse::<u32>().ok();
    let (year, month, day) = if let Some(c) = DAY_MONTH_YEAR.captures(&text) {
        (num(&c[3])?, month_number(&c[2])?, num(&c[1])?)
    } else if let Some(c) = MONTH_DAY_YEAR.captures(&text) {
        (num(&c[3])?, month_number(&c[1])?, num(&c[2])?)
    } else if let Some(c) = NUMERIC_DMY.captures(&text) {
        if c[2] != c[4] {
            return None;
        }
        (num(&c[5])?, num(&c[3])?, num(&c[1])?)
    } else {
        let c = ISO.captures(&text)?;
        (num(&c[1])?, num(&c[2])?, num(&c[3])?)
    };
    if year == 0 || !(1..=12).contains(&month) || day == 0 || day > days_in_month(year, month) {
        return None;
    }
    Some(SimpleDate { year: i32::try_from(year).ok()?, month: month as u8, day: day as u8 })
}

/// "oct" / "October" / "Sept" → 10 / 10 / 9.
fn month_number(name: &str) -> Option<u32> {
    let name = name.to_ascii_lowercase();
    let i = MONTHS
        .iter()
        .position(|m| *m == name || (name.len() == 3 && m.starts_with(&name)))
        .or_else(|| (name == "sept").then_some(8))?;
    u32::try_from(i + 1).ok()
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        2 if year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400)) => {
            29
        }
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

// ---- home screen ------------------------------------------------------------------------------

/// A "Popular genres" tile on the home screen. Tapping it runs a relevance `search(query)`.
#[derive(
    uniffi::Record, serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq, Default,
)]
#[serde(rename_all = "camelCase")]
pub struct Genre {
    /// Stable id the apps map to an icon and colour: "biography", "fiction", "science",
    /// "history", "fantasy", "crime", "cooking", "children".
    pub key: String,
    /// Display name, e.g. "Fiction".
    pub label: String,
    /// Keyword query for `search`.
    pub query: String,
}

/// A horizontal cover shelf on the home screen: `search_sorted(query, sort)`'s first page.
#[derive(
    uniffi::Record, serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq, Default,
)]
#[serde(rename_all = "camelCase")]
pub struct Shelf {
    /// Stable id: "new_arrivals", "new_for_kids", "new_biographies".
    pub key: String,
    /// Section title, e.g. "New arrivals".
    pub title: String,
    /// Keyword query for `search_sorted`.
    pub query: String,
    pub sort: SearchSort,
}

/// `(key, label, query)`. Each query returned thousands of works, with covers on 90–100% of the
/// first page, on Canning, Cumberland, Hutt City and Brazoria County (relevance order).
/// "children" searches the subject subdivision "juvenile fiction"; the keyword "children" alone
/// also finds books about children.
const GENRES: [(&str, &str, &str); 8] = [
    ("biography", "Biography", "biography"),
    ("fiction", "Fiction", "fiction"),
    ("science", "Science", "science"),
    ("history", "History", "history"),
    ("fantasy", "Fantasy", "fantasy"),
    ("crime", "Crime", "crime"),
    ("cooking", "Cooking", "cooking"),
    ("children", "Children", "juvenile fiction"),
];

/// `(key, title, query, sort)`. The OPAC has no "everything" keyword (`*` is refused), and very
/// broad words such as "the" take over 60 s on large catalogues, so "New arrivals" uses
/// "fiction": 74k–279k works per library, newest first, with covers.
const SHELVES: [(&str, &str, &str, SearchSort); 3] = [
    ("new_arrivals", "New arrivals", "fiction", SearchSort::Newest),
    ("new_for_kids", "New for kids", "juvenile fiction", SearchSort::Newest),
    ("new_biographies", "New biographies", "biography", SearchSort::Newest),
];

/// The home screen's genre tiles, in display order. Keys are stable; labels and queries may
/// change between core versions.
#[uniffi::export]
pub fn home_genres() -> Vec<Genre> {
    GENRES
        .iter()
        .map(|&(key, label, query)| Genre {
            key: key.into(),
            label: label.into(),
            query: query.into(),
        })
        .collect()
}

/// The home screen's cover shelves, in display order. The first is always "New arrivals"
/// (key `new_arrivals`, newest first).
#[uniffi::export]
pub fn home_shelves() -> Vec<Shelf> {
    SHELVES
        .iter()
        .map(|&(key, title, query, sort)| Shelf {
            key: key.into(),
            title: title.into(),
            query: query.into(),
            sort,
        })
        .collect()
}

// ---- account snapshot JSON ------------------------------------------------------------------

/// The `AccountSnapshot` JSON format. Bump it when the record (or anything inside it) changes
/// shape, so a snapshot saved by an older build is refetched instead of misread.
pub const ACCOUNT_SNAPSHOT_VERSION: u32 = 1;

/// The JSON the apps save on the device (camelCase, like every other record).
#[uniffi::export]
pub fn encode_account_snapshot(snapshot: AccountSnapshot) -> String {
    serde_json::to_string(&snapshot).expect("AccountSnapshot always serializes")
}

/// A saved snapshot back. `Parse` when it is malformed or was written with another
/// `ACCOUNT_SNAPSHOT_VERSION` — the app then drops it and waits for a fresh one.
#[uniffi::export]
pub fn decode_account_snapshot(json: String) -> Result<AccountSnapshot, SpydusError> {
    let snapshot: AccountSnapshot = serde_json::from_str(&json)
        .map_err(|e| SpydusError::Parse { message: format!("account snapshot: {e}") })?;
    if snapshot.version != ACCOUNT_SNAPSHOT_VERSION {
        let message =
            format!("account snapshot version {} (expected {ACCOUNT_SNAPSHOT_VERSION})", snapshot.version);
        return Err(SpydusError::Parse { message });
    }
    Ok(snapshot)
}

// ---- loans and reservations on the book page -------------------------------------------------

/// A URL that loads `item`'s bib record with `SpydusClient::record`, without a session.
///
/// Loan and reservation rows link to `…/BIBENQ/{setId}?SETLVL=&QRY=IRN({bibIrn})&NAVLVL=SET`,
/// and that set lives in the patron's OPAC session: fetched without it, the OPAC answers "Session
/// must be logged in" and the record comes back empty. `…/BIBENQ?SETLVL=&IRN={bibIrn}` loads the
/// full bib record anonymously (Canning, Hutt City and Geelong, 2026-10-12). `bibUrl` as it is
/// when there's no numeric bib IRN or the link isn't a BIBENQ one; `None` without a `bibUrl`.
#[uniffi::export]
pub fn patron_item_record_url(item: PatronItem) -> Option<String> {
    let bib_url = item.bib_url.filter(|url| !url.is_empty())?;
    let irn = item.bib_irn.filter(|irn| !irn.is_empty() && irn.bytes().all(|b| b.is_ascii_digit()));
    // ASCII upper-casing keeps byte offsets, so `at` indexes `bib_url` too.
    let at = bib_url.to_ascii_uppercase().find("/BIBENQ");
    match (irn, at) {
        (Some(irn), Some(at)) => Some(format!("{}?SETLVL=&IRN={irn}", &bib_url[..at + "/BIBENQ".len()])),
        _ => Some(bib_url),
    }
}

/// How the patron already has a title (`AccountHolding.kind`).
#[derive(uniffi::Enum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoldingKind {
    Loan,
    /// Waiting at the pickup branch (`reservations.ready`).
    ReservationReady,
    /// Queued or in transit (`reservations.pending`).
    ReservationPending,
}

/// A loan or reservation of the title a book page shows.
#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct AccountHolding {
    pub kind: HoldingKind,
    /// The row as Your Books shows it (`fields` has Due, or Status/Rank/Placed).
    pub item: PatronItem,
}

/// The loan or reservation in `snapshot` for any of `bib_irns`, so a book page can say "On loan"
/// or "Reserved" instead of offering Reserve again. Pass the IRNs the page knows for the format
/// it shows: `Availability.bibRecordIrn`, else `RecordDetail.bibIrn` and the opened item's
/// `PatronItem.bibIrn`. A loan wins over a ready reservation, which wins over a pending one.
/// Bib IRNs only: a loan of the book doesn't match its large-print edition, a separate bib.
#[uniffi::export]
pub fn account_holding(snapshot: AccountSnapshot, bib_irns: Vec<String>) -> Option<AccountHolding> {
    let wanted: Vec<&str> = bib_irns.iter().map(|irn| irn.trim()).filter(|irn| !irn.is_empty()).collect();
    let matches = |item: &PatronItem| item.bib_irn.as_deref().is_some_and(|irn| wanted.contains(&irn.trim()));
    let AccountSnapshot { loans, reservations, .. } = snapshot;
    [
        (HoldingKind::Loan, loans.items),
        (HoldingKind::ReservationReady, reservations.ready.items),
        (HoldingKind::ReservationPending, reservations.pending.items),
    ]
    .into_iter()
    .find_map(|(kind, items)| items.into_iter().find(|item| matches(item)).map(|item| AccountHolding { kind, item }))
}

// ---- alerts: ready to collect, due-date reminders ---------------------------------------------

/// What to alert about from a fresh account snapshot (`ready_alerts`).
#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct ReadyAlerts {
    /// Ready-to-collect reservations not alerted before, in list order: one notification each.
    pub new_items: Vec<PatronItem>,
    /// Save these as the alerted keys for next time. They are the keys of every reservation
    /// ready now, replacing the saved set, so one that stops being ready (collected or expired)
    /// and later becomes ready again is alerted again.
    pub alerted_keys: Vec<String>,
}

/// A loan or reservation's identity across refreshes: its IRN (the SVL value), else `bib:` and
/// the bib IRN, else `title:` and the title, else empty.
#[uniffi::export]
pub fn patron_item_key(item: &PatronItem) -> String {
    let some = |s: &Option<String>| s.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned);
    some(&item.irn)
        .or_else(|| some(&item.bib_irn).map(|irn| format!("bib:{irn}")))
        .or_else(|| some(&item.title).map(|title| format!("title:{title}")))
        .unwrap_or_default()
}

/// The reservations in `snapshot.reservations.ready` to alert about: those whose
/// `patron_item_key` isn't in `alerted_keys` (what the previous call returned, kept per library).
/// When the patron turns the alerts on, call it once with no keys and save only `alerted_keys`,
/// so what was already waiting isn't announced as new.
#[uniffi::export]
pub fn ready_alerts(snapshot: AccountSnapshot, alerted_keys: Vec<String>) -> ReadyAlerts {
    let mut new_items = Vec::new();
    let mut keys = Vec::new();
    for item in snapshot.reservations.ready.items {
        let key = patron_item_key(&item);
        if key.is_empty() || keys.contains(&key) {
            continue;
        }
        if !alerted_keys.contains(&key) {
            new_items.push(item);
        }
        keys.push(key);
    }
    ReadyAlerts { new_items, alerted_keys: keys }
}

/// One due-date reminder: every loan due on `due`, announced on `remind_on`.
#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct DueReminder {
    /// `due:YYYY-MM-DD`: stable while the loans keep that due date (a notification identifier).
    pub key: String,
    pub due: SimpleDate,
    /// `days_before` days before `due`, or `today` when that has passed. Never after `due`.
    pub remind_on: SimpleDate,
    /// The loans due that day, in list order.
    pub items: Vec<PatronItem>,
}

/// One reminder per due date for the loans in `snapshot`, sorted by date: due on or after
/// `today` (the device's local date), reminded `days_before` days ahead. Overdue loans and due
/// dates `parse_due_date` can't read are left out. The apps show each at the morning alert time
/// on `remind_on`, and recompute them after every refresh: a renewal moves the due date.
#[uniffi::export]
pub fn due_reminders(snapshot: AccountSnapshot, today: SimpleDate, days_before: u32) -> Vec<DueReminder> {
    let today_n = day_number(today);
    let mut reminders: Vec<DueReminder> = Vec::new();
    for item in snapshot.loans.items {
        let Some(due) = item.fields.get("Due").and_then(|text| parse_due_date(text.clone())) else {
            continue;
        };
        let due_n = day_number(due);
        if due_n < today_n {
            continue;
        }
        match reminders.iter_mut().find(|r| r.due == due) {
            Some(reminder) => reminder.items.push(item),
            None => reminders.push(DueReminder {
                key: format!("due:{:04}-{:02}-{:02}", due.year, due.month, due.day),
                due,
                remind_on: date_of_day_number((due_n - i64::from(days_before)).max(today_n)),
                items: vec![item],
            }),
        }
    }
    reminders.sort_by_key(|r| day_number(r.due));
    reminders
}

/// Days since 1970-01-01 (proleptic Gregorian; Howard Hinnant's `days_from_civil`).
fn day_number(date: SimpleDate) -> i64 {
    let (m, d) = (i64::from(date.month), i64::from(date.day));
    let y = i64::from(date.year) - i64::from(m <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The inverse of `day_number` (`civil_from_days`).
fn date_of_day_number(n: i64) -> SimpleDate {
    let z = n + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    SimpleDate {
        year: i32::try_from(year).unwrap_or(i32::MAX),
        month: u8::try_from(month).unwrap_or(1),
        day: u8::try_from(day).unwrap_or(1),
    }
}
