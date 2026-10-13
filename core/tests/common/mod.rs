//! Shared helpers for the golden tests (contract §9). Use with `mod common;`.
//!
//! Goldens are the TS parsers' output (`middleware/scripts/emit-golden.ts`); see
//! `tests/golden/MANIFEST.md` for the exact fixture → function → args → golden rows.
#![allow(dead_code)]

use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Differences listed in a mismatch panic.
const MAX_REPORTED: usize = 20;
/// Longest JSON excerpt shown per side.
const MAX_EXCERPT: usize = 160;

fn tests_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests")
}

/// `<target-dir>/golden-actual/`, honoring a custom target dir.
fn actual_dir() -> PathBuf {
    let target = option_env!("CARGO_TARGET_TMPDIR")
        .and_then(|tmp| Path::new(tmp).parent().map(Path::to_path_buf))
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("target"));
    target.join("golden-actual")
}

/// Fixture text by path under `tests/fixtures/`, read like Bun's `file.text()`
/// (lossy UTF-8, leading BOM dropped) so the Rust parser sees what the TS one saw.
pub fn fixture(rel: &str) -> String {
    let path = tests_dir().join("fixtures").join(rel);
    let bytes = fs::read(&path).unwrap_or_else(|e| panic!("fixture {}: {e}", path.display()));
    let text = String::from_utf8_lossy(&bytes);
    text.strip_prefix('\u{FEFF}').unwrap_or(&text).to_owned()
}

/// Golden JSON by path under `tests/golden/`.
pub fn golden(rel: &str) -> Value {
    let path = tests_dir().join("golden").join(rel);
    let text =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("golden {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("golden {}: {e}", path.display()))
}

/// True when the coordinator-staged private fixtures and their goldens are present.
/// Tests over `private/` rows must return early (skip) when this is false.
pub fn private_available() -> bool {
    let has_files = |dir: PathBuf| {
        fs::read_dir(dir).is_ok_and(|mut entries| entries.any(|e| e.is_ok()))
    };
    let t = tests_dir();
    has_files(t.join("fixtures/private")) && has_files(t.join("golden/private"))
}

/// `true` (and a note on stderr) when a private-data test should skip.
pub fn skip_private(test: &str) -> bool {
    let skip = !private_available();
    if skip {
        eprintln!("skipping {test}: tests/fixtures/private or tests/golden/private absent");
    }
    skip
}

/// Serialize `actual` and compare it with the golden at `rel` (as JSON values, so object key
/// order is ignored). On mismatch, write the actual JSON to `target/golden-actual/<rel>` and
/// panic listing the first differing JSON pointers (a pass removes any stale file there). For `private/` goldens only the pointers
/// are listed, never the values.
pub fn assert_golden<T: Serialize + ?Sized>(actual: &T, rel: &str) {
    let actual = serde_json::to_value(actual).expect("serialize actual");
    let expected = golden(rel);
    let out = actual_dir().join(rel);
    if actual == expected {
        let _ = fs::remove_file(&out); // drop a stale actual from an earlier failing run
        return;
    }

    if let Some(parent) = out.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let pretty = serde_json::to_string_pretty(&actual).expect("pretty actual");
    let written = match fs::write(&out, pretty + "\n") {
        Ok(()) => out.display().to_string(),
        Err(e) => format!("<not written: {e}>"),
    };

    let mut diffs = Vec::new();
    diff(&expected, &actual, String::new(), &mut diffs);
    let redact = rel.starts_with("private/");
    let mut report = String::new();
    for d in diffs.iter().take(MAX_REPORTED) {
        let ptr = if d.ptr.is_empty() { "(root)" } else { &d.ptr };
        if redact {
            report.push_str(&format!("  {ptr}\n"));
        } else {
            report.push_str(&format!(
                "  {ptr}\n    expected: {}\n    actual:   {}\n",
                excerpt(d.expected.as_ref()),
                excerpt(d.actual.as_ref()),
            ));
        }
    }
    if diffs.len() > MAX_REPORTED {
        report.push_str(&format!("  … and {} more\n", diffs.len() - MAX_REPORTED));
    }
    panic!(
        "golden mismatch for {rel}: {} difference(s); actual written to {written}\n{report}",
        diffs.len()
    );
}

struct Diff {
    ptr: String,
    expected: Option<Value>,
    actual: Option<Value>,
}

/// RFC 6901 reference token escaping.
fn token(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

fn diff(expected: &Value, actual: &Value, ptr: String, out: &mut Vec<Diff>) {
    match (expected, actual) {
        (Value::Object(e), Value::Object(a)) => {
            let keys: BTreeSet<&String> = e.keys().chain(a.keys()).collect();
            for key in keys {
                let p = format!("{ptr}/{}", token(key));
                match (e.get(key), a.get(key)) {
                    (Some(x), Some(y)) => diff(x, y, p, out),
                    (x, y) => out.push(Diff { ptr: p, expected: x.cloned(), actual: y.cloned() }),
                }
            }
        }
        (Value::Array(e), Value::Array(a)) => {
            for i in 0..e.len().max(a.len()) {
                let p = format!("{ptr}/{i}");
                match (e.get(i), a.get(i)) {
                    (Some(x), Some(y)) => diff(x, y, p, out),
                    (x, y) => out.push(Diff { ptr: p, expected: x.cloned(), actual: y.cloned() }),
                }
            }
        }
        _ if expected != actual => out.push(Diff {
            ptr,
            expected: Some(expected.clone()),
            actual: Some(actual.clone()),
        }),
        _ => {}
    }
}

fn excerpt(v: Option<&Value>) -> String {
    let Some(v) = v else { return "<missing>".to_owned() };
    let s = v.to_string();
    if s.chars().count() <= MAX_EXCERPT {
        s
    } else {
        format!("{}…", s.chars().take(MAX_EXCERPT).collect::<String>())
    }
}
