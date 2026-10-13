# librespydus-core

The shared on-device core of LibreSpydus. It's a Rust port of the Bun/TS `middleware/` that
the native iOS (SwiftUI) and Android (Compose) apps call through [UniFFI](https://github.com/mozilla/uniffi-rs).
It talks to a Spydus OPAC directly from the device, and it parses OPAC HTML into the same
camelCase JSON-shaped records the middleware returns. The apps therefore need no server in
between. Any library in Civica's public Spydus directory can be used (see
[Multi-library use](#multi-library-use)); Canning (`https://canning.spydus.com`) is the default.

The TS middleware is the **source of truth**. Every parser is golden-tested against the TS
output, and `scripts/oracle-diff.sh` diffs the two live. The porting contract is in
[`docs/rust-core-port.md`](../docs/rust-core-port.md).

## Layout

```
core/
  Cargo.toml            lib name `librespydus_core`; crate-type lib + staticlib + cdylib
  src/
    lib.rs              SpydusClient — the UniFFI facade (replicates middleware/src/server.ts)
                        + list_libraries, user_agent
    app.rs              pure helpers both apps share: digital formats, availability label,
                        due dates, home-screen genres and shelves
    config.rs           ORIGIN, USER_AGENT, UI_COOKIES, search/login/encrypt.js paths, timeouts
    types.rs            UniFFI records (SearchResults, Availability, RecordDetail, LibraryInfo, …)
                        + the SearchSort enum
    error.rs            SpydusError
    directory.rs        Civica's library directory → LibraryInfo (region, test-instance flag)
    fetch.rs            HTTP: search, fragments, authed GET, login, reservation wizards;
                        origin guard, timeouts, AWS WAF detection, verification cookie
    timing.rs           opt-in timing log (set_timing_sink): network and parse time per page
    crypto.rs           encrypt.js → PEM, RSA PKCS#1 v1.5 (JSEncrypt-compatible) PIN encryption
    cache.rs            thread-safe TTL cache
    parse/              HTML → records (search, availability, record, reserve, account, decode)
    bin/spydus-cli.rs   host CLI printing a public endpoint's JSON
    bin/uniffi-bindgen.rs
  tests/                one integration test per module + common/ helpers
    fixtures/           captured public pages, synthetic/ hand-written pages, private/ (see below)
    golden/             TS parser output for each fixture; MANIFEST.md maps fixture → TS fn → Rust fn
  scripts/oracle-diff.sh      live middleware-vs-core diff (Canning)
  scripts/library-survey.py   live multi-library compatibility survey
  generated/{swift,kotlin}  generated bindings (checked in)
```

## API

`listLibraries(includeTest)` → `[LibraryInfo]` (async): the library picker's list (see
[Multi-library use](#multi-library-use)).

`userAgent()` → `String`: the User-Agent every core request sends (`config::USER_AGENT`). Give
the human-verification web view the same one (see [Human verification](#multi-library-use)).

`SpydusClient(origin)` → one client per library; `origin` is a `LibraryInfo.origin`, and `nil`
means Canning. The async methods each return a record or throw `SpydusError`.

| method | middleware route | cache |
|---|---|---|
| `origin()` → `String` (sync) | | |
| `setVerificationCookie(cookie)` (sync) | | drops every cache |
| `search(query)` | `GET /search?q=` | 10 min, case-insensitive query |
| `searchSorted(query, sort)` | (`search` with `SORTS`) | 10 min, per sort + case-insensitive query |
| `page(url)` | `GET /page?url=` (next/prev page) | 10 min |
| `availability(url)` | `GET /availability?url=` (a format's `availabilityUrl`) | 60 s |
| `record(url)` | `GET /record?url=` (`availabilityUrl` → bib page, or a direct bib URL) | 10 min |
| `related(url)` | `GET /related?url=` (one of `RecordDetail.related.*`) → `RelatedList` | 10 min |
| `login(username, password)` → session | `POST /login` | |
| `account(session)` → `AccountLinks` | (account entry) | 60 s per session |
| `loans(session)` / `reservations(session)` | `GET /account/{loans,reservations}` | via `account` |
| `accountSnapshot(session)` → `AccountSnapshot{version, accountName?, loans, reservations, fetchedAtMs}` | — (app-only) | name + loans + both reservation lists in one call, loans/reservations concurrently; what the apps save on the device |
| `reserveOptions(url, session)` / `editOptions(url, session)` | `GET /reserve/{options,edit}` | |
| `placeReservation(url, session, xdays?, pickup?)` | `POST /reserve` | clears `account` |
| `cancelReservation(url, session)` / `editReservation(url, session, overrides)` | `POST /reserve/{cancel,edit}` | clears `account` |

- Every URL argument must start with `origin + "/"`, else `OriginForbidden`. Pass back the URLs
  that earlier calls returned.
- `SearchResults.noResults` is `true` only for the OPAC's own empty result ("Sorry, your search
  for … did not find any records."), and `records` is then empty. A page with no records and
  `noResults: false` wasn't understood: show an error, not "no results".
- `searchSorted(query, .relevance)` is exactly `search(query)` (same request, same cache entry).
  `.newest` lists the titles most recently added to the catalogue first ("new arrivals"), using
  the same order as the OPAC's own RSS feed. Later pages (`nextPageUrl` → `page`) keep the
  order.
- A blank session gives `NotLoggedIn`. A login the library doesn't accept gives `CredentialsRejected`.
- Place, cancel and edit are **mutating**, so call them only after the user confirms. If one
  can't be done, the result is `ok: false` data, not an error (the middleware's HTTP 409). A
  logged-out session gives `message: "Please sign in again."`.
- Errors: `Network` (transport, timeouts), `OriginForbidden`, `CredentialsRejected`,
  `NotLoggedIn`, `Upstream` (non-2xx, "no bib record link in format fragment"), `Parse` (bad
  input or an invalid origin), `HumanVerificationRequired` (AWS WAF challenge, see below).
  Swift and Kotlin see each variant with its display message (`flat_error`): Swift as
  `SpydusError.HumanVerificationRequired(message:)`, Kotlin as
  `SpydusException.HumanVerificationRequired`.
- Timing log: `set_timing_sink(sink)` installs a `TimingSink` (a foreign trait the app
  implements; `None` turns it off). Every page a call loads reports a `Timing`: `operation`
  (`search "q"`, `search newest "q"`, `results page`, `availability`, `record format`,
  `record`, `related`, `account`, `loans`, `reservations ready` / `pending`), `networkMs`
  (request sent → whole page received; for `account`, both of its GETs), `parseMs`, `bytes`
  (decompressed), `cached` (a cache hit: no request, times 0) and `failed` (how long the
  request took to fail). `record(sink:)` runs on the core's worker threads. The apps install
  one in debug builds only.

### App helpers (`app.rs`)

Pure functions with no I/O, so both apps label things the same way. The calibration behind each
rule is in [`docs/rust-core-port.md` §11](../docs/rust-core-port.md).

| function | returns |
|---|---|
| `isDigitalFormat(format)` → `Bool` | `true` for e-resources. The codes EBK, EAUD, EVID, EMAG, EMUS, EJNL and ECMC are digital, and BK, LP, TB, AB, PM, DVD, CD, MU, SE, MAG, OB, GR and KIT never are. An unknown code falls back to its label ("eBook", "Online …", "Streaming …", "Downloadable …"), unless the label names a CD, DVD, Playaway, kit and so on. Some libraries (Brazoria County) allow holds on e-copies, so check `reserveUrl` rather than assume. |
| `availabilitySummary(availability)` → `AvailabilitySummary{available?, copies?, reserved?, label}` | The counts plus a one-line label: "2 of 3 available", "0 of 3 available", "3 copies · 1 reserved", "1 copy", "No copies", or "No copies listed" when the OPAC shows no counts (most digital formats). |
| `parseDueDate(text)` → `SimpleDate{year, month, day}?` | A loan's `fields["Due"]` as a date. Accepts "27 Oct 2026", "27 October 2026", "27/10/2026" (day first; also `-` or `.`), "2026-10-27" and "Oct 27, 2026". Anything else, or an impossible date, is `nil`. |
| `homeGenres()` → `[Genre{key, label, query}]` | The 8 "Popular genres" tiles: `biography`, `fiction`, `science`, `history`, `fantasy`, `crime`, `cooking`, `children`. The keys are stable ids to map to icons. Run `search(query)` on a tap. |
| `homeShelves()` → `[Shelf{key, title, query, sort}]` | The home cover shelves, run with `searchSorted(query, sort)`: `new_arrivals` "New arrivals" (always first), `new_for_kids` "New for kids" and `new_biographies` "New biographies", all `.newest`. |
| `patronItemRecordUrl(item)` → `String?` | The URL that loads a loan's or reservation's bib record with `record(url)` **without a session**: `…/BIBENQ?SETLVL=&IRN={bibIrn}`. The row's own `bibUrl` (`…/BIBENQ/{setId}?QRY=IRN(…)`) uses a set from the patron's OPAC session, so `record(bibUrl)` comes back empty ("Session must be logged in"). Falls back to `bibUrl` without a numeric `bibIrn` or a BIBENQ link. |
| `accountHolding(snapshot, bibIrns)` → `AccountHolding{kind, item}?` | The loan or reservation in an `AccountSnapshot` whose `bibIrn` is one of `bibIrns`, for a book page to show "On loan" / "Reserved" instead of Reserve. `kind` is `.loan`, `.reservationReady` or `.reservationPending`; a loan wins over a ready reservation, which wins over a pending one. Pass the IRNs known for the format shown: `Availability.bibRecordIrn`, else `RecordDetail.bibIrn` and the opened row's `bibIrn`. Editions are separate bibs, so a book loan doesn't match the large print. |

Alerts (`docs/native-apps.md` §5.8):

| function | returns |
|---|---|
| `patronItemKey(item)` → `String` | A loan's or reservation's identity across refreshes: its IRN, else `bib:<bibIrn>`, else `title:<title>`, else empty. |
| `readyAlerts(snapshot, alertedKeys)` → `ReadyAlerts{newItems, alertedKeys}` | The ready-to-collect reservations whose key isn't in `alertedKeys` (one notification each), and the keys to save for next time: every reservation ready now, so one that is collected and later ready again is alerted again. When alerts are turned on, call it with no keys and save only `alertedKeys`. |
| `dueReminders(snapshot, today, daysBefore)` → `[DueReminder{key, due, remindOn, items}]` | One reminder per due date (`key` = `due:YYYY-MM-DD`), sorted by date, for loans due on or after `today`, reminded `daysBefore` days ahead or `today` if that has passed. Overdue loans and unreadable due dates are left out. |

Saving the account on the device: `encodeAccountSnapshot(snapshot)` → JSON `String` and
`decodeAccountSnapshot(json)` → `AccountSnapshot` (throws `Parse` for malformed JSON or a snapshot
written with another `ACCOUNT_SNAPSHOT_VERSION`, currently 1 — drop it and fetch a new one).

`Availability.counts` keys seen on live libraries are only `copies` (item records, any status),
`available` (on the shelf now; some libraries only, present even at 0) and `reserved`
(reservations on the title; the other libraries, only when there are any). An `available` of 0
reads "0 of N available", not "on loan", because the OPAC doesn't say why.

## Multi-library use

**Directory.** `listLibraries(includeTest: false)` makes one public GET of Civica's directory
(`https://librarylist.spydus.com/api/1.0/mobile/libraries?groupCode=`, not cached) and returns
the production libraries sorted by name. Each `LibraryInfo` has `id`, `name`, `origin`,
`logoUrl`, `country` (a display name), `region` (`AU`, `UK`, `US`, `NZ`, `SG` or `OTHER`) and
`isTest`. Civica's own test, UAT, training, demo and QA instances (154 of 351 entries) are
left out unless `includeTest` is true. Malformed directory URLs are repaired: `https//host`
becomes `https://host`.

**One client per library.** Create `SpydusClient(origin: library.origin)` when the user picks a
library, and keep it while that library is selected. `origin()` returns it normalised (no
trailing `/`). The client only talks to its own origin:
- every URL argument must start with `origin + "/"`;
- redirects are followed only within the origin, and a redirect elsewhere is `OriginForbidden`;
- `Location` hops and form actions in the login and reservation flows are guarded the same way.

A session token from one library is meaningless at another, so sign in again after switching.

Search builds the OPAC's work-level `ALLWRKENQ` results URL directly. It doesn't use the mobile
discover API, which returns 404 on many NZ and US libraries.

**Slow libraries.** Some catalogues take a long time on large result sets, whatever the sort.
Brazoria County (US) took 40–60 s for any keyword with more than about 20,000 hits (for example
`fiction`, `biography`, `crime`), against 3 s for `cooking` (11,653 hits). That is close to the
60 s request timeout, so a home screen there can fail with `Network` (timeout) until it is retried.

**Timeouts.** Every request has a 10 s connect timeout and a 60 s total timeout
(`config::CONNECT_TIMEOUT` and `config::REQUEST_TIMEOUT`; the directory GET uses the same
values). A timeout is `Network`, with a message naming the limit, for example
`network error: connect timed out after 10s (https://…)`. An unreachable library therefore
fails in about 10 s instead of hanging.

**Human verification (AWS WAF).** Some libraries sit behind AWS WAF. Every `*.spydus.co.uk`
library does, and the WAF answers each request with HTTP 405, the header
`x-amzn-waf-action: captcha` and a "Human Verification" page. Any response that carries
`x-amzn-waf-action` becomes `HumanVerificationRequired`, whatever its status. The challenge
page is never parsed as data, and the core never fetches, solves or bypasses the challenge.
The native apps follow this contract:

1. When a call throws `HumanVerificationRequired`, show `client.origin() + "/"` in an in-app
   web view (`WKWebView` / Android `WebView`). Set the web view's User-Agent to `userAgent()`
   (the core's `config::USER_AGENT`), because the WAF token may be tied to it. The **user**
   completes the verification there.
2. Read the `aws-waf-token` cookie for that host from the web view's cookie store
   (`WKHTTPCookieStore` / `CookieManager.getCookie(origin)`), then call
   `client.setVerificationCookie("aws-waf-token=<value>")`. The cookie is appended to the
   `Cookie` header of every later request from that client, public or signed in.
   `setVerificationCookie` also drops every cached response, so nothing fetched with the old
   cookie, or without one, is served again. A call that was already running finishes into the
   discarded caches.
3. Retry the call. If it throws `HumanVerificationRequired` again (the token expired, or the
   WAF doesn't accept it outside a browser), go back to step 1 once. After that, tell the user
   the library can't be used from the app right now and offer its website instead.
   Whether a WAF token works from a non-browser client is **unverified**.
4. Keep the token per client (per library). Pass `nil` to clear it, for example when the user
   switches library or the web view's cookies are cleared. Never store the token anywhere
   other than the web view's own cookie store.

**Current regional support** (from `scripts/library-survey.py`, 2026-10-12: a 21-library
stratified sample searched for "harry potter", then availability and the record on the first
format that has an availability URL):

| region | production libraries | status |
|---|---|---|
| AU | 95 | works end to end (6/6) |
| NZ | 10 | works end to end (3/4). The miss, Wairarapa Library Service, is a stale directory entry: the service split into `cartertonlibrary` and `swlibrary`, and the old host answers 404 (`Upstream`). |
| US | 4 | works end to end (3/4). The miss, "LIVE - Microsoft Lib", has no public records for any query. |
| UK | 84 | `HumanVerificationRequired` in about 1.2 s (4/4). Usable only through the verification flow above. |
| SG | 3 | unreachable. `*.spydus.com.sg` gives a connect timeout at 10 s (school/private networks), and MOE's `singaporelib.com` doesn't resolve (DNS). Both are `Network`. |

Off Canning, search pages parse 100% for title, formats and availability URL. Author coverage
is 40–100% per library: films, games and compilations whose cards have no person line have no
author.

## Build

```sh
cd core
cargo build                 # lib + cdylib/staticlib + bins (debug)
cargo build --release
```

For the app targets, build the static library or cdylib for each platform triple. That means
`aarch64-apple-ios`, `aarch64-apple-ios-sim` and so on for iOS, and the Android ABIs through
`cargo ndk`. Then ship the library together with the generated bindings below.

## Test

```sh
cargo test                  # every suite, offline
cargo test --test search    # one module
cargo clippy --all-targets
```

- **Golden tests** run each Rust parser on a fixture and compare the JSON with the TS parser's
  output in `tests/golden/`. Key order is ignored. On a mismatch, the actual JSON is written to
  `target/golden-actual/<golden>` and the first differing JSON pointers are listed. To regenerate
  the goldens after changing a fixture or a TS parser, run
  `bun middleware/scripts/emit-golden.ts`.
- **Private goldens.** `tests/fixtures/private/` and `tests/golden/private/` hold one real
  account's pages (loans, holds, the reserve form). They're staged locally and are **not
  distributed**. Tests that use them check `common::private_available()` and skip cleanly when
  the files are absent. A mismatch report for a private golden lists JSON pointers only, never
  values.
- **Network tests** (`tests/fetch.rs`, and the facade's cache test in `src/lib.rs`) run
  login, redirect, timeout, WAF, verification-cookie and reservation-wizard flows against a
  throwaway HTTP server on `127.0.0.1`. No test ever POSTs to a library. The live tests are
  public GETs and are `#[ignore]`d by default: a Canning search, an NZ search, a UK search that
  expects `HumanVerificationRequired`, and the directory. Run them with
  `cargo test --test fetch --test directory -- --ignored`.

## CLI

`spydus-cli` prints a public endpoint's result as pretty camelCase JSON, the same shape as the
middleware's response:

```sh
cargo run --bin spydus-cli -- search "the midnight library"
cargo run --bin spydus-cli -- search --sort newest "fiction"     # or --sort relevance
cargo run --bin spydus-cli -- availability "<a format's availabilityUrl>"
cargo run --bin spydus-cli -- record "<availabilityUrl or bib URL>"
cargo run --bin spydus-cli -- related "<RecordDetail.related.* URL>"
cargo run --bin spydus-cli -- page "<nextPageUrl>"
cargo run --bin spydus-cli -- libraries          # the directory (JSON array), production only
cargo run --bin spydus-cli -- libraries --all    # …including Civica's test instances
SPYDUS_ORIGIN=https://hutt.spydus.co.nz cargo run --bin spydus-cli -- search "harry potter"
```

It has public reads only, with no account commands. `SPYDUS_ORIGIN` overrides the library origin.
`SPYDUS_VERIFICATION_COOKIE`, if set, is passed to `set_verification_cookie` and never printed.
On an error, stderr gets `spydus-cli <command>: <error>`, for example
`spydus-cli search: human verification required`. The exit code is 1 on an error and 2 on a
usage error.

## Live oracle diff

```sh
core/scripts/oracle-diff.sh
```

The script needs `cargo`, `curl` and `jq`. It uses the middleware on `http://localhost:8787`
(override with `MW_URL`). If the middleware isn't running, the script starts it with
`bun run src/server.ts` on `MW_URL`'s port (`PORT=…`) and stops it at the end. The TS middleware
has no sort parameter, so the diff covers relevance search only (`searchSorted(q, .relevance)`
sends the same request).

For four fixed queries, it diffs `/search`. It then takes the first record that has a format
availability URL (BK preferred) and diffs `/availability` and `/record` on that URL, followed by
`/related` on the record's first related panel. The fourth query, `zzqxjv`, is nonsense, so it
diffs the OPAC's "did not find any records" page (`noResults: true`) and SKIPs the rest. Both
sides are normalised with `jq -S`, and the script prints a per-endpoint PASS/DIFF summary. It
exits non-zero on any DIFF.

Two kinds of live nondeterminism are handled, and each one is shown in the PASS line:

- **Set IDs.** Spydus creates a new result-set ID on every search or bib-page fetch and puts it in
  URLs (`/BIBENQ/<set>?…&SETID=<set>`), so the two sides always see different numbers. When the
  raw JSON differs, each side's own set IDs are masked and the pair is compared again. This gives
  `PASS … (set ids masked)`.
- **Availability counts.** Counts can change between the two fetches. On a counts-only diff, the
  pair is re-run once after the middleware's 60 s availability cache has expired.

The script only makes public GETs. It never logs in or sends a POST.

## Multi-library survey

```sh
python3 -I core/scripts/library-survey.py              # needs cargo + python3 (stdlib only)
python3 -I core/scripts/library-survey.py --dry-run    # print the sample; contact no library
python3 -I core/scripts/library-survey.py --sample AU=10,NZ=10 --jobs 4 --json out.json
```

The script gets the production directory from `spydus-cli libraries` and takes a deterministic,
region-stratified sample. By default that's AU 6, NZ 4, US 4, UK 4 and SG 3. For each region,
the name-sorted list is cut into k equal strata and the middle library of each is taken. Test
instances are never sampled. Each library goes through `spydus-cli` with `SPYDUS_ORIGIN` set to
it, in this order:

1. `search "harry potter"`. If that finds nothing, as at an academic or corporate library,
   `--fallback-query` (default `history`) is searched instead, marked `*` in the matrix.
2. `availability` and `record` on the first format that has an `availabilityUrl`.

The output is a per-library matrix and a per-region summary:

- per library: search status and time, total, records on page 1, % with title, author, formats
  and availability URL, whether there's a next page, availability, and record;
- per region: successes, fallbacks, median and maximum search time, and error kinds.

Each library is then checked against its region's expectation:

- AU, NZ, US and OTHER: end to end.
- UK: `HumanVerificationRequired` within 5 s.
- SG: a `Network` error within the connect timeout.

The exit code is 1 if any library missed its expectation. This is live data, and a library can
be down or a directory entry stale.

Safety: public GETs only, one request at a time per library, and at most 6 libraries at once
(`--jobs`, capped at 6). There's no login, session or POST. `SPYDUS_VERIFICATION_COOKIE` is
removed from the CLI's environment, so no verification token is ever used.

## Bindings

The bindings are generated from the built library. Library mode reads the UniFFI metadata embedded
in the cdylib:

```sh
cd core
cargo build --lib
cargo run --bin uniffi-bindgen -- generate --library target/debug/liblibrespydus_core.dylib \
  --language swift --out-dir generated/swift
cargo run --bin uniffi-bindgen -- generate --library target/debug/liblibrespydus_core.dylib \
  --language kotlin --out-dir generated/kotlin
```

(Use `liblibrespydus_core.so` on Linux.) The output is:

- **Swift:** `librespydus_core.swift`, `librespydus_coreFFI.h` and `librespydus_coreFFI.modulemap`.
- **Kotlin:** `uniffi/librespydus_core/librespydus_core.kt`. It needs **JNA ≥ 5.12** (it uses
  `com.sun.jna.internal.Cleaner`) and `kotlinx-coroutines-core` for the async methods.

Kotlin output is formatted with `ktlint` when it's on `PATH`; otherwise it's left unformatted.
Regenerate the bindings whenever the exported API or the types change.

Compile checks on the host (macOS), linking the debug dylib:

```sh
G=core/generated/swift T=core/target/debug
swiftc -parse-as-library -module-name Smoke -Xcc -fmodule-map-file=$G/librespydus_coreFFI.modulemap \
  -I $G $G/librespydus_core.swift main.swift -L $T -llibrespydus_core \
  -Xlinker -rpath -Xlinker $T -o smoke

# kotlinc isn't on PATH. It's bundled with Android Studio.
KOTLINC="/Applications/Android Studio.app/Contents/plugins/Kotlin/kotlinc/bin/kotlinc"
"$KOTLINC" -cp "jna-5.17.0.jar:kotlinx-coroutines-core-jvm-1.9.0.jar" \
  core/generated/kotlin/uniffi/librespydus_core/librespydus_core.kt Smoke.kt -include-runtime -d smoke.jar
java -Djna.library.path=core/target/debug -cp "smoke.jar:jna-5.17.0.jar:kotlinx-coroutines-core-jvm-1.9.0.jar" SmokeKt
```

`main.swift` and `Smoke.kt` are small callers you write yourself. JNA 5.6 (an old Gradle cache
copy) fails with `unresolved reference 'Cleaner'`. Gradle 9's distribution includes JNA 5.17.0
under `lib/plugins/`.
