//! Upstream constants. Strings are copied verbatim from
//! `middleware/src/fetch/spydus.ts` and `middleware/src/fetch/auth.ts`.

use std::time::Duration;

/// The one library this core talks to (spydus.ts:4).
pub const ORIGIN: &str = "https://canning.spydus.com";

/// Browser UA sent on every request (auth.ts:20-21, identical to spydus.ts:6-7).
pub const USER_AGENT: &str = "Mozilla/5.0 (Linux; Android 14; Pixel 10 Pro) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/129.0.0.0 Mobile Safari/537.36";

/// UI cookies the native app assigns; authed reads need them (auth.ts:25).
pub const UI_COOKIES: &str =
    "SPYDUS_DISPLAYCONTEXT_443=MOB; consentSettings_443=ALL; SPYDUS_MYACCOUNT_443=";

/// Browse-session cookie name the authed `Cookie` header carries (auth.ts:23).
pub const SESSION_COOKIE: &str = "SPYDUS_BRWSESSIONID_443";

/// Login form POST target (auth.ts:27).
pub const LOGIN_POST_PATH: &str = "/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/LB/2";

/// Login redirect target / account dashboard entry (auth.ts:28).
pub const LOGIN_RDT: &str = "/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/LB/1?ISGLB=0";

/// Script carrying the OPAC RSA public key (auth.ts:29).
pub const ENCRYPT_JS: &str = "/docs/OPAC/js/encrypt.js";

/// Keyword search results, built directly instead of asking the mobile discover API (which 404s
/// on many libraries). Byte-for-byte what Canning's `discover/search` returns, with every `{q}`
/// replaced by the form-urlencoded query (`harry potter` → `harry+potter`). `ALLWRKENQ`
/// (work-level results) works on every library tested.
pub const SEARCH_PATH: &str = "/cgi-bin/spydus.exe/ENQ/OPAC/ALLWRKENQ?ENTRY={q}&ENTRY_NAME=BS&ENTRY_TYPE=K&SORTS=SQL_REL_GENWRK&GQ={q}";

/// TCP (+ TLS) connect timeout on every request; unreachable servers fail fast.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Total per-request timeout, from connecting until the response body is read.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

/// Response header AWS WAF sets when it answers with a captcha or challenge instead of the
/// library (`captcha` on every `*.spydus.co.uk` request, with status 405).
pub const WAF_ACTION_HEADER: &str = "x-amzn-waf-action";
