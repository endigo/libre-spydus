//! The commands end to end, in process (docs/cli.md §8): numbering and its refusals, golden
//! human output on the demo account with a fixed clock, JSON shapes, and how each failure maps
//! to a message and an exit code.

use std::sync::Arc;

use librespydus_core::{Availability, Holding, RecordDetail, SpydusError};

use crate::backend::fake::{FAKE_PASSWORD, FakeBackend, test_data};
use crate::error::{EXIT_FAILED, EXIT_OK, EXIT_SIGN_IN, EXIT_USAGE, EXIT_VERIFY};
use crate::testing::{TestWorld, assert_golden, data_dir};

/// A fake public catalogue with two records, their availability and holdings.
fn catalogue() -> Arc<FakeBackend> {
    let mut first = test_data::search_record();
    // Both formats can be reserved; the Book is the demo account's overdue loan (bib 19244682).
    first.formats[1].reserve_url = Some(test_data::RESERVE_URL.replace("RECFMT=BK", "RECFMT=EBK"));
    let fake = FakeBackend::with_records("https://canning.spydus.com", vec![first, test_data::second_record()]);
    let detail = RecordDetail {
        bib_irn: Some("19244682".into()),
        title: Some("Baby-Sitter's Club graphic novels. 18, Jessi Ramsey, pet-sitter".into()),
        summary: Some("Jessi is pet-sitting for the Mancusis, who have a houseful of animals.".into()),
        bookmark_url: Some("https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&BRN=1".into()),
        ..Default::default()
    };
    fake.records.lock().unwrap().insert(test_data::BK_AVAILABILITY_URL.to_owned(), detail.clone());
    fake.records.lock().unwrap().insert(test_data::DETAIL_URL.to_owned(), detail);
    fake.availabilities.lock().unwrap().insert(
        test_data::BK_AVAILABILITY_URL.to_owned(),
        Availability {
            bib_record_irn: Some("19244682".into()),
            counts: [("available".to_owned(), 2), ("copies".to_owned(), 3)].into_iter().collect(),
            copies: Some(3),
            holdings_url: Some("https://canning.spydus.com/xhld/1".into()),
            ..Default::default()
        },
    );
    fake.holdings.lock().unwrap().insert(
        "https://canning.spydus.com/xhld/1".into(),
        vec![
            Holding {
                location: Some("RIVERTON LIBRARY".into()),
                collection: Some("Junior graphic novels".into()),
                call_number: Some("JGN MAR".into()),
                status: Some("Available".into()),
                available: true,
                ..Default::default()
            },
            Holding {
                location: Some("CANNINGTON LIBRARY".into()),
                status: Some("On loan".into()),
                ..Default::default()
            },
        ],
    );
    fake
}

fn demo_args<'a>(dir: &'a std::path::Path, rest: &[&'a str]) -> Vec<&'a str> {
    let mut args = vec!["--demo", "--data-dir", dir.to_str().unwrap()];
    args.extend_from_slice(rest);
    args
}

// ---- Golden human output (demo account, 14 Oct 2026, 80 columns, no colour) ----

#[test]
fn golden_loans_reservations_books_profile() {
    let dir = data_dir();
    let world = TestWorld::new();
    for command in ["loans", "reservations", "books", "profile", "card"] {
        let out = world.run(&demo_args(dir.path(), &[command]));
        assert_eq!(out.code, EXIT_OK, "{command}: {}", out.stderr);
        assert_golden(command, &out.stdout);
    }
}

#[test]
fn golden_search_and_show() {
    let dir = data_dir();
    let world = TestWorld::with_public(catalogue());
    let out = world.run(&demo_args(dir.path(), &["search", "baby-sitters", "club"]));
    assert_eq!(out.code, EXIT_OK, "{}", out.stderr);
    assert_golden("search", &out.stdout);
    let out = world.run(&demo_args(dir.path(), &["show", "1", "--holdings"]));
    assert_eq!(out.code, EXIT_OK, "{}", out.stderr);
    assert_golden("show", &out.stdout);
    let out = world.run(&demo_args(dir.path(), &["picks"]));
    assert_eq!(out.code, EXIT_OK, "{}", out.stderr);
    assert_golden("picks", &out.stdout);
    let out = world.run(&demo_args(dir.path(), &["edit", "1"]));
    assert_eq!(out.code, EXIT_OK, "{}", out.stderr);
    assert_golden("edit", &out.stdout);
}

// ---- Numbering ----

#[test]
fn numbers_come_from_the_last_list_and_are_refused_when_stale_or_elsewhere() {
    let dir = data_dir();
    let world = TestWorld::with_public(catalogue());
    let out = world.run(&demo_args(dir.path(), &["show", "1"]));
    assert_eq!(out.code, EXIT_FAILED);
    assert_eq!(out.stderr, "Run spydus search first.\n");

    assert_eq!(world.run(&demo_args(dir.path(), &["search", "x"])).code, EXIT_OK);
    let out = world.run(&demo_args(dir.path(), &["show", "3"]));
    assert_eq!(out.code, EXIT_FAILED);
    assert_eq!(out.stderr, "There is no #3: the list runs from #1 to #2.\n");
    let out = world.run(&demo_args(dir.path(), &["show", "0"]));
    assert_eq!(out.code, EXIT_USAGE);
    let out = world.run(&demo_args(dir.path(), &["show", "two"]));
    assert_eq!(out.code, EXIT_USAGE);
    assert_eq!(out.stderr, "“two” is not a number from a list or a URL.\n");

    // Another library's list.
    let out = world.run(&demo_args(dir.path(), &["--library", "https://hutt.spydus.co.nz", "show", "1"]));
    assert_eq!(out.code, EXIT_FAILED);
    assert_eq!(out.stderr, "That number is from a list for City of Canning. Search again at this library.\n");

    // A stale list.
    let list_file = dir.path().join("lists").join("demo").join("last-list.json");
    let mut list: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&list_file).unwrap()).unwrap();
    list["savedAt"] = serde_json::Value::String("2026-10-12T00:00:00Z".into());
    std::fs::write(&list_file, list.to_string()).unwrap();
    let out = world.run(&demo_args(dir.path(), &["show", "1"]));
    assert_eq!(out.code, EXIT_FAILED);
    assert_eq!(out.stderr, "The list for “x” is more than a day old. Run that search again.\n");

    // picks remove needs the picks list; picks add takes the search list.
    assert_eq!(world.run(&demo_args(dir.path(), &["search", "x"])).code, EXIT_OK);
    let out = world.run(&demo_args(dir.path(), &["picks", "remove", "1"]));
    assert_eq!(out.code, EXIT_FAILED);
    assert_eq!(out.stderr, "Run spydus picks first.\n");
    let out = world.run(&demo_args(dir.path(), &["picks", "add", "2"]));
    assert_eq!(out.code, EXIT_OK, "{}", out.stderr);
    assert!(out.stdout.starts_with("Saved to My Picks\nThe midnight library\n"), "{}", out.stdout);
}

#[test]
fn account_numbers_follow_the_lists_order() {
    let dir = data_dir();
    let world = TestWorld::new();
    let out = world.run(&demo_args(dir.path(), &["renew", "2", "--yes"]));
    assert_eq!(out.code, EXIT_FAILED, "{}", out.stdout);
    assert_eq!(out.stdout, "#2 Whistler: This item cannot be renewed as it is reserved by another borrower.\n");
    assert_eq!(out.stderr, "");
    let out = world.run(&demo_args(dir.path(), &["renew", "1", "3", "--yes"]));
    assert_eq!(out.code, EXIT_OK);
    assert_eq!(out.stdout, "#1 The long night: Renewed · Due 2 Nov\n#3 The Love Library: Renewed · Due 9 Nov\n");
    let out = world.run(&demo_args(dir.path(), &["renew", "--all", "--yes"]));
    assert_eq!(out.code, EXIT_FAILED);
    assert_eq!(out.stdout.lines().count(), 5);
    let out = world.run(&demo_args(dir.path(), &["renew", "6", "--yes"]));
    assert_eq!(out.code, EXIT_USAGE);
    assert_eq!(out.stderr, "There is no #6: the list has 5 entries.\n");

    let out = world.run(&demo_args(dir.path(), &["cancel", "1", "--yes"]));
    assert_eq!(out.code, EXIT_OK, "{}", out.stderr);
    assert_eq!(out.stdout, "#1 The midnight library: Reservation cancelled\n");
    let out = world.run(&demo_args(dir.path(), &["cancel", "2", "--yes"]));
    assert_eq!(out.code, EXIT_USAGE);
    assert!(out.stderr.contains("can’t be cancelled from here"), "{}", out.stderr);

    let out = world
        .run(&demo_args(dir.path(), &["edit", "3", "--pickup", "cannington", "--delay-until", "2026-11-20", "--yes"]));
    assert_eq!(out.code, EXIT_OK, "{}", out.stderr);
    assert_eq!(
        out.stdout,
        "Reservation updated\n#3 Hollow bones: Collect from: Cannington Library · Delay until: 20 Nov\n"
    );
    let out = world.run(&demo_args(dir.path(), &["edit", "3", "--pickup", "nowhere", "--yes"]));
    assert_eq!(out.code, EXIT_USAGE);
    assert_eq!(
        out.stderr,
        "No branch called “nowhere”. Choose from: Cannington Library, Riverton Library, Willetton & District\n"
    );
    let out = world.run(&demo_args(dir.path(), &["edit", "3", "--not-needed-after", "soon", "--yes"]));
    assert_eq!(out.code, EXIT_USAGE);
    assert_eq!(out.stderr, "“soon” is not a date (use YYYY-MM-DD).\n");
}

// ---- Confirmations ----

#[test]
fn confirmations_ask_on_a_terminal_and_need_yes_elsewhere() {
    let dir = data_dir();
    let out = TestWorld::new().run(&demo_args(dir.path(), &["renew", "1"]));
    assert_eq!(out.code, EXIT_USAGE);
    assert_eq!(out.stderr, "Not a terminal: add --yes to confirm.\n");

    let out = TestWorld::new().typing("n\n").run(&demo_args(dir.path(), &["renew", "1"]));
    assert_eq!(out.code, EXIT_FAILED);
    assert!(out.stderr.starts_with("Renew this loan?\n“The long night” is due 12 Oct."), "{}", out.stderr);
    assert!(out.stderr.ends_with("[y/N] Nothing changed.\n"), "{}", out.stderr);
    assert_eq!(out.stdout, "");

    let out = TestWorld::new().typing("y\n").run(&demo_args(dir.path(), &["renew", "1"]));
    assert_eq!(out.code, EXIT_OK);
    assert_eq!(out.stdout, "#1 The long night: Renewed · Due 2 Nov\n");

    // reserve: the format is chosen on the terminal, then confirmed.
    let world = TestWorld::with_public(catalogue()).typing("1\ny\n");
    assert_eq!(world.run(&demo_args(dir.path(), &["search", "x"])).code, EXIT_OK);
    let out = world.run(&demo_args(dir.path(), &["reserve", "1"]));
    assert_eq!(out.code, EXIT_OK, "{}", out.stderr);
    assert!(out.stderr.contains("Formats\n #1  Book\n #2  eBook\nNumber: "), "{}", out.stderr);
    assert!(out.stderr.contains("Confirm reservation\nReserve “Baby-Sitter's Club graphic novels. 18, Jessi Ramsey, pet-sitter” · Book\nCollect from: Riverton Library\nAvailability: Canning - Reservable items 3, available 0\n[y/N] "), "{}", out.stderr);
    assert_eq!(
        out.stdout,
        "Reservation placed\nReserve “Baby-Sitter's Club graphic novels. 18, Jessi Ramsey, pet-sitter” · Book\nCollect from: Riverton Library\n"
    );
    // Piped: the format must be named.
    let world = TestWorld::with_public(catalogue());
    let out = world.run(&demo_args(dir.path(), &["reserve", "1", "--yes"]));
    assert_eq!(out.code, EXIT_USAGE);
    assert_eq!(out.stderr, "Which format? Add --format <name>: Book, eBook\n");
    let out = world.run(&demo_args(dir.path(), &["reserve", "1", "--format", "dvd", "--yes"]));
    assert_eq!(out.code, EXIT_USAGE);
    assert_eq!(out.stderr, "No format called “dvd”. Choose from: Book, eBook\n");
    let out = world
        .run(&demo_args(dir.path(), &["reserve", "1", "--format", "ebk", "--pickup", "WILLETTON & DISTRICT", "--yes"]));
    assert_eq!(out.code, EXIT_OK, "{}", out.stderr);
    assert!(out.stdout.ends_with("Collect from: Willetton & District\n"), "{}", out.stdout);
    // The same reservation twice: the library refuses.
    let out = world.run(&demo_args(dir.path(), &["reserve", "1", "--format", "Book", "--yes"]));
    assert_eq!(out.code, EXIT_OK, "demo state is per run");
}

// ---- JSON ----

#[test]
fn json_shapes() {
    let dir = data_dir();
    let world = TestWorld::with_public(catalogue());
    let out = world.run(&demo_args(dir.path(), &["--json", "search", "x"]));
    assert_eq!(out.code, EXIT_OK, "{}", out.stderr);
    let json = out.json();
    assert_eq!(json["query"], "x");
    assert_eq!(json["page"], 1);
    assert_eq!(json["totalResults"], 2);
    assert_eq!(json["hasNextPage"], false);
    assert_eq!(json["records"][0]["number"], 1);
    assert_eq!(json["records"][0]["workIrn"], "15974354");
    assert_eq!(json["records"][1]["number"], 2);
    assert_eq!(json["records"][0]["formats"][0]["code"], "BK");

    let out = world.run(&demo_args(dir.path(), &["--json", "show", "1", "--holdings"]));
    assert_eq!(out.code, EXIT_OK, "{}", out.stderr);
    let json = out.json();
    assert_eq!(json["number"], 1);
    assert_eq!(json["title"], "Baby-Sitter's Club graphic novels. 18, Jessi Ramsey, pet-sitter");
    assert_eq!(json["formats"][0]["code"], "BK");
    assert_eq!(json["formats"][0]["availability"]["bibRecordIrn"], "19244682");
    assert_eq!(json["formats"][0]["holdings"][0]["location"], "RIVERTON LIBRARY");
    assert_eq!(json["formats"][1]["availability"], serde_json::Value::Null);
    assert_eq!(json["record"]["bibIrn"], "19244682");
    assert_eq!(json["held"]["kind"], "overdue");
    assert_eq!(json["held"]["text"], "On loan · Overdue");
    assert_eq!(json["held"]["item"]["irn"], "18000001");

    let out = world.run(&demo_args(dir.path(), &["--json", "loans"]));
    let json = out.json();
    assert_eq!(json["library"]["origin"], "https://canning.spydus.com");
    assert_eq!(json["name"], "Test Borrower");
    assert_eq!(json["demo"], true);
    assert_eq!(json["loans"][0]["number"], 1);
    assert_eq!(json["loans"][0]["title"], "The long night");
    assert_eq!(json["loans"][0]["dueDate"], "2026-10-12");
    assert_eq!(json["loans"][0]["daysLeft"], -2);
    assert_eq!(json["loans"][0]["overdue"], true);
    assert_eq!(json["loans"][0]["canRenew"], true);
    assert_eq!(json["loans"][0]["fields"]["Due"], "12 Oct 2026");
    assert!(json.get("reservations").is_none());

    let out = world.run(&demo_args(dir.path(), &["--json", "reservations"]));
    let json = out.json();
    assert_eq!(json["reservations"][0]["number"], 1);
    assert_eq!(json["reservations"][0]["kind"], "ready");
    assert_eq!(json["reservations"][0]["status"], "Ready to collect");
    assert_eq!(json["reservations"][0]["pickup"], "Riverton Library");
    assert_eq!(json["reservations"][2]["kind"], "pending");
    assert_eq!(json["reservations"][2]["canCancel"], true);

    let out = world.run(&demo_args(dir.path(), &["--json", "books"]));
    let json = out.json();
    assert_eq!(json["loans"].as_array().unwrap().len(), 5);
    assert_eq!(json["reservations"].as_array().unwrap().len(), 4);

    let out = world.run(&demo_args(dir.path(), &["--json", "profile"]));
    let json = out.json();
    assert_eq!(json["cardNumber"], "DEMO0012345");
    assert_eq!(json["loans"], 5);
    assert_eq!(json["reservations"], 4);
    assert_eq!(json["ready"], 1);

    let out = world.run(&demo_args(dir.path(), &["--json", "renew", "1", "2", "--yes"]));
    assert_eq!(out.code, EXIT_FAILED);
    let json = out.json();
    assert_eq!(json["ok"], false);
    assert_eq!(json["results"][0]["ok"], true);
    assert_eq!(json["results"][0]["due"], "2 Nov 2026");
    assert_eq!(json["results"][1]["ok"], false);
    assert_eq!(json["results"][1]["message"], "This item cannot be renewed as it is reserved by another borrower.");

    let out = world.run(&demo_args(dir.path(), &["--json", "picks"]));
    let json = out.json();
    assert_eq!(json["onAccount"], true);
    assert_eq!(json["picks"][0]["number"], 1);

    let out = world.run(&demo_args(dir.path(), &["--json", "libraries", "canning"]));
    let json = out.json();
    assert_eq!(json[0]["name"], "City of Canning");

    let out = world.run(&demo_args(dir.path(), &["--json", "edit", "1"]));
    let json = out.json();
    assert_eq!(json["options"]["pickupChoices"][1]["code"], "25363");

    // Errors: one object on stderr, nothing on stdout.
    let out = world.run(&demo_args(dir.path(), &["--json", "show", "9"]));
    assert_eq!(out.code, EXIT_FAILED);
    assert_eq!(out.stdout, "");
    assert_eq!(
        out.error_json(),
        serde_json::json!({ "error": { "kind": "List", "message": "There is no #9: the list runs from #1 to #2." } })
    );
}

// ---- Errors → message and exit code ----

#[test]
fn errors_map_to_messages_and_exit_codes() {
    let dir = data_dir();
    let plain = |args: &[&str]| {
        let mut all = vec!["--data-dir", dir.path().to_str().unwrap()];
        all.extend_from_slice(args);
        TestWorld::with_public(catalogue()).run(&all)
    };
    // No library.
    let out = plain(&["loans"]);
    assert_eq!(out.code, EXIT_FAILED);
    assert_eq!(out.stderr, "No library selected\nChoose one with spydus use <name>, or look with spydus libraries.\n");
    let out = plain(&["--json", "search", "x"]);
    assert_eq!(out.error_json()["error"]["kind"], "NoLibrary");
    // Not signed in.
    let out = plain(&["--library", "canning", "loans"]);
    assert_eq!(out.code, EXIT_SIGN_IN);
    assert_eq!(out.stderr, "You’re not signed in\nSign in with spydus login.\n");
    // Usage.
    let out = plain(&["renew"]);
    assert_eq!(out.code, EXIT_USAGE);
    assert!(out.stderr.contains("required arguments"), "{}", out.stderr);
    let out = plain(&["nonsense"]);
    assert_eq!(out.code, EXIT_USAGE);
    // Deep pages are refused before any request: the fake sees no search.
    let world = TestWorld::with_public(catalogue());
    let out =
        world.run(&["--data-dir", dir.path().to_str().unwrap(), "--library", "canning", "search", "x", "--page", "11"]);
    assert_eq!(out.code, EXIT_USAGE);
    assert!(out.stderr.starts_with("Only the first 10 pages"), "{}", out.stderr);
    assert!(world.public.searches.lock().unwrap().is_empty());
    let out =
        world.run(&["--data-dir", dir.path().to_str().unwrap(), "--library", "canning", "search", "x", "--page", "2"]);
    assert_eq!(out.code, EXIT_USAGE);
    assert_eq!(out.stderr, "There is no such page.\n");
    let out = plain(&["--help"]);
    assert_eq!(out.code, EXIT_OK);
    assert!(out.stdout.contains("Your Books"), "{}", out.stdout);
    assert!(out.stdout.contains("--data-dir"), "{}", out.stdout);
    // Library errors.
    let fake = catalogue();
    *fake.public_error.lock().unwrap() = Some(SpydusError::Network { message: "timed out".into() });
    let world = TestWorld::with_public(fake.clone());
    let out = world.run(&["--data-dir", dir.path().to_str().unwrap(), "--library", "canning", "search", "x"]);
    assert_eq!(out.code, EXIT_FAILED);
    assert_eq!(out.stderr, "Can’t reach City of Canning. Check your connection.\nnetwork error: timed out\n");
    *fake.public_error.lock().unwrap() = Some(SpydusError::HumanVerificationRequired);
    let out = world.run(&["--data-dir", dir.path().to_str().unwrap(), "--library", "canning", "--json", "search", "x"]);
    assert_eq!(out.code, EXIT_VERIFY);
    let json = out.error_json();
    assert_eq!(json["error"]["kind"], "HumanVerificationRequired");
    assert_eq!(json["error"]["message"], "City of Canning asks you to confirm you’re human.");
    assert_eq!(json["error"]["hint"], "Complete the check with spydus verify.");
    *fake.public_error.lock().unwrap() = Some(SpydusError::Upstream { message: "502".into() });
    let out = world.run(&["--data-dir", dir.path().to_str().unwrap(), "--library", "canning", "search", "x"]);
    assert_eq!(out.code, EXIT_FAILED);
    assert_eq!(out.stderr, "The library’s website returned something unexpected.\nupstream error: 502\n");
    // The directory.
    let mut world = TestWorld::with_public(catalogue());
    world.directory_error = Some(SpydusError::Network { message: "offline".into() });
    let out = world.run(&["--data-dir", dir.path().to_str().unwrap(), "libraries"]);
    assert_eq!(out.code, EXIT_FAILED);
    assert_eq!(out.stderr, "Can’t reach the library directory. Check your connection.\nnetwork error: offline\n");
    let world = TestWorld::with_public(catalogue());
    let out = world.run(&["--data-dir", dir.path().to_str().unwrap(), "use", "city", "of"]);
    assert_eq!(out.code, EXIT_FAILED);
    assert!(out.stdout.contains("City of Canning") && out.stdout.contains("City of Cockburn"), "{}", out.stdout);
    assert_eq!(out.stderr, "2 libraries match. Narrow it down with more words, or give the origin.\n");
    let out = world.run(&["--data-dir", dir.path().to_str().unwrap(), "--json", "use", "city", "of"]);
    assert_eq!(out.code, EXIT_FAILED);
    assert_eq!(out.json()[1]["name"], "City of Cockburn");
    assert_eq!(out.error_json()["error"]["kind"], "NoMatch");
    let out = world.run(&["--data-dir", dir.path().to_str().unwrap(), "use", "atlantis"]);
    assert_eq!(out.code, EXIT_FAILED);
    assert_eq!(out.stderr, "No library matches “atlantis”.\n");
}

// ---- A real (fake) library: use, login, the session, logout, verify, local picks ----

#[test]
fn use_login_logout_verify_and_local_picks() {
    let dir = data_dir();
    let d = dir.path().to_str().unwrap();
    let fake = catalogue();
    let world = TestWorld::with_public(fake.clone()).piping(&format!("{FAKE_PASSWORD}\n"));
    let out = world.run(&["--data-dir", d, "use", "hutt"]);
    assert_eq!(out.code, EXIT_OK, "{}", out.stderr);
    assert_eq!(out.stdout, "Current library\nLibrary  Hutt City · https://hutt.spydus.co.nz\n");
    let out = world.run(&["--data-dir", d, "use", "https://canning.spydus.com/"]);
    assert_eq!(out.stdout, "Current library\nLibrary  City of Canning · https://canning.spydus.com\n");
    let config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.path().join("cli.json")).unwrap()).unwrap();
    assert_eq!(config["library"]["origin"], "https://canning.spydus.com");

    // Picks on this computer while signed out.
    assert_eq!(world.run(&["--data-dir", d, "search", "x"]).code, EXIT_OK);
    let out = world.run(&["--data-dir", d, "picks", "add", "1"]);
    assert_eq!(out.code, EXIT_OK, "{}", out.stderr);
    assert!(out.stdout.ends_with("Saved on this computer\n"), "{}", out.stdout);
    let out = world.run(&["--data-dir", d, "picks"]);
    assert_eq!(out.code, EXIT_OK, "{}", out.stderr);
    assert!(out.stdout.starts_with("My Picks · Saved on this computer\n"), "{}", out.stdout);
    assert!(out.stdout.contains("1  Baby-Sitter's Club"), "{}", out.stdout);
    let out = world.run(&["--data-dir", d, "picks", "remove", "1"]);
    assert_eq!(out.code, EXIT_OK, "{}", out.stderr);
    let out = world.run(&["--data-dir", d, "picks"]);
    assert!(out.stdout.contains("No picks yet"), "{}", out.stdout);

    // Sign in: the card is asked for, the PIN read from stdin; the session is saved, not the PIN.
    let out = world.run(&["--data-dir", d, "login", "--card", "123", "--password-stdin"]);
    assert_eq!(out.code, EXIT_OK, "{}", out.stderr);
    assert_eq!(out.stdout, "Signed in to City of Canning\nSigned in as Fake Borrower\n");
    let secrets = std::fs::read_to_string(dir.path().join("secrets.json")).unwrap();
    assert!(secrets.contains("fake-session"), "{secrets}");
    assert!(!secrets.contains(FAKE_PASSWORD));
    let out = world.run(&["--data-dir", d, "loans"]);
    assert_eq!(out.code, EXIT_OK, "{}", out.stderr);
    assert_eq!(
        out.stdout,
        "Signed in to City of Canning · Fake Borrower · Updated just now\nNo loans right now\nBooks on loan from City of Canning show up here with their due dates.\n"
    );
    let out = world.run(&["--data-dir", d, "reservations"]);
    assert!(out.stdout.contains("No reservations\n"), "{}", out.stdout);
    // A wrong PIN.
    let out = TestWorld::with_public(fake.clone()).piping("wrong\n").run(&[
        "--data-dir",
        d,
        "login",
        "--card",
        "123",
        "--password-stdin",
    ]);
    assert_eq!(out.code, EXIT_FAILED);
    assert_eq!(out.stderr, "Card number or PIN not recognised.\n");
    // The library ends the session: it's forgotten and the reader is told to sign in again.
    std::fs::write(
        dir.path().join("secrets.json"),
        r#"{"com.librespydus.cli.session":{"https://canning.spydus.com":"stale"}}"#,
    )
    .unwrap();
    let out = world.run(&["--data-dir", d, "loans"]);
    assert_eq!(out.code, EXIT_SIGN_IN);
    assert_eq!(out.stderr, "Please sign in again.\nSign in with spydus login.\n");
    assert!(!std::fs::read_to_string(dir.path().join("secrets.json")).unwrap().contains("stale"));
    let out = world.run(&["--data-dir", d, "loans"]);
    assert_eq!(out.code, EXIT_SIGN_IN);
    assert_eq!(out.stderr, "You’re not signed in\nSign in with spydus login.\n");

    // verify keeps the cookie and sends it; --clear forgets it.
    let out = TestWorld::with_public(fake.clone()).piping("aws-waf-token=abc123\n").run(&["--data-dir", d, "verify"]);
    assert_eq!(out.code, EXIT_OK, "{}", out.stderr);
    assert_eq!(out.stdout, "Verification saved for this library.\n");
    assert!(std::fs::read_to_string(dir.path().join("secrets.json")).unwrap().contains("abc123"));
    fake.cookies.lock().unwrap().clear();
    assert_eq!(world.run(&["--data-dir", d, "search", "x"]).code, EXIT_OK);
    assert_eq!(fake.cookies.lock().unwrap().as_slice(), [Some("aws-waf-token=abc123".to_owned())]);
    let out = world.run(&["--data-dir", d, "verify", "--clear"]);
    assert_eq!(out.stdout, "Verification cleared.\n");
    assert!(!std::fs::read_to_string(dir.path().join("secrets.json")).unwrap().contains("abc123"));

    // logout.
    world.run(&["--data-dir", d, "login", "--card", "123", "--password-stdin"]);
    let out = world.run(&["--data-dir", d, "logout"]);
    assert_eq!(out.stdout, "Signed out of City of Canning\n");
    assert_eq!(world.run(&["--data-dir", d, "loans"]).code, EXIT_SIGN_IN);
}

#[test]
fn demo_saves_no_config_and_no_secrets() {
    let dir = data_dir();
    let world = TestWorld::with_public(catalogue());
    assert_eq!(world.run(&demo_args(dir.path(), &["login"])).code, EXIT_OK);
    assert_eq!(world.run(&demo_args(dir.path(), &["search", "x"])).code, EXIT_OK);
    assert_eq!(world.run(&demo_args(dir.path(), &["use", "hutt"])).code, EXIT_OK);
    assert!(!dir.path().join("cli.json").exists());
    assert!(!dir.path().join("secrets.json").exists());
    assert!(dir.path().join("lists").join("demo").join("last-list.json").exists());
    assert!(!dir.path().join("lists").join("last-list.json").exists());
}

#[test]
fn completions_and_version() {
    let world = TestWorld::new();
    let out = world.run(&["completions", "bash"]);
    assert_eq!(out.code, EXIT_OK);
    assert!(out.stdout.contains("_spydus"), "{}", out.stdout);
    let out = world.run(&["--version"]);
    assert_eq!(out.code, EXIT_OK);
    assert!(out.stdout.starts_with("spydus "), "{}", out.stdout);
}
