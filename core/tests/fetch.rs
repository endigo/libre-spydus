//! fetch.rs + cache.rs — offline. Pure helpers are tested directly; the HTTP flows run against a
//! throwaway HTTP/1.1 server on 127.0.0.1 (no request ever reaches a library, contract §3). The
//! live tests are `#[ignore]`d and are public GETs only.
//!
//! The wizard tests (place/cancel/edit with a form) need `parse::reserve` (task D); while it is
//! still a stub they print a note and return early.

mod common;

use std::sync::{Arc, LazyLock};
use std::thread;
use std::time::{Duration, Instant};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use librespydus_core::cache::TtlCache;
use librespydus_core::config::{self, LOGIN_POST_PATH, LOGIN_RDT, UI_COOKIES};
use librespydus_core::fetch::{
    Http, meta_refresh_url, normalize_brwenq_url, session_from_set_cookies,
};
use librespydus_core::parse::account::parse_account_links;
use librespydus_core::parse::reserve::{
    ManageKind, parse_edit_form, parse_manage_result, parse_reserve_form, parse_reserve_result,
    to_reserve_options,
};
use librespydus_core::{EditOverrides, ReserveResult, SearchSort, SpydusError};
use mock::{Reply, Server, Stall};
use rsa::pkcs8::{EncodePublicKey, LineEnding};
use rsa::{Pkcs1v15Encrypt, RsaPrivateKey};

const SESSION: &str = "TESTSESSION0001";
const SIGN_IN_AGAIN: &str = "Please sign in again.";
const LOGIN_PAGE: &str = r#"<html><body><form id="frmLogin" method="post"></form></body></html>"#;

fn authed_cookie() -> String {
    format!("SPYDUS_BRWSESSIONID_443={SESSION}; {UI_COOKIES}")
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

// --- session_from_set_cookies (auth.ts:60-68) ---------------------------------------------

#[test]
fn session_cookie_with_and_without_port_suffix() {
    let with_port = strings(&["SPYDUS_BRWSESSIONID_443=ABC123; path=/; secure; HttpOnly"]);
    assert_eq!(session_from_set_cookies(&with_port).as_deref(), Some("ABC123"));
    let bare = strings(&["SPYDUS_BRWSESSIONID=XYZ; path=/"]);
    assert_eq!(session_from_set_cookies(&bare).as_deref(), Some("XYZ"));
    let other_port = strings(&["SPYDUS_BRWSESSIONID_8443=P8443"]);
    assert_eq!(session_from_set_cookies(&other_port).as_deref(), Some("P8443"));
}

#[test]
fn cleared_session_cookie_is_ignored() {
    let cleared = "SPYDUS_BRWSESSIONID_443=; expires=Thu, 01 Jan 1970 00:00:00 GMT; path=/";
    assert_eq!(session_from_set_cookies(&strings(&[cleared])), None);
    let then_real = strings(&[cleared, "SPYDUS_BRWSESSIONID_443=REAL; path=/"]);
    assert_eq!(session_from_set_cookies(&then_real).as_deref(), Some("REAL"));
}

#[test]
fn first_session_across_multiple_set_cookies_wins() {
    let jar = strings(&[
        "SPYDUS_DISPLAYCONTEXT_443=MOB; path=/",
        "PQID=12345; path=/",
        "SPYDUS_BRWSESSIONID=FIRST; path=/",
        "SPYDUS_BRWSESSIONID_443=SECOND; path=/",
    ]);
    assert_eq!(session_from_set_cookies(&jar).as_deref(), Some("FIRST"));
    assert_eq!(session_from_set_cookies(&strings(&["PQID=1", "OTHER=2"])), None);
    assert_eq!(session_from_set_cookies(&[]), None);
}

#[test]
fn session_value_stops_at_semicolon_or_js_whitespace() {
    assert_eq!(
        session_from_set_cookies(&strings(&["SPYDUS_BRWSESSIONID_443=A B"])).as_deref(),
        Some("A")
    );
    // NBSP is JS `\s` (and so a terminator), matching the TS regex exactly.
    let nbsp = strings(&["SPYDUS_BRWSESSIONID_443=TOK\u{a0}TAIL"]);
    assert_eq!(session_from_set_cookies(&nbsp).as_deref(), Some("TOK"));
}

// --- meta_refresh_url / normalize_brwenq_url (auth.ts:311-314) ----------------------------

#[test]
fn meta_refresh_target_is_extracted() {
    let html = r#"<html><head>
<meta http-equiv="Refresh" content="0; url=https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/WQUP/BRWENQ/12345?QRY=BRWIRN%3C%3E&amp;FMT=X">
</head></html>"#;
    assert_eq!(
        meta_refresh_url(html).as_deref(),
        Some(
            "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/WQUP/BRWENQ/12345?QRY=BRWIRN%3C%3E&amp;FMT=X"
        )
    );
}

#[test]
fn meta_refresh_is_case_insensitive_and_stops_at_quotes() {
    let html =
        r#"<META HTTP-EQUIV="refresh" CONTENT='0;URL=/cgi-bin/spydus.exe/ENQ/OPAC/BRWENQ/1'>"#;
    assert_eq!(meta_refresh_url(html).as_deref(), Some("/cgi-bin/spydus.exe/ENQ/OPAC/BRWENQ/1"));
}

#[test]
fn meta_refresh_absent_or_outside_the_tag() {
    assert_eq!(meta_refresh_url("<html><body>Please log in</body></html>"), None);
    // `[^>]*` never crosses the tag end.
    assert_eq!(meta_refresh_url(r#"<meta http-equiv="Refresh"><a href="?url=/x">"#), None);
    // The attribute must be quoted as in the TS regex.
    assert_eq!(meta_refresh_url("<meta http-equiv=Refresh content=\"0; url=/x\">"), None);
}

#[test]
fn brwenq_context_is_normalised_to_opac() {
    let wqup = "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/WQUP/BRWENQ/1?QRY=X";
    assert_eq!(
        normalize_brwenq_url(wqup),
        "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BRWENQ/1?QRY=X"
    );
    let opac = "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BRWENQ/1";
    assert_eq!(normalize_brwenq_url(opac), opac);
    // `[A-Z]+` is case-sensitive, and only the first occurrence is replaced.
    assert_eq!(normalize_brwenq_url("/ENQ/wqup/BRWENQ/1"), "/ENQ/wqup/BRWENQ/1");
    assert_eq!(
        normalize_brwenq_url("/ENQ/A/BRWENQ/ENQ/B/BRWENQ/"),
        "/ENQ/OPAC/BRWENQ/ENQ/B/BRWENQ/"
    );
    assert_eq!(
        normalize_brwenq_url("/cgi-bin/spydus.exe/MSGTRN/OPAC/LOGINB"),
        "/cgi-bin/spydus.exe/MSGTRN/OPAC/LOGINB"
    );
}

// --- TtlCache -------------------------------------------------------------------------------

#[test]
fn cache_get_insert_overwrite_remove() {
    let cache = TtlCache::new();
    assert_eq!(cache.get("k"), None);
    cache.insert("k".into(), 1, Duration::from_secs(60));
    assert_eq!(cache.get("k"), Some(1));
    cache.insert("k".into(), 2, Duration::from_secs(60));
    assert_eq!(cache.get("k"), Some(2));
    cache.remove("k");
    assert_eq!(cache.get("k"), None);
    cache.remove("missing"); // no-op
}

#[test]
fn cache_entries_expire() {
    let cache = TtlCache::new();
    cache.insert("zero".into(), "z", Duration::ZERO);
    assert_eq!(cache.get("zero"), None, "a zero TTL is never readable (exp > now)");

    cache.insert("short".into(), "s", Duration::from_millis(40));
    cache.insert("long".into(), "l", Duration::from_secs(60));
    cache.insert("forever".into(), "f", Duration::MAX);
    assert_eq!(cache.get("short"), Some("s"));
    thread::sleep(Duration::from_millis(80));
    assert_eq!(cache.get("short"), None);
    assert_eq!(cache.get("long"), Some("l"));
    assert_eq!(cache.get("forever"), Some("f"));

    // An expired entry stays absent; re-inserting revives the key.
    cache.insert("short".into(), "s2", Duration::from_secs(60));
    assert_eq!(cache.get("short"), Some("s2"));
}

#[test]
fn cache_is_shared_across_threads() {
    let cache = Arc::new(TtlCache::<usize>::new());
    let workers: Vec<_> = (0..8)
        .map(|i| {
            let cache = Arc::clone(&cache);
            thread::spawn(move || {
                for j in 0..50 {
                    cache.insert(format!("{i}:{j}"), i * 100 + j, Duration::from_secs(60));
                }
            })
        })
        .collect();
    for w in workers {
        w.join().unwrap();
    }
    for i in 0..8 {
        for j in 0..50 {
            assert_eq!(cache.get(&format!("{i}:{j}")), Some(i * 100 + j));
        }
    }
}

// --- Http::new / guard ------------------------------------------------------------------------

#[test]
fn guard_accepts_only_urls_under_the_origin() {
    let http = Http::new(config::ORIGIN).unwrap();
    assert_eq!(http.origin(), config::ORIGIN);
    for ok in [
        "https://canning.spydus.com/",
        "https://canning.spydus.com/cgi-bin/spydus.exe/XHLD/OPAC/BIBENQ/1/2?RECFMT=BK",
    ] {
        assert!(http.guard(ok).is_ok(), "{ok}");
    }
    for bad in [
        "https://canning.spydus.com",
        "https://canning.spydus.com.evil.example/x",
        "https://canning.spydus.com@evil.example/x",
        "http://canning.spydus.com/x",
        "https://evil.example/?u=https://canning.spydus.com/x",
        "/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ",
        "",
    ] {
        assert!(matches!(http.guard(bad), Err(SpydusError::OriginForbidden)), "{bad}");
    }
}

#[test]
fn new_normalises_and_validates_the_origin() {
    let http = Http::new("https://canning.spydus.com/").unwrap();
    assert_eq!(http.origin(), "https://canning.spydus.com");
    assert!(http.guard("https://canning.spydus.com/x").is_ok());
    for bad in ["", "canning.spydus.com", "ftp://canning.spydus.com"] {
        assert!(matches!(Http::new(bad), Err(SpydusError::Parse { .. })), "{bad:?}");
    }
}

// --- public reads (spydus.ts) -----------------------------------------------------------------

/// `{origin}/cgi-bin/spydus.exe/ENQ/OPAC/ALLWRKENQ?…` with `encoded` as `ENTRY` and `GQ`.
fn search_target(encoded: &str) -> String {
    format!(
        "/cgi-bin/spydus.exe/ENQ/OPAC/ALLWRKENQ?ENTRY={encoded}&ENTRY_NAME=BS&ENTRY_TYPE=K&SORTS=SQL_REL_GENWRK&GQ={encoded}"
    )
}

#[tokio::test]
async fn search_gets_the_allwrkenq_url_directly() {
    let server = Server::start(|req| match req.path() {
        "/cgi-bin/spydus.exe/ENQ/OPAC/ALLWRKENQ" => {
            Reply::ok("\u{feff}<fieldset class=\"card-list\"></fieldset>")
        }
        _ => Reply::status(404, "nope"),
    });
    let http = Http::new(&server.origin).unwrap();
    let html = http.fetch_search_html("harry potter").await.unwrap();
    assert_eq!(
        html, "<fieldset class=\"card-list\"></fieldset>",
        "UTF-8 text with the BOM dropped"
    );
    // Byte-for-byte what Canning's discover/search returned for "Harry Potter" (native HAR).
    http.fetch_search_html("Harry Potter").await.unwrap();

    let reqs = server.requests();
    assert_eq!(
        reqs.iter().map(|r| r.target.as_str()).collect::<Vec<_>>(),
        [
            "/cgi-bin/spydus.exe/ENQ/OPAC/ALLWRKENQ?ENTRY=harry+potter&ENTRY_NAME=BS&ENTRY_TYPE=K&SORTS=SQL_REL_GENWRK&GQ=harry+potter",
            "/cgi-bin/spydus.exe/ENQ/OPAC/ALLWRKENQ?ENTRY=Harry+Potter&ENTRY_NAME=BS&ENTRY_TYPE=K&SORTS=SQL_REL_GENWRK&GQ=Harry+Potter",
        ],
        "one GET per search, no discover/search call"
    );
    for req in &reqs {
        assert_eq!(req.method, "GET");
        assert_eq!(req.header("user-agent"), Some(config::USER_AGENT));
        assert_eq!(req.header("accept"), Some("text/html,application/json"));
        assert_eq!(req.header("spydus-lang"), None);
        assert_eq!(req.header("cookie"), None, "public reads carry no cookie");
    }
}

#[tokio::test]
async fn sorted_search_sets_sorts_and_relevance_is_unchanged() {
    let server = Server::start(|_| Reply::ok("<html></html>"));
    let http = Http::new(&server.origin).unwrap();
    http.fetch_search_html_sorted("harry potter", SearchSort::Newest).await.unwrap();
    http.fetch_search_html_sorted("harry potter", SearchSort::Relevance).await.unwrap();
    http.fetch_search_html("harry potter").await.unwrap();

    let reqs = server.requests();
    let targets: Vec<&str> = reqs.iter().map(|r| r.target.as_str()).collect();
    assert_eq!(
        targets[0],
        "/cgi-bin/spydus.exe/ENQ/OPAC/ALLWRKENQ?ENTRY=harry+potter&ENTRY_NAME=BS&ENTRY_TYPE=K&SORTS=MAIN.CREATED_DATE.DESC%5DMAIN.CREATED_TIME.DESC&GQ=harry+potter"
    );
    assert_eq!(targets[1], search_target("harry+potter"), "relevance = the discover/search URL");
    assert_eq!(targets[2], targets[1], "fetch_search_html is the relevance search");
    assert_eq!(reqs[0].query_pairs()[3], ("SORTS".to_owned(), config::SORTS_NEWEST.to_owned()));
    assert_eq!(reqs[1].query_pairs()[3], ("SORTS".to_owned(), config::SORTS_RELEVANCE.to_owned()));
}

#[tokio::test]
async fn search_query_is_form_urlencoded() {
    let server = Server::start(|_| Reply::ok("<html></html>"));
    let http = Http::new(&server.origin).unwrap();
    let cases = [
        ("harry potter's (1) ünï", "harry+potter%27s+%281%29+%C3%BCn%C3%AF"),
        ("tom & jerry", "tom+%26+jerry"),
        ("a+b=c#d%e/f?g;h", "a%2Bb%3Dc%23d%25e%2Ff%3Fg%3Bh"),
        ("*-._~!", "*-._%7E%21"),
        ("日本 {q}", "%E6%97%A5%E6%9C%AC+%7Bq%7D"),
        ("", ""),
    ];
    for (query, _) in cases {
        http.fetch_search_html(query).await.unwrap();
    }
    let reqs = server.requests();
    assert_eq!(reqs.len(), cases.len());
    for (req, (query, encoded)) in reqs.iter().zip(cases) {
        assert_eq!(req.target, search_target(encoded), "{query:?}");
        // …and the server decodes it back to the query in both slots.
        let pairs = req.query_pairs();
        assert_eq!(pairs[0], ("ENTRY".to_owned(), query.to_owned()));
        assert_eq!(pairs[4], ("GQ".to_owned(), query.to_owned()));
    }
}

#[tokio::test]
async fn search_non_2xx_is_upstream() {
    let server = Server::start(|_| Reply::status(500, "boom"));
    let http = Http::new(&server.origin).unwrap();
    match http.fetch_search_html("fail").await {
        Err(SpydusError::Upstream { message }) => assert_eq!(message, "results 500"),
        other => panic!("{other:?}"),
    }
    assert_eq!(server.requests().len(), 1);
}

#[tokio::test]
async fn fragment_non_2xx_is_upstream_and_transport_failure_is_network() {
    let server = Server::start(|_| Reply::status(404, "missing"));
    let http = Http::new(&server.origin).unwrap();
    match http.fetch_fragment(&format!("{}/cgi-bin/x", server.origin)).await {
        Err(SpydusError::Upstream { message }) => assert_eq!(message, "fragment 404"),
        other => panic!("{other:?}"),
    }
    // Nothing listens on port 9 of localhost (discard) in a test sandbox.
    let dead = Http::new("http://127.0.0.1:9").unwrap();
    assert!(matches!(
        dead.fetch_fragment("http://127.0.0.1:9/x").await,
        Err(SpydusError::Network { .. })
    ));
}

// --- fetch_authed (auth.ts:142-146) ---------------------------------------------------------

#[tokio::test]
async fn authed_get_follows_object_moved_and_keeps_the_cookie() {
    let server = Server::start(|req| match req.path() {
        "/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/1/0/V/1" => {
            Reply::redirect(302, "/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/1/0/V/2?RDT=/x")
        }
        "/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/1/0/V/2" => Reply::ok("the form"),
        _ => Reply::status(500, "boom"),
    });
    let http = Http::new(&server.origin).unwrap();
    let url = format!("{}/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/1/0/V/1", server.origin);
    assert_eq!(http.fetch_authed(&url, SESSION).await.unwrap(), "the form");

    let reqs = server.requests();
    assert_eq!(reqs.len(), 2);
    for req in &reqs {
        assert_eq!(req.method, "GET");
        assert_eq!(req.header("cookie"), Some(authed_cookie().as_str()));
        assert_eq!(req.header("user-agent"), Some(config::USER_AGENT));
    }

    match http.fetch_authed(&format!("{}/other", server.origin), SESSION).await {
        Err(SpydusError::Upstream { message }) => assert_eq!(message, "authed GET 500"),
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn cross_origin_redirect_is_not_followed() {
    let other = Server::start(|_| Reply::ok("elsewhere"));
    let elsewhere = format!("{}/landing", other.localhost_origin());
    let server = Server::start(move |req| match req.path() {
        "/start" => Reply::redirect(302, "/next"),
        _ => Reply::redirect(302, &elsewhere),
    });
    let http = Http::new(&server.origin).unwrap();
    let start = format!("{}/start", server.origin);
    assert!(matches!(http.fetch_authed(&start, SESSION).await, Err(SpydusError::OriginForbidden)));
    assert!(matches!(http.fetch_fragment(&start).await, Err(SpydusError::OriginForbidden)));

    assert!(other.requests().is_empty(), "nothing (and no session) leaves the origin");
    let reqs = server.requests();
    assert_eq!(
        reqs.iter().map(|r| r.path()).collect::<Vec<_>>(),
        ["/start", "/next", "/start", "/next"],
        "same-origin hops are still followed"
    );
    assert_eq!(reqs[1].header("cookie"), Some(authed_cookie().as_str()));
}

#[tokio::test]
async fn same_origin_redirects_are_capped() {
    let server = Server::start(|req| {
        let n: u32 = req.path().trim_start_matches("/hop/").parse().unwrap_or(0);
        Reply::redirect(302, &format!("/hop/{}", n + 1))
    });
    let http = Http::new(&server.origin).unwrap();
    match http.fetch_fragment(&format!("{}/hop/0", server.origin)).await {
        Err(SpydusError::Network { message }) => {
            assert!(message.contains("too many redirects"), "{message}")
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(server.requests().len(), 11, "the request + 10 redirects");
}

#[tokio::test]
async fn urls_outside_the_origin_are_refused_before_sending() {
    let other = Server::start(|_| Reply::ok("elsewhere"));
    let server = Server::start(|_| Reply::ok("home"));
    let http = Http::new(&server.origin).unwrap();
    for url in [
        format!("{}/x", other.origin),
        format!("{}/x", server.localhost_origin()),
        format!("{}.evil.example/x", server.origin),
        format!("{}@evil.example/x", server.origin),
        "/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ".to_owned(),
    ] {
        assert!(
            matches!(http.fetch_fragment(&url).await, Err(SpydusError::OriginForbidden)),
            "{url}"
        );
        assert!(
            matches!(http.fetch_authed(&url, SESSION).await, Err(SpydusError::OriginForbidden)),
            "{url}"
        );
    }
    assert!(other.requests().is_empty() && server.requests().is_empty());
}

// --- login (auth.ts:93-135) -----------------------------------------------------------------

static KEYPAIR: LazyLock<RsaPrivateKey> =
    LazyLock::new(|| RsaPrivateKey::new(&mut rand::thread_rng(), 1024).expect("generate RSA key"));

/// encrypt.js as Canning serves it, carrying the local test key.
fn encrypt_js() -> String {
    let pem = KEYPAIR.to_public_key().to_public_key_pem(LineEnding::LF).unwrap();
    let continued = pem.trim_end().lines().collect::<Vec<_>>().join("\\\n");
    format!("function rsaEncrypt(val) {{\n    crypt.setPublicKey('{continued}\\\n');\n}}\n")
}

fn decrypt(b64: &str) -> String {
    let bytes = STANDARD.decode(b64).unwrap();
    String::from_utf8(KEYPAIR.decrypt(Pkcs1v15Encrypt, &bytes).unwrap()).unwrap()
}

/// A login server whose LB/2 POST answers with `post`, and whose RDT hop answers with `hop`.
fn login_server(post: Reply, hop: Reply) -> Server {
    let js = encrypt_js();
    Server::start(move |req| match (req.method.as_str(), req.path()) {
        ("GET", "/docs/OPAC/js/encrypt.js") => Reply::ok(js.clone()),
        ("POST", "/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/LB/2") => post.clone(),
        ("GET", "/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/LB/1") => hop.clone(),
        _ => Reply::status(404, "unexpected"),
    })
}

#[tokio::test]
async fn login_posts_the_rsa_encrypted_pin_and_reads_the_session() {
    let server = login_server(
        Reply::redirect(302, LOGIN_RDT)
            .header("Set-Cookie", "SPYDUS_BRWSESSIONID_443=NEWSESSION; path=/; secure; HttpOnly"),
        Reply::status(500, "the hop must not be followed when the POST set the session"),
    );
    let http = Http::new(&server.origin).unwrap();
    assert_eq!(http.login("21234000123456", "pin & ünïcode").await.unwrap(), "NEWSESSION");

    let reqs = server.requests();
    assert_eq!(
        reqs.iter().map(|r| r.path()).collect::<Vec<_>>(),
        ["/docs/OPAC/js/encrypt.js", LOGIN_POST_PATH]
    );
    assert_eq!(reqs[0].header("cookie"), None);
    let post = &reqs[1];
    assert_eq!(post.method, "POST");
    assert_eq!(post.header("cookie"), Some(UI_COOKIES));
    assert_eq!(post.header("content-type"), Some("application/x-www-form-urlencoded"));
    let form = post.form();
    assert_eq!(
        form.iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>(),
        ["BRWLID", "BRWLPWD", "RDT"]
    );
    assert_eq!(form[0].1, "21234000123456");
    assert_eq!(form[1].1.len(), 172, "base64 of a 1024-bit RSA block");
    assert_eq!(decrypt(&form[1].1), "pin & ünïcode");
    assert_eq!(form[2].1, LOGIN_RDT);
}

#[tokio::test]
async fn login_follows_one_hop_carrying_the_cookies_set_so_far() {
    let server = login_server(
        Reply::redirect(302, LOGIN_RDT).header("Set-Cookie", "PQID=777; path=/").header(
            "Set-Cookie",
            "SPYDUS_BRWSESSIONID_443=; expires=Thu, 01 Jan 1970 00:00:00 GMT; path=/",
        ),
        Reply::redirect(302, "/cgi-bin/spydus.exe/MSGTRN/OPAC/HOME")
            .header("Set-Cookie", "SPYDUS_BRWSESSIONID=HOPSESSION; path=/"),
    );
    let http = Http::new(&server.origin).unwrap();
    assert_eq!(http.login("user", "pin").await.unwrap(), "HOPSESSION");

    let reqs = server.requests();
    assert_eq!(reqs.len(), 3, "exactly one hop, and the hop's own redirect is not followed");
    let hop = &reqs[2];
    assert_eq!((hop.method.as_str(), hop.target.as_str()), ("GET", LOGIN_RDT));
    assert_eq!(
        hop.header("cookie"),
        Some(format!("{UI_COOKIES}; PQID=777; SPYDUS_BRWSESSIONID_443=").as_str())
    );
}

#[tokio::test]
async fn login_without_a_session_is_credentials_rejected() {
    // Bad PIN: the form is re-rendered (200) with no session cookie.
    let server = login_server(Reply::ok(LOGIN_PAGE), Reply::status(500, "unused"));
    let http = Http::new(&server.origin).unwrap();
    assert!(matches!(http.login("user", "bad").await, Err(SpydusError::CredentialsRejected)));
    assert_eq!(server.requests().len(), 2, "a 200 is never followed");

    // A redirect whose hop issues no session either.
    let server = login_server(Reply::redirect(302, LOGIN_RDT), Reply::ok(LOGIN_PAGE));
    let http = Http::new(&server.origin).unwrap();
    assert!(matches!(http.login("user", "bad").await, Err(SpydusError::CredentialsRejected)));
    assert_eq!(server.requests().len(), 3);
}

#[tokio::test]
async fn login_needs_encrypt_js() {
    let server = Server::start(|_| Reply::status(404, "gone"));
    let http = Http::new(&server.origin).unwrap();
    match http.login("user", "pin").await {
        Err(SpydusError::Upstream { message }) => assert_eq!(message, "encrypt.js 404"),
        other => panic!("{other:?}"),
    }
    assert_eq!(server.requests().len(), 1, "no POST without a key");
}

#[tokio::test]
async fn login_hop_outside_the_origin_is_refused() {
    let other = Server::start(|_| {
        Reply::ok("").header("Set-Cookie", "SPYDUS_BRWSESSIONID_443=STOLEN; path=/")
    });
    for location in [
        format!("{}{LOGIN_RDT}", other.localhost_origin()),
        format!("{}{LOGIN_RDT}", other.origin),
        "@evil.example/x".to_owned(),
        ".evil.example/x".to_owned(),
    ] {
        let server = login_server(
            Reply::redirect(302, &location).header("Set-Cookie", "PQID=1; path=/"),
            Reply::status(500, "unused"),
        );
        let http = Http::new(&server.origin).unwrap();
        assert!(
            matches!(http.login("user", "pin").await, Err(SpydusError::OriginForbidden)),
            "{location}"
        );
        assert_eq!(server.requests().len(), 2, "encrypt.js + the POST, no hop");
    }
    assert!(other.requests().is_empty(), "the hop never left the origin");
}

// --- resolve_account (auth.ts:309-317) ------------------------------------------------------

#[tokio::test]
async fn account_follows_the_dashboard_refresh_to_the_opac_brwenq_page() {
    let account_html = common::fixture("synthetic/account-links.html");
    let page = account_html.clone();
    let server = Server::start(move |req| match req.path() {
        "/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/LB/1" => Reply::ok(format!(
            r#"<html><head><meta http-equiv="Refresh" content="0; url=http://{}/cgi-bin/spydus.exe/ENQ/WQUP/BRWENQ/42?QRY=BRWIRN"></head></html>"#,
            req.header("host").unwrap()
        )),
        "/cgi-bin/spydus.exe/ENQ/OPAC/BRWENQ/42" => Reply::ok(page.clone()),
        _ => Reply::status(404, "unexpected"),
    });
    let http = Http::new(&server.origin).unwrap();
    let links = http.resolve_account(SESSION).await.unwrap();
    assert_eq!(links, parse_account_links(&account_html, &server.origin));

    let reqs = server.requests();
    assert_eq!(
        reqs.iter().map(|r| r.target.as_str()).collect::<Vec<_>>(),
        [LOGIN_RDT, "/cgi-bin/spydus.exe/ENQ/OPAC/BRWENQ/42?QRY=BRWIRN"]
    );
    assert!(reqs.iter().all(|r| r.header("cookie") == Some(authed_cookie().as_str())));
}

#[tokio::test]
async fn account_refresh_outside_the_origin_is_refused() {
    let other = Server::start(|_| Reply::ok("elsewhere"));
    let target = format!("{}/cgi-bin/spydus.exe/ENQ/WQUP/BRWENQ/42", other.origin);
    let server = Server::start(move |_| {
        Reply::ok(format!(r#"<meta http-equiv="Refresh" content="0; url={target}">"#))
    });
    let http = Http::new(&server.origin).unwrap();
    assert!(matches!(http.resolve_account(SESSION).await, Err(SpydusError::OriginForbidden)));
    assert_eq!(server.requests().len(), 1);
    assert!(other.requests().is_empty(), "the session never left the origin");
}

#[tokio::test]
async fn account_without_a_refresh_is_not_logged_in() {
    let server = Server::start(|_| Reply::ok(LOGIN_PAGE));
    let http = Http::new(&server.origin).unwrap();
    assert!(matches!(http.resolve_account(SESSION).await, Err(SpydusError::NotLoggedIn)));
    assert_eq!(server.requests().len(), 1);
}

// --- reservation wizards: logged-out and no-form paths (no parser needed) ---------------------

#[tokio::test]
async fn logged_out_wizard_gets_return_sign_in_again_data() {
    let server = Server::start(|_| Reply::ok(LOGIN_PAGE));
    let http = Http::new(&server.origin).unwrap();
    let url = format!("{}/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/1/0/V/1", server.origin);

    let opts = http.reserve_options(&url, SESSION).await.unwrap();
    assert!(!opts.can_reserve);
    assert_eq!(opts.message.as_deref(), Some(SIGN_IN_AGAIN));
    assert_eq!(
        (opts.pickup_location, opts.available_summary, opts.expiry_choices.len()),
        (None, None, 0)
    );

    let edit = http.edit_reservation_options(&url, SESSION).await.unwrap();
    assert!(!edit.can_edit);
    assert_eq!(edit.message.as_deref(), Some(SIGN_IN_AGAIN));
    assert!(edit.pickup_choices.is_empty() && edit.pickup_selected.is_none());
    assert!(edit.delay_until.is_none() && edit.not_required_after.is_none());

    let sign_in = ReserveResult { ok: false, message: Some(SIGN_IN_AGAIN.into()) };
    assert_eq!(http.cancel_reservation(&url, SESSION).await.unwrap(), sign_in);
    assert_eq!(
        http.edit_reservation(&url, SESSION, &EditOverrides::default()).await.unwrap(),
        sign_in
    );
    assert!(server.requests().iter().all(|r| r.method == "GET"), "never POSTs when logged out");
}

#[tokio::test]
async fn cancel_and_edit_without_a_form_are_not_ok_data() {
    let server = Server::start(|_| Reply::ok("<html><body>Reservation not found</body></html>"));
    let http = Http::new(&server.origin).unwrap();
    let url = format!("{}/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/1/0/C/5/RSVCENQ?SVL=1", server.origin);
    let cancel = http.cancel_reservation(&url, SESSION).await.unwrap();
    assert_eq!(
        cancel,
        ReserveResult {
            ok: false,
            message: Some("This reservation can't be cancelled right now.".into())
        }
    );
    let edit = http.edit_reservation(&url, SESSION, &EditOverrides::default()).await.unwrap();
    assert_eq!(
        edit,
        ReserveResult {
            ok: false,
            message: Some("This reservation can't be edited right now.".into())
        }
    );
    assert!(server.requests().iter().all(|r| r.method == "GET"));
}

#[tokio::test]
async fn reserve_options_reads_the_form_without_posting() {
    let form_html = common::fixture("synthetic/reserve-form.html");
    let page = form_html.clone();
    let server = Server::start(move |_| Reply::ok(page.clone()));
    let http = Http::new(&server.origin).unwrap();
    let opts = http.reserve_options(&format!("{}/r", server.origin), SESSION).await.unwrap();
    assert_eq!(opts, to_reserve_options(parse_reserve_form(&form_html, &server.origin).as_ref()));
    assert_eq!(server.requests().len(), 1);
}

// --- reservation wizards with a form (need task D's parse::reserve) ---------------------------

/// True once `parse_reserve_form` is implemented. Also asserts the fixture's action resolves to
/// the mock server, so a wizard test can never POST anywhere else.
fn reserve_parser_ready(origin: &str) -> bool {
    match parse_reserve_form(&common::fixture("synthetic/reserve-form.html"), origin) {
        Some(form) => {
            assert!(
                form.action.starts_with(&format!("{origin}/")),
                "form action left the mock: {}",
                form.action
            );
            true
        }
        None => {
            eprintln!("skipping: parse::reserve::parse_reserve_form is still a stub (task D)");
            false
        }
    }
}

fn wizard_form(action: &str, csrf: &str) -> String {
    format!(
        r#"<form id="mainForm" method="post" action="{action}">
<input type="hidden" name="csrf" value="{csrf}"><input type="hidden" name="XDAYS" value="0">
<button type="submit" name="submitButton" class="btn">Continue</button></form>"#
    )
}

const PLACED: &str = r#"<html><body><div class="alert alert-success">Your reservation request has been placed.</div></body></html>"#;

#[tokio::test]
async fn place_reservation_submits_once_with_overrides_and_follows_redirects() {
    let form_html = common::fixture("synthetic/reserve-form.html");
    let page = form_html.clone();
    let server = Server::start(move |req| match (req.method.as_str(), req.path()) {
        ("GET", "/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/78500000/0/V/1") => {
            Reply::redirect(302, "/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/78500000/0/V/2?RDT=x")
        }
        ("GET", "/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/78500000/0/V/2") => Reply::ok(page.clone()),
        ("POST", "/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/78500000/12345/R/1") => {
            Reply::redirect(302, "/cgi-bin/spydus.exe/ENQ/OPAC/RSVCENQ?FMT=WR")
        }
        ("GET", "/cgi-bin/spydus.exe/ENQ/OPAC/RSVCENQ") => Reply::ok(PLACED),
        _ => Reply::status(404, "unexpected"),
    });
    if !reserve_parser_ready(&server.origin) {
        return;
    }
    let http = Http::new(&server.origin).unwrap();
    let url = format!("{}/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/78500000/0/V/1", server.origin);
    let result = http
        .place_reservation(&url, SESSION, Some("730".into()), Some("25379".into()))
        .await
        .unwrap();
    assert_eq!(result, parse_reserve_result(PLACED));

    let reqs = server.requests();
    let methods: Vec<_> = reqs.iter().map(|r| r.method.as_str()).collect();
    assert_eq!(methods, ["GET", "GET", "POST", "GET"]);
    assert!(reqs.iter().all(|r| r.header("cookie") == Some(authed_cookie().as_str())));
    let post = &reqs[2];
    assert_eq!(post.target, "/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/78500000/12345/R/1?A=1&B=2");
    // The form body in document order, with XDAYS/PICKUP overridden in place on step 0.
    let mut expected = parse_reserve_form(&form_html, &server.origin).unwrap().body;
    expected.insert("XDAYS".into(), "730".into());
    expected.insert("PICKUP".into(), "25379".into());
    assert_eq!(post.form(), expected.into_iter().collect::<Vec<_>>());
}

#[tokio::test]
async fn place_reservation_stops_when_the_same_step_repeats() {
    // A validation error re-renders the same step (same action).
    let rejected = format!(
        "<p class=\"alert\">Please choose a pickup location.</p>{}",
        wizard_form("/wiz/1", "c1")
    );
    let page = rejected.clone();
    let server = Server::start(move |req| match req.method.as_str() {
        "GET" => Reply::ok(wizard_form("/wiz/1", "c0")),
        _ => Reply::ok(page.clone()),
    });
    if !reserve_parser_ready(&server.origin) {
        return;
    }
    let http = Http::new(&server.origin).unwrap();
    let result =
        http.place_reservation(&format!("{}/r", server.origin), SESSION, None, None).await.unwrap();
    assert_eq!(result, parse_reserve_result(&rejected));
    assert_eq!(server.requests().iter().filter(|r| r.method == "POST").count(), 1);
}

#[tokio::test]
async fn place_reservation_submits_at_most_three_steps_overriding_only_the_first() {
    let server = Server::start(|req| match (req.method.as_str(), req.path()) {
        ("GET", _) => Reply::ok(wizard_form("/wiz/1", "c1")),
        ("POST", path) => {
            let n: u32 = path.trim_start_matches("/wiz/").parse().unwrap();
            Reply::ok(wizard_form(&format!("/wiz/{}", n + 1), &format!("c{}", n + 1)))
        }
        _ => Reply::status(404, "unexpected"),
    });
    if !reserve_parser_ready(&server.origin) {
        return;
    }
    let http = Http::new(&server.origin).unwrap();
    let result = http
        .place_reservation(&format!("{}/r", server.origin), SESSION, Some("90".into()), None)
        .await
        .unwrap();
    assert_eq!(result, parse_reserve_result(&wizard_form("/wiz/4", "c4")));

    let posts: Vec<_> = server.requests().into_iter().filter(|r| r.method == "POST").collect();
    assert_eq!(
        posts.iter().map(|r| r.path().to_owned()).collect::<Vec<_>>(),
        ["/wiz/1", "/wiz/2", "/wiz/3"]
    );
    let xdays: Vec<_> = posts.iter().map(|r| r.form_value("XDAYS").unwrap()).collect();
    assert_eq!(xdays, ["90", "0", "0"]);
    assert!(
        posts.iter().all(|r| r.form_value("PICKUP").is_none()),
        "no PICKUP override → none added"
    );
    let csrf: Vec<_> = posts.iter().map(|r| r.form_value("csrf").unwrap()).collect();
    assert_eq!(csrf, ["c1", "c2", "c3"], "each step posts its own form's csrf");
}

#[tokio::test]
async fn cancel_posts_the_confirm_form_to_its_action() {
    let confirm =
        wizard_form("/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/78500000/0/C/6/RSVCENQ", "cancel-csrf");
    let success = common::fixture("synthetic/manage-cancel-success.html");
    let page = success.clone();
    let server = Server::start(move |req| match (req.method.as_str(), req.path()) {
        ("GET", "/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/78500000/0/C/5/RSVCENQ") => {
            Reply::ok(confirm.clone())
        }
        ("POST", "/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/78500000/0/C/6/RSVCENQ") => {
            Reply::redirect(303, "/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/78500000/0/C/7/RSVCENQ")
        }
        ("GET", "/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/78500000/0/C/7/RSVCENQ") => {
            Reply::ok(page.clone())
        }
        _ => Reply::status(404, "unexpected"),
    });
    if !reserve_parser_ready(&server.origin) {
        return;
    }
    let http = Http::new(&server.origin).unwrap();
    let url = format!(
        "{}/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/78500000/0/C/5/RSVCENQ?SVL=987",
        server.origin
    );
    let result = http.cancel_reservation(&url, SESSION).await.unwrap();
    assert_eq!(result, parse_manage_result(&success, ManageKind::Cancel));

    let reqs = server.requests();
    assert_eq!(reqs.iter().map(|r| r.method.as_str()).collect::<Vec<_>>(), ["GET", "POST", "GET"]);
    assert!(reqs.iter().all(|r| r.header("cookie") == Some(authed_cookie().as_str())));
    let form: Vec<(String, String)> = reqs[1].form();
    assert_eq!(
        form,
        [
            ("csrf".to_owned(), "cancel-csrf".to_owned()),
            ("XDAYS".to_owned(), "0".to_owned()),
            ("submitButton".to_owned(), "Continue".to_owned())
        ]
    );
}

#[tokio::test]
async fn wizard_post_follows_at_most_four_redirect_hops() {
    let server = Server::start(|req| match (req.method.as_str(), req.path()) {
        ("GET", "/confirm") => Reply::ok(wizard_form("/commit", "x")),
        ("POST", "/commit") => Reply::redirect(302, "/hop/1"),
        ("GET", path) => {
            let n: u32 = path.trim_start_matches("/hop/").parse().unwrap();
            Reply::redirect(302, &format!("/hop/{}", n + 1))
        }
        _ => Reply::status(404, "unexpected"),
    });
    if !reserve_parser_ready(&server.origin) {
        return;
    }
    let http = Http::new(&server.origin).unwrap();
    let result =
        http.cancel_reservation(&format!("{}/confirm", server.origin), SESSION).await.unwrap();
    // The 4th hop's (still 302) body is what gets parsed.
    assert_eq!(result, parse_manage_result(mock::OBJECT_MOVED, ManageKind::Cancel));
    let paths: Vec<_> = server.requests().iter().map(|r| r.path().to_owned()).collect();
    assert_eq!(paths, ["/confirm", "/commit", "/hop/1", "/hop/2", "/hop/3", "/hop/4"]);
}

#[tokio::test]
async fn wizard_form_action_outside_the_origin_is_never_posted() {
    let other = Server::start(|_| Reply::ok(PLACED));
    let action = format!("{}/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/1/0/C/6/RSVCENQ", other.origin);
    let server = Server::start(move |_| Reply::ok(wizard_form(&action, "csrf")));
    if parse_reserve_form(&wizard_form("/x", "c"), &server.origin).is_none() {
        eprintln!("skipping: parse::reserve::parse_reserve_form is still a stub");
        return;
    }
    let http = Http::new(&server.origin).unwrap();
    let url = format!("{}/confirm", server.origin);
    let forbidden = |r: Result<ReserveResult, SpydusError>| {
        assert!(matches!(r, Err(SpydusError::OriginForbidden)), "{r:?}")
    };
    forbidden(http.place_reservation(&url, SESSION, Some("30".into()), None).await);
    forbidden(http.cancel_reservation(&url, SESSION).await);
    forbidden(http.edit_reservation(&url, SESSION, &EditOverrides::default()).await);

    assert!(other.requests().is_empty(), "no POST left the origin");
    let reqs = server.requests();
    assert_eq!(reqs.len(), 3, "only the three form GETs");
    assert!(reqs.iter().all(|r| r.method == "GET"));
}

#[tokio::test]
async fn wizard_location_hop_outside_the_origin_is_refused() {
    let other = Server::start(|_| Reply::ok(PLACED));
    let elsewhere = format!("{}/cgi-bin/spydus.exe/ENQ/OPAC/RSVCENQ", other.localhost_origin());
    let server = Server::start(move |req| match (req.method.as_str(), req.path()) {
        ("GET", "/confirm") => Reply::ok(wizard_form("/commit", "x")),
        ("POST", "/commit") => Reply::redirect(302, "/hop"),
        ("GET", "/hop") => Reply::redirect(302, &elsewhere),
        _ => Reply::status(404, "unexpected"),
    });
    if !reserve_parser_ready(&server.origin) {
        return;
    }
    let http = Http::new(&server.origin).unwrap();
    let url = format!("{}/confirm", server.origin);
    assert!(matches!(
        http.cancel_reservation(&url, SESSION).await,
        Err(SpydusError::OriginForbidden)
    ));
    assert!(other.requests().is_empty());
    let paths: Vec<_> = server.requests().iter().map(|r| r.path().to_owned()).collect();
    assert_eq!(paths, ["/confirm", "/commit", "/hop"], "same-origin hops are followed");
}

#[tokio::test]
async fn edit_applies_overrides_and_clear_flags() {
    let form_html = common::fixture("synthetic/edit-form.html");
    let success = common::fixture("synthetic/manage-edit-success.html");
    let (page, done) = (form_html.clone(), success.clone());
    let server = Server::start(move |req| match (req.method.as_str(), req.path()) {
        ("GET", "/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/78500000/0/ER/1/RSVCENQ") => {
            Reply::ok(page.clone())
        }
        ("POST", "/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/78500000/0/ER/2/RSVCENQ") => {
            Reply::ok(done.clone())
        }
        _ => Reply::status(404, "unexpected"),
    });
    if !reserve_parser_ready(&server.origin) {
        return;
    }
    let http = Http::new(&server.origin).unwrap();
    let url = format!(
        "{}/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/78500000/0/ER/1/RSVCENQ?SVL=987",
        server.origin
    );

    let all = EditOverrides {
        pickup: Some("25379".into()),
        delay_until: Some("2026-12-01".into()),
        not_required_after: Some("2027-01-31".into()),
        clear_delay: true,
        clear_expiry: true,
    };
    let result = http.edit_reservation(&url, SESSION, &all).await.unwrap();
    assert_eq!(result, parse_manage_result(&success, ManageKind::Edit));
    // No overrides → the form body as served.
    http.edit_reservation(&url, SESSION, &EditOverrides::default()).await.unwrap();

    let posts: Vec<_> = server.requests().into_iter().filter(|r| r.method == "POST").collect();
    assert_eq!(posts.len(), 2);
    let form = parse_edit_form(&form_html, &server.origin).unwrap();
    let mut expected = form.body.clone();
    expected.insert("PICKUP".into(), "25379".into());
    expected.insert("SDTE".into(), "2026-12-01".into());
    expected.insert("XDTE".into(), "2027-01-31".into());
    expected.insert("CLRSDTE".into(), "1".into());
    expected.insert("CLRXDTE".into(), "1".into());
    assert_eq!(posts[0].form(), expected.into_iter().collect::<Vec<_>>());
    assert_eq!(posts[1].form(), form.body.into_iter().collect::<Vec<_>>());
    assert!(posts.iter().all(|r| r.header("cookie") == Some(authed_cookie().as_str())));
}

// --- AWS WAF (x-amzn-waf-action) -------------------------------------------------------------

const WAF_PAGE: &str = "<html><head><title>Human Verification</title></head></html>";

fn human_verification<T: std::fmt::Debug>(result: Result<T, SpydusError>) {
    assert!(matches!(result, Err(SpydusError::HumanVerificationRequired)), "{result:?}");
}

#[test]
fn human_verification_display() {
    assert_eq!(SpydusError::HumanVerificationRequired.to_string(), "human verification required");
}

#[tokio::test]
async fn aws_waf_answer_is_human_verification_required_on_any_status() {
    let server = Server::start(|req| match req.path() {
        // The UK libraries: 405 + captcha on every request.
        "/captcha" | "/cgi-bin/spydus.exe/ENQ/OPAC/ALLWRKENQ" | "/docs/OPAC/js/encrypt.js" => {
            Reply::status(405, WAF_PAGE).header("x-amzn-waf-action", "captcha")
        }
        "/challenge" => Reply::ok(WAF_PAGE).header("X-Amzn-Waf-Action", "challenge"),
        "/accepted" => Reply::status(202, "").header("x-amzn-waf-action", "challenge"),
        "/via-redirect" => Reply::redirect(302, "/captcha"),
        "/plain-405" => Reply::status(405, "Method Not Allowed"),
        _ => Reply::status(404, "unexpected"),
    });
    let http = Http::new(&server.origin).unwrap();
    let at = |path: &str| format!("{}{path}", server.origin);

    human_verification(http.fetch_search_html("harry potter").await);
    for path in ["/captcha", "/challenge", "/accepted", "/via-redirect"] {
        human_verification(http.fetch_fragment(&at(path)).await);
        human_verification(http.fetch_authed(&at(path), SESSION).await);
    }
    human_verification(http.login("user", "pin").await);
    assert!(server.requests().iter().all(|r| r.method == "GET"), "no login POST after a captcha");

    // Detection is the header, not the status.
    match http.fetch_fragment(&at("/plain-405")).await {
        Err(SpydusError::Upstream { message }) => assert_eq!(message, "fragment 405"),
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn aws_waf_answer_mid_wizard_is_human_verification_required() {
    let server = Server::start(|req| match (req.method.as_str(), req.path()) {
        ("GET", "/confirm") => Reply::ok(wizard_form("/commit", "x")),
        ("POST", "/commit") => Reply::status(405, WAF_PAGE).header("x-amzn-waf-action", "captcha"),
        ("GET", "/confirm-2") => Reply::ok(wizard_form("/commit-2", "x")),
        ("POST", "/commit-2") => Reply::redirect(302, "/done"),
        ("GET", "/done") => Reply::ok(WAF_PAGE).header("x-amzn-waf-action", "challenge"),
        _ => Reply::status(404, "unexpected"),
    });
    if !reserve_parser_ready(&server.origin) {
        return;
    }
    let http = Http::new(&server.origin).unwrap();
    human_verification(
        http.cancel_reservation(&format!("{}/confirm", server.origin), SESSION).await,
    );
    human_verification(
        http.cancel_reservation(&format!("{}/confirm-2", server.origin), SESSION).await,
    );
}

// --- timeouts ---------------------------------------------------------------------------------

#[test]
fn default_timeouts() {
    assert_eq!(config::CONNECT_TIMEOUT, Duration::from_secs(10));
    assert_eq!(config::REQUEST_TIMEOUT, Duration::from_secs(60));
}

fn timed_out(result: Result<String, SpydusError>, expected_prefix: &str) {
    match result {
        Err(SpydusError::Network { message }) => {
            assert!(message.starts_with(expected_prefix), "{message}")
        }
        other => panic!("expected a Network timeout, got {other:?}"),
    }
}

#[tokio::test]
async fn total_timeout_covers_a_silent_server_and_a_stalled_body() {
    for stall in [Stall::Response, Stall::Body] {
        let origin = format!("http://{}", mock::stalled(stall));
        let http = Http::with_timeouts(&origin, Duration::from_secs(5), Duration::from_millis(300))
            .unwrap();
        let started = Instant::now();
        timed_out(
            http.fetch_fragment(&format!("{origin}/x")).await,
            "request timed out after 300ms",
        );
        let elapsed = started.elapsed();
        assert!(elapsed < Duration::from_secs(3), "{stall:?}: took {elapsed:?}");
    }
}

#[tokio::test]
async fn connect_timeout_covers_a_server_that_never_completes_the_handshake() {
    // reqwest's connect timeout spans TCP + TLS; a peer that accepts TCP and then never answers
    // the ClientHello is a connect that never completes (macOS resets a full accept queue, so
    // a hung SYN can't be staged on loopback).
    let origin = format!("https://{}", mock::stalled(Stall::Silent));
    let http =
        Http::with_timeouts(&origin, Duration::from_millis(200), Duration::from_secs(30)).unwrap();
    let started = Instant::now();
    timed_out(http.fetch_fragment(&format!("{origin}/x")).await, "connect timed out after 200ms");
    let elapsed = started.elapsed();
    assert!(elapsed < Duration::from_secs(5), "took {elapsed:?}");
}

// --- set_extra_cookie (verification cookie) -----------------------------------------------------

const WAF_TOKEN: &str = "aws-waf-token=0f1e2d3c-test:AAAAAAAA:BBBB==";

#[test]
fn http_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Http>();
}

#[tokio::test]
async fn extra_cookie_rides_on_every_request_until_cleared() {
    let js = encrypt_js();
    let server = Server::start(move |req| match (req.method.as_str(), req.path()) {
        ("GET", "/docs/OPAC/js/encrypt.js") => Reply::ok(js.clone()),
        ("POST", "/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/LB/2") => Reply::redirect(302, LOGIN_RDT),
        ("GET", "/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/LB/1") => {
            Reply::ok("").header("Set-Cookie", "SPYDUS_BRWSESSIONID_443=HOP; path=/")
        }
        ("GET", "/start") => Reply::redirect(302, "/landing"),
        _ => Reply::ok("page"),
    });
    let http = Arc::new(Http::new(&server.origin).unwrap());
    let at = |path: &str| format!("{}{path}", server.origin);

    // Set from another thread: the setter is `&self` and thread-safe.
    let setter = Arc::clone(&http);
    thread::spawn(move || setter.set_extra_cookie(Some(format!("  {WAF_TOKEN} ")))).join().unwrap();
    http.fetch_search_html("x").await.unwrap();
    http.fetch_fragment(&at("/start")).await.unwrap();
    http.fetch_authed(&at("/start"), SESSION).await.unwrap();
    assert_eq!(http.login("user", "pin").await.unwrap(), "HOP");

    let with_token = |base: &str| format!("{base}; {WAF_TOKEN}");
    let cookies: Vec<_> =
        server.requests().iter().map(|r| r.header("cookie").map(str::to_owned)).collect();
    assert_eq!(
        cookies,
        [
            Some(WAF_TOKEN.to_owned()),         // search
            Some(WAF_TOKEN.to_owned()),         // fragment
            Some(WAF_TOKEN.to_owned()),         // …and its same-origin redirect
            Some(with_token(&authed_cookie())), // authed
            Some(with_token(&authed_cookie())), // …and its same-origin redirect
            Some(WAF_TOKEN.to_owned()),         // encrypt.js
            Some(with_token(UI_COOKIES)),       // login POST
            Some(with_token(UI_COOKIES)),       // login hop
        ]
    );

    // Cleared by `None` and by a blank string.
    for cleared in [None, Some("  ".to_owned())] {
        http.set_extra_cookie(Some(WAF_TOKEN.to_owned()));
        http.set_extra_cookie(cleared);
        let before = server.requests().len();
        http.fetch_search_html("x").await.unwrap();
        http.fetch_fragment(&at("/page")).await.unwrap();
        http.fetch_authed(&at("/page"), SESSION).await.unwrap();
        let cookies: Vec<_> = server.requests()[before..]
            .iter()
            .map(|r| r.header("cookie").map(str::to_owned))
            .collect();
        assert_eq!(cookies, [None, None, Some(authed_cookie())]);
    }
}

#[tokio::test]
async fn extra_cookie_rides_on_wizard_posts_and_hops() {
    let server = Server::start(|req| match (req.method.as_str(), req.path()) {
        ("GET", "/confirm") => Reply::ok(wizard_form("/commit", "x")),
        ("POST", "/commit") => Reply::redirect(303, "/done"),
        _ => Reply::ok(PLACED),
    });
    if !reserve_parser_ready(&server.origin) {
        return;
    }
    let http = Http::new(&server.origin).unwrap();
    http.set_extra_cookie(Some(WAF_TOKEN.to_owned()));
    http.cancel_reservation(&format!("{}/confirm", server.origin), SESSION).await.unwrap();
    let reqs = server.requests();
    assert_eq!(reqs.iter().map(|r| r.method.as_str()).collect::<Vec<_>>(), ["GET", "POST", "GET"]);
    let expected = format!("{}; {WAF_TOKEN}", authed_cookie());
    assert!(reqs.iter().all(|r| r.header("cookie") == Some(expected.as_str())));
}

// --- live (public GETs only) -----------------------------------------------------------------

#[tokio::test]
#[ignore = "live: one public GET to canning.spydus.com"]
async fn live_search_returns_card_list_html() {
    let http = Http::new(config::ORIGIN).unwrap();
    let html = http.fetch_search_html("harry potter").await.unwrap();
    assert!(html.contains("card-list"));
}

#[tokio::test]
#[ignore = "live: one public GET to hutt.spydus.co.nz (no discover API needed)"]
async fn live_direct_search_on_hutt_returns_card_list_html() {
    let http = Http::new("https://hutt.spydus.co.nz").unwrap();
    let html = http.fetch_search_html("harry potter").await.unwrap();
    assert!(html.contains("card-list"));
}

#[tokio::test]
#[ignore = "live: one public GET to stockport.spydus.co.uk (AWS WAF captcha)"]
async fn live_uk_library_is_human_verification_required() {
    let http = Http::new("https://stockport.spydus.co.uk").unwrap();
    human_verification(http.fetch_search_html("harry potter").await);
}

/// A minimal, sequential HTTP/1.1 server on 127.0.0.1 (one request per connection) that records
/// every request and answers from a closure. std-only, so no test dependency is needed. Also
/// servers that never (fully) answer, for the timeout tests.
mod mock {
    use std::io::{self, BufRead, BufReader, Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::{Arc, Mutex};
    use std::thread;

    pub const OBJECT_MOVED: &str =
        "<head><title>Object moved</title></head><body><h1>Object Moved</h1></body>";

    #[derive(Debug, Clone)]
    pub struct Request {
        pub method: String,
        /// Origin-form request target: path + optional `?query`.
        pub target: String,
        pub headers: Vec<(String, String)>,
        pub body: String,
    }

    impl Request {
        pub fn path(&self) -> &str {
            self.target.split('?').next().unwrap_or_default()
        }

        pub fn query(&self) -> Option<&str> {
            self.target.split_once('?').map(|(_, q)| q)
        }

        pub fn header(&self, name: &str) -> Option<&str> {
            self.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
        }

        /// The `application/x-www-form-urlencoded` body, decoded, in order.
        pub fn form(&self) -> Vec<(String, String)> {
            decode_form(&self.body)
        }

        pub fn form_value(&self, name: &str) -> Option<String> {
            self.form().into_iter().find(|(k, _)| k == name).map(|(_, v)| v)
        }

        /// The query string, form-decoded, in order.
        pub fn query_pairs(&self) -> Vec<(String, String)> {
            decode_form(self.query().unwrap_or_default())
        }
    }

    fn decode_form(encoded: &str) -> Vec<(String, String)> {
        reqwest::Url::parse(&format!("http://form/?{encoded}"))
            .unwrap()
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect()
    }

    #[derive(Debug, Clone)]
    pub struct Reply {
        status: u16,
        headers: Vec<(String, String)>,
        body: String,
    }

    impl Reply {
        pub fn ok(body: impl Into<String>) -> Self {
            Self::status(200, body)
        }

        pub fn status(status: u16, body: impl Into<String>) -> Self {
            Self { status, headers: Vec::new(), body: body.into() }
        }

        /// Spydus answers redirects with an IIS "Object Moved" body.
        pub fn redirect(status: u16, location: &str) -> Self {
            Self::status(status, OBJECT_MOVED).header("Location", location)
        }

        pub fn header(mut self, name: &str, value: &str) -> Self {
            self.headers.push((name.to_owned(), value.to_owned()));
            self
        }
    }

    pub struct Server {
        /// `http://127.0.0.1:{port}`
        pub origin: String,
        log: Arc<Mutex<Vec<Request>>>,
    }

    impl Server {
        pub fn start(handler: impl Fn(&Request) -> Reply + Send + 'static) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1");
            let origin = format!("http://{}", listener.local_addr().unwrap());
            let log = Arc::new(Mutex::new(Vec::new()));
            let server_log = Arc::clone(&log);
            thread::spawn(move || {
                for stream in listener.incoming().flatten() {
                    let _ = serve(stream, &handler, &server_log);
                }
            });
            Self { origin, log }
        }

        /// `http://127.0.0.1:{port}` → `http://localhost:{port}`: the same server under another
        /// origin (another host for reqwest and for the guard).
        pub fn localhost_origin(&self) -> String {
            self.origin.replace("127.0.0.1", "localhost")
        }

        pub fn requests(&self) -> Vec<Request> {
            self.log.lock().unwrap().clone()
        }
    }

    /// How a [`stalled`] server misbehaves.
    #[derive(Clone, Copy, Debug)]
    pub enum Stall {
        /// Accepts the TCP connection, then never reads or writes (a TLS handshake hangs).
        Silent,
        /// Reads the request, never answers.
        Response,
        /// Reads the request, sends the headers and only part of the promised body.
        Body,
    }

    /// A server that holds every connection open without (fully) answering. Returns its
    /// `host:port`.
    pub fn stalled(stall: Stall) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1");
        let addr = listener.local_addr().unwrap().to_string();
        thread::spawn(move || {
            let mut held = Vec::new();
            for mut stream in listener.incoming().flatten() {
                let read = !matches!(stall, Stall::Silent) && read_request(&stream).is_ok();
                if read && matches!(stall, Stall::Body) {
                    let head = "HTTP/1.1 200 Mock\r\nContent-Length: 1000\r\n\r\n<html>partial";
                    let _ = stream.write_all(head.as_bytes()).and_then(|()| stream.flush());
                }
                held.push(stream);
            }
        });
        addr
    }

    fn read_request(stream: &TcpStream) -> io::Result<Request> {
        let mut reader = BufReader::new(stream.try_clone()?);
        let mut line = String::new();
        reader.read_line(&mut line)?;
        let mut parts = line.split_whitespace();
        let (method, target) = (
            parts.next().unwrap_or_default().to_owned(),
            parts.next().unwrap_or_default().to_owned(),
        );
        let mut headers = Vec::new();
        loop {
            line.clear();
            reader.read_line(&mut line)?;
            let header = line.trim_end_matches(['\r', '\n']);
            if header.is_empty() {
                break;
            }
            if let Some((name, value)) = header.split_once(':') {
                headers.push((name.trim().to_owned(), value.trim().to_owned()));
            }
        }
        let len = headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
            .and_then(|(_, v)| v.parse().ok())
            .unwrap_or(0);
        let mut body = vec![0; len];
        reader.read_exact(&mut body)?;
        Ok(Request { method, target, headers, body: String::from_utf8_lossy(&body).into_owned() })
    }

    fn serve(
        mut stream: TcpStream,
        handler: &impl Fn(&Request) -> Reply,
        log: &Mutex<Vec<Request>>,
    ) -> io::Result<()> {
        let request = read_request(&stream)?;
        let reply = handler(&request);
        log.lock().unwrap().push(request);

        let mut out = format!("HTTP/1.1 {} Mock\r\n", reply.status);
        for (name, value) in &reply.headers {
            out.push_str(&format!("{name}: {value}\r\n"));
        }
        out.push_str(&format!(
            "Content-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            reply.body.len(),
            reply.body
        ));
        stream.write_all(out.as_bytes())?;
        stream.flush()
    }
}
