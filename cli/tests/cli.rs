//! The `spydus` binary end to end (docs/cli.md §8): the demo account under a temporary
//! `--data-dir`, help, completions and the error cases, all without a network; one live Canning
//! search, `#[ignore]`d.

use assert_cmd::Command;
use predicates::prelude::*;

fn spydus(dir: &tempfile::TempDir) -> Command {
    let mut command = Command::cargo_bin("spydus").unwrap();
    command.env("NO_COLOR", "1").arg("--data-dir").arg(dir.path());
    command
}

#[test]
fn help_and_completions() {
    let dir = tempfile::tempdir().unwrap();
    spydus(&dir)
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Your Books").and(predicate::str::contains("spydus verify")));
    spydus(&dir).args(["completions", "fish"]).assert().success().stdout(predicate::str::contains("spydus"));
    spydus(&dir).arg("--version").assert().success().stdout(predicate::str::starts_with("spydus "));
}

#[test]
fn demo_account_reads_and_changes() {
    let dir = tempfile::tempdir().unwrap();
    spydus(&dir).args(["--demo", "loans"]).assert().success().stdout(
        predicate::str::contains("Signed in to City of Canning · Test Borrower")
            .and(predicate::str::contains("Whistler")),
    );
    spydus(&dir)
        .args(["--demo", "reservations"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Ready to collect").and(predicate::str::contains("Riverton Library")));
    spydus(&dir).args(["--demo", "profile"]).assert().success().stdout(predicate::str::contains("DEMO0012345"));
    spydus(&dir)
        .args(["--demo", "card"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Library card number  DEMO0012345"));
    spydus(&dir)
        .args(["--demo", "renew", "1", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("#1 The long night: Renewed · Due "));
    spydus(&dir)
        .args(["--demo", "renew", "2", "--yes"])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("cannot be renewed"));
    spydus(&dir)
        .args(["--demo", "cancel", "1", "--yes"])
        .assert()
        .success()
        .stdout("#1 The midnight library: Reservation cancelled\n");
    spydus(&dir)
        .args(["--demo", "edit", "1", "--pickup", "Willetton", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Reservation updated"));
    spydus(&dir)
        .args(["--demo", "picks"])
        .assert()
        .success()
        .stdout(predicate::str::contains("My Picks · Saved on your library account"));
    spydus(&dir)
        .args(["--demo", "picks", "remove", "1"])
        .assert()
        .success()
        .stdout("Removed The Midnight Library from My Picks\n");
    spydus(&dir)
        .args(["--demo", "--json", "loans"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"canRenew\": true"));
    assert!(!dir.path().join("cli.json").exists(), "demo saves no config");
    assert!(!dir.path().join("secrets.json").exists(), "demo saves no secrets");
}

#[test]
fn errors_and_exit_codes() {
    let dir = tempfile::tempdir().unwrap();
    spydus(&dir)
        .arg("loans")
        .assert()
        .code(1)
        .stderr("No library selected\nChoose one with spydus use <name>, or look with spydus libraries.\n");
    spydus(&dir)
        .args(["--library", "https://canning.spydus.com", "loans"])
        .assert()
        .code(3)
        .stderr("You’re not signed in\nSign in with spydus login.\n");
    spydus(&dir).args(["--demo", "show", "1"]).assert().code(1).stderr("Run spydus search first.\n");
    spydus(&dir)
        .args(["--demo", "renew", "nine", "--yes"])
        .assert()
        .code(2)
        .stderr("“nine” is not a number from a list.\n");
    spydus(&dir).args(["--demo", "renew", "1"]).assert().code(2).stderr("Not a terminal: add --yes to confirm.\n");
    spydus(&dir).arg("renew").assert().code(2).stderr(predicate::str::contains("Usage: spydus renew"));
    spydus(&dir).arg("nonsense").assert().code(2);
    spydus(&dir)
        .args(["--json", "--library", "https://canning.spydus.com", "loans"])
        .assert()
        .code(3)
        .stdout("")
        .stderr("{\"error\":{\"kind\":\"NotSignedIn\",\"message\":\"You’re not signed in\",\"hint\":\"Sign in with spydus login.\"}}\n");
}

/// A public GET against Canning: `cargo test -- --ignored live_canning_search`.
#[test]
#[ignore]
fn live_canning_search() {
    let dir = tempfile::tempdir().unwrap();
    spydus(&dir)
        .args(["--library", "https://canning.spydus.com", "search", "the", "midnight", "library"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Midnight Library").and(predicate::str::contains("Matt Haig")));
    spydus(&dir)
        .args(["--library", "https://canning.spydus.com", "--json", "show", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"bookmarkUrl\""));
}
