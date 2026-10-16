//! A `World` for tests: a fixed clock (Tuesday 14 October 2026), 80 columns, no colour, canned
//! stdin, captured stdout and stderr, a fake public backend (the demo wraps it under `--demo`),
//! a canned directory and no keychain.

use std::io::Write;
use std::sync::{Arc, Mutex};

use chrono::{NaiveDate, TimeZone, Utc};
use librespydus_core::{LibraryInfo, SpydusError};

use crate::app::{World, run};
use crate::backend::fake::FakeBackend;
use crate::backend::{LibraryBackend, ready};

pub const TODAY: NaiveDate = match NaiveDate::from_ymd_opt(2026, 10, 14) {
    Some(date) => date,
    None => unreachable!(),
};

#[derive(Clone, Default)]
pub struct Captured(Arc<Mutex<Vec<u8>>>);

impl Captured {
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().unwrap()).into_owned()
    }
}

impl Write for Captured {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// What a run printed and returned.
pub struct Outcome {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Outcome {
    pub fn json(&self) -> serde_json::Value {
        serde_json::from_str(&self.stdout).unwrap_or_else(|e| panic!("{e}: {}", self.stdout))
    }

    pub fn error_json(&self) -> serde_json::Value {
        serde_json::from_str(&self.stderr).unwrap_or_else(|e| panic!("{e}: {}", self.stderr))
    }
}

pub fn libraries() -> Vec<LibraryInfo> {
    let info = |name: &str, origin: &str, region: &str, country: &str, is_test: bool| LibraryInfo {
        id: origin.to_owned(),
        name: name.to_owned(),
        origin: origin.to_owned(),
        logo_url: None,
        country: Some(country.to_owned()),
        region: region.to_owned(),
        is_test,
    };
    vec![
        info("City of Canning", "https://canning.spydus.com", "AU", "Australia", false),
        info("Hutt City", "https://hutt.spydus.co.nz", "NZ", "New Zealand", false),
        info("City of Cockburn", "https://cockburn.spydus.com", "AU", "Australia", false),
        info("Civica Demo UK", "https://demouk.spydus.co.uk", "UK", "United Kingdom", true),
    ]
}

/// A test world around `public` (every origin gets the same fake), reading `input` as stdin.
pub struct TestWorld {
    pub public: Arc<FakeBackend>,
    pub stdin: String,
    pub interactive: bool,
    pub secret: String,
    pub directory_error: Option<SpydusError>,
    pub width: usize,
}

impl TestWorld {
    pub fn new() -> Self {
        Self {
            public: FakeBackend::new("https://canning.spydus.com"),
            stdin: String::new(),
            interactive: false,
            secret: String::new(),
            directory_error: None,
            width: 80,
        }
    }

    pub fn with_public(public: Arc<FakeBackend>) -> Self {
        Self { public, ..Self::new() }
    }

    pub fn typing(mut self, input: &str) -> Self {
        self.stdin = input.to_owned();
        self.interactive = true;
        self
    }

    pub fn piping(mut self, input: &str) -> Self {
        self.stdin = input.to_owned();
        self.interactive = false;
        self
    }

    pub fn secret(mut self, secret: &str) -> Self {
        self.secret = secret.to_owned();
        self
    }

    /// Runs `spydus <args>` in this world.
    pub fn run(&self, args: &[&str]) -> Outcome {
        let stdout = Captured::default();
        let stderr = Captured::default();
        let public = self.public.clone();
        let directory_error = self.directory_error.as_ref().map(crate::backend::clone_error);
        let secret = self.secret.clone();
        let mut world = World {
            today: TODAY,
            now: Utc.with_ymd_and_hms(2026, 10, 14, 1, 30, 0).unwrap(),
            width: self.width,
            color: false,
            interactive: self.interactive,
            stdin: Box::new(std::io::Cursor::new(self.stdin.clone().into_bytes())),
            stdout: Box::new(stdout.clone()),
            stderr: Box::new(stderr.clone()),
            read_secret: Box::new(move |_| Ok(secret.clone())),
            backends: Arc::new(move |_origin| -> Arc<dyn LibraryBackend> { public.clone() }),
            directory: Arc::new(move |_all| match &directory_error {
                Some(error) => ready(Err(crate::backend::clone_error(error))),
                None => ready(Ok(libraries())),
            }),
            keychain: None,
        };
        let argv: Vec<String> = std::iter::once("spydus").chain(args.iter().copied()).map(str::to_owned).collect();
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let code = runtime.block_on(run(argv, &mut world));
        Outcome { code, stdout: stdout.text(), stderr: stderr.text() }
    }
}

impl Default for TestWorld {
    fn default() -> Self {
        Self::new()
    }
}

/// A fresh `--data-dir`.
pub fn data_dir() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

/// Compares `actual` with `tests/golden/<name>.txt`; `UPDATE_GOLDEN=1` rewrites the file.
pub fn assert_golden(name: &str, actual: &str) {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("golden").join(format!("{name}.txt"));
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e} (run with UPDATE_GOLDEN=1)", path.display()));
    assert_eq!(actual, expected, "{} differs", path.display());
}
