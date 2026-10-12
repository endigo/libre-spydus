//! directory.rs — offline over the real public directory (`fixtures/directory.json`, captured from
//! `DIRECTORY_URL`) plus synthetic entries for the edge cases. The one live test is `#[ignore]`d
//! and is the public directory GET only (contract §3).

mod common;

use std::collections::BTreeMap;

use librespydus_core::directory::{DIRECTORY_URL, list_libraries, parse_directory};
use librespydus_core::{LibraryInfo, SpydusError};
use serde_json::{Value, json};

fn fixture(include_test: bool) -> Vec<LibraryInfo> {
    parse_directory(&common::fixture("directory.json"), include_test).expect("fixture parses")
}

fn by_origin<'a>(libraries: &'a [LibraryInfo], origin: &str) -> &'a LibraryInfo {
    libraries
        .iter()
        .find(|lib| lib.origin == origin)
        .unwrap_or_else(|| panic!("no library at {origin}"))
}

fn region_counts(libraries: &[LibraryInfo]) -> BTreeMap<&str, usize> {
    let mut counts = BTreeMap::new();
    for lib in libraries {
        *counts.entry(lib.region.as_str()).or_default() += 1;
    }
    counts
}

/// A directory entry shaped like the real ones; `fields` overrides / adds keys.
fn entry(name: &str, url: &str, fields: Value) -> Value {
    let mut entry = json!({
        "name": name,
        "id": format!("id-{name}"),
        "url": url,
        "logo": null,
        "activationCode": null,
        "location": { "country": "Australia", "state": "WA", "latitude": null, "longitude": null },
        "pinningHash": null,
        "groupCode": null,
    });
    if let (Value::Object(entry), Value::Object(fields)) = (&mut entry, fields) {
        entry.extend(fields);
    }
    entry
}

fn parse(entries: &[Value]) -> Vec<LibraryInfo> {
    parse_directory(&Value::Array(entries.to_vec()).to_string(), true).expect("parses")
}

/// The single library parsed from one entry (`None` when the entry is skipped).
fn parse_one(entry: Value) -> Option<LibraryInfo> {
    let mut libraries = parse(&[entry]);
    assert!(libraries.len() <= 1);
    libraries.pop()
}

fn origin_of(url: &str) -> Option<String> {
    parse_one(entry("Library", url, json!({}))).map(|lib| lib.origin)
}

#[test]
fn fixture_counts() {
    let all = fixture(true);
    let production = fixture(false);
    let tests = all.iter().filter(|lib| lib.is_test).count();
    assert_eq!(all.len(), 351, "every directory entry is usable");
    assert_eq!(production.len(), 197);
    assert_eq!(tests, 154);
    assert!(production.iter().all(|lib| !lib.is_test));
    let expected = all.iter().filter(|lib| !lib.is_test).cloned().collect::<Vec<_>>();
    assert_eq!(production, expected, "include_test=false is the production subset, same order");

    let expected =
        BTreeMap::from([("AU", 95), ("NZ", 10), ("OTHER", 1), ("SG", 3), ("UK", 84), ("US", 4)]);
    assert_eq!(region_counts(&production), expected);
    let expected = BTreeMap::from([
        ("AU", 183),
        ("NZ", 14),
        ("OTHER", 2),
        ("SG", 13),
        ("UK", 108),
        ("US", 31),
    ]);
    assert_eq!(region_counts(&all), expected);
}

#[test]
fn canning() {
    let canning = by_origin(&fixture(false), "https://canning.spydus.com").clone();
    assert_eq!(
        canning,
        LibraryInfo {
            id: "24fdee8d-ae0f-41b3-b97e-c96059bd06f4".into(),
            name: "City of Canning".into(),
            origin: "https://canning.spydus.com".into(),
            logo_url: Some(
                "https://canning.spydus.com/api/maintenance/1.0/imagebrowser/image?blobName=d226b0c4-06d6-44b7-ba98-e42f06b9fd15.png"
                    .into()
            ),
            country: Some("Australia".into()),
            region: "AU".into(),
            is_test: false,
        }
    );
    assert!(by_origin(&fixture(true), "https://canning-uat.spydus.com").is_test);
}

#[test]
fn repairs_the_missing_scheme_colon() {
    // The real entry's url is `https//singaporelib.com`.
    let production = fixture(false);
    let moe = by_origin(&production, "https://singaporelib.com");
    assert_eq!(moe.name, "MOE");
    assert_eq!(moe.country.as_deref(), Some("Singapore"));
    assert_eq!(moe.region, "SG");
    assert!(!moe.is_test);
}

#[test]
fn flags_known_test_instances() {
    let all = fixture(true);
    let production = fixture(false);
    for origin in [
        "https://aftrs-uat.spydus.com",
        "https://testuk.spydus.co.uk",
        "https://train11.spydus.com",
        "https://schoolibrarytests11.spydus.com.sg",
        "https://sub-vbay.qa.spydus.dev",
        // Flagged by host or vendor name alone — the name has no test word.
        "https://demouk.spydus.co.uk",
        "https://demous.spydus.com",
        "https://playuk.spydus.co.uk",
        "https://techops-manual.spydus.app",
        "https://vbay-f2945.spydus.app",
        "https://nationaljcdemo.spydus.com.sg",
    ] {
        assert!(by_origin(&all, origin).is_test, "{origin} should be a test instance");
        assert!(production.iter().all(|lib| lib.origin != origin), "{origin} is production");
    }
    let qa = all.iter().filter(|lib| lib.origin.ends_with(".qa.spydus.dev")).collect::<Vec<_>>();
    assert!(!qa.is_empty() && qa.iter().all(|lib| lib.is_test));
    // Production libraries that merely look close to a test word.
    for origin in [
        "https://aftrs.spydus.com",
        "https://jaslibrary.spydus.com.sg",
        "https://clarence.spydus.com",
        "https://liveborders.spydus.co.uk",
    ] {
        assert!(!by_origin(&all, origin).is_test, "{origin} should be production");
    }
}

#[test]
fn test_detection_rules() {
    let is_test = |name: &str, url: &str| parse_one(entry(name, url, json!({}))).unwrap().is_test;
    // Name: whole words only, any case.
    assert!(is_test("Library STAGING", "https://a.spydus.com"));
    assert!(is_test("TEST-Admiralty Primary", "https://a.spydus.com"));
    assert!(is_test("QA-Cloud", "https://a.spydus.com"));
    assert!(is_test("Civica - PlayUK", "https://a.spydus.com"));
    assert!(!is_test("Contested Lands Library", "https://a.spydus.com"));
    assert!(!is_test("Devonport Library", "https://a.spydus.com"));
    // Host labels before the spydus.* suffix.
    assert!(is_test("Library", "https://lib-preprod.spydus.com"));
    assert!(is_test("Library", "https://sandbox.spydus.com"));
    assert!(is_test("Library", "https://demo10.spydus.com"));
    assert!(is_test("Library", "https://libuat.spydus.co.nz"));
    assert!(is_test("Library", "https://trainus.spydus.com"));
    assert!(is_test("Library", "https://anything.spydus.dev"));
    assert!(!is_test("Library", "https://devonport.spydus.com"));
    assert!(!is_test("Library", "https://testvalley.spydus.co.uk"));
    assert!(!is_test("Library", "https://lib.spydus.test.example"), "labels after spydus ignored");
}

#[test]
fn origin_normalization() {
    let origin = |url: &str| origin_of(url);
    assert_eq!(origin("https://lib.spydus.com").as_deref(), Some("https://lib.spydus.com"));
    assert_eq!(origin("https://lib.spydus.com/").as_deref(), Some("https://lib.spydus.com"));
    assert_eq!(
        origin("https://schoolibrarytest.spydus.com.sg/admiraltypri/").as_deref(),
        Some("https://schoolibrarytest.spydus.com.sg"),
        "path dropped"
    );
    assert_eq!(
        origin(
            "https://murray-uat.spydus.com/api/maintenance/1.0/imagebrowser/image?blobName=x.png#f"
        )
        .as_deref(),
        Some("https://murray-uat.spydus.com"),
        "path, query and fragment dropped"
    );
    assert_eq!(
        origin("HTTPS://Canning.SPYDUS.com/").as_deref(),
        Some("https://canning.spydus.com")
    );
    assert_eq!(origin("  https://lib.spydus.com \n").as_deref(), Some("https://lib.spydus.com"));
    assert_eq!(origin("https//singaporelib.com").as_deref(), Some("https://singaporelib.com"));
    assert_eq!(origin("HTTP//lib.example/x").as_deref(), Some("http://lib.example"));
    assert_eq!(origin("http://lib.example:8080/x").as_deref(), Some("http://lib.example:8080"));
    assert_eq!(origin("https://lib.example:443/").as_deref(), Some("https://lib.example"));
    for garbage in [
        "",
        "   ",
        "not a url",
        "lib.spydus.com",
        "//lib.spydus.com",
        "https://",
        "https//",
        "ftp://lib.spydus.com",
        "javascript:alert(1)",
        "https://exa mple.com",
    ] {
        assert_eq!(origin(garbage), None, "{garbage:?} is not an http(s) origin");
    }
}

#[test]
fn country_and_region() {
    let lib = |url: &str, country: Value| {
        let lib = parse_one(entry("Library", url, json!({ "location": { "country": country } })))
            .unwrap();
        (lib.country, lib.region)
    };
    let some = |s: &str| Some(s.to_owned());
    let au = "https://a.spydus.com";
    for raw in ["AUS", "AUSTRALIA", "Australia", "NSW", " aus "] {
        assert_eq!(lib(au, json!(raw)), (some("Australia"), "AU".into()), "{raw}");
    }
    assert_eq!(lib(au, json!("UK")), (some("United Kingdom"), "UK".into()));
    assert_eq!(lib(au, json!("USA")), (some("United States"), "US".into()));
    assert_eq!(lib(au, json!("US")), (some("United States"), "US".into()));
    assert_eq!(lib(au, json!("NZ")), (some("New Zealand"), "NZ".into()));
    for raw in ["SING", "SINGAPORE", "Singapore"] {
        assert_eq!(lib(au, json!(raw)), (some("Singapore"), "SG".into()), "{raw}");
    }
    // Unknown → the trimmed raw value, region OTHER; blank / missing / non-string → None.
    assert_eq!(lib(au, json!(" Canada ")), (some("Canada"), "OTHER".into()));
    assert_eq!(lib(au, json!("")), (None, "OTHER".into()));
    assert_eq!(lib(au, Value::Null), (None, "OTHER".into()));
    assert_eq!(lib(au, json!(61)), (None, "OTHER".into()));
    // The host's country TLD wins over the country.
    assert_eq!(lib("https://x.spydus.co.uk", json!("AUS")), (some("Australia"), "UK".into()));
    assert_eq!(lib("https://x.spydus.co.nz", Value::Null), (None, "NZ".into()));
    assert_eq!(lib("https://x.spydus.com.sg", json!("SG")), (some("Singapore"), "SG".into()));
    assert_eq!(lib("https://x.spydus.com.sg", json!("USA")), (some("United States"), "SG".into()));
}

#[test]
fn logo_url_must_be_http() {
    let logo = |value: Value| {
        parse_one(entry("Library", "https://a.spydus.com", json!({ "logo": value })))
            .unwrap()
            .logo_url
    };
    assert_eq!(
        logo(json!("https://a.spydus.com/logo.png")).as_deref(),
        Some("https://a.spydus.com/logo.png")
    );
    assert_eq!(logo(json!(" http://a.example/l.jpg ")).as_deref(), Some("http://a.example/l.jpg"));
    assert_eq!(logo(json!("/api/maintenance/1.0/imagebrowser/image?blobName=x.png")), None);
    assert_eq!(logo(json!("")), None);
    assert_eq!(logo(json!("data:image/png;base64,AAAA")), None);
    assert_eq!(logo(Value::Null), None);
    assert_eq!(logo(json!(7)), None);
}

#[test]
fn tolerates_missing_and_null_fields() {
    let minimal =
        parse_one(json!({ "name": " Minimal ", "url": "https://min.spydus.com/" })).unwrap();
    assert_eq!(
        minimal,
        LibraryInfo {
            id: "https://min.spydus.com".into(), // no id → the origin
            name: "Minimal".into(),
            origin: "https://min.spydus.com".into(),
            logo_url: None,
            country: None,
            region: "OTHER".into(),
            is_test: false,
        }
    );
    let nulls = json!({ "name": "Nulls", "id": null, "url": "https://n.spydus.co.uk", "logo": null,
        "activationCode": null, "location": null, "pinningHash": null, "groupCode": null });
    let nulls = parse_one(nulls).unwrap();
    assert_eq!((nulls.id.as_str(), nulls.region.as_str()), ("https://n.spydus.co.uk", "UK"));

    // No usable name or url → skipped; non-object entries → skipped.
    let skipped = [
        json!({ "url": "https://a.spydus.com" }),
        json!({ "name": "   ", "url": "https://a.spydus.com" }),
        json!({ "name": null, "url": "https://a.spydus.com" }),
        json!({ "name": "No URL" }),
        json!({ "name": "Null URL", "url": null }),
        json!({ "name": "Number URL", "url": 42 }),
        json!({ "name": "Bad URL", "url": "garbage" }),
        json!("https://a.spydus.com"),
        json!(null),
        json!([]),
    ];
    assert!(parse(&skipped).is_empty());
}

#[test]
fn dedupes_identical_origin_and_name() {
    let libraries = parse(&[
        entry("Alpha", "https://alpha.spydus.com", json!({ "id": "first" })),
        entry("Alpha", "https://ALPHA.spydus.com/", json!({ "id": "second" })), // same origin
        entry("Alpha", "https://alpha2.spydus.com", json!({})),                 // other origin
        entry("Alpha Library", "https://alpha.spydus.com", json!({})),          // other name
    ]);
    let keys = libraries.iter().map(|l| (l.name.as_str(), l.origin.as_str(), l.id.as_str()));
    assert_eq!(
        keys.collect::<Vec<_>>(),
        [
            ("Alpha", "https://alpha.spydus.com", "first"),
            ("Alpha", "https://alpha2.spydus.com", "id-Alpha"),
            ("Alpha Library", "https://alpha.spydus.com", "id-Alpha Library"),
        ]
    );
}

#[test]
fn sorted_by_name_case_insensitively() {
    let names = parse(&[
        entry("banana", "https://b.spydus.com", json!({})),
        entry("Cherry", "https://c.spydus.com", json!({})),
        entry("apple", "https://a1.spydus.com", json!({})),
        entry("Apple", "https://a2.spydus.com", json!({})),
        entry("  Avocado ", "https://av.spydus.com", json!({})),
    ])
    .into_iter()
    .map(|lib| lib.name)
    .collect::<Vec<_>>();
    assert_eq!(names, ["Apple", "apple", "Avocado", "banana", "Cherry"]);

    let fixture = fixture(true);
    let lowered = fixture.iter().map(|lib| lib.name.to_lowercase()).collect::<Vec<_>>();
    assert!(lowered.is_sorted(), "fixture output is sorted by name");
    assert_eq!(fixture.first().map(|l| l.name.as_str()), Some("Aberdeen City Libraries"));
}

#[test]
fn test_entries_are_dropped_unless_requested() {
    let entries = [
        entry("Real Library", "https://real.spydus.com", json!({})),
        entry("UAT - Real Library", "https://real-uat.spydus.com", json!({})),
    ];
    let json = Value::Array(entries.to_vec()).to_string();
    let production = parse_directory(&json, false).unwrap();
    assert_eq!(production.iter().map(|l| l.name.as_str()).collect::<Vec<_>>(), ["Real Library"]);
    assert_eq!(parse_directory(&json, true).unwrap().len(), 2);
}

#[test]
fn malformed_json_is_a_parse_error() {
    for json in ["", "not json", "[{\"name\": \"x\",", "{\"libraries\": []}", "null", "\"[]\""] {
        assert!(
            matches!(parse_directory(json, true), Err(SpydusError::Parse { .. })),
            "{json:?} should be a parse error"
        );
    }
    assert!(parse_directory("[]", false).unwrap().is_empty());
}

#[test]
fn serializes_camel_case() {
    let canning = by_origin(&fixture(false), "https://canning.spydus.com").clone();
    let value = serde_json::to_value(&canning).unwrap();
    let mut keys = value.as_object().unwrap().keys().cloned().collect::<Vec<_>>();
    keys.sort();
    assert_eq!(keys, ["country", "id", "isTest", "logoUrl", "name", "origin", "region"]);
    assert_eq!(serde_json::from_value::<LibraryInfo>(value).unwrap(), canning);
}

/// Live: the public directory GET (contract §3 — no library site is contacted).
#[tokio::test]
#[ignore = "network: GET librarylist.spydus.com"]
async fn live_list_libraries() {
    assert!(DIRECTORY_URL.starts_with("https://librarylist.spydus.com/"));
    let libraries = list_libraries(false).await.expect("directory GET");
    assert!(libraries.len() > 100, "only {} production libraries", libraries.len());
    assert!(libraries.iter().all(|lib| !lib.is_test));
    let canning = by_origin(&libraries, "https://canning.spydus.com");
    assert_eq!((canning.name.as_str(), canning.region.as_str()), ("City of Canning", "AU"));
}
