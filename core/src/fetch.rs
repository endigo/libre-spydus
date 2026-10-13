//! Spydus HTTP layer (`middleware/src/fetch/{spydus,auth}.ts`). Owner: task E.
//!
//! Cookies travel as explicit `Cookie:` header strings, exactly like the middleware (no jar):
//! public reads send none, the login POST sends `UI_COOKIES`, and authed calls send
//! `SPYDUS_BRWSESSIONID_443={session}; {UI_COOKIES}`. The verification cookie
//! ([`Http::set_extra_cookie`]) is appended to every request's header. The redirect-following
//! client keeps that header on same-origin hops, which the reserve GET relies on.
//!
//! Every request goes through `Http::request` + `Http::send`: a URL outside the origin is
//! refused before anything is sent (manual `Location` hops and form actions included), the
//! following client never follows a redirect off the origin, transport failures and timeouts
//! become `Network`, and an AWS WAF answer becomes `HumanVerificationRequired` on any status.
//!
//! Safety (contract §3): never POST to the live site from tests; never use a real session.

use std::error::Error as _;
use std::sync::{LazyLock, PoisonError, RwLock};
use std::time::Duration;

use indexmap::IndexMap;
use regex::{NoExpand, Regex};
use reqwest::header::{ACCEPT, COOKIE, HeaderMap, LOCATION, SET_COOKIE};
use reqwest::{Client, Method, RequestBuilder, Response, redirect};

use crate::config;
use crate::crypto::{public_key_pem_from_encrypt_js, rsa_encrypt_base64};
use crate::error::SpydusError;
use crate::timing;
use crate::parse::account::parse_account_links;
use crate::parse::decode::JS_WS;
use crate::parse::reserve::{
    ManageKind, parse_edit_form, parse_manage_result, parse_reserve_form, parse_reserve_result,
    to_edit_options, to_reserve_options,
};
use crate::types::{
    AccountLinks, EditOptions, EditOverrides, ReserveOptions, ReserveResult, SearchSort,
};

/// `Accept` on public reads (spydus.ts:10).
const ACCEPT_PUBLIC: &str = "text/html,application/json";
/// Marker of the reserve/manage wizard form (auth.ts:217, reserve.ts:23).
const MAIN_FORM: &str = r#"id="mainForm""#;
/// Marker of the login form a logged-out session lands on (auth.ts:161).
const LOGIN_FORM: &str = r#"id="frmLogin""#;
const SIGN_IN_AGAIN: &str = "Please sign in again.";
/// Wizard forms `place_reservation` submits at most (auth.ts:185).
const MAX_RESERVE_STEPS: usize = 3;
/// Redirect hops followed by hand after a wizard POST (auth.ts:204, 239).
const MAX_POST_HOPS: usize = 4;
/// Same-origin redirects the following client takes (reqwest's default limit).
const MAX_REDIRECTS: usize = 10;

// auth.ts:60 — `/SPYDUS_BRWSESSIONID(?:_\d+)?=([^;\s]+)/`, with JS `\d` and `\s`.
static SESSION_RE: LazyLock<Regex> = LazyLock::new(|| {
    let js_ws = &JS_WS[1..JS_WS.len() - 1]; // the class body, without its brackets
    Regex::new(&format!(r"SPYDUS_BRWSESSIONID(?:_[0-9]+)?=([^;{js_ws}]+)")).unwrap()
});
// auth.ts:311 — `/http-equiv="Refresh"[^>]*url=([^"'>]+)/i`; ASCII-only case folding like JS `/i`.
static META_REFRESH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i-u:http-equiv="Refresh")[^>]*(?i-u:url=)([^"'>]+)"#).unwrap()
});
// auth.ts:314 — first `/ENQ/{A-Z+}/BRWENQ/` only.
static BRWENQ_CONTEXT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"/ENQ/[A-Z]+/BRWENQ/").unwrap());

pub struct Http {
    origin: String,
    /// Follows same-origin redirects only (public reads, `fetch_authed`, encrypt.js).
    client: reqwest::Client,
    /// `redirect: "manual"` client (login POST + its one hop, wizard POSTs + their hops).
    no_redirect: reqwest::Client,
    connect_timeout: Duration,
    request_timeout: Duration,
    /// Verification cookie (e.g. `aws-waf-token=…`) appended to every request.
    extra_cookie: RwLock<Option<String>>,
}

impl Http {
    /// `origin` is scheme + host (a trailing `/` is dropped), e.g. [`config::ORIGIN`].
    /// Uses [`config::CONNECT_TIMEOUT`] and [`config::REQUEST_TIMEOUT`].
    pub fn new(origin: &str) -> Result<Self, SpydusError> {
        Self::with_timeouts(origin, config::CONNECT_TIMEOUT, config::REQUEST_TIMEOUT)
    }

    /// [`Http::new`] with explicit connect and total per-request timeouts.
    pub fn with_timeouts(
        origin: &str,
        connect_timeout: Duration,
        request_timeout: Duration,
    ) -> Result<Self, SpydusError> {
        let origin = origin.trim_end_matches('/');
        match reqwest::Url::parse(origin) {
            Ok(url) if matches!(url.scheme(), "http" | "https") => {}
            _ => return Err(SpydusError::Parse { message: format!("invalid origin {origin:?}") }),
        }
        let build = |policy| {
            Client::builder()
                .user_agent(config::USER_AGENT)
                .redirect(policy)
                .connect_timeout(connect_timeout)
                .timeout(request_timeout)
                .build()
                .map_err(|e| SpydusError::Network { message: error_chain(&e) })
        };
        Ok(Self {
            origin: origin.to_owned(),
            client: build(same_origin_redirects(origin.to_owned()))?,
            no_redirect: build(redirect::Policy::none())?,
            connect_timeout,
            request_timeout,
            extra_cookie: RwLock::new(None),
        })
    }

    pub fn origin(&self) -> &str {
        &self.origin
    }

    /// `url.starts_with(origin + "/")`, else `OriginForbidden` (server.ts:72).
    pub fn guard(&self, url: &str) -> Result<(), SpydusError> {
        if under_origin(&self.origin, url) { Ok(()) } else { Err(SpydusError::OriginForbidden) }
    }

    /// Sets (or, with `None` / a blank string, clears) a cookie appended to the `Cookie` header
    /// of every later request, public or authed: the `aws-waf-token=…` the native app obtains
    /// once the user has completed AWS's human verification in a web view.
    pub fn set_extra_cookie(&self, cookie: Option<String>) {
        let cookie = cookie.map(|c| c.trim().to_owned()).filter(|c| !c.is_empty());
        *self.extra_cookie.write().unwrap_or_else(PoisonError::into_inner) = cookie;
    }

    /// Keyword search → results HTML: one GET of [`config::SEARCH_PATH`] with the query
    /// form-urlencoded into `ENTRY` and `GQ` (the URL spydus.ts:27-49 got from discover/search).
    /// Relevance order; [`Http::fetch_search_html_sorted`] with `Relevance` sends the same URL.
    pub async fn fetch_search_html(&self, query: &str) -> Result<String, SpydusError> {
        self.fetch_search_html_sorted(query, SearchSort::Relevance).await
    }

    /// [`Http::fetch_search_html`] in `sort` order: `SORTS` is [`config::SORTS_RELEVANCE`] or
    /// [`config::SORTS_NEWEST`], form-urlencoded.
    pub async fn fetch_search_html_sorted(
        &self,
        query: &str,
        sort: SearchSort,
    ) -> Result<String, SpydusError> {
        let sorts = match sort {
            SearchSort::Relevance => config::SORTS_RELEVANCE,
            SearchSort::Newest => config::SORTS_NEWEST,
        };
        let path = config::SEARCH_PATH
            .replace("{sorts}", &form_urlencode(sorts))
            .replace("{q}", &form_urlencode(query));
        self.get_public(&format!("{}{path}", self.origin), "results").await
    }

    /// spydus.ts:52-56 — public GET of an OPAC page/fragment (no cookie but the verification one).
    pub async fn fetch_fragment(&self, url: &str) -> Result<String, SpydusError> {
        self.get_public(url, "fragment").await
    }

    /// The Spydus mobile app's public library config (`/api/mobile/1.0/general/config`, JSON),
    /// e.g. its card `barcodeType`. Not every library has it (Hutt City, Geelong: 404 → `Upstream`).
    pub async fn fetch_mobile_config(&self) -> Result<String, SpydusError> {
        let url = format!("{}{}", self.origin, config::MOBILE_CONFIG_PATH);
        let req = self.request(&self.client, Method::GET, &url, None)?.header(ACCEPT, "application/json");
        let res = self.follow(req).await?;
        self.ok_text(res, "mobile config").await
    }

    /// auth.ts:142-146 — authed GET (follows same-origin redirects, keeping the `Cookie`).
    pub async fn fetch_authed(&self, url: &str, session: &str) -> Result<String, SpydusError> {
        let res = self.get(url, Some(&authed_cookie(session))).await?;
        self.ok_text(res, "authed GET").await
    }

    /// auth.ts:93-135 — returns the session token. The password leaves this process only
    /// RSA-encrypted and is never logged.
    pub async fn login(&self, username: &str, password: &str) -> Result<String, SpydusError> {
        // auth.ts:32-41 — the same key JSEncrypt uses in the browser.
        let res = self.get(&format!("{}{}", self.origin, config::ENCRYPT_JS), None).await?;
        let pem = public_key_pem_from_encrypt_js(&self.ok_text(res, "encrypt.js").await?)?;
        let encrypted = rsa_encrypt_base64(&pem, password)?;
        let form = [("BRWLID", username), ("BRWLPWD", &encrypted), ("RDT", config::LOGIN_RDT)];
        let login = format!("{}{}", self.origin, config::LOGIN_POST_PATH);
        let post =
            self.request(&self.no_redirect, Method::POST, &login, Some(config::UI_COOKIES))?;
        let res = self.send(post.form(&form)).await?;

        let mut jar = set_cookies(res.headers());
        let mut session = session_from_set_cookies(&jar);
        // Success may 302 to RDT with the cookie set only there: follow ONE hop carrying the
        // UI cookies + every cookie set so far (auth.ts:111-119).
        if session.is_none()
            && res.status().is_redirection()
            && let Some(location) = location(res.headers())
        {
            let carried = std::iter::once(config::UI_COOKIES)
                .chain(jar.iter().map(|c| cookie_pair(c)))
                .collect::<Vec<_>>()
                .join("; ");
            let hop_url = self.resolve_location(&location);
            let hop = self.request(&self.no_redirect, Method::GET, &hop_url, Some(&carried))?;
            let hop = self.send(hop).await?;
            jar.extend(set_cookies(hop.headers()));
            session = session_from_set_cookies(&jar);
        }
        // No session cookie → bad BRWLID/PIN or an id this library doesn't accept (auth.ts:129-133).
        session.ok_or(SpydusError::CredentialsRejected)
    }

    /// auth.ts:309-317 — dashboard meta-refresh → BRWENQ "My details" page → account links.
    /// Both GETs are timed together as `account`.
    pub async fn resolve_account(&self, session: &str) -> Result<AccountLinks, SpydusError> {
        let account_html = async {
            let entry =
                self.fetch_authed(&format!("{}{}", self.origin, config::LOGIN_RDT), session).await?;
            let refresh = meta_refresh_url(&entry).ok_or(SpydusError::NotLoggedIn)?;
            self.fetch_authed(&normalize_brwenq_url(&refresh), session).await
        };
        timing::fetch_and_parse("account", account_html, |html| {
            parse_account_links(html, &self.origin)
        })
        .await
    }

    /// auth.ts:158-165 (read-only GET of the reserve form).
    pub async fn reserve_options(
        &self,
        reserve_url: &str,
        session: &str,
    ) -> Result<ReserveOptions, SpydusError> {
        let html = self.fetch_authed(reserve_url, session).await?;
        if html.contains(LOGIN_FORM) {
            return Ok(ReserveOptions {
                can_reserve: false,
                pickup_location: None,
                available_summary: None,
                expiry_choices: Vec::new(),
                message: Some(SIGN_IN_AGAIN.to_owned()),
            });
        }
        Ok(to_reserve_options(parse_reserve_form(&html, &self.origin).as_ref()))
    }

    /// auth.ts:173-221 (mutating). Submits the form, then auto-continues through up to two
    /// further wizard forms; stops when there's no form, the same step repeats (a validation
    /// error), or the page is no longer a reserve form.
    pub async fn place_reservation(
        &self,
        reserve_url: &str,
        session: &str,
        xdays: Option<String>,
        pickup: Option<String>,
    ) -> Result<ReserveResult, SpydusError> {
        let cookie = authed_cookie(session);
        let mut form_html = self.fetch_authed(reserve_url, session).await?;
        let mut prev_action = String::new();

        for step in 0..MAX_RESERVE_STEPS {
            let Some(form) = parse_reserve_form(&form_html, &self.origin) else { break };
            if form.action == prev_action {
                break; // same step again → not progressing
            }
            let mut body = form.body;
            if step == 0 {
                if let Some(xdays) = &xdays {
                    body.insert("XDAYS".to_owned(), xdays.clone());
                }
                if let Some(pickup) = &pickup {
                    body.insert("PICKUP".to_owned(), pickup.clone());
                }
            }
            form_html = self.post_form_follow(&form.action, &body, &cookie).await?;
            prev_action = form.action;
            // No reserve form left → we've reached the result page.
            if !form_html.contains(MAIN_FORM) {
                break;
            }
        }
        Ok(parse_reserve_result(&form_html))
    }

    /// auth.ts:251-263 (mutating). GET `…/C/5/RSVCENQ?SVL=` (confirm form, side-effect-free) →
    /// POST its action (`…/C/6/RSVCENQ`) to commit.
    pub async fn cancel_reservation(
        &self,
        cancel_url: &str,
        session: &str,
    ) -> Result<ReserveResult, SpydusError> {
        let cookie = authed_cookie(session);
        let confirm_html = self.fetch_authed(cancel_url, session).await?;
        if confirm_html.contains(LOGIN_FORM) {
            return Ok(not_ok(SIGN_IN_AGAIN));
        }
        let Some(form) = parse_reserve_form(&confirm_html, &self.origin) else {
            return Ok(not_ok("This reservation can't be cancelled right now."));
        };
        let html = self.post_form_follow(&form.action, &form.body, &cookie).await?;
        Ok(parse_manage_result(&html, ManageKind::Cancel))
    }

    /// auth.ts:266-272 (read-only GET of the edit form, `…/ER/1/RSVCENQ?SVL=`).
    pub async fn edit_reservation_options(
        &self,
        edit_url: &str,
        session: &str,
    ) -> Result<EditOptions, SpydusError> {
        let html = self.fetch_authed(edit_url, session).await?;
        if html.contains(LOGIN_FORM) {
            return Ok(EditOptions {
                can_edit: false,
                pickup_choices: Vec::new(),
                pickup_selected: None,
                delay_until: None,
                not_required_after: None,
                message: Some(SIGN_IN_AGAIN.to_owned()),
            });
        }
        Ok(to_edit_options(parse_edit_form(&html, &self.origin).as_ref()))
    }

    /// auth.ts:283-303 (mutating). GET `…/ER/1/…` → apply overrides → POST its action (`…/ER/2/…`).
    pub async fn edit_reservation(
        &self,
        edit_url: &str,
        session: &str,
        ov: &EditOverrides,
    ) -> Result<ReserveResult, SpydusError> {
        let cookie = authed_cookie(session);
        let form_html = self.fetch_authed(edit_url, session).await?;
        if form_html.contains(LOGIN_FORM) {
            return Ok(not_ok(SIGN_IN_AGAIN));
        }
        let Some(form) = parse_edit_form(&form_html, &self.origin) else {
            return Ok(not_ok("This reservation can't be edited right now."));
        };

        let mut body = form.body;
        if let Some(pickup) = &ov.pickup {
            body.insert("PICKUP".to_owned(), pickup.clone());
        }
        // A blank date means "no change"; the CLR flags explicitly remove an existing one.
        if let Some(delay_until) = &ov.delay_until {
            body.insert("SDTE".to_owned(), delay_until.clone());
        }
        if let Some(not_required_after) = &ov.not_required_after {
            body.insert("XDTE".to_owned(), not_required_after.clone());
        }
        if ov.clear_delay {
            body.insert("CLRSDTE".to_owned(), "1".to_owned());
        }
        if ov.clear_expiry {
            body.insert("CLRXDTE".to_owned(), "1".to_owned());
        }

        let html = self.post_form_follow(&form.action, &body, &cookie).await?;
        Ok(parse_manage_result(&html, ManageKind::Edit))
    }

    /// Public GET (spydus.ts:9-13, 44-47, 52-55): browser UA + `Accept`, no cookie but the
    /// verification one; non-2xx → `Upstream("{label} {status}")`.
    async fn get_public(&self, url: &str, label: &str) -> Result<String, SpydusError> {
        let req = self.request(&self.client, Method::GET, url, None)?.header(ACCEPT, ACCEPT_PUBLIC);
        let res = self.follow(req).await?;
        self.ok_text(res, label).await
    }

    /// GET through the redirect-following client.
    async fn get(&self, url: &str, cookie: Option<&str>) -> Result<Response, SpydusError> {
        self.follow(self.request(&self.client, Method::GET, url, cookie)?).await
    }

    /// Sends on the following client. Its policy stops at a redirect off the origin, which is
    /// refused here rather than surfacing as a 3xx.
    async fn follow(&self, req: RequestBuilder) -> Result<Response, SpydusError> {
        let res = self.send(req).await?;
        let off_origin = res.status().is_redirection()
            && location(res.headers())
                .and_then(|loc| res.url().join(&loc).ok())
                .is_some_and(|next| !under_origin(&self.origin, next.as_str()));
        if off_origin { Err(SpydusError::OriginForbidden) } else { Ok(res) }
    }

    /// A `method` request to `url` with `cookie` (+ the verification cookie) as its `Cookie`
    /// header. A URL outside the origin is `OriginForbidden`, and nothing is sent.
    fn request(
        &self,
        client: &Client,
        method: Method,
        url: &str,
        cookie: Option<&str>,
    ) -> Result<RequestBuilder, SpydusError> {
        self.guard(url)?;
        let req = client.request(method, url);
        Ok(match self.cookie_header(cookie) {
            Some(cookie) => req.header(COOKIE, cookie),
            None => req,
        })
    }

    /// `{cookie}; {extra}`, either alone, or none.
    fn cookie_header(&self, cookie: Option<&str>) -> Option<String> {
        let extra = self.extra_cookie.read().unwrap_or_else(PoisonError::into_inner);
        match (cookie, extra.as_deref()) {
            (Some(cookie), Some(extra)) => Some(format!("{cookie}; {extra}")),
            (cookie, extra) => cookie.or(extra).map(str::to_owned),
        }
    }

    /// Sends `req`: transport failures (timeouts included) → `Network`; a response carrying
    /// [`config::WAF_ACTION_HEADER`] → `HumanVerificationRequired`, whatever its status.
    async fn send(&self, req: RequestBuilder) -> Result<Response, SpydusError> {
        let res = req.send().await.map_err(|e| self.network(e))?;
        if res.headers().contains_key(config::WAF_ACTION_HEADER) {
            return Err(SpydusError::HumanVerificationRequired);
        }
        Ok(res)
    }

    /// auth.ts:232-248 — POST a CCOPT form body (url-encoded, insertion order) and follow up to
    /// 4 redirects by hand, carrying the cookie. The status is not checked (as in the TS).
    async fn post_form_follow(
        &self,
        action: &str,
        body: &IndexMap<String, String>,
        cookie: &str,
    ) -> Result<String, SpydusError> {
        let post = self.request(&self.no_redirect, Method::POST, action, Some(cookie))?;
        let mut res = self.send(post.form(body)).await?;
        for _ in 0..MAX_POST_HOPS {
            if !res.status().is_redirection() {
                break;
            }
            let Some(loc) = location(res.headers()) else { break };
            let hop_url = self.resolve_location(&loc);
            let hop = self.request(&self.no_redirect, Method::GET, &hop_url, Some(cookie))?;
            res = self.send(hop).await?;
        }
        self.read_text(res).await
    }

    /// `loc.startsWith("http") ? loc : ORIGIN + loc` (auth.ts:114, 207, 242). The hop is then
    /// guarded, so e.g. `.evil.example/x` or `@evil.example/x` never leave the origin.
    fn resolve_location(&self, loc: &str) -> String {
        if loc.starts_with("http") { loc.to_owned() } else { format!("{}{loc}", self.origin) }
    }

    /// `res.ok ? res.text() : throw` — non-2xx → `Upstream("{label} {status}")`.
    async fn ok_text(&self, res: Response, label: &str) -> Result<String, SpydusError> {
        let status = res.status();
        if !status.is_success() {
            return Err(SpydusError::Upstream { message: format!("{label} {}", status.as_u16()) });
        }
        self.read_text(res).await
    }

    /// `Response.text()`: always UTF-8 (lossy), leading BOM dropped — whatever the charset header.
    async fn read_text(&self, res: Response) -> Result<String, SpydusError> {
        let bytes = res.bytes().await.map_err(|e| self.network(e))?;
        let text = String::from_utf8_lossy(&bytes);
        Ok(text.strip_prefix('\u{FEFF}').unwrap_or(&text).to_owned())
    }

    /// Transport failure → `Network`. A timeout says which limit fired; anything else carries
    /// reqwest's source chain (its `Display` alone omits the cause).
    fn network(&self, e: reqwest::Error) -> SpydusError {
        let message = if e.is_timeout() {
            let (what, after) = if e.is_connect() {
                ("connect", self.connect_timeout)
            } else {
                ("request", self.request_timeout)
            };
            let url = e.url().map(|u| format!(" ({u})")).unwrap_or_default();
            format!("{what} timed out after {after:?}{url}")
        } else {
            error_chain(&e)
        };
        SpydusError::Network { message }
    }
}

/// The following client's policy: same-origin hops only (a cross-origin one is not followed and
/// `Http::follow` refuses it), at most [`MAX_REDIRECTS`] of them.
fn same_origin_redirects(origin: String) -> redirect::Policy {
    redirect::Policy::custom(move |attempt| {
        if !under_origin(&origin, attempt.url().as_str()) {
            attempt.stop()
        } else if attempt.previous().len() > MAX_REDIRECTS {
            attempt.error("too many redirects")
        } else {
            attempt.follow()
        }
    })
}

/// `url.starts_with(origin + "/")`.
fn under_origin(origin: &str, url: &str) -> bool {
    url.strip_prefix(origin).is_some_and(|rest| rest.starts_with('/'))
}

/// First non-empty `SPYDUS_BRWSESSIONID(?:_\d+)?=([^;\s]+)` value (auth.ts:60-68).
pub fn session_from_set_cookies(set_cookies: &[String]) -> Option<String> {
    set_cookies.iter().find_map(|c| SESSION_RE.captures(c).map(|m| m[1].to_owned()))
}

/// `http-equiv="Refresh"[^>]*url=([^"'>]+)` (case-insensitive) target (auth.ts:311).
pub fn meta_refresh_url(html: &str) -> Option<String> {
    META_REFRESH.captures(html).map(|m| m[1].to_owned())
}

/// `/ENQ/{ANY}/BRWENQ/` → `/ENQ/OPAC/BRWENQ/` (auth.ts:314): Canning serves the correct
/// context under OPAC, so any WQUP-style target is normalised (first occurrence only).
pub fn normalize_brwenq_url(url: &str) -> String {
    BRWENQ_CONTEXT.replace(url, NoExpand("/ENQ/OPAC/BRWENQ/")).into_owned()
}

/// auth.ts:137-139.
fn authed_cookie(session: &str) -> String {
    format!("{}={session}; {}", config::SESSION_COOKIE, config::UI_COOKIES)
}

fn not_ok(message: &str) -> ReserveResult {
    ReserveResult { ok: false, message: Some(message.to_owned()) }
}

/// Every `Set-Cookie` header value (auth.ts:51-55).
fn set_cookies(headers: &HeaderMap) -> Vec<String> {
    headers
        .get_all(SET_COOKIE)
        .iter()
        .map(|v| String::from_utf8_lossy(v.as_bytes()).into_owned())
        .collect()
}

/// `name=value` of a `Set-Cookie` line: `c.split(";", 1)[0]` (auth.ts:115).
fn cookie_pair(set_cookie: &str) -> &str {
    set_cookie.split(';').next().unwrap_or_default()
}

/// A non-empty `Location` header (JS `if (!loc)` treats "" as absent).
fn location(headers: &HeaderMap) -> Option<String> {
    headers
        .get(LOCATION)
        .map(|v| String::from_utf8_lossy(v.as_bytes()).into_owned())
        .filter(|loc| !loc.is_empty())
}

/// `e` with its source chain: `e: cause: cause…`.
fn error_chain(e: &reqwest::Error) -> String {
    let mut message = e.to_string();
    let mut source = e.source();
    while let Some(cause) = source {
        message.push_str(": ");
        message.push_str(&cause.to_string());
        source = cause.source();
    }
    message
}

/// `application/x-www-form-urlencoded` (WHATWG byte serializer, as `URLSearchParams` and
/// reqwest's `.form()`): keep `A-Z a-z 0-9 * - . _`, space → `+`, percent-encode other UTF-8
/// bytes (upper-case hex).
fn form_urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b' ' => out.push('+'),
            b'*' | b'-' | b'.' | b'_' => out.push(char::from(b)),
            _ if b.is_ascii_alphanumeric() => out.push(char::from(b)),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
