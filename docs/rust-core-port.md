# LibreSpydus Rust core — port contract

Shared spec for every worker porting the Bun/TS middleware (`middleware/`) to the Rust crate
`librespydus-core` in `core/`. **Read this whole file before editing anything.**
Approved plan: `/Users/tsogt/.claude/plans/partitioned-wondering-otter.md`.

The TS middleware is the **source of truth and parity oracle**: every Rust parser must produce
JSON identical to its TS counterpart on the same input (golden tests). Port faithfully — do not
"improve" behavior; replicate quirks (they're documented below).

---

## 1. Ownership map (concurrent workers share ONE folder + ONE crate)

| Task | Owns (may create/edit) |
|---|---|
| **A scaffold** | `core/Cargo.toml`, `core/Cargo.lock`, `core/src/lib.rs` (initial), `core/src/config.rs`, `core/src/types.rs`, `core/src/error.rs`, `core/src/parse/mod.rs`, `core/src/parse/decode.rs`, **stubs** of every other module, `core/src/bin/uniffi-bindgen.rs`, `core/tests/common/mod.rs`, `core/tests/decode.rs`, `core/tests/fixtures/**` (public + `synthetic/`), `core/tests/golden/**`, `middleware/scripts/emit-golden.ts`, `middleware/scripts/capture-fixtures.ts` |
| **B search+availability** | `core/src/parse/search.rs`, `core/src/parse/availability.rs`, `core/tests/search.rs`, `core/tests/availability.rs` |
| **C record+related** | `core/src/parse/record.rs`, `core/tests/record.rs` |
| **D reserve+account** | `core/src/parse/reserve.rs`, `core/src/parse/account.rs`, `core/tests/reserve.rs`, `core/tests/account.rs` |
| **E net layer** | `core/src/crypto.rs`, `core/src/cache.rs`, `core/src/fetch.rs`, `core/tests/crypto.rs`, `core/tests/fetch.rs` |
| **F integration** | `core/src/lib.rs` (facade), `core/src/bin/spydus-cli.rs`, `core/scripts/**`, `core/generated/**`, `core/README.md`; minimal compile fixes elsewhere only if blocking, each documented in its report |

Pre-existing (do not delete): `core/tests/fixtures/private/*` — staged by the coordinator.

## 2. Concurrency rules (B–E run in parallel against the same crate)

- **Edit only files you own.** Never touch `Cargo.toml`, `Cargo.lock`, `lib.rs`, `parse/mod.rs`,
  or another task's files. Need a new dependency or a signature change? **Ask the coordinator.**
- Keep your owned files **compiling at every save**: make small incremental edits; never leave a
  file broken while you think. Siblings are building the same crate.
- If `cargo` fails with errors in a file **you don't own**, it's a sibling mid-edit: wait ~60 s and
  retry (up to ~10 min), then ask the coordinator. Never "fix" it yourself.
- Run your own tests with `cargo test --test <name>` (from `core/`). Use `cargo check` often.
  Shared `core/target/` is fine (cargo serializes via its lock). Never `cargo clean` / `cargo update`.

## 3. Live-site safety rules (real library, real user account)

- **Never send POST** requests to `canning.spydus.com` (no login, reserve, cancel, edit, renew).
- **Never read or use** `.private/web/auth/ui-cookies.txt` or any session token. No authed requests.
- Public GETs are allowed **only** where your task says (A: capturing public fixtures; F: live oracle
  diff of public endpoints). Sequential, a handful of requests, browser UA.
- `core/tests/fixtures/private/` and `core/tests/golden/private/` hold the user's personal account
  data (name, holds, csrf). Read them only as test input; never paste their contents into reports;
  never copy them elsewhere. Tests that need them must **skip cleanly if absent** (`common::private_available()`).

## 4. Crate layout

```
core/
  Cargo.toml          # [lib] name="librespydus_core", crate-type=["lib","staticlib","cdylib"]
  src/
    lib.rs            # uniffi::setup_scaffolding!(); mod decls; SpydusClient facade (F)
    config.rs         # ORIGIN, USER_AGENT, UI_COOKIES, SESSION_COOKIE, paths
    types.rs          # UniFFI records (below)
    error.rs          # SpydusError
    cache.rs crypto.rs fetch.rs
    parse/ mod.rs decode.rs search.rs availability.rs record.rs reserve.rs account.rs
    bin/ uniffi-bindgen.rs   # fn main(){ uniffi::uniffi_bindgen_main() }
         spydus-cli.rs        # (F) host CLI printing JSON
  tests/ common/mod.rs  <module>.rs
         fixtures/{*.html, encrypt.js, synthetic/*.html, private/*.html}
         golden/{*.json, synthetic/*.json, private/*.json, MANIFEST.md}
```

Dependencies (A pins latest compatible via `cargo add`; verify it builds): `uniffi` (features
`tokio`, `cli`), `tokio` (rt-multi-thread, macros), `reqwest` (default-features=false;
`rustls-tls`, `gzip`), `lol_html`, `regex`, `rsa`, `rand`, `base64`, `serde` (derive),
`serde_json`, `thiserror`, `indexmap` (serde). Use `std::sync::LazyLock` for compiled regexes.
Docs: Context7 `/mozilla/uniffi-rs`, `/cloudflare/lol-html`; docs.rs for `rsa`/`reqwest`.

## 5. Type contract (`types.rs`) — exact

All records: `#[derive(uniffi::Record, serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]`
+ `#[serde(rename_all = "camelCase")]`. JSON must match the TS output key-for-key.
TS `x | null` → `Option<T>` (serializes `null`); arrays → `Vec<T>` (never null); JS numbers → `i64`;
`Record<string,…>` → `std::collections::HashMap`.

```rust
pub struct Showing { pub from: i64, pub to: i64 }
pub struct SearchResults { pub query: Option<String>, pub set_id: Option<String>, pub total_results: Option<i64>,
  pub showing: Option<Showing>, pub next_page_url: Option<String>, pub prev_page_url: Option<String>,
  pub records: Vec<SearchRecord> }
pub struct SearchRecord { pub rank: Option<i64>, pub work_irn: Option<String>, pub title: Option<String>,
  pub detail_url: Option<String>, pub author: Option<String>, pub author_role: Option<String>,
  pub author_url: Option<String>, pub year: Option<String>, pub language: Option<String>,
  pub summary: Option<String>, pub isbn: Option<String>, pub cover_urls: Vec<String>, pub formats: Vec<RecordFormat> }
pub struct RecordFormat { pub code: String, pub label: Option<String>, pub reserve_url: Option<String>,
  pub availability_url: Option<String> }
pub struct Availability { pub bib_record_irn: Option<String>, pub title: Option<String>, pub publisher: Option<String>,
  pub counts: HashMap<String, i64>, pub reserved: Option<i64>, pub copies: Option<i64>,
  pub holdings_url: Option<String>, pub record_url: Option<String> }
pub struct DetailValue { pub text: String, pub url: Option<String> }
pub struct DetailRow { pub label: String, pub values: Vec<DetailValue> }
pub struct RecordRelated { pub similar: Option<String>, pub author: Option<String>, pub series: Option<String>, pub versions: Option<String> }
pub struct RecordDetail { pub bib_irn: Option<String>, pub brn: Option<String>, pub title: Option<String>,
  pub series_title: Option<String>, pub isbn: Option<String>, pub cover_urls: Vec<String>,
  pub summary: Option<String>, pub details: Vec<DetailRow>, pub bookmark_url: Option<String>, pub related: RecordRelated }
pub struct RelatedItem { pub title: Option<String>, pub bib_url: Option<String>, pub brn: Option<String>,
  pub author: Option<String>, pub year: Option<String>, pub isbn: Option<String>, pub cover_urls: Vec<String> }
pub struct RelatedList { pub items: Vec<RelatedItem> }
pub struct PatronAction { pub label: String, pub href: String }
pub struct PatronItem { pub irn: Option<String>, pub bib_irn: Option<String>, pub title: Option<String>,
  pub bib_url: Option<String>, pub author: Option<String>, pub year: Option<String>, pub cover_urls: Vec<String>,
  pub isbn: Option<String>, pub fields: HashMap<String, String>, pub actions: Vec<PatronAction> }
pub struct PatronList { pub total: Option<i64>, pub items: Vec<PatronItem> }
pub struct Reservations { pub ready: PatronList, pub pending: PatronList }
pub struct AccountLinks { pub name: Option<String>, pub loans_url: Option<String>,
  pub reservations_ready_url: Option<String>, pub reservations_pending_url: Option<String> }
pub struct ExpiryChoice { pub days: String, pub label: String }
pub struct ReserveOptions { pub can_reserve: bool, pub pickup_location: Option<String>,
  pub available_summary: Option<String>, pub expiry_choices: Vec<ExpiryChoice>, pub message: Option<String> }
pub struct ReserveResult { pub ok: bool, pub message: Option<String> }
pub struct PickupChoice { pub code: String, pub name: String }
pub struct EditOptions { pub can_edit: bool, pub pickup_choices: Vec<PickupChoice>, pub pickup_selected: Option<String>,
  pub delay_until: Option<String>, pub not_required_after: Option<String>, pub message: Option<String> }
pub struct EditOverrides { pub pickup: Option<String>, pub delay_until: Option<String>,
  pub not_required_after: Option<String>, pub clear_delay: bool, pub clear_expiry: bool }
```

## 6. Error contract (`error.rs`)

```rust
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum SpydusError {
  #[error("network error: {message}")] Network { message: String },   // transport failure
  #[error("forbidden origin")] OriginForbidden,                        // url !starts_with ORIGIN+"/"
  #[error("credentials rejected")] CredentialsRejected,                // login issued no session
  #[error("not logged in")] NotLoggedIn,                               // no account meta-refresh / frmLogin
  #[error("upstream error: {message}")] Upstream { message: String },  // non-2xx, "no bib record link", …
  #[error("parse error: {message}")] Parse { message: String },
}
```
"Couldn't place/cancel/edit" is **data** (`ReserveResult{ok:false}`), never an error — mirrors HTTP 409.

> **Accepted deviation (Task A):** `SpydusError` carries `#[uniffi(flat_error)]`. A variant field named
> `message` generates Kotlin that clashes with `Throwable.message` and fails to compile. The Rust API is
> unchanged; foreign code (Swift/Kotlin) sees each variant with its `Display` string only.

### Scaffold additions available to everyone (Task A)
- Pinned: uniffi 0.32.2, tokio 1.53, reqwest 0.12.28 (rustls-tls+gzip), lol_html 3.0.1, rsa 0.9.10,
  rand 0.8 (rsa-compatible), base64 0.23, indexmap 2.14 (serde). Do not change versions.
- `config::DISCOVER_SEARCH_PATH` (the `/api/mobile/1.0/discover/search` path).
- `parse::decode::{JS_WS, js_trim, is_js_whitespace}` — use these where TS calls `.trim()` or `\s`
  so whitespace semantics match JavaScript exactly (Rust's `trim`/`\s` differ from JS).
- Extra goldens: `golden/availability-midnight.json`, `golden/biblink-bsc.json`, and synthetic
  `account-links.json` / `patron-items.json` / reserve+edit form goldens (run without private data).
  See `core/tests/golden/MANIFEST.md` for every row's fixture → TS fn → args → Rust fn, plus TS quirks.

## 7. Module signatures — exact (stubs by A, bodies by owners)

```rust
// config.rs (A) — copy exact strings from middleware/src/fetch/{auth,spydus}.ts
pub const ORIGIN: &str = "https://canning.spydus.com";
pub const USER_AGENT: &str = /* auth.ts:20-21 */;  pub const UI_COOKIES: &str = /* auth.ts:25 */;
pub const SESSION_COOKIE: &str = "SPYDUS_BRWSESSIONID_443";
pub const LOGIN_POST_PATH: &str; pub const LOGIN_RDT: &str; pub const ENCRYPT_JS: &str; // auth.ts:27-29

// parse/decode.rs (A, fully implemented)
pub fn decode(input: &str) -> String;            // decode.ts:25-38 (+named map 6-22)
pub fn collapse(input: &str) -> String;          // decode.ts:49-51
pub fn to_int(input: &str) -> Option<i64>;       // decode.ts:54-58
pub fn absolute(origin: &str, href: Option<&str>) -> Option<String>; // decode.ts:61-67
pub struct Covers { pub covers: Vec<String>, pub isbn: Option<String> }
pub fn parse_covers(data_imgurls: Option<&str>) -> Covers; // search-results.ts:77-95 (verify record.ts:9-25 & account.ts copies are identical)

// parse/search.rs (B)          pub fn parse_search_results(html: &str, origin: &str, query: Option<&str>) -> SearchResults;
// parse/availability.rs (B)    pub fn parse_availability(html: &str, origin: &str) -> Availability;
// parse/record.rs (C)
#[derive(serde::Serialize, Debug, Clone, PartialEq)] #[serde(rename_all="camelCase")]
pub struct BibLink { pub bib_url: String, pub bib_irn: String }
pub fn bib_link_from_fragment(html: &str, origin: &str) -> Option<BibLink>;
pub fn parse_record_detail(html: &str, origin: &str, bib_irn: Option<String>) -> RecordDetail;
pub fn parse_brief_cards(html: &str, origin: &str) -> Vec<RelatedItem>;
// parse/reserve.rs (D) — internal structs derive serde::Serialize camelCase (goldens compare them)
pub struct ReserveForm { pub action: String, pub body: indexmap::IndexMap<String,String>,
  pub pickup_location: Option<String>, pub available_summary: Option<String>, pub expiry_choices: Vec<ExpiryChoice> }
pub struct EditForm { pub action: String, pub body: indexmap::IndexMap<String,String>, pub pickup_choices: Vec<PickupChoice>,
  pub pickup_selected: Option<String>, pub delay_until: Option<String>, pub not_required_after: Option<String> }
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum ManageKind { Cancel, Edit }
pub fn parse_reserve_form(html: &str, origin: &str) -> Option<ReserveForm>;
pub fn to_reserve_options(form: Option<&ReserveForm>) -> ReserveOptions;
pub fn parse_edit_form(html: &str, origin: &str) -> Option<EditForm>;
pub fn to_edit_options(form: Option<&EditForm>) -> EditOptions;
pub fn parse_manage_result(html: &str, kind: ManageKind) -> ReserveResult;
pub fn parse_reserve_result(html: &str) -> ReserveResult;
// parse/account.rs (D)
pub fn parse_account_links(html: &str, origin: &str) -> AccountLinks;
pub fn parse_patron_items(html: &str, origin: &str) -> PatronList;

// crypto.rs (E)
pub fn public_key_pem_from_encrypt_js(js: &str) -> Result<String, SpydusError>; // auth.ts:32-41
pub fn rsa_encrypt_base64(pem: &str, value: &str) -> Result<String, SpydusError>; // PKCS#1 v1.5, auth.ts:44-49
// cache.rs (E) — thread-safe (Mutex), &self API
pub struct TtlCache<V: Clone + Send> { /* … */ }
impl<V: Clone + Send> TtlCache<V> { pub fn new() -> Self; pub fn get(&self, key: &str) -> Option<V>;
  pub fn insert(&self, key: String, value: V, ttl: std::time::Duration); pub fn remove(&self, key: &str); }
// fetch.rs (E)
pub struct Http { /* reqwest::Client (redirects followed) + a no-redirect client */ }
impl Http {
  pub fn new(origin: &str) -> Result<Self, SpydusError>;
  pub fn origin(&self) -> &str;
  pub fn guard(&self, url: &str) -> Result<(), SpydusError>;           // starts_with(origin + "/")
  pub async fn fetch_search_html(&self, query: &str) -> Result<String, SpydusError>; // spydus.ts:27-49
  pub async fn fetch_fragment(&self, url: &str) -> Result<String, SpydusError>;      // spydus.ts:52-56
  pub async fn fetch_authed(&self, url: &str, session: &str) -> Result<String, SpydusError>; // auth.ts:142-146
  pub async fn login(&self, username: &str, password: &str) -> Result<String, SpydusError>; // auth.ts:93-135 → session
  pub async fn resolve_account(&self, session: &str) -> Result<AccountLinks, SpydusError>; // auth.ts:309-317
  pub async fn reserve_options(&self, reserve_url: &str, session: &str) -> Result<ReserveOptions, SpydusError>;
  pub async fn place_reservation(&self, reserve_url: &str, session: &str, xdays: Option<String>, pickup: Option<String>) -> Result<ReserveResult, SpydusError>;
  pub async fn cancel_reservation(&self, cancel_url: &str, session: &str) -> Result<ReserveResult, SpydusError>;
  pub async fn edit_reservation_options(&self, edit_url: &str, session: &str) -> Result<EditOptions, SpydusError>;
  pub async fn edit_reservation(&self, edit_url: &str, session: &str, ov: &EditOverrides) -> Result<ReserveResult, SpydusError>;
}
pub fn session_from_set_cookies(set_cookies: &[String]) -> Option<String>; // SESSION_RE auth.ts:60
pub fn meta_refresh_url(html: &str) -> Option<String>;
pub fn normalize_brwenq_url(url: &str) -> String;                         // /ENQ/{ANY}/BRWENQ/ → /ENQ/OPAC/BRWENQ/
```

### Facade (`lib.rs`, F) — `#[derive(uniffi::Object)] SpydusClient`, `#[uniffi::export(async_runtime = "tokio")]`
`new(origin: Option<String>) -> Arc<Self>`; async: `search, page, availability, record, related, login,
account (→AccountLinks), loans, reservations (→Reservations), reserve_options, place_reservation,
cancel_reservation, edit_options, edit_reservation`. Replicates `middleware/src/server.ts` routing:
origin guard, `/record` branch on `[?&]FMT=RECFMT` (fragment → `bib_link_from_fragment` → bib page),
reservations ready+pending concurrently, TTL caches (search/page/record/related 10 min,
availability 60 s, `acct:{session}` 60 s, removed after place/cancel/edit).

## 8. Port notes (from the contract inventory; TS file:line)

**Engine.** `search`, `availability`, `record`, `account` use Bun `HTMLRewriter` = `lol_html` → port to
`lol_html::rewrite_str` with `element!`/`text!` handlers. State shared across handlers via
`Rc<RefCell<…>>` (end-tag handlers must be `'static`, so clone the `Rc` in). `reserve.rs` and
`bib_link_from_fragment` are **regex over raw HTML** in TS → port with `regex`, same patterns.
Descendant text IS delivered to `text!` handlers of an ancestor selector (same as Bun).

**decode.ts** — `decode` named map (amp lt gt quot apos nbsp copy reg hellip mdash ndash rsquo lsquo
ldquo rdquo) + numeric dec/hex via `/&(#x?[0-9a-f]+|[a-z][a-z0-9]*);/gi`; short-circuit if no `&`;
invalid code point → "". `collapse` = decode → `\s+`→" " → trim. `to_int` strips commas, first `-?\d+`.
`absolute`: decode+trim; pass through `^https?://`; else origin (trailing `/` stripped) + leading `/` ensured.
`parse_covers`: decode → split `**` → trim → drop empty → drop `/syndetics\.com\/index\.aspx\?isbn=\/mc\.gif/i`
→ dedupe keeping first occurrence order → isbn = first part matching `/isbn=(\d{9,13}[Xx]?)/`.

**search-results.ts** (`parseSearchResults`) — stateful `current` draft. Record boundary
`fieldset.card-list` opens a draft; its end tag pushes `finalize` (`:190-199`). rank: `fieldset.card-list
span.item-num` → `to_int` once (`:200-204`). workIrn: `input[name="SVL"]` value (`:205-209`). title:
`h3.card-title a` — first href→detailUrl, text→titleBuf (`:211-218`). `div.recdetails a` first href→authorUrl
(`:220-224`); each `div.recdetails > span.d-block` is a line (open: buf="", end: push) (`:225-238`).
`finalize` (`:97-144`): `AUTHOR_ROLE = /,\s*(author|illustrator|editor|translator|adapter|artist|writer|narrator|composer|contributor|creator)s?\.?$/i`
(`:34`) → first match → author + authorRole (lower-cased, role stripped); year `/^\d{4}(\s*[-–]\s*\d{4})?$/`;
language `/^[A-Z][a-z]+(?:,\s*[A-Z][a-z]+)*$/`. summary `div.summary .text-html` text (`:240-244`).
covers `img.imgsc[data-imgurls]` (`:246-253`). Formats keyed by code: tab labels `.card-body-tabs .nav-link`,
code from id `/^([A-Z0-9]+)-(?:tab|pane)-/i` upper-cased, label buffered+end flush (`:63-68,255-271`); reserve
link `.tab-pane a[href*="/PGM/OPAC/CCOPT/"]`, code from `[?&]RECFMT=([A-Z0-9]+)` (`:70-75,273-280`); lazy
availability `.tab-pane-url` id→code, `data-tab-href`→url (`:282-289`); new format draft has `label: ""` (`:57`).
Page-level: `.result-pages-prvnxt .nxt a` / `.prv a` first href (`:165-174`); `.result-header-brief` text
(`:176-180`); setId from first `a[href*="/OPAC/ALLWRKENQ/"]` `/\/OPAC\/ALLWRKENQ\/(\d+)/` (`:182-188`).
After: `showing` from `/Showing\s+([\d,]+)\s*-\s*([\d,]+)\s+of\s+([\d,]+)/i`; total = group3 else `records.len()` or null (`:293-297`).

**availability.ts** — counts: `span.availability-text` label (end: collapse, strip trailing `:`, lower-case
→ pending) paired with the next `span.availability-val` (end: to_int → counts[pending], clear) (`:33-56`).
title `td[data-caption="Title"] a` text, latched after first non-empty (`:58-67`). publisher: each
`.recfmt-container .col > span` buffered (`:69-81`); pick first containing ` : ` AND a 4-digit year, else
first containing ` : `, else null (`:110-113`). `a[href*="/XHLD/OPAC/"]` → holdingsUrl + bibRecordIrn
`/\/XHLD\/OPAC\/\w+\/\d+\/(\d+)/` (`:83-92`); `a[href*="/XFULL/OPAC/"]` → recordUrl + bibIrn fallback
`/\/XFULL\/OPAC\/\w+\/\d+\/(\d+)/` (`:93-104`). reserved = counts["reserved"], copies = counts["copies"].

**record.ts** — `bibLinkFromFragment`: regex `/href="([^"]*\/BIBENQ\/\d+\?[^"]*IRN[=(](\d+)[^"]*)"/` over
**`decode(html)`** (not the raw HTML — corrected by Task C) → `{bibUrl (absolute), bibIrn}` (`:32-40`). `parseRecordDetail` (`:53-173`): rows
`#divtabRECDETAILS .row` (end: flush value buf, strip trailing `:` from label, push only if label && values)
(`:63-77`); label `.fd-caption` text (`:78-82`); values `.col span.d-block` — each span one value; on open
flush previous then reset; on end flush again (handles sibling + nested) (`:84-105`); link
`.col span.d-block a[href]` first href (`:106-110`). Cover: FIRST `img.imgsc` with real covers wins
(`:112-121`). related: `a[id]` whose id starts with `urltab` → key = id[6..].lower() (`:123-132`);
**`related.versions` reads key `version`** (`:170`). summary `#divtabSUMMARY` text; the TS guard is
`summary && /^(loading\.?\.?\.?)?$/i` → a "Loading…" text is nulled, but an **empty** `#divtabSUMMARY`
stays `""` (not null — corrected by Task C) (`:134-141,153-154`). Derived (`:144-171`): title = "Main title" value split on
`" / "` first part; seriesTitle `^series`; isbn `^isbn$` value `\d{9,13}[Xx]?` else cover isbn; brn `^brn$`
else `bookmarkUrl` `BRN=(\d+)`; bookmarkUrl `^bookmark` value url else
`/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&BRN={brn}` (absolute). `parseBriefCards` (`:207-265`):
boundary `a.thumblink` (flush prev, new card, bibUrl=href) (`:218-224`); `span.card-title a` sets bibUrl +
title (`:226-238`); `img.imgsc` covers/isbn (`:239-246`); `.card-text.recdetails span.d-block` lines
(`:247-260`); final flush (`:263`). `finalizeCard` (`:192-205`): needs title+bibUrl; year regex else author;
brn from `BRN=` then `IRN[=(]`.

**reserve.ts (regex)** — `parseReserveForm` (`:22-74`): find `id="mainForm"`; slice from preceding `<form`
to `</form>`; action `action="([^"]*)"` absolutized; inputs `/<input\b[^>]*>/g` — checkbox/radio only if
`\bchecked\b` (value or "on"), others take value; selects → selected option (either attr order) else first;
submit `<button…type="submit"…name="X"…>…</button>` → body[X] = stripped text (only `type="submit"`, so
`cancelButton` type=button is never chosen); pickupLocation `id="pickuplocs"…form-static-text">([^<]+)`;
availableSummary = first `form-static-text` matching `/reservable|available/i`; expiryChoices from
`name="XDAYS"` options. `toReserveOptions` (`:76-93`). `parseEditForm` (`:114-138`) wraps it + `PICKUP`
select → choices (code=decode(value), name=collapse(text); skip empty code/name) + pickupSelected (either
attr order); delayUntil=body.SDTE, notRequiredAfter=body.XDTE (empty → null). `toEditOptions` (`:140-159`).
`parseManageResult` (`:162-185`): strip tags→collapse; `id="formValidationMsg"…>([^<]{2,160})<` (unless
"error") → fail; cancel ok = `/cancell?ed/i` && `/reservation|request|hold/i`; edit ok =
`/reservation[s]?[^.]{0,40}(updated|changed|amended|saved)/i` or `/successfully (updated|changed|amended)/i`;
**no `id="mainForm"` left ⇒ success**; else fail msg `/(cannot|could not|unable to|not allowed|error)[^.]{0,80}/i`.
`parseReserveResult` (`:188-207`) analogous (see TS for exact regexes).

**account.ts** — `parseAccountLinks` (`:16-60`): `a[href]` containing `/OPAC/`, `FMT` `/[?&]FMT=([A-Z]+)/`:
`/LOANRENQ/`+CL → loans, `/RSVCENQ/`+AR → ready, `/RSVCENQ/`+WR → pending (first wins); name from
`.navbar .dropdown-toggle-account, .account-name, #accountDropdown` text. `parsePatronItems` (`:146-247`):
one draft per `tr`; end pushes only if irn && title; irn `tr input[name="SVL"]`; title+bibUrl
`tr h3.card-title a`; lines `tr div.card-text > span.d-block`; covers `tr img.imgsc` (last wins);
fields `tr td[data-caption]` caption → collapse(text); actions `tr a.dropdown-item` (decoded+absolute href,
collapsed label, dedupe by href); total `.result-header-brief` `/of\s+([\d,]+)/i` else items.len();
finalize: author/year from lines, bibIrn `IRN[=(](\d+)` from bibUrl.

**fetch/auth.ts (E)** — login: GET encrypt.js → PEM (strip `[\\\s]`, rewrap 64 cols); POST
`LOGIN_POST_PATH` form `BRWLID, BRWLPWD=rsa(pwd), RDT=LOGIN_RDT`, Cookie UI_COOKIES, **no redirect**;
session from Set-Cookie via `/SPYDUS_BRWSESSIONID(?:_\d+)?=([^;\s]+)/` (non-empty); if none and 3xx+Location,
follow ONE hop carrying UI_COOKIES + prior name=value pairs; none ⇒ `CredentialsRejected`. Authed cookie =
`SPYDUS_BRWSESSIONID_443={session}; {UI_COOKIES}`. **`fetch_authed` follows redirects** (the reserve GET is
a `302 Object Moved` → `/CCOPT/{set}/0/V/2?RDT=…` → form; reqwest keeps `Cookie` on same-host hops).
`resolve_account`: GET `ORIGIN+LOGIN_RDT`, meta-refresh `http-equiv="Refresh"[^>]*url=([^"'>]+)` (none ⇒
`NotLoggedIn`), normalize BRWENQ, GET, `parse_account_links`. Wizards: reserve = GET form → up to 3 POST
steps (stop if form null or action repeats), step 0 overrides XDAYS/PICKUP, follow ≤4 redirects, stop when no
`id="mainForm"`, `parse_reserve_result`. cancel = GET `…/C/5/RSVCENQ?SVL=` → POST form.action (`…/C/6/…`) →
`parse_manage_result(Cancel)`. edit = GET `…/ER/1/…` → overrides `PICKUP, SDTE, XDTE, CLRSDTE="1", CLRXDTE="1"`
→ POST (`…/ER/2/…`) → `parse_manage_result(Edit)`. `id="frmLogin"` in a GET ⇒ "Please sign in again."
data result. Search: GET `/api/mobile/1.0/discover/search?keywords=` with header `spydus-lang: ""` →
JSON string path → GET `ORIGIN+path`.

## 10. Wave 2 — multi-library support + hardening (2026-10-12)

Goal: LibreSpydus works with **every** reachable Spydus library, not just Canning. Survey evidence
(see `~/.claude/projects/-Users-tsogt-Project-personal-spydus-clone/memory/spydus-multi-library-compat.md`):
- Directory `GET https://librarylist.spydus.com/api/1.0/mobile/libraries?groupCode=` → 351 entries
  `{name,id,url,logo,activationCode,location{country,…},pinningHash,groupCode}`; ~142 are Civica
  test/UAT/training instances; ~209 production (AU 103, UK 87, NZ 10, SG 5, US 4); one malformed url
  `https//singaporelib.com`.
- The mobile **discover API is not universal** (404 on many NZ/US servers) → build the OPAC search URL
  directly. Some libraries default to edition-level `ALLENQ` results, but **`ALLWRKENQ` works everywhere
  tested** and our parser handles it fully.
- UK (`*.spydus.co.uk`) answers every request with AWS WAF `405` + `x-amzn-waf-action: captcha`
  ("Human Verification"). We **never** automate or bypass it: the core reports a distinct error so the
  native app can show the page for the *user* to complete, then hands the resulting cookie back.
- Singapore servers resolve but TCP-connect times out (private school networks).

### Wave 2 ownership (G1–G3 run in parallel; G4 after all three)

| Task | Owns |
|---|---|
| **G1 parsers** | `middleware/src/parse/search-results.ts`, `middleware/src/parse/reserve.ts`, `middleware/scripts/emit-golden.ts`, `middleware/scripts/capture-fixtures.ts`, `core/src/parse/search.rs`, `core/src/parse/reserve.rs`, `core/tests/search.rs`, `core/tests/reserve.rs`, `core/tests/fixtures/multi/**`, `core/tests/golden/**` (**sole** golden regenerator) |
| **G2 net hardening** | `core/src/fetch.rs`, `core/src/config.rs`, `core/src/error.rs`, `core/tests/fetch.rs` |
| **G3 directory** | `core/src/directory.rs` (new), `core/src/types.rs` (**add `LibraryInfo` only**), `core/tests/directory.rs`, `core/tests/fixtures/directory.json`; may add exactly one line `pub mod directory;` to `core/src/lib.rs` |
| **G4 integration** | `core/src/lib.rs`, `core/src/bin/spydus-cli.rs`, `core/scripts/**`, `core/generated/**`, `core/README.md` |

Same §2 concurrency rules and §3 safety rules apply (no POST, no session, public GETs only where a task allows).

### New / changed contract

```rust
// types.rs (G3) — same derives + serde camelCase as all records
pub struct LibraryInfo {
  pub id: String, pub name: String,
  pub origin: String,          // normalized scheme://host[:port], no path/trailing slash; "https//x" repaired
  pub logo_url: Option<String>,
  pub country: Option<String>, // normalized display name: "Australia","United Kingdom","United States","New Zealand","Singapore", else raw trimmed
  pub region: String,          // "AU" | "UK" | "US" | "NZ" | "SG" | "OTHER"
  pub is_test: bool,           // Civica test/UAT/training/demo/QA instance
}
// directory.rs (G3)
pub const DIRECTORY_URL: &str = "https://librarylist.spydus.com/api/1.0/mobile/libraries?groupCode=";
pub fn parse_directory(json: &str, include_test: bool) -> Result<Vec<LibraryInfo>, SpydusError>; // pure, sorted by name
pub async fn list_libraries(include_test: bool) -> Result<Vec<LibraryInfo>, SpydusError>;          // fetch + parse

// error.rs (G2) — new variant
#[error("human verification required")] HumanVerificationRequired,   // AWS WAF `x-amzn-waf-action` present

// fetch.rs (G2)
impl Http {
  pub async fn fetch_search_html(&self, query: &str) -> Result<String, SpydusError>; // SAME signature, now builds
  //   {origin}/cgi-bin/spydus.exe/ENQ/OPAC/ALLWRKENQ?ENTRY={q}&ENTRY_NAME=BS&ENTRY_TYPE=K&SORTS=SQL_REL_GENWRK&GQ={q}
  //   (form-urlencoded, spaces as '+', exactly like Canning's discover output) — no discover API call.
  pub fn set_extra_cookie(&self, cookie: Option<String>); // e.g. "aws-waf-token=…"; appended to EVERY request's Cookie
}

// lib.rs (G4)
#[uniffi::export(async_runtime = "tokio")] pub async fn list_libraries(include_test: bool) -> Result<Vec<LibraryInfo>, SpydusError>;
impl SpydusClient { pub fn origin(&self) -> String; pub fn set_verification_cookie(&self, cookie: Option<String>); }
```

**Deliberate divergences from the original TS** (applied to **both** TS and Rust so the TS stays the oracle):
author detection broadened (G1), cancel-refusal false positive fixed (G1), setId accepts `ALLENQ` (G1).

## 9. Fixtures, goldens & tests

- Goldens are produced by running the **TS parsers** (`middleware/scripts/emit-golden.ts`, Bun) over the
  exact fixture files; `core/tests/golden/MANIFEST.md` lists fixture → TS function → args → golden.
  **Rust tests must call the Rust function with the same args** listed in the manifest.
- `tests/common/mod.rs` (A): `fixture(rel) -> String`, `golden(rel) -> serde_json::Value`,
  `assert_golden(actual: &impl Serialize, rel)` (to_value + compare; on mismatch write actual to
  `target/golden-actual/<rel>` and panic listing the first differing JSON pointers),
  `private_available() -> bool`.
- Each owner adds `tests/<module>.rs` with one golden test per manifest row plus focused unit tests for
  tricky regexes/edge cases. Live-network tests (if any) are `#[ignore]`.
