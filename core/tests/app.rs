//! `app` helpers: format classification (calibrated on every format in the goldens and on the
//! codes/labels seen on live libraries), availability labels (on the availability goldens and
//! the live `counts` shapes), due dates, and the home screen's genres and shelves.

mod common;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use librespydus_core::app::{
    ACCOUNT_SNAPSHOT_VERSION, AvailabilitySummary, SimpleDate, availability_summary,
    decode_account_snapshot, encode_account_snapshot, home_genres, home_shelves,
    is_digital_format, parse_due_date,
};
use librespydus_core::types::{
    AccountSnapshot, Availability, PatronItem, PatronList, RecordFormat, Reservations, SearchSort,
};
use librespydus_core::{SpydusError, config, user_agent};
use serde_json::Value;

fn format(code: &str, label: Option<&str>) -> RecordFormat {
    RecordFormat { code: code.into(), label: label.map(Into::into), ..RecordFormat::default() }
}

// ---- is_digital_format ----------------------------------------------------------------------

#[test]
fn live_codes_and_labels_are_classified() {
    // Every (code, label) pair seen on Canning, Cumberland, Esperance, Camden, Hutt City,
    // Porirua, Wellington, Niagara and Brazoria County results pages (2026-10-12).
    let digital = [
        ("EBK", "eBook"),
        ("EAUD", "eAudioBook"),
        ("EAUD", "eAudio"),
        ("EMAG", "eMagazine"),
        ("ECMC", "eComics"),
    ];
    let physical = [
        ("BK", "Book"),
        ("LP", "Large Print"),
        ("TB", "Audiobooks"),
        ("PM", "DVDs and BluRay"),
        ("MU", "Music CDs"),
        ("MU", "Music CDs & Vinyl"),
        ("SE", "Magazines"),
        ("OB", "Objects"),
        ("GR", "Photographs"),
    ];
    for (code, label) in digital {
        assert!(is_digital_format(format(code, Some(label))), "{code} {label}");
    }
    for (code, label) in physical {
        assert!(!is_digital_format(format(code, Some(label))), "{code} {label}");
    }
}

#[test]
fn known_codes_decide_before_the_label() {
    for code in ["EBK", "EAUD", "EVID", "EMAG", "EMUS", "EJNL", "ECMC", "ebk", " eaud "] {
        assert!(is_digital_format(format(code, None)), "{code:?}");
        assert!(is_digital_format(format(code, Some("Book"))), "{code:?} with a physical label");
    }
    for code in ["BK", "DVD", "CD", "AB", "LP", "MAG", "PM", "TB", "MU", "SE", "OB", "GR", "KIT"] {
        assert!(!is_digital_format(format(code, None)), "{code}");
        assert!(!is_digital_format(format(code, Some("eBook"))), "{code} with a digital label");
    }
}

#[test]
fn unknown_codes_fall_back_to_the_label() {
    let digital = [
        "eBook",
        "E-Book",
        "e-audiobook",
        "eAudio Book",
        "eVideo",
        "eNewspaper",
        "eResources",
        "eLearning",
        "Online resource",
        "Streaming video",
        "Downloadable audiobook",
        "Electronic book",
        "Digital magazine",
        "Digital audiobook",
    ];
    for label in digital {
        assert!(is_digital_format(format("XYZ", Some(label))), "{label}");
    }
    let physical = [
        "Book",
        "Board book",
        "Audiobook",
        "Talking book",
        "Magazine",
        "Equipment",
        "Edition",
        "Ereader device kit",
        "Playaway digital audiobook",
        "Audio CD",
        "CD-ROM",
        "Blu-ray",
        "Digital camera",
        "",
    ];
    for label in physical {
        assert!(!is_digital_format(format("XYZ", Some(label))), "{label}");
    }
    assert!(!is_digital_format(format("XYZ", None)));
}

/// Every format object (`{code, label, reserveUrl, availabilityUrl}`) in a JSON value.
fn collect_formats(v: &Value, out: &mut Vec<RecordFormat>) {
    match v {
        Value::Object(map) if map.contains_key("code") && map.contains_key("availabilityUrl") => {
            out.push(serde_json::from_value(v.clone()).expect("a RecordFormat"));
        }
        Value::Object(map) => map.values().for_each(|v| collect_formats(v, out)),
        Value::Array(items) => items.iter().for_each(|v| collect_formats(v, out)),
        _ => {}
    }
}

fn golden_formats(dir: &Path, out: &mut Vec<RecordFormat>) {
    for entry in fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n != "private") {
                golden_formats(&path, out);
            }
        } else if path.extension().is_some_and(|e| e == "json") {
            let value: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
            collect_formats(&value, out);
        }
    }
}

#[test]
fn every_golden_format_is_classified_by_its_code() {
    let mut formats = Vec::new();
    golden_formats(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden"), &mut formats);
    assert!(formats.len() > 50, "found only {} formats", formats.len());
    let mut seen = HashSet::new();
    for f in &formats {
        let expected = matches!(f.code.as_str(), "EBK" | "EAUD" | "EMAG" | "ECMC");
        assert_eq!(is_digital_format(f.clone()), expected, "{} {:?}", f.code, f.label);
        seen.insert(f.code.as_str());
    }
    // The goldens cover both sides.
    for code in ["BK", "EBK", "EAUD", "LP", "MU", "OB", "PM", "TB"] {
        assert!(seen.contains(code), "no {code} format in the goldens");
    }
}

// ---- availability_summary -------------------------------------------------------------------

fn counts(pairs: &[(&str, i64)]) -> Availability {
    let counts: HashMap<String, i64> = pairs.iter().map(|&(k, v)| (k.to_owned(), v)).collect();
    Availability {
        copies: counts.get("copies").copied(),
        reserved: counts.get("reserved").copied(),
        counts,
        ..Availability::default()
    }
}

fn label(pairs: &[(&str, i64)]) -> String {
    availability_summary(counts(pairs)).label
}

#[test]
fn availability_goldens() {
    let reserved: Availability =
        serde_json::from_value(common::golden("availability.json")).unwrap();
    assert_eq!(
        availability_summary(reserved),
        AvailabilitySummary {
            available: None,
            copies: Some(3),
            reserved: Some(1),
            label: "3 copies · 1 reserved".into(),
        }
    );
    let plain: Availability =
        serde_json::from_value(common::golden("availability-midnight.json")).unwrap();
    assert_eq!(availability_summary(plain).label, "2 copies");
}

#[test]
fn live_counts_shapes() {
    // "Available"/"Copies" libraries (Cumberland, Wellington, Niagara, Brazoria County).
    assert_eq!(label(&[("available", 2), ("copies", 3)]), "2 of 3 available");
    assert_eq!(label(&[("available", 3), ("copies", 3)]), "3 of 3 available");
    assert_eq!(label(&[("available", 0), ("copies", 4)]), "0 of 4 available");
    // "Reserved"/"Copies" libraries (Canning, Hutt City).
    assert_eq!(label(&[("copies", 1)]), "1 copy");
    assert_eq!(label(&[("reserved", 2), ("copies", 3)]), "3 copies · 2 reserved");
    assert_eq!(label(&[("reserved", 10), ("copies", 1)]), "1 copy · 10 reserved");
    // Most digital formats, and anything else without counts.
    assert_eq!(label(&[]), "No copies listed");
}

#[test]
fn other_counts_shapes() {
    assert_eq!(label(&[("copies", 0)]), "No copies");
    assert_eq!(label(&[("available", 0), ("copies", 0)]), "No copies");
    assert_eq!(label(&[("available", 0)]), "None available");
    assert_eq!(label(&[("available", 2)]), "2 available");
    assert_eq!(label(&[("reserved", 2)]), "2 reserved");
    assert_eq!(label(&[("reserved", 0), ("copies", 2)]), "2 copies");
    assert_eq!(
        label(&[("available", 1), ("copies", 2), ("reserved", 3)]),
        "1 of 2 available · 3 reserved"
    );
    // Unknown labels are ignored.
    assert_eq!(label(&[("on order", 2), ("copies", 2)]), "2 copies");
}

#[test]
fn summary_numbers_come_from_counts_or_the_typed_fields() {
    let s = availability_summary(counts(&[("available", 1), ("copies", 2), ("reserved", 4)]));
    assert_eq!((s.available, s.copies, s.reserved), (Some(1), Some(2), Some(4)));
    // A hand-built Availability with only the typed fields set.
    let typed = Availability { copies: Some(2), reserved: Some(1), ..Availability::default() };
    assert_eq!(availability_summary(typed).label, "2 copies · 1 reserved");
    // The typed fields win over the map (they are the same value in parsed data).
    let both = Availability {
        copies: Some(5),
        counts: HashMap::from([("copies".to_owned(), 9)]),
        ..Availability::default()
    };
    assert_eq!(availability_summary(both).copies, Some(5));
}

// ---- parse_due_date -------------------------------------------------------------------------

fn date(year: i32, month: u8, day: u8) -> Option<SimpleDate> {
    Some(SimpleDate { year, month, day })
}

#[test]
fn due_date_formats() {
    let oct27 = date(2026, 10, 27);
    for text in [
        "27 Oct 2026",
        "27 October 2026",
        "27/10/2026",
        "2026-10-27",
        "Oct 27, 2026",
        "October 27 2026",
        "27 Oct. 2026",
        "27 OCT 2026",
        "27-10-2026",
        "27.10.2026",
        "  27\u{a0}Oct\t 2026\n",
        "27&nbsp;Oct 2026",
    ] {
        assert_eq!(parse_due_date(text.into()), oct27, "{text:?}");
    }
    assert_eq!(parse_due_date("1 Feb 2027".into()), date(2027, 2, 1));
    assert_eq!(parse_due_date("01/02/2027".into()), date(2027, 2, 1), "day first");
    assert_eq!(parse_due_date("2027-2-1".into()), date(2027, 2, 1));
    assert_eq!(parse_due_date("5 Sept 2026".into()), date(2026, 9, 5));
    assert_eq!(parse_due_date("Sep 5, 2026".into()), date(2026, 9, 5));
    assert_eq!(parse_due_date("31 December 2026".into()), date(2026, 12, 31));
    assert_eq!(parse_due_date("29 Feb 2028".into()), date(2028, 2, 29), "leap year");
    assert_eq!(parse_due_date("29 Feb 2000".into()), date(2000, 2, 29), "leap century");
}

#[test]
fn due_date_rejects_everything_else() {
    for text in [
        "",
        "Overdue",
        "Oct 2026",
        "27 Oct",
        "27 Oct 26",
        "27/10/26",
        "2026/10/27",
        "27/10-2026",
        "13/13/2026",
        "32 Oct 2026",
        "0 Oct 2026",
        "31 Feb 2026",
        "29 Feb 2026",
        "29 Feb 1900",
        "31 Apr 2026",
        "27 Foo 2026",
        "27 Octo 2026",
        "27 Oct 0000",
        "27 Oct 2026 17:00",
        "Tue 27 Oct 2026",
        "2026-10-27T10:00",
        "27th Oct 2026",
    ] {
        assert_eq!(parse_due_date(text.into()), None, "{text:?}");
    }
}

// ---- home screen ------------------------------------------------------------------------------

#[test]
fn genres_have_stable_keys() {
    let genres = home_genres();
    let keys: Vec<&str> = genres.iter().map(|g| g.key.as_str()).collect();
    assert_eq!(
        keys,
        ["biography", "fiction", "science", "history", "fantasy", "crime", "cooking", "children"]
    );
    for g in &genres {
        assert!(!g.label.is_empty(), "{}", g.key);
        assert_eq!(g.query.trim(), g.query, "{}", g.key);
        assert!(!g.query.is_empty(), "{}", g.key);
    }
    let children = genres.iter().find(|g| g.key == "children").unwrap();
    assert_eq!(children.query, "juvenile fiction");
}

#[test]
fn shelves_start_with_new_arrivals() {
    let shelves = home_shelves();
    assert!((2..=3).contains(&shelves.len()), "{} shelves", shelves.len());
    assert_eq!(shelves[0].key, "new_arrivals");
    assert_eq!(shelves[0].title, "New arrivals");
    assert_eq!(shelves[0].sort, SearchSort::Newest);
    let keys: HashSet<&str> = shelves.iter().map(|s| s.key.as_str()).collect();
    assert_eq!(keys.len(), shelves.len(), "unique keys");
    for s in &shelves {
        assert!(!s.title.is_empty() && !s.query.is_empty(), "{}", s.key);
    }
}

#[test]
fn user_agent_is_the_core_user_agent() {
    assert_eq!(user_agent(), config::USER_AGENT);
}

#[test]
fn search_sort_defaults_to_relevance_and_serializes_camel_case() {
    assert_eq!(SearchSort::default(), SearchSort::Relevance);
    assert_eq!(serde_json::to_value(SearchSort::Newest).unwrap(), "newest");
    let shelf = serde_json::to_value(&home_shelves()[0]).unwrap();
    assert_eq!(shelf["sort"], "newest");
}

// ---- account snapshot JSON ----

fn snapshot() -> AccountSnapshot {
    let book = PatronItem { title: Some("A book".into()), ..PatronItem::default() };
    AccountSnapshot {
        version: ACCOUNT_SNAPSHOT_VERSION,
        account_name: Some("Test Borrower".into()),
        loans: PatronList { total: Some(1), items: vec![book] },
        reservations: Reservations::default(),
        fetched_at_ms: 1_791_792_000_000,
    }
}

#[test]
fn account_snapshot_round_trips() {
    let json = encode_account_snapshot(snapshot());
    assert!(json.contains(r#""accountName":"Test Borrower""#), "camelCase JSON: {json}");
    assert_eq!(decode_account_snapshot(json).unwrap(), snapshot());
}

#[test]
fn account_snapshot_rejects_other_versions_and_garbage() {
    let newer = AccountSnapshot { version: ACCOUNT_SNAPSHOT_VERSION + 1, ..snapshot() };
    assert!(matches!(
        decode_account_snapshot(encode_account_snapshot(newer)),
        Err(SpydusError::Parse { .. })
    ));
    assert!(matches!(decode_account_snapshot("{not json".into()), Err(SpydusError::Parse { .. })));
}

