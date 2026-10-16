//! The helper process itself (E2; docs/desktop-app.md §11.2): `librespydus --verify-helper …`
//! with `LS_FAKE_VERIFICATION=1` (debug builds) prints one cookie line after 2 s and exits 0,
//! without a window or a page; bad arguments exit 2. The real window is checked by hand only.

use std::process::Command;
use std::time::{Duration, Instant};

use librespydus_desktop::features::verification::{HELPER_FLAG, parse_cookie_line};

fn helper() -> Command {
    Command::new(env!("CARGO_BIN_EXE_librespydus"))
}

#[test]
fn the_fake_helper_prints_one_cookie_line_and_exits_0() {
    let started = Instant::now();
    let output = helper()
        .args([HELPER_FLAG, "https://canning.spydus.com", "LibreSpydus-test", "City of Canning"])
        .env("LS_FAKE_VERIFICATION", "1")
        .output()
        .expect("helper runs");
    assert_eq!(output.status.code(), Some(0));
    assert!(started.elapsed() >= Duration::from_secs(2), "answers after 2 s");
    let stdout = String::from_utf8(output.stdout).expect("utf-8");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 1, "{stdout:?}");
    assert_eq!(parse_cookie_line(lines[0]).as_deref(), Some("aws-waf-token=fake-verification-token"));
}

#[test]
fn bad_arguments_exit_2() {
    let output = helper().args([HELPER_FLAG, "not-a-url"]).env("LS_FAKE_VERIFICATION", "1").output().expect("runs");
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("usage"));
}
