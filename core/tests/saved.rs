//! Saved pages (`saved.rs`, `docs/native-apps.md` §5.13): book pages, Home shelves and the Featured
//! links, saved per library under the app's folder and read back without the network.

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use librespydus_core::SpydusClient;
use librespydus_core::saved::{MAX_SAVED_BOOKS, SAVED_PAGES_VERSION, file_name};
use librespydus_core::types::{FeaturedKind, FeaturedLink, RecordDetail, RelatedList, SavedBook, SearchResults};
use serde::de::DeserializeOwned;

const CANNING: &str = "https://canning.spydus.com";
const HUTT: &str = "https://hutt.spydus.co.nz";

/// A new folder under the system's temp folder, deleted when dropped.
struct TempDir(PathBuf);

impl TempDir {
    fn new(test: &str) -> Self {
        let path = std::env::temp_dir().join(format!("librespydus-saved-{}-{test}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn arg(&self) -> Option<String> {
        Some(self.0.to_string_lossy().into_owned())
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn golden<T: DeserializeOwned>(name: &str) -> T {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden").join(name);
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn client(origin: &str, dir: &TempDir) -> Arc<SpydusClient> {
    let client = SpydusClient::new(Some(origin.into()));
    client.set_saved_pages_dir(dir.arg());
    client
}

fn book() -> SavedBook {
    let related = RelatedList { items: golden("related.json") };
    SavedBook { record: golden("record-bib.json"), similar: Some(related), ..SavedBook::default() }
}

fn book_titled(title: &str) -> SavedBook {
    SavedBook { record: RecordDetail { title: Some(title.into()), ..RecordDetail::default() }, ..SavedBook::default() }
}

fn featured() -> Vec<FeaturedLink> {
    vec![FeaturedLink {
        title: "Library of Things".into(),
        description: None,
        icon: Some("game-board".into()),
        kind: FeaturedKind::Collection,
        url: format!("{CANNING}/cgi-bin/spydus.exe/MSGTRN/OPAC/MSGTRNGEN?HOMEPRMS=THINGS"),
    }]
}

/// Every saved file under `dir`, at any depth.
fn files(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(files(&path));
        } else {
            found.push(path);
        }
    }
    found
}

fn the_one_file(dir: &TempDir) -> PathBuf {
    let found = files(&dir.0);
    assert_eq!(found.len(), 1, "{found:?}");
    found.into_iter().next().unwrap()
}

#[tokio::test]
async fn a_saved_book_reads_back() {
    let dir = TempDir::new("book");
    let client = client(CANNING, &dir);
    assert_eq!(client.saved_book("bib:8753080".into()), None);
    client.save_book("bib:8753080".into(), book()).await;
    assert_eq!(client.saved_book("bib:8753080".into()), Some(book()));
    assert_eq!(client.saved_book("work:15642035".into()), None);
}

#[tokio::test]
async fn saving_again_replaces_the_book() {
    let dir = TempDir::new("replace");
    let client = client(CANNING, &dir);
    client.save_book("work:1".into(), book_titled("Old")).await;
    client.save_book("work:1".into(), book_titled("New")).await;
    assert_eq!(client.saved_book("work:1".into()), Some(book_titled("New")));
    assert_eq!(files(&dir.0).len(), 1);
}

#[tokio::test]
async fn shelves_and_featured_read_back() {
    let dir = TempDir::new("home");
    let client = client(CANNING, &dir);
    let arrivals: SearchResults = golden("search-midnight.json");
    assert_eq!(client.saved_shelf("new_arrivals".into()), None);
    assert_eq!(client.saved_featured(), None);
    client.save_shelf("new_arrivals".into(), arrivals.clone()).await;
    client.save_featured(featured()).await;
    assert_eq!(client.saved_shelf("new_arrivals".into()), Some(arrivals));
    assert_eq!(client.saved_shelf("new_for_kids".into()), None);
    assert_eq!(client.saved_featured(), Some(featured()));
    // A library without a Featured list: saved as empty, read back as empty (not as absent).
    client.save_featured(Vec::new()).await;
    assert_eq!(client.saved_featured(), Some(Vec::new()));
}

#[tokio::test]
async fn each_library_has_its_own_pages() {
    let dir = TempDir::new("origins");
    let canning = client(CANNING, &dir);
    let hutt = client(HUTT, &dir);
    canning.save_book("bib:1".into(), book_titled("Canning's")).await;
    canning.save_featured(featured()).await;
    assert_eq!(hutt.saved_book("bib:1".into()), None);
    assert_eq!(hutt.saved_featured(), None);
    // A new client for the same library (the app relaunched) finds them.
    assert_eq!(client(CANNING, &dir).saved_book("bib:1".into()), Some(book_titled("Canning's")));
}

#[tokio::test]
async fn off_until_a_folder_is_set_and_after_it_is_cleared() {
    let dir = TempDir::new("off");
    let client = SpydusClient::new(Some(CANNING.into()));
    client.save_book("bib:1".into(), book()).await;
    assert!(files(&dir.0).is_empty());

    client.set_saved_pages_dir(dir.arg());
    client.save_book("bib:1".into(), book()).await;
    assert_eq!(client.saved_book("bib:1".into()), Some(book()));

    client.set_saved_pages_dir(None);
    assert_eq!(client.saved_book("bib:1".into()), None);
    client.set_saved_pages_dir(Some("  ".into()));
    assert_eq!(client.saved_book("bib:1".into()), None);
}

#[tokio::test]
async fn a_file_that_does_not_read_back_is_deleted() {
    let dir = TempDir::new("malformed");
    let client = client(CANNING, &dir);
    client.save_book("bib:1".into(), book()).await;
    let path = the_one_file(&dir);
    fs::write(&path, "{\"version\":1,\"value\":{\"record\":").unwrap();
    assert_eq!(client.saved_book("bib:1".into()), None);
    assert!(!path.exists());
}

#[tokio::test]
async fn a_file_from_another_format_version_is_deleted() {
    let dir = TempDir::new("version");
    let client = client(CANNING, &dir);
    client.save_book("bib:1".into(), book()).await;
    let path = the_one_file(&dir);
    let saved = fs::read_to_string(&path).unwrap();
    let current = format!("\"version\":{SAVED_PAGES_VERSION}");
    assert!(saved.contains(&current), "{saved}");
    fs::write(&path, saved.replace(&current, &format!("\"version\":{}", SAVED_PAGES_VERSION + 1))).unwrap();
    assert_eq!(client.saved_book("bib:1".into()), None);
    assert!(!path.exists());
}

#[tokio::test]
async fn the_least_recently_saved_books_go_first() {
    let dir = TempDir::new("evict");
    let client = client(CANNING, &dir);
    client.save_book("bib:oldest".into(), book_titled("Oldest")).await;
    let oldest = the_one_file(&dir);
    File::options()
        .write(true)
        .open(&oldest)
        .unwrap()
        .set_modified(SystemTime::now() - Duration::from_secs(3600))
        .unwrap();
    // A temporary file left by a write that never finished, two hours ago.
    let leftover = oldest.with_file_name(".bib_3a1.json.0000000000000001.tmp");
    File::create(&leftover).unwrap().set_modified(SystemTime::now() - Duration::from_secs(7200)).unwrap();

    for n in 0..MAX_SAVED_BOOKS {
        client.save_book(format!("bib:{n}"), book_titled(&n.to_string())).await;
    }
    assert_eq!(files(&dir.0).len(), MAX_SAVED_BOOKS);
    assert_eq!(client.saved_book("bib:oldest".into()), None);
    assert!(!leftover.exists());
    assert_eq!(client.saved_book("bib:0".into()), Some(book_titled("0")));
    assert_eq!(client.saved_book(format!("bib:{}", MAX_SAVED_BOOKS - 1)), Some(book_titled("299")));
}

#[tokio::test]
async fn shelves_and_featured_are_not_counted_as_books() {
    let dir = TempDir::new("evict-books-only");
    let client = client(CANNING, &dir);
    client.save_shelf("new_arrivals".into(), SearchResults::default()).await;
    client.save_featured(featured()).await;
    for n in 0..MAX_SAVED_BOOKS + 1 {
        client.save_book(format!("bib:{n}"), book_titled("Book")).await;
    }
    assert_eq!(client.saved_shelf("new_arrivals".into()), Some(SearchResults::default()));
    assert_eq!(client.saved_featured(), Some(featured()));
}

#[tokio::test]
async fn a_key_too_long_for_a_file_name_is_not_saved() {
    let dir = TempDir::new("long-key");
    let client = client(CANNING, &dir);
    // A format's availability URL (search-midnight.json): 169 bytes, 327 once encoded.
    let key = format!(
        "url:{CANNING}/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/78554983?SETLVL=SET&FMT=RECFMT\
         &XSLT=XRecFmtDisplay.xsl&SETID=78554983&QRY=WRK01%5C15642035+%2B+03902\\BK"
    );
    client.save_book(key.clone(), book()).await;
    assert!(files(&dir.0).is_empty());
    assert_eq!(client.saved_book(key), None);
    client.save_book(" ".into(), book()).await;
    assert!(files(&dir.0).is_empty());
}

#[test]
fn file_names_keep_keys_apart() {
    assert_eq!(file_name("work:15642035").as_deref(), Some("work_3a15642035"));
    assert_eq!(file_name("new_arrivals").as_deref(), Some("new_5farrivals"));
    assert_eq!(file_name(CANNING).as_deref(), Some("https_3a_2f_2fcanning.spydus.com"));
    // Upper case is encoded, so a case-insensitive file system can't merge two keys.
    assert_ne!(file_name("work:ABC"), file_name("work:abc"));
    assert_ne!(file_name("work:1"), file_name("work_1"));
    assert_ne!(file_name("bib:1/2"), file_name("bib:1_2f2"));
    assert_eq!(file_name(""), None);
    assert_eq!(file_name("."), None);
    assert_eq!(file_name(".."), None);
    assert_eq!(file_name(&"9".repeat(201)), None);
}
