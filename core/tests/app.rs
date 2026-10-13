//! `app` helpers: format classification (calibrated on every format in the goldens and on the
//! codes/labels seen on live libraries), availability labels (on the availability goldens and
//! the live `counts` shapes), due dates, the home screen's genres and shelves, loans and
//! reservations on the book page, and alerts (ready to collect, due-date reminders).

mod common;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use librespydus_core::app::{
    ACCOUNT_SNAPSHOT_VERSION, AccountHolding, AvailabilitySummary, HoldingKind, SimpleDate,
    account_holding, availability_summary, decode_account_snapshot, due_reminders,
    encode_account_snapshot, home_genres, home_shelves, is_digital_format, parse_due_date,
    patron_item_key, patron_item_record_url, ready_alerts,
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
        card_number: Some("XX123A4567890B".into()),
        card_expires: Some("12 Mar 2027".into()),
    }
}

#[test]
fn account_snapshot_round_trips() {
    let json = encode_account_snapshot(snapshot());
    assert!(json.contains(r#""accountName":"Test Borrower""#), "camelCase JSON: {json}");
    assert_eq!(decode_account_snapshot(json).unwrap(), snapshot());
}

#[test]
fn a_snapshot_saved_before_card_numbers_still_decodes() {
    let mut json: Value = serde_json::from_str(&encode_account_snapshot(snapshot())).unwrap();
    let saved = json.as_object_mut().unwrap();
    saved.remove("cardNumber");
    saved.remove("cardExpires");
    let decoded = decode_account_snapshot(json.to_string()).unwrap();
    assert_eq!((decoded.card_number, decoded.card_expires), (None, None));
    assert_eq!(decoded.loans, snapshot().loans);
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

// ---- loans and reservations on the book page ----

const CANNING_BIBENQ: &str = "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ";

fn patron_item(bib_irn: Option<&str>, bib_url: Option<&str>) -> PatronItem {
    PatronItem { bib_irn: bib_irn.map(Into::into), bib_url: bib_url.map(Into::into), ..PatronItem::default() }
}

#[test]
fn patron_item_record_url_drops_the_session_set() {
    // The shape every loan and reservation row in the private goldens has.
    let row = format!("{CANNING_BIBENQ}/78577835?SETLVL=&QRY=IRN(8753080)&NAVLVL=SET");
    assert_eq!(
        patron_item_record_url(patron_item(Some("8753080"), Some(&row))),
        Some(format!("{CANNING_BIBENQ}?SETLVL=&IRN=8753080"))
    );
    // Lower-case path, as some templates write it.
    let lower = "https://hutt.spydus.co.nz/cgi-bin/spydus.exe/ENQ/OPAC/bibenq/1?QRY=IRN(1965306)";
    assert_eq!(
        patron_item_record_url(patron_item(Some("1965306"), Some(lower))).as_deref(),
        Some("https://hutt.spydus.co.nz/cgi-bin/spydus.exe/ENQ/OPAC/bibenq?SETLVL=&IRN=1965306")
    );
}

#[test]
fn patron_item_record_url_keeps_what_it_cannot_rewrite() {
    let row = format!("{CANNING_BIBENQ}/78577835?SETLVL=&QRY=IRN(8753080)&NAVLVL=SET");
    assert_eq!(patron_item_record_url(patron_item(None, Some(&row))), Some(row.clone()));
    assert_eq!(patron_item_record_url(patron_item(Some("87a"), Some(&row))), Some(row.clone()));
    let work = "https://canning.spydus.com/cgi-bin/spydus.exe/FULL/OPAC/ALLWRKENQ/1/15642035,1";
    assert_eq!(patron_item_record_url(patron_item(Some("8753080"), Some(work))).as_deref(), Some(work));
    assert_eq!(patron_item_record_url(patron_item(Some("8753080"), None)), None);
    assert_eq!(patron_item_record_url(patron_item(Some("8753080"), Some(""))), None);
}

fn holdings_snapshot() -> AccountSnapshot {
    let item = |irn: &str, title: &str| PatronItem {
        bib_irn: Some(irn.into()),
        title: Some(title.into()),
        ..PatronItem::default()
    };
    AccountSnapshot {
        version: ACCOUNT_SNAPSHOT_VERSION,
        account_name: None,
        loans: PatronList { total: Some(1), items: vec![item("100", "Loan")] },
        reservations: Reservations {
            ready: PatronList { total: Some(1), items: vec![item("200", "Ready"), item("100", "Ready again")] },
            pending: PatronList { total: Some(2), items: vec![item("300", "Pending"), item("200", "Pending too")] },
        },
        fetched_at_ms: 0,
        card_number: None,
        card_expires: None,
    }
}

fn holding(irns: &[&str]) -> Option<(HoldingKind, String)> {
    let irns = irns.iter().map(|irn| irn.to_string()).collect();
    account_holding(holdings_snapshot(), irns)
        .map(|AccountHolding { kind, item }| (kind, item.title.unwrap_or_default()))
}

#[test]
fn account_holding_finds_the_loan_or_reservation_by_bib_irn() {
    assert_eq!(holding(&["100"]), Some((HoldingKind::Loan, "Loan".into())));
    assert_eq!(holding(&["200"]), Some((HoldingKind::ReservationReady, "Ready".into())));
    assert_eq!(holding(&["300"]), Some((HoldingKind::ReservationPending, "Pending".into())));
    // Any of the page's IRNs; a loan beats a reservation whichever IRN comes first.
    assert_eq!(holding(&["300", " 100 "]), Some((HoldingKind::Loan, "Loan".into())));
}

#[test]
fn account_holding_needs_a_matching_bib_irn() {
    assert_eq!(holding(&["999"]), None);
    assert_eq!(holding(&[]), None);
    assert_eq!(holding(&["", " "]), None);
    // A row without a bib IRN never matches an empty one.
    let mut snapshot = holdings_snapshot();
    snapshot.loans.items[0].bib_irn = None;
    assert_eq!(account_holding(snapshot, vec![String::new()]), None);
}

// ---- alerts ----

fn row(irn: Option<&str>, bib_irn: Option<&str>, title: &str, fields: &[(&str, &str)]) -> PatronItem {
    PatronItem {
        irn: irn.map(Into::into),
        bib_irn: bib_irn.map(Into::into),
        title: Some(title.into()),
        fields: fields.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect(),
        ..PatronItem::default()
    }
}

fn account(loans: Vec<PatronItem>, ready: Vec<PatronItem>) -> AccountSnapshot {
    AccountSnapshot {
        version: ACCOUNT_SNAPSHOT_VERSION,
        account_name: None,
        loans: PatronList { total: None, items: loans },
        reservations: Reservations {
            ready: PatronList { total: None, items: ready },
            pending: PatronList { total: None, items: vec![row(Some("9"), None, "Queued", &[])] },
        },
        fetched_at_ms: 0,
        card_number: None,
        card_expires: None,
    }
}

fn titles(items: &[PatronItem]) -> Vec<&str> {
    items.iter().filter_map(|i| i.title.as_deref()).collect()
}

#[test]
fn patron_item_keys_prefer_the_irn() {
    assert_eq!(patron_item_key(&row(Some(" 29000004 "), Some("8753080"), "A", &[])), "29000004");
    assert_eq!(patron_item_key(&row(None, Some("8753080"), "A", &[])), "bib:8753080");
    assert_eq!(patron_item_key(&row(Some(""), None, "A", &[])), "title:A");
    assert_eq!(patron_item_key(&PatronItem::default()), "");
}

#[test]
fn ready_alerts_announce_each_ready_reservation_once() {
    let midnight = row(Some("1"), Some("8753080"), "The midnight library", &[]);
    let gift = row(Some("2"), Some("7000001"), "The gift", &[]);

    // Turning the alerts on: what's already waiting is remembered, not announced.
    let first = ready_alerts(account(vec![], vec![midnight.clone()]), vec![]);
    assert_eq!(titles(&first.new_items), ["The midnight library"]);
    assert_eq!(first.alerted_keys, ["1"]);

    // The next morning another one is ready: only it is new. Pending ones never alert.
    let next = ready_alerts(account(vec![], vec![midnight.clone(), gift.clone()]), first.alerted_keys);
    assert_eq!(titles(&next.new_items), ["The gift"]);
    assert_eq!(next.alerted_keys, ["1", "2"]);

    // Nothing changed: nothing to say.
    let same = ready_alerts(account(vec![], vec![midnight.clone(), gift.clone()]), next.alerted_keys);
    assert!(same.new_items.is_empty());

    // Collected, then reserved again and ready again later: announced again.
    let collected = ready_alerts(account(vec![], vec![gift.clone()]), same.alerted_keys);
    assert!(collected.new_items.is_empty());
    assert_eq!(collected.alerted_keys, ["2"]);
    let again = ready_alerts(account(vec![], vec![gift, midnight]), collected.alerted_keys);
    assert_eq!(titles(&again.new_items), ["The midnight library"]);
}

#[test]
fn ready_alerts_skip_rows_without_a_key_and_duplicates() {
    let blank = PatronItem::default();
    let a = row(Some("1"), None, "A", &[]);
    let alerts = ready_alerts(account(vec![], vec![blank, a.clone(), a]), vec![]);
    assert_eq!(titles(&alerts.new_items), ["A"]);
    assert_eq!(alerts.alerted_keys, ["1"]);
}

fn day(year: i32, month: u8, day: u8) -> SimpleDate {
    SimpleDate { year, month, day }
}

#[test]
fn due_reminders_group_loans_by_due_date() {
    let loans = vec![
        row(Some("1"), None, "Later", &[("Due", "30 Oct 2026")]),
        row(Some("2"), None, "Soon A", &[("Due", "16 Oct 2026")]),
        row(Some("3"), None, "Overdue", &[("Due", "12 Oct 2026")]),
        row(Some("4"), None, "Soon B", &[("Due", "16/10/2026")]),
        row(Some("5"), None, "No date", &[("Due", "soon")]),
        row(Some("6"), None, "No field", &[]),
    ];
    let reminders = due_reminders(account(loans, vec![]), day(2026, 10, 13), 2);
    assert_eq!(reminders.len(), 2);
    assert_eq!(reminders[0].key, "due:2026-10-16");
    assert_eq!(reminders[0].due, day(2026, 10, 16));
    assert_eq!(reminders[0].remind_on, day(2026, 10, 14));
    assert_eq!(titles(&reminders[0].items), ["Soon A", "Soon B"]);
    assert_eq!(reminders[1].key, "due:2026-10-30");
    assert_eq!(reminders[1].remind_on, day(2026, 10, 28));
}

#[test]
fn due_reminders_never_fall_before_today() {
    let loans = vec![
        row(Some("1"), None, "Today", &[("Due", "13 Oct 2026")]),
        row(Some("2"), None, "Tomorrow", &[("Due", "14 Oct 2026")]),
    ];
    let reminders = due_reminders(account(loans, vec![]), day(2026, 10, 13), 2);
    assert_eq!(reminders.iter().map(|r| r.remind_on).collect::<Vec<_>>(), [day(2026, 10, 13); 2]);
    assert_eq!(due_reminders(account(vec![], vec![]), day(2026, 10, 13), 2), []);
}

#[test]
fn due_reminders_cross_months_years_and_leap_days() {
    let on = |due: &str, days| {
        let loans = vec![row(Some("1"), None, "A", &[("Due", due)])];
        due_reminders(account(loans, vec![]), day(2000, 1, 1), days)[0].remind_on
    };
    assert_eq!(on("2 Nov 2026", 2), day(2026, 10, 31));
    assert_eq!(on("1 Jan 2027", 2), day(2026, 12, 30));
    assert_eq!(on("1 Mar 2028", 1), day(2028, 2, 29));
    assert_eq!(on("1 Mar 2100", 1), day(2100, 2, 28));
    assert_eq!(on("1 Mar 2000", 1), day(2000, 2, 29));
    assert_eq!(on("15 Oct 2026", 0), day(2026, 10, 15));
    assert_eq!(on("15 Oct 2026", 400), day(2025, 9, 10));
}
