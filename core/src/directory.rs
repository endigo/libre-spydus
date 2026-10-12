//! The public Spydus library directory → the library picker list. Owner: task G3.
//!
//! `GET` [`DIRECTORY_URL`] returns every library on Civica's directory as
//! `{name, id, url, logo, activationCode, location{country,…}, pinningHash, groupCode}`.
//! [`parse_directory`] normalizes each entry into a [`LibraryInfo`] and flags Civica's own
//! test/UAT/training/demo instances, which make up ~40% of the list.
//!
//! Safety (contract §3): the only request here is that public GET — no library site is contacted.

use std::collections::HashSet;
use std::error::Error as _;
use std::sync::LazyLock;
use std::time::Duration;

use regex::Regex;
use reqwest::Url;
use reqwest::header::ACCEPT;
use serde_json::Value;

use crate::config;
use crate::error::SpydusError;
use crate::types::LibraryInfo;

pub const DIRECTORY_URL: &str =
    "https://librarylist.spydus.com/api/1.0/mobile/libraries?groupCode=";

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const TOTAL_TIMEOUT: Duration = Duration::from_secs(60);

/// Words marking a test instance, in a library name or as a host label.
const TEST_WORDS: [&str; 12] = [
    "uat", "test", "tests", "testing", "training", "train", "demo", "sandbox", "staging", "dev",
    "preprod", "qa",
];
/// Host-label endings marking a test instance: `aftrs-uat`, `bananashiretest`,
/// `schoolibrarytests11`, `nationaljcdemo`.
const TEST_LABEL_SUFFIXES: [&str; 5] = ["uat", "test", "tests", "demo", "train"];
/// A test word fused with a region code is a test label too: `testuk`, `demouk`, `demous`.
const TEST_LABEL_PREFIXES: [&str; 4] = ["uat", "test", "demo", "train"];
const REGION_CODES: [&str; 5] = ["uk", "us", "au", "nz", "sg"];
/// Civica's internal cloud label (`vbay-sub-dev.spydus.app`, `vbay-f2945.spydus.app`, …).
const INTERNAL_LABELS: [&str; 1] = ["vbay"];
/// Civica's QA domain.
const QA_DOMAIN: &str = ".spydus.dev";

/// A test word as a whole word in the name, or the vendor's name ("Civica - PlayUK",
/// "Civica - Techops Manual" are Civica's own instances, not libraries).
static TEST_NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"(?i)\b(?:{}|civica)\b", TEST_WORDS.join("|"))).unwrap());
/// `https//host` — a scheme missing its colon (one real entry: `https//singaporelib.com`).
static MISSING_COLON: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^(https?)//").unwrap());

/// Every library in the directory, normalized (see [`parse_directory`]). Test instances are
/// dropped unless `include_test`.
pub async fn list_libraries(include_test: bool) -> Result<Vec<LibraryInfo>, SpydusError> {
    let client = reqwest::Client::builder()
        .user_agent(config::USER_AGENT)
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(TOTAL_TIMEOUT)
        .build()
        .map_err(network)?;
    let res = client
        .get(DIRECTORY_URL)
        .header(ACCEPT, "application/json")
        .send()
        .await
        .map_err(network)?;
    let status = res.status();
    if !status.is_success() {
        return Err(SpydusError::Upstream {
            message: format!("library directory {}", status.as_u16()),
        });
    }
    let json = res.text().await.map_err(network)?;
    parse_directory(&json, include_test)
}

/// The directory JSON (an array of entries) → libraries sorted by name, case-insensitively.
///
/// Entries without a name or an http(s) URL are skipped, as are test instances unless
/// `include_test`; entries with the same origin and name are kept once. Malformed JSON, or
/// JSON that is not an array, is `Parse`.
pub fn parse_directory(json: &str, include_test: bool) -> Result<Vec<LibraryInfo>, SpydusError> {
    let parse_error =
        |detail: String| SpydusError::Parse { message: format!("library directory: {detail}") };
    let entries = match serde_json::from_str(json) {
        Ok(Value::Array(entries)) => entries,
        Ok(_) => return Err(parse_error("expected a JSON array".into())),
        Err(e) => return Err(parse_error(e.to_string())),
    };
    let mut seen = HashSet::new();
    let mut libraries: Vec<LibraryInfo> = entries
        .iter()
        .filter_map(library_info)
        .filter(|lib| include_test || !lib.is_test)
        .filter(|lib| seen.insert((lib.origin.clone(), lib.name.clone())))
        .collect();
    // Exact name, then origin, break ties so the order never depends on the directory's.
    libraries
        .sort_by_cached_key(|lib| (lib.name.to_lowercase(), lib.name.clone(), lib.origin.clone()));
    Ok(libraries)
}

/// One directory entry → a library, or `None` when it has no usable name or URL.
fn library_info(entry: &Value) -> Option<LibraryInfo> {
    let name = text(entry.get("name"))?;
    let (origin, host) = origin_and_host(entry.get("url")?.as_str()?)?;
    let country = country(entry.get("location").and_then(|l| l.get("country")));
    Some(LibraryInfo {
        // The directory always has one; an entry without it is still pickable by origin.
        id: text(entry.get("id")).unwrap_or_else(|| origin.clone()),
        is_test: is_test(&name, &host),
        region: region(&host, country.as_deref()).to_owned(),
        logo_url: text(entry.get("logo")).filter(|logo| http_url(logo).is_some()),
        country,
        name,
        origin,
    })
}

/// A trimmed, non-empty string field.
fn text(value: Option<&Value>) -> Option<String> {
    let s = value?.as_str()?.trim();
    (!s.is_empty()).then(|| s.to_owned())
}

/// An absolute http(s) URL with a host.
fn http_url(raw: &str) -> Option<Url> {
    let url = Url::parse(raw).ok()?;
    (matches!(url.scheme(), "http" | "https") && url.host_str().is_some()).then_some(url)
}

/// `(scheme://host[:port], host)`: path, query and trailing slash dropped, scheme and host
/// lower-cased, the default port omitted, and a missing scheme colon repaired.
fn origin_and_host(raw: &str) -> Option<(String, String)> {
    let raw = raw.trim();
    let url = http_url(&MISSING_COLON.replace(raw, "${1}://"))?;
    let host = url.host_str()?.to_owned();
    Some((url.origin().ascii_serialization(), host))
}

/// The directory's free-text country → a display name.
fn country(value: Option<&Value>) -> Option<String> {
    let raw = text(value)?;
    let name = match raw.to_ascii_uppercase().as_str() {
        "AUSTRALIA" | "AUS" | "AU" | "NSW" | "AUSTRALIAN" | "AUSTRLIA" => "Australia",
        "UNITED KINGDOM" | "UK" | "GB" => "United Kingdom",
        "UNITED STATES" | "USA" | "US" => "United States",
        "NEW ZEALAND" | "NZ" => "New Zealand",
        "SINGAPORE" | "SING" | "SG" => "Singapore",
        _ => return Some(raw),
    };
    Some(name.to_owned())
}

/// The host's country-code TLD (`.co.uk`, `.co.nz`, `.com.sg`, …) wins; else the country.
fn region(host: &str, country: Option<&str>) -> &'static str {
    const BY_TLD: [(&str, &str); 4] = [(".uk", "UK"), (".nz", "NZ"), (".sg", "SG"), (".au", "AU")];
    if let Some((_, region)) = BY_TLD.iter().find(|(tld, _)| host.ends_with(tld)) {
        return region;
    }
    match country {
        Some("Australia") => "AU",
        Some("United Kingdom") => "UK",
        Some("United States") => "US",
        Some("New Zealand") => "NZ",
        Some("Singapore") => "SG",
        _ => "OTHER",
    }
}

fn is_test(name: &str, host: &str) -> bool {
    TEST_NAME.is_match(name) || host.ends_with(QA_DOMAIN) || host_labels(host).any(is_test_label)
}

/// The host's labels before the `spydus.*` suffix, split on `.` and `-`:
/// `sjvls-tp-train.spydus.com` → `sjvls`, `tp`, `train`.
fn host_labels(host: &str) -> impl Iterator<Item = &str> {
    host.split('.').take_while(|label| *label != "spydus").flat_map(|label| label.split('-'))
}

/// Trailing digits are ignored: `train11`, `demo10`, `schoolibrarytests11`.
fn is_test_label(label: &str) -> bool {
    let label = label.trim_end_matches(|c: char| c.is_ascii_digit());
    TEST_WORDS.contains(&label)
        || INTERNAL_LABELS.contains(&label)
        || TEST_LABEL_SUFFIXES.iter().any(|suffix| label.ends_with(suffix))
        || TEST_LABEL_PREFIXES.iter().any(|prefix| {
            label.strip_prefix(prefix).is_some_and(|rest| REGION_CODES.contains(&rest))
        })
}

/// Transport failure, with reqwest's source chain (its `Display` alone omits the cause).
fn network(e: reqwest::Error) -> SpydusError {
    let mut message = e.to_string();
    let mut source = e.source();
    while let Some(cause) = source {
        message.push_str(": ");
        message.push_str(&cause.to_string());
        source = cause.source();
    }
    SpydusError::Network { message }
}
