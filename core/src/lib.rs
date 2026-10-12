//! LibreSpydus core: the on-device Spydus OPAC client + HTML parsers shared by the native
//! iOS (SwiftUI) and Android (Compose) apps via UniFFI. Port of the Bun/TS `middleware/`.
//! Contract: `docs/rust-core-port.md`.
//!
//! [`SpydusClient`] is the FFI facade. It replicates `middleware/src/server.ts`: the origin guard
//! on every URL argument, the `/record` fragment → bib-page resolution, concurrent reservation
//! lists, and the same in-memory TTL caches. One client talks to one library (its origin);
//! [`list_libraries`] is the directory the app picks that library from. [`app`] holds the pure
//! helpers both apps share (format, availability and due-date display, the home screen's
//! genres and shelves), and [`user_agent`] is the User-Agent every request sends.

uniffi::setup_scaffolding!();

pub mod app;
pub mod cache;
pub mod config;
pub mod crypto;
pub mod directory;
pub mod error;
pub mod fetch;
pub mod parse;
pub mod types;

pub use error::SpydusError;
pub use types::*;

use std::future::Future;
use std::sync::{Arc, LazyLock, PoisonError, RwLock};
use std::time::Duration;

use regex::Regex;

use cache::TtlCache;
use fetch::Http;
use parse::account::parse_patron_items;
use parse::availability::parse_availability;
use parse::decode::js_trim;
use parse::record::{bib_link_from_fragment, parse_brief_cards, parse_record_detail};
use parse::search::parse_search_results;

/// search / page / record / related (server.ts:59, 73, 102, 130).
const LIST_TTL: Duration = Duration::from_secs(10 * 60);
/// availability counts and the per-session account links (server.ts:86, 146).
const SHORT_TTL: Duration = Duration::from_secs(60);

// server.ts:105 — a RECFMT format fragment must be resolved to its bib page first.
static RECFMT_URL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[?&]FMT=RECFMT").unwrap());
// server.ts:113 — `/IRN[=(](\d+)/`.
static IRN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"IRN[=(]([0-9]+)").unwrap());

/// Every library in Civica's public Spydus directory, sorted by name: the list the app's library
/// picker shows. Each entry's `origin` is what [`SpydusClient::new`] takes. Civica's own
/// test/UAT/training/demo instances are left out unless `include_test`.
///
/// One public GET of `directory::DIRECTORY_URL` per call (not cached); no library is contacted.
#[uniffi::export(async_runtime = "tokio")]
pub async fn list_libraries(include_test: bool) -> Result<Vec<LibraryInfo>, SpydusError> {
    directory::list_libraries(include_test).await
}

/// The User-Agent every core request sends (`config::USER_AGENT`). The app's human-verification
/// web view must send the same one: the AWS WAF token it obtains may be tied to the UA.
#[uniffi::export]
pub fn user_agent() -> String {
    config::USER_AGENT.to_owned()
}

/// The library client the native apps hold (one per selected library). Cheap to share: every
/// method takes `&self` and the caches are internally synchronised.
///
/// URL arguments must be absolute URLs on the client's origin (as returned by earlier calls),
/// else `OriginForbidden`. Account methods take the session token returned by `login`.
#[derive(uniffi::Object)]
pub struct SpydusClient {
    /// `Err` holds why the HTTP client couldn't be built (e.g. an invalid origin); every call
    /// then fails with it instead of the constructor panicking across the FFI.
    http: Result<Http, String>,
    /// The library origin, as `Http` normalised it (no trailing `/`).
    origin: String,
    /// Replaced wholesale by `set_verification_cookie` (see there). Each call works on the set
    /// it read when it started.
    caches: RwLock<Arc<Caches>>,
}

/// The middleware's TTL caches (server.ts:59-146).
#[derive(Default)]
struct Caches {
    /// `s:{query}` search results and `p:{url}` result pages.
    pages: TtlCache<SearchResults>,
    availability: TtlCache<Availability>,
    records: TtlCache<RecordDetail>,
    related: TtlCache<RelatedList>,
    /// `acct:{session}` → the session's account links.
    accounts: TtlCache<AccountLinks>,
}

#[uniffi::export(async_runtime = "tokio")]
impl SpydusClient {
    /// `origin` is a library's `scheme://host` — a `LibraryInfo.origin` from
    /// [`list_libraries`] — and defaults to the Canning library (`https://canning.spydus.com`).
    #[uniffi::constructor]
    pub fn new(origin: Option<String>) -> Arc<Self> {
        let origin = origin.unwrap_or_else(|| config::ORIGIN.to_owned());
        let http = Http::new(&origin);
        Arc::new(Self {
            origin: match &http {
                Ok(http) => http.origin().to_owned(),
                Err(_) => origin.trim_end_matches('/').to_owned(),
            },
            http: http.map_err(|e| match e {
                SpydusError::Network { message }
                | SpydusError::Upstream { message }
                | SpydusError::Parse { message } => message,
                other => other.to_string(),
            }),
            caches: RwLock::default(),
        })
    }

    /// The library this client talks to (no trailing `/`). Every URL argument must start with
    /// it followed by `/`.
    pub fn origin(&self) -> String {
        self.origin.clone()
    }

    /// Sets, or with `None` / a blank string clears, the cookie sent with every later request:
    /// the `aws-waf-token=…` cookie the app reads from its web view after the **user** completed
    /// the library's AWS WAF human verification (see `HumanVerificationRequired`). The core never
    /// obtains or solves the verification itself, and keeps the cookie in memory only.
    ///
    /// Every cached response is dropped, so nothing fetched under the previous cookie (or
    /// without one) is served afterwards. A call already running when this is called finishes
    /// into the discarded caches.
    pub fn set_verification_cookie(&self, cookie: Option<String>) {
        let Ok(http) = &self.http else { return };
        // The cookie first: a call that sees the new caches then also sends the new cookie.
        http.set_extra_cookie(cookie);
        *self.caches.write().unwrap_or_else(PoisonError::into_inner) = Arc::default();
    }

    /// Keyword search → the first results page (server.ts:55-67), in relevance order: exactly
    /// `search_sorted(query, Relevance)`. Cached 10 min per case-insensitive query. Availability
    /// counts are not included; fetch them per format with `availability`. `noResults` is true
    /// when the library found nothing.
    pub async fn search(&self, query: String) -> Result<SearchResults, SpydusError> {
        self.search_sorted(query, SearchSort::Relevance).await
    }

    /// `search` in `sort` order: `Relevance` sends the same request as `search`; `Newest` puts
    /// the titles most recently added to the catalogue first ("new arrivals"). Later pages
    /// (`nextPageUrl` → `page`) keep the order. Cached 10 min per sort and case-insensitive query.
    pub async fn search_sorted(
        &self,
        query: String,
        sort: SearchSort,
    ) -> Result<SearchResults, SpydusError> {
        let http = self.http()?;
        let q = js_trim(&query);
        if q.is_empty() {
            return Err(SpydusError::Parse { message: "missing search query".into() });
        }
        cached(&self.caches().pages, search_key(sort, q), LIST_TTL, async {
            let html = http.fetch_search_html_sorted(q, sort).await?;
            Ok(parse_search_results(&html, http.origin(), Some(q)))
        })
        .await
    }

    /// Another results page: a `nextPageUrl` / `prevPageUrl` (server.ts:69-79). Cached 10 min.
    pub async fn page(&self, url: String) -> Result<SearchResults, SpydusError> {
        let http = self.guarded(&url)?;
        cached(&self.caches().pages, format!("p:{url}"), LIST_TTL, async {
            let html = http.fetch_fragment(&url).await?;
            Ok(parse_search_results(&html, http.origin(), None))
        })
        .await
    }

    /// Live counts for one format: a format's `availabilityUrl` (server.ts:81-95). Cached 60 s.
    pub async fn availability(&self, url: String) -> Result<Availability, SpydusError> {
        let http = self.guarded(&url)?;
        cached(&self.caches().availability, format!("a:{url}"), SHORT_TTL, async {
            let html = http.fetch_fragment(&url).await?;
            Ok(parse_availability(&html, http.origin()))
        })
        .await
    }

    /// The full bib record (server.ts:99-122). Pass a format's `availabilityUrl` (BK preferred;
    /// resolved via its bib link) or a direct BIBENQ bib URL such as a related item's `bibUrl`.
    /// Cached 10 min.
    pub async fn record(&self, url: String) -> Result<RecordDetail, SpydusError> {
        let http = self.guarded(&url)?;
        cached(&self.caches().records, format!("rec:{url}"), LIST_TTL, async {
            if RECFMT_URL.is_match(&url) {
                let fragment = http.fetch_fragment(&url).await?;
                let bib = bib_link_from_fragment(&fragment, http.origin()).ok_or_else(|| {
                    SpydusError::Upstream {
                        message: "no bib record link in format fragment".into(),
                    }
                })?;
                let html = http.fetch_fragment(&bib.bib_url).await?;
                return Ok(parse_record_detail(&html, http.origin(), Some(bib.bib_irn)));
            }
            let html = http.fetch_fragment(&url).await?;
            let bib_irn = IRN.captures(&url).map(|m| m[1].to_owned());
            Ok(parse_record_detail(&html, http.origin(), bib_irn))
        })
        .await
    }

    /// A related-titles panel: one of `RecordDetail.related.*` (server.ts:126-139). Cached 10 min.
    pub async fn related(&self, url: String) -> Result<RelatedList, SpydusError> {
        let http = self.guarded(&url)?;
        cached(&self.caches().related, format!("rel:{url}"), LIST_TTL, async {
            let html = http.fetch_fragment(&url).await?;
            Ok(RelatedList { items: parse_brief_cards(&html, http.origin()) })
        })
        .await
    }

    /// Sign in (server.ts:153-182) and return the session token to pass to the account
    /// methods. The PIN is RSA-encrypted before it leaves the device and is never stored.
    /// The session is confirmed by resolving the account; a login ID or PIN the library
    /// doesn't accept is `CredentialsRejected`.
    pub async fn login(&self, username: String, password: String) -> Result<String, SpydusError> {
        let http = self.http()?;
        let username = js_trim(&username);
        if username.is_empty() || password.is_empty() {
            return Err(SpydusError::CredentialsRejected);
        }
        let session = http.login(username, &password).await?;
        // server.ts:167-175 — "not logged in" from the confirmation is a rejected login too.
        match self.account_links(&session).await {
            Ok(_) => Ok(session),
            Err(SpydusError::NotLoggedIn) => Err(SpydusError::CredentialsRejected),
            Err(e) => Err(e),
        }
    }

    /// The session's account entry points (name, loans, ready/pending reservations).
    /// Cached 60 s per session (server.ts:145-147).
    pub async fn account(&self, session: String) -> Result<AccountLinks, SpydusError> {
        self.account_links(&session).await
    }

    /// Current loans (server.ts:290-302). No loans link → an empty list.
    pub async fn loans(&self, session: String) -> Result<PatronList, SpydusError> {
        let http = self.http()?;
        let session = signed_in(&session)?;
        let links = self.account_links(session).await?;
        let Some(loans_url) = links.loans_url else { return Ok(empty_list()) };
        let html = http.fetch_authed(&loans_url, session).await?;
        Ok(parse_patron_items(&html, http.origin()))
    }

    /// Ready-for-pickup and pending reservations, fetched concurrently (server.ts:305-322).
    /// A missing link → an empty list for that half.
    pub async fn reservations(&self, session: String) -> Result<Reservations, SpydusError> {
        let http = self.http()?;
        let session = signed_in(&session)?;
        let links = self.account_links(session).await?;
        let list = |url: Option<String>| async move {
            let Some(url) = url else { return Ok(empty_list()) };
            let html = http.fetch_authed(&url, session).await?;
            Ok::<_, SpydusError>(parse_patron_items(&html, http.origin()))
        };
        let (ready, pending) =
            tokio::join!(list(links.reservations_ready_url), list(links.reservations_pending_url));
        Ok(Reservations { ready: ready?, pending: pending? })
    }

    /// The reservation request form for a format's `reserveUrl` (server.ts:186-198). Read-only:
    /// no hold is placed. A logged-out session yields `message: "Please sign in again."`.
    pub async fn reserve_options(
        &self,
        url: String,
        session: String,
    ) -> Result<ReserveOptions, SpydusError> {
        let session = signed_in(&session)?;
        self.guarded(&url)?.reserve_options(&url, session).await
    }

    /// Place a reservation (server.ts:201-220). MUTATING — call only on explicit user
    /// confirmation. `xdays` / `pickup` override the form's expiry and pickup choices. Not
    /// being able to place it is `ok: false` data, not an error.
    pub async fn place_reservation(
        &self,
        url: String,
        session: String,
        xdays: Option<String>,
        pickup: Option<String>,
    ) -> Result<ReserveResult, SpydusError> {
        let session = signed_in(&session)?;
        let result = self.guarded(&url)?.place_reservation(&url, session, xdays, pickup).await?;
        // A changed hold invalidates the cached account for this session.
        self.caches().accounts.remove(&account_key(session));
        Ok(result)
    }

    /// Cancel a reservation via a reservation row's "Cancel" action href (`…/C/5/RSVCENQ?SVL=…`)
    /// (server.ts:225-243). MUTATING — call only on explicit user confirmation.
    pub async fn cancel_reservation(
        &self,
        url: String,
        session: String,
    ) -> Result<ReserveResult, SpydusError> {
        let session = signed_in(&session)?;
        let result = self.guarded(&url)?.cancel_reservation(&url, session).await?;
        self.caches().accounts.remove(&account_key(session));
        Ok(result)
    }

    /// The edit-reservation form (pickup choices + current dates) for a reservation row's
    /// "Edit" action href (server.ts:248-258). Read-only.
    pub async fn edit_options(
        &self,
        url: String,
        session: String,
    ) -> Result<EditOptions, SpydusError> {
        let session = signed_in(&session)?;
        self.guarded(&url)?.edit_reservation_options(&url, session).await
    }

    /// Change a reservation's pickup location / dates (server.ts:261-288). MUTATING — call only
    /// on explicit user confirmation. A `None` field keeps the current value; `clear_*` removes
    /// an existing date.
    pub async fn edit_reservation(
        &self,
        url: String,
        session: String,
        overrides: EditOverrides,
    ) -> Result<ReserveResult, SpydusError> {
        let session = signed_in(&session)?;
        let result = self.guarded(&url)?.edit_reservation(&url, session, &overrides).await?;
        self.caches().accounts.remove(&account_key(session));
        Ok(result)
    }
}

impl SpydusClient {
    /// The current cache set; the `Arc` keeps it alive for a call that outlives a swap.
    fn caches(&self) -> Arc<Caches> {
        self.caches.read().unwrap_or_else(PoisonError::into_inner).clone()
    }

    fn http(&self) -> Result<&Http, SpydusError> {
        self.http.as_ref().map_err(|message| SpydusError::Parse { message: message.clone() })
    }

    /// The origin guard every URL argument passes first (server.ts:72 etc.).
    fn guarded(&self, url: &str) -> Result<&Http, SpydusError> {
        let http = self.http()?;
        http.guard(url)?;
        Ok(http)
    }

    /// server.ts:145-147 — `acct:{session}`, cached 60 s.
    async fn account_links(&self, session: &str) -> Result<AccountLinks, SpydusError> {
        let http = self.http()?;
        let session = signed_in(session)?;
        let caches = self.caches();
        cached(&caches.accounts, account_key(session), SHORT_TTL, http.resolve_account(session))
            .await
    }
}

/// The middleware's `cached()` (server.ts:40-50): a live hit is returned as-is; otherwise
/// `fetch` runs and only a success is stored.
async fn cached<V: Clone + Send>(
    cache: &TtlCache<V>,
    key: String,
    ttl: Duration,
    fetch: impl Future<Output = Result<V, SpydusError>>,
) -> Result<V, SpydusError> {
    if let Some(hit) = cache.get(&key) {
        return Ok(hit);
    }
    let value = fetch.await?;
    cache.insert(key, value.clone(), ttl);
    Ok(value)
}

/// `X-Spydus-Session` is trimmed, and a blank one is "not signed in" (server.ts:149-151).
fn signed_in(session: &str) -> Result<&str, SpydusError> {
    match js_trim(session) {
        "" => Err(SpydusError::NotLoggedIn),
        session => Ok(session),
    }
}

fn account_key(session: &str) -> String {
    format!("acct:{session}")
}

/// `s:{sort}:{query}` — the middleware's `s:{query}` (server.ts:59) plus the sort.
fn search_key(sort: SearchSort, query: &str) -> String {
    let sort = match sort {
        SearchSort::Relevance => "relevance",
        SearchSort::Newest => "newest",
    };
    format!("s:{sort}:{}", query.to_lowercase())
}

/// `{ total: 0, items: [] }` for an absent account link (server.ts:295, 314-320).
fn empty_list() -> PatronList {
    PatronList { total: Some(0), items: Vec::new() }
}

#[cfg(test)]
mod tests {
    //! Offline facade checks: every rejection below happens before any request is made.
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const OFF_ORIGIN: &str = "https://canning.spydus.com.evil.example/cgi-bin/x";

    fn client() -> Arc<SpydusClient> {
        SpydusClient::new(None)
    }

    #[tokio::test]
    async fn url_arguments_must_be_on_the_origin() {
        let c = client();
        let url = || OFF_ORIGIN.to_owned();
        let s = || "SESSION".to_owned();
        assert!(matches!(c.page(url()).await, Err(SpydusError::OriginForbidden)));
        assert!(matches!(c.availability(url()).await, Err(SpydusError::OriginForbidden)));
        assert!(matches!(c.record(url()).await, Err(SpydusError::OriginForbidden)));
        assert!(matches!(c.related(url()).await, Err(SpydusError::OriginForbidden)));
        assert!(matches!(c.reserve_options(url(), s()).await, Err(SpydusError::OriginForbidden)));
        let placed = c.place_reservation(url(), s(), None, None).await;
        assert!(matches!(placed, Err(SpydusError::OriginForbidden)));
        assert!(matches!(
            c.cancel_reservation(url(), s()).await,
            Err(SpydusError::OriginForbidden)
        ));
        assert!(matches!(c.edit_options(url(), s()).await, Err(SpydusError::OriginForbidden)));
        let edited = c.edit_reservation(url(), s(), EditOverrides::default()).await;
        assert!(matches!(edited, Err(SpydusError::OriginForbidden)));
    }

    #[tokio::test]
    async fn blank_session_is_not_logged_in() {
        let c = client();
        let url = || format!("{}/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/1/0/V/1", config::ORIGIN);
        let blank = || " \t ".to_owned();
        assert!(matches!(c.account(blank()).await, Err(SpydusError::NotLoggedIn)));
        assert!(matches!(c.loans(blank()).await, Err(SpydusError::NotLoggedIn)));
        assert!(matches!(c.reservations(blank()).await, Err(SpydusError::NotLoggedIn)));
        assert!(matches!(c.reserve_options(url(), blank()).await, Err(SpydusError::NotLoggedIn)));
        let placed = c.place_reservation(url(), blank(), None, None).await;
        assert!(matches!(placed, Err(SpydusError::NotLoggedIn)));
        assert!(matches!(
            c.cancel_reservation(url(), blank()).await,
            Err(SpydusError::NotLoggedIn)
        ));
        assert!(matches!(c.edit_options(url(), blank()).await, Err(SpydusError::NotLoggedIn)));
        let edited = c.edit_reservation(url(), blank(), EditOverrides::default()).await;
        assert!(matches!(edited, Err(SpydusError::NotLoggedIn)));
    }

    #[tokio::test]
    async fn blank_inputs_are_rejected_without_a_request() {
        let c = client();
        assert!(matches!(c.search(" \n ".into()).await, Err(SpydusError::Parse { .. })));
        let no_user = c.login("  ".into(), "1234".into()).await;
        assert!(matches!(no_user, Err(SpydusError::CredentialsRejected)));
        let no_pin = c.login("user".into(), String::new()).await;
        assert!(matches!(no_pin, Err(SpydusError::CredentialsRejected)));
    }

    #[tokio::test]
    async fn an_invalid_origin_fails_every_call_instead_of_panicking() {
        let c = SpydusClient::new(Some("not a url".into()));
        match c.search("atomic habits".into()).await {
            Err(SpydusError::Parse { message }) => assert!(message.contains("invalid origin")),
            other => panic!("{other:?}"),
        }
        assert!(matches!(c.page("not a url/x".into()).await, Err(SpydusError::Parse { .. })));
    }

    #[test]
    fn origin_is_the_normalised_constructor_argument() {
        assert_eq!(client().origin(), config::ORIGIN);
        let lib = SpydusClient::new(Some("https://lib.spydus.com.au/".into()));
        assert_eq!(lib.origin(), "https://lib.spydus.com.au");
        let invalid = SpydusClient::new(Some("not a url/".into()));
        assert_eq!(invalid.origin(), "not a url");
        invalid.set_verification_cookie(Some("aws-waf-token=x".into())); // a no-op, no panic
    }

    #[tokio::test]
    async fn a_new_verification_cookie_drops_cached_responses() {
        let (origin, hits) = cookie_echo_server();
        let c = SpydusClient::new(Some(origin));
        let search = || async { c.search("q".into()).await.unwrap().records[0].title.clone() };
        let hits = || hits.load(Ordering::SeqCst);

        assert_eq!(search().await.as_deref(), Some("none"));
        assert_eq!(search().await.as_deref(), Some("none"));
        assert_eq!(hits(), 1, "a repeat is served from the cache");

        c.set_verification_cookie(Some(" aws-waf-token=T1 ".into()));
        assert_eq!(search().await.as_deref(), Some("aws-waf-token=T1"));
        assert_eq!(search().await.as_deref(), Some("aws-waf-token=T1"));
        assert_eq!(hits(), 2);

        c.set_verification_cookie(None);
        assert_eq!(search().await.as_deref(), Some("none"));
        assert_eq!(hits(), 3);
    }

    #[tokio::test]
    async fn search_results_are_cached_per_sort() {
        let (origin, hits) = cookie_echo_server();
        let c = SpydusClient::new(Some(origin));
        let hits = || hits.load(Ordering::SeqCst);

        c.search("Atomic Habits".into()).await.unwrap();
        c.search_sorted("atomic habits".into(), SearchSort::Relevance).await.unwrap();
        assert_eq!(hits(), 1, "search is search_sorted(Relevance), same cache entry");

        c.search_sorted("atomic habits".into(), SearchSort::Newest).await.unwrap();
        assert_eq!(hits(), 2, "another sort is another entry");
        c.search_sorted(" ATOMIC habits ".into(), SearchSort::Newest).await.unwrap();
        assert_eq!(hits(), 2);
        assert!(matches!(
            c.search_sorted(" ".into(), SearchSort::Newest).await,
            Err(SpydusError::Parse { .. })
        ));
        assert_eq!(search_key(SearchSort::Newest, "Q"), "s:newest:q");
    }

    /// An HTTP/1.1 server on 127.0.0.1 answering every request with a one-card results page
    /// titled with the request's `Cookie` header ("none" without one), counting requests.
    fn cookie_echo_server() -> (String, Arc<AtomicUsize>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let hits = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&hits);
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                counter.fetch_add(1, Ordering::SeqCst);
                let mut cookie = "none".to_owned();
                let request = BufReader::new(&stream).lines().map_while(Result::ok);
                for line in request.take_while(|line| !line.is_empty()) {
                    if let Some((name, value)) = line.split_once(':')
                        && name.eq_ignore_ascii_case("cookie")
                    {
                        cookie = value.trim().to_owned();
                    }
                }
                let body = format!(
                    r#"<fieldset class="card-list"><h3 class="card-title"><a href="/b">{cookie}</a></h3></fieldset>"#
                );
                let head = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nConnection: close";
                let reply = format!("{head}\r\nContent-Length: {}\r\n\r\n{body}", body.len());
                let _ = (&stream).write_all(reply.as_bytes());
            }
        });
        (origin, hits)
    }

    #[test]
    fn record_url_branching() {
        let fragment = "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/1?SETLVL=SET&FMT=RECFMT&SETID=1";
        assert!(RECFMT_URL.is_match(fragment));
        let bib = "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/2?QRY=IRN(9361967)&FMT=GD";
        assert!(!RECFMT_URL.is_match(bib));
        assert_eq!(IRN.captures(bib).map(|m| m[1].to_owned()).as_deref(), Some("9361967"));
        assert_eq!(
            IRN.captures("…/BIBENQ?SETLVL=&IRN=123").map(|m| m[1].to_owned()).as_deref(),
            Some("123")
        );
        assert!(IRN.captures("…/BIBENQ?SETLVL=&BRN=123").is_none());
    }
}
