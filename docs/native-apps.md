# LibreSpydus native apps — build contract (Phase 2 iOS, Phase 3 Android)

Two native apps, **SwiftUI (iOS)** and **Jetpack Compose (Android)**, on the shared Rust core
`core/` (crate `librespydus-core`, UniFFI). The core does every fetch, parse, RSA login and cache
on-device; the apps are UI + local storage only. This file is the contract every worker reads
first. The core's own contract is `docs/rust-core-port.md`; its public API is `core/README.md`.

| | iOS | Android |
|---|---|---|
| Display name | LibreSpydus | LibreSpydus |
| ID | bundle `com.librespydus` | applicationId / namespace `com.librespydus.app` |
| Min OS | iOS 17.0 | minSdk 26, compile/target 36 |
| UI | SwiftUI, `@Observable` | Jetpack Compose (Material 3, custom theme) |
| Core artifact | `LibreSpydusCore.xcframework` (static lib, device + sim) + generated Swift | `jniLibs/<abi>/liblibrespydus_core.so` (cargo-ndk) + generated Kotlin + JNA ≥ 5.12 |
| Secure session store | Keychain | Android Keystore-encrypted prefs |

## 1. Workflow (Orca supervised DAG, per-worker git worktrees)

- Integration branch: **`native-apps`** (local only). Each worker runs in its **own git worktree**
  branched from it, commits its work **on its worktree branch only**, and stops. The coordinator
  merges. Workers never push, merge, rebase other branches, switch branches, or touch the main
  checkout at `/Users/tsogt/Project/personal/spydus_clone`.
- Commit only files you own (§3); list any minimal out-of-ownership fix in your report.
- Main checkout path (read-only for workers): `MAIN=/Users/tsogt/Project/personal/spydus_clone`.
  Prebuilt core artifacts for screen workers live there (gitignored build output), see §8.

| Wave | Tasks (parallel within a wave) |
|---|---|
| 1 | **P** core app prereqs · **I1** iOS pipeline + shell + design system + app core · **A1** Android pipeline + shell + design system + app core |
| 2 (iOS screens) | **I2** onboarding/account (library picker, login, verification, Profile) · **I3** Home + Search · **I4** Book detail (+ reserve sheet, favourite toggle) · **I5** Borrowed (+ cancel/edit) + Favourites |
| 3 (Android screens, mirror iOS) | **A2** onboarding/account + Borrowed + Favourites + Profile · **A3** Home + Search + Book detail |

## 2. Safety rules (non-negotiable, every task)

1. **No account access.** Never read `.private/web/auth/**` (it holds the user's session cookie),
   never use a real session, never log in with real credentials. Account features are built and
   exercised through **demo mode** (§5.4), which serves fixture data and never touches the network
   for account calls.
2. **No POST to any library.** Workers may make **public GETs only** (search, page, availability,
   record, related, the library directory, a library's public pages). `login`,
   `place_reservation`, `cancel_reservation`, `edit_reservation` are never called against a real
   library by a worker; `reserve_options` / `edit_options` need a session, so also demo-only.
   The user performs the first real login / reserve / cancel / edit themselves.
3. **Never automate, solve, or bypass an AWS WAF captcha.** UK libraries (`*.spydus.co.uk`)
   answer with `HumanVerificationRequired`; the app shows the library page in a web view for the
   **user** to complete, then reads the resulting `aws-waf-token` cookie. A worker may open that
   page to check it renders, and nothing more.
4. Private fixtures (`core/tests/fixtures/private`, `core/tests/golden/private`) are gitignored and
   absent from worktrees; never copy their contents anywhere.
5. Be gentle with libraries: no load tests, no crawling, ≤ 6 concurrent requests.

## 3. Repository layout & ownership

```
design/tokens.json                      # coordinator (read-only for workers)
docs/native-apps.md                     # coordinator (this file)
core/                                   # P (wave 1) — see its spec
middleware/src/**, middleware/scripts/  # P (TS side of P1 only)
ios/
  LibreSpydus.xcodeproj                 # I1 — Xcode 16+ folder-synchronized groups: adding a file
                                        #      under ios/LibreSpydus/ must NOT edit project.pbxproj
  LibreSpydusCore/                      # I1 — Swift package: generated Swift + binaryTarget xcframework
  scripts/build-core.sh                 # I1
  LibreSpydus/
    App/            LibreSpydusApp.swift, RootView.swift (tabs), LaunchOptions.swift     # I1
    Core/           AppModel, LibraryBackend (+Core/Demo impls), stores, Route, BookRef,  # I1
                    UserFacingError, Formatting
    DesignSystem/   Theme (tokens) + Components/                                         # I1
    Features/Onboarding/  (LibraryPicker, Login, Verification)                           # I2
    Features/Profile/                                                                    # I2
    Features/Home/                                                                       # I3
    Features/Search/      (SearchView + SearchResultsView)                               # I3
    Features/Detail/      (BookDetailView, ReserveSheet, related carousels)              # I4
    Features/Borrowed/    (loans, reservations, cancel/edit sheets)                      # I5
    Features/Favourites/                                                                 # I5
    Resources/      Assets.xcassets, DemoData/*.json                                     # I1
  LibreSpydusTests/  shared test target; each worker adds its own files Tests/<Feature>*  # I1 creates
android/
  settings.gradle.kts, build.gradle.kts, gradle/, gradlew*, gradle.properties             # A1
  scripts/build-core.sh                                                                  # A1
  app/build.gradle.kts, app/src/main/AndroidManifest.xml                                 # A1
  app/src/main/java/com/librespydus/app/
    MainActivity.kt, LibreSpydusApplication.kt                                           # A1
    core/        AppState, LibraryBackend (+Core/Demo impls), stores, Route, BookRef,     # A1
                 UserFacingError, Formatting, LaunchOptions
    ui/theme/, ui/components/, ui/nav/ (NavHost + bottom bar)                            # A1
    feature/onboarding/, feature/profile/, feature/borrowed/, feature/favourites/        # A2
    feature/home/, feature/search/, feature/detail/                                      # A3
  app/src/main/assets/demo/*.json                                                        # A1
  app/src/test/, app/src/androidTest/   each worker adds its own test files              # A1 creates
```

**Wave 1 creates every feature file as a compiling stub** (e.g. `HomeView` showing a placeholder)
so wave-2/3 workers only replace files inside their own feature folder. Shared files (`App/`,
`Core/`, `DesignSystem/`, `ui/`, `core/`) are owned by I1/A1; a later worker may make a **minimal,
additive** change there only when unavoidable, and must list it in its report.

## 4. Core API the apps use (UniFFI; names as generated)

Free functions: `listLibraries(includeTest:) -> [LibraryInfo]` · (P) `userAgent() -> String` ·
(P) helpers in §4.1.

`SpydusClient(origin:)` — one per selected library origin, kept for the app's lifetime:
`origin()`, `setVerificationCookie(cookie:)`, and async throwing:
`search(query)`, (P) `searchSorted(query, sort)`, `page(url)` (next/prev page), `availability(url)`,
`record(url)`, `related(url)`, `login(username,password) -> session`, `account(session) ->
AccountLinks{name,…}`, `loans(session) -> PatronList`, `reservations(session) ->
Reservations{ready,pending}`, `reserveOptions(url,session)`, `placeReservation(url,session,xdays?,
pickup?)`, `cancelReservation(url,session)`, `editOptions(url,session)`,
`editReservation(url,session,EditOverrides)`. Mutations return `ReserveResult{ok,message}` —
`ok:false` is data to show, not an error.

`SpydusError` (flat; message = Display string): `Network`, `OriginForbidden`,
`CredentialsRejected`, `NotLoggedIn`, `Upstream`, `Parse`, `HumanVerificationRequired`.

Data notes (verified on real pages):
- `SearchRecord.detailUrl` / `RelatedItem.bibUrl` load with `record(url)`; a loan or reservation
  loads with `record(patronItemRecordUrl(item))` — its own `bibUrl` uses a set from the patron's
  OPAC session and comes back empty without it. URLs embed a per-search set id that can expire; `RecordDetail.bookmarkUrl` is the best stable
  link to store (verify it loads with `record()`; else fall back to a title/ISBN `search`).
- `RecordFormat{code,label,reserveUrl,availabilityUrl}` lives on search records only; a detail
  opened from a loan/favourite without formats may re-find them with a `search` by ISBN or title.
- Digital formats (`isDigitalFormat`) usually have `reserveUrl == null` and no branch counts →
  show "Available online", no pickup UI. **"Reserve" is keyed on `reserveUrl` alone**: some
  libraries (Brazoria County, US) take holds on ebooks and give them `{available, copies}` counts.
- `Availability.counts` keys are only `copies`, `available`, `reserved` (two template families:
  Reserved+Copies, e.g. Canning/Hutt; Available+Copies, e.g. Cumberland/Wellington/Niagara/Brazoria).
  Render with `availabilitySummary(availability:).label`.
- Some libraries are slow (Brazoria County: 40–60 s for big result sets, near the core's 60 s
  timeout): show progress patiently, keep shelves independent so one slow shelf doesn't block Home.
- Loans: `PatronItem.fields["Due"]` (e.g. "27 Oct 2026"), action "Renew loan" (renew is NOT
  implemented yet — show no Renew button). Reservations: `fields` `Status`, `Rank`, `Placed`,
  `Pickup location`; actions labelled like "Cancel reservation" / "Edit reservation" (match
  `/cancel/i`, `/edit/i`).
- Covers: `coverUrls` (Syndetics). Missing cover → generated placeholder (title on a token colour).

### 4.1 P additions (merged; exact API in `core/README.md`, contract in `docs/rust-core-port.md` §11)
`SearchResults.noResults: Bool` (true only for a genuine "no records found" page) ·
`enum SearchSort { relevance, newest }` + `searchSorted` · `userAgent()` ·
`isDigitalFormat(format: RecordFormat) -> Bool` · `availabilitySummary(Availability) ->
AvailabilitySummary{available?, copies?, reserved?, label}` · `parseDueDate(text) -> SimpleDate?` ·
`homeGenres() -> [Genre{key,label,query}]` · `homeShelves() -> [Shelf{key,title,query,sort}]` ·
`patronItemRecordUrl(item) -> String?` · `accountHolding(snapshot, bibIrns) -> AccountHolding{kind,
item}?` (`HoldingKind`: loan, reservationReady, reservationPending) · (I9/A7, §5.8)
`patronItemKey(item) -> String` · `readyAlerts(snapshot, alertedKeys) -> ReadyAlerts{newItems,
alertedKeys}` · `dueReminders(snapshot, today: SimpleDate, daysBefore) -> [DueReminder{key, due,
remindOn, items}]` · (I10/A8, §5.9) `cardBarcode(cardNumber, symbology) -> CardBarcode{symbology,
modules: [Bool], text}?` · `SpydusClient.cardBarcodeSymbology() async -> BarcodeSymbology`
(`.code128`, `.code39`, `.codabar`) · `AccountSnapshot.cardNumber` / `.cardExpires` and
`AccountLinks.personalDetailsUrl` (default nil).
Newest = `SORTS=MAIN.CREATED_DATE.DESC]MAIN.CREATED_TIME.DESC` (the OPAC's own new-items RSS order).

## 5. App architecture (same shape on both platforms)

### 5.1 `LibraryBackend`
A hand-written protocol/interface mirroring the `SpydusClient` methods the app uses (plus
`origin`). Implementations: **`CoreBackend`** (wraps one `SpydusClient`) and **`DemoBackend`**
(§5.4). Screens talk only to `LibraryBackend`, never to `SpydusClient` directly.

### 5.2 App state (`AppModel` on iOS — `@Observable`, injected via environment; `AppState` on Android — StateFlows, one instance in the Application)
- `library: LibraryInfo?` — selected library, persisted (UserDefaults / DataStore). Nil → onboarding.
- `backend: LibraryBackend` — rebuilt when the library changes; one `SpydusClient` per origin.
- `session: String?` per origin in Keychain / Keystore; `accountName: String?` (from `account`).
- `signIn(user, pwd)`, `signOut()`, `selectLibrary(info)`, `refreshAccount()`.
- `favourites` — local store (§5.3). `selectedTab`, `pendingSearch` (Home search field → Search tab).
- Error routing: `report(error)` → `HumanVerificationRequired` sets `verificationNeeded` (root
  presents the verification sheet); `NotLoggedIn` clears the session and sets `loginNeeded`.
  `verificationGeneration` / `accountGeneration` counters let screens re-run their loads.
- Verification cookie per origin persisted and re-applied with `setVerificationCookie` whenever a
  client is created; `completeVerification(cookie)` stores + applies it and bumps the generation.

### 5.2a Saved account (stale-while-revalidate)
`accountSnapshot(session)` fetches name + loans + reservations in one call; the app saves it per
origin (`encodeAccountSnapshot` JSON, app-private storage) and on launch / foreground / pull to
refresh shows the saved one at once ("Updated 10 min ago") while a fresh one loads in the
background. A failed refresh keeps the saved data with a "Couldn't refresh" note; sign-out and
session expiry (NotLoggedIn) delete that library's file (switching library keeps each library's
own file); demo mode never saves it; a successful reserve / cancel / edit triggers a refresh.
iOS: `Application Support/AccountCache/<sha256(origin)>.json`, complete-until-first-unlock,
excluded from backup. Android: `noBackupFilesDir`, AES-GCM with an Android Keystore key.

### 5.3 Favourites (local only)
Per-origin list of `BookRef` (Codable/serializable): `id` (stable key: bib IRN / BRN / work IRN,
else URL), `title`, `author`, `coverUrl`, `url` (bookmarkUrl preferred), `isbn`, `year`,
`formats` (code,label,reserveUrl,availabilityUrl), `savedAt`. Toggle, contains, list (newest first),
remove. JSON file in Application Support / app files dir.

### 5.4 Demo mode (DEBUG builds only)
Launch flag (§5.6) `demo` → `DemoBackend`: public calls (search/page/availability/record/related)
go to a real `CoreBackend` (public GETs are allowed), account calls are served from bundled JSON
built from the **synthetic** goldens (`core/tests/golden/synthetic/account-links.json`,
`patron-items.json`, `reserve-options.json`, `edit-options.json`; fabricated "Test Borrower")
plus a few loans with due dates relative to today. `login("demo","demo")` succeeds,
anything else → `CredentialsRejected`. Mutations succeed after ~0.6 s and update the in-memory
demo lists (cancel removes the row, reserve adds a pending row). This is how workers build,
test and screenshot every account screen.

### 5.5 Navigation
Five tabs: **Home · Your Books · Search · Favourites · Profile**, all ordinary tab-bar items (the
inspiration's raised centre Search button was dropped at the user's request). Each tab has its own stack. Shared routes: `detail(BookRef)` → Book detail (I4/A3) and
`results(query, sort, title)` → Search results list (I3/A3, used by Home "View all" and genre
tiles). `BookRef` constructors from `SearchRecord`, `PatronItem`, `RelatedItem`, favourite.

### 5.6 Debug launch hooks (DEBUG only)
iOS launch arguments / Android intent extras:
- `-LSLibraryOrigin <origin>` / `--es ls_origin <origin>` — select a library, skip the picker.
- `-LSRoute <route>` / `--es ls_route <route>`: `home`, `search:<query>`, `results:<query>`,
  `detail:<url>`, `borrowed`, `favourites`, `profile`, `libraries`, `login`, `verify`.
- `-LSDemo YES` / `--ez ls_demo true` — demo mode, already signed in as "Test Borrower".
- `-LSReset YES` / `--ez ls_reset true` — clear persisted library/session/favourites first.
Example: `xcrun simctl launch <udid> com.librespydus -LSLibraryOrigin https://canning.spydus.com -LSDemo YES -LSRoute borrowed`;
`adb -s emulator-5554 shell am start -n com.librespydus.app/.MainActivity --es ls_origin https://canning.spydus.com --ez ls_demo true --es ls_route borrowed`.

- (I9/A7) `-LSMorningCheckNow YES` / `--ez ls_morning_check_now true` — run the morning check
  (§5.8) right after launch instead of waiting for 07:45. `-LSDemoNewReady YES` /
  `--ez ls_demo_new_ready true` (with demo) — the demo account gains one more ready reservation on
  its next refresh, so a "Ready to collect" alert can be shown.

**Timing log (DEBUG only).** At launch the app installs a core `TimingSink` that prints one
`⏱ core <operation>: library <ms> · parse <ms> (<size>)` line per page the core loads
(`from cache` for a hit, `failed after <ms>` for an error). `CoreBackend` adds one
`⏱ app <call>: <ms> total` line per call, the time as the app saw it (core work, the hand-off
and waiting for the caller's thread). iOS: `ios/LibreSpydus/Core/TimingLog.swift`, printed to
stdout. Read it in Xcode's console, or on a device with
`xcrun devicectl device process launch --console --device <udid> com.librespydus`.

### 5.8 Alerts: ready to collect, due-date reminders (I9 iOS / A7 Android)
Local notifications only. Civica's own pushes go to the official app and can't reach LibreSpydus;
Canning sends its notices at about 07:20–07:45 local time, so LibreSpydus checks once a morning.
- **Settings.** Profile gets a "Notifications" section while signed in, with two switches, both
  off by default and app-wide (they apply to the selected library): **"Ready to collect"**
  ("Checked each morning and when you open the app") and **"Due-date reminders"** ("2 days before
  books are due, at 7:45"). Turning one on asks for notification permission (iOS
  `requestAuthorization([.alert, .sound])`; Android 13+ `POST_NOTIFICATIONS`). Denied → the switch
  stays off and a line "Notifications are turned off for LibreSpydus." with an **Open Settings**
  button shows under the section.
- **Turning "Ready to collect" on** remembers what's already waiting: call
  `readyAlerts(savedSnapshot, [])` and save only its `alertedKeys` (no notification).
- **Alerted keys** per library (keyed by sha256(origin) like the account cache; IRNs, not
  secret): iOS `UserDefaults`, Android `SharedPreferences`. Deleted on sign-out and session expiry,
  and when the switch is turned off.
- **Processing a fresh snapshot** (`AlertCenter` / `Alerts`, one place): run after every
  successful `accountSnapshot` — foreground refreshes and the morning check alike, never for demo
  data unless demo mode is on (demo processes the in-memory snapshot the same way).
  1. Ready on: `readyAlerts(snapshot, savedKeys)` → one notification per `newItems` entry, then
     save `alertedKeys`. A library with **no saved keys** (none recorded — not an empty list:
     signed in again after a sign-out or an expired session, or switched on without a saved
     account) only saves `alertedKeys` and posts nothing, so what is already waiting is never
     announced as new.
  2. Due on: `dueReminders(snapshot, today, 2)` with `today` = the device's local date. iOS
     replaces all pending `due:` requests for this library with calendar triggers at 07:45 on
     each `remindOn` (skipping one whose 07:45 has already passed). Android posts, during the
     morning check only, the reminders whose `remindOn` is today (at most once per `key` per day).
- **Content** (both apps identical):
  - Ready: title **"Ready to collect"**, body **"<Title> · <Branch>"**, where the branch is what
    Your Books shows for that row (pickup branch from Status), the body is just the title when the
    branch is unknown, and the title falls back to "A reservation". ID `ready:<originHash>:<key>`.
    Tap → Your Books, Reservations, Ready segment.
  - Due: title **"Due Thursday 16 Oct"** (local weekday, day, short month; **"Due today"** when
    due today, **"Due tomorrow"** when due tomorrow); body: one loan "<Title>", two "<A> and <B>",
    more "<A> and N more". ID `due:<originHash>:<YYYY-MM-DD>`. Tap → Your Books, Loans.
  - Android channels: `ready` "Ready to collect", `due` "Due-date reminders" (default importance).
- **Morning check** (next 07:45 local; scheduled while signed in with any switch on, cancelled
  otherwise): fetch `accountSnapshot`, save it like a foreground refresh (so the app opens with
  fresh data), process it, schedule the next one. A failure is silent; Android still posts that
  day's due reminders from the saved snapshot. NotLoggedIn clears the session as in the app.
  - iOS: `BGAppRefreshTask` `com.librespydus.morning-check` (Info.plist
    `BGTaskSchedulerPermittedIdentifiers`, `UIBackgroundModes` `fetch`), handled with SwiftUI
    `.backgroundTask(.appRefresh(…))`; `earliestBeginDate` = next 07:45, submitted at launch,
    when the app moves to the background, when a switch changes and after each run. iOS picks
    the actual time (often soon after 07:45 for a phone used in the morning; it can be later, or
    skipped in Low Power Mode or after the app is swiped away). Foreground notifications show as
    banners (`willPresent` → `.banner, .list, .sound`).
  - Android: WorkManager unique one-time work `morning-check` (REPLACE) with the delay to the
    next 07:45, re-enqueued at the end of each run; no network constraint (it falls back to the
    saved snapshot).

### 5.9 My card (I10 iOS / A8 Android)
The library card as a barcode to scan at the desk or self-checkout, like the official app's card.
- **The number** is the library card number (the official app's barcode is the same: the `BrwId`
  claim of its OpenID token; Canning's start `CL114…`). Sign-in can use a **username** instead, so
  the sign-in ID is never used. Sources, first wins: (1) a number the patron entered or edited on
  the card, saved per library in the secure store (iOS Keychain service `com.librespydus.card`,
  account = origin; Android the encrypted secrets store, key `card.<origin>`), deleted on
  sign-out, kept when the session expires; (2) `AccountSnapshot.cardNumber`, which the core reads
  from the account's **Personal details** page (`AccountLinks.personalDetailsUrl`, the
  "Card number" row) on every `accountSnapshot`, with `cardExpires` ("Card expires"). Demo mode:
  the demo snapshot carries `DEMO0012345` / an expiry a year out.
- **Symbology:** `client.cardBarcodeSymbology()` (the library's `barcodeType` from the Spydus
  mobile config: Canning Code 128, Cumberland Code 39; Code 128 when the library has none; cached
  in the core). `cardBarcode(number, symbology)` → `modules` (true = black, quiet zones not
  included) drawn by the app; it falls back to Code 128 for numbers the library's symbology can't
  encode, and is nil only for numbers Code 128 can't encode either.
- **Home:** while signed in, a capsule button **"My card"** with a barcode icon in the header,
  right of the greeting (surfaceElevated fill, primary-tinted icon). Tap → the card sheet.
- **Card sheet** "My card": a **white** rounded card (white whatever the theme — scanners need
  black on white) with the library logo + name, the borrower's name, the barcode (black modules on
  white, ≥ 10 white modules of quiet zone each side, a whole number of device pixels per module,
  ~100 pt / dp tall, as wide as fits), the number under it in a monospaced font and, when known,
  "Expires 12 Mar 2027" (`cardExpires` as written) in secondary text. Below the
  card: "Show this at the library desk or self-checkout." and an **"Edit card number"** button.
  Screen brightness goes to full while the sheet is up and back when it closes or the app leaves
  the foreground. While the symbology loads the barcode area shows a spinner (first open only).
  - No number from either source (the account hasn't loaded yet, or its Personal details page has
    none): the sheet asks for it — field "Library card number", note "Enter the number on your
    library card or in your library account's Personal details (e.g. CL114D…).", **Save**
    (disabled while blank) — then shows the card.
  - A number `cardBarcode` can't encode: the number shown large, with "This number can't be
    shown as a barcode."
  - VoiceOver / TalkBack: the barcode is one element, "Barcode for card <number>".

### 5.7 User-facing errors
`Network` → "Can't reach {library}. Check your connection." · `HumanVerificationRequired` →
verification sheet · `NotLoggedIn` → "Please sign in again." (+ login) · `CredentialsRejected` →
"Card number or PIN not recognised." · `OriginForbidden` / `Upstream` / `Parse` → "The library
site returned something unexpected." (detail in a disclosure). Every loading screen has
loading / empty / error(+Retry) states.

## 6. Screens (adapted to real data — no ratings, no reviews)

**Library picker (I2/A2).** First launch: search field + list grouped by region (AU, NZ, US, UK,
SG, Other) with logo, name, country. UK rows note "Asks you to verify you're human"; SG rows note
"May be unreachable outside school networks". Selecting → Home. Reachable later from Profile.

**Home (I3/A3).** Navy. Top: library logo (small, left) and greeting **"Hello {first name}!"**
(signed out: "Hello!" + library name). Search field "Search by title, author, etc." → focuses the
Search tab. Sections (each with "View All"): **"New arrivals"** (first `homeShelves()` shelf,
horizontal cover cards, tap → detail), **"Popular genres"** (`homeGenres()` rounded colour tiles
with a platform icon per `key`, tap → results), **"Your borrowed books"** (signed in: current
loans as covers with "N days left"/"Due {date}", overdue in danger colour; signed out: a sign-in
card). No bell icon. Signed in: a **"My card"** button in the header (§5.9).

**Search (I3/A3).** Search field (autofocus from Home), sort toggle Relevance / Newest, result rows
(cover, title, author, year, format chips; digital formats tinted `digital`), infinite scroll via
`nextPageUrl` → `page()`, total count, `noResults` → empty state "No results for …", unrecognised
page → error state. Tap → detail.

**Book detail (I4/A3).** Top half navy: back button, cover (detailWidth), title, "By {author}",
availability line for the selected format (helper label, e.g. "2 of 3 available"; digital →
"Available online"), format chips when several. Primary coral pill **"Reserve"** (the inspiration's "Borrow Now", renamed because the library places a hold rather than lending immediately;
physical formats with `reserveUrl` only; signed out → login first) and a blue circular bookmark
button (favourite toggle, filled when saved). Bottom: white sheet with **Synopsis** (`summary`),
subject chips (from `details` rows like Subjects/Genre/Audience, max ~6), a compact details list
(publisher, ISBN, series, …), and carousels **"Similar books"**, **"More by this author"**,
**"In this series"** from `related` panels (hide empty ones). Reserve sheet: pickup location,
available summary, expiry picker (`expiryChoices`), Confirm → `placeReservation` → result banner.
**Already yours (I8/A6):** when the saved account (§5.2a) has the shown format's bib on loan or
reserved (`accountHolding`), the Reserve pill is replaced by a status pill — "On loan · Due 14 Oct",
"Ready to collect · Riverton Library", "Reserved · Riverton Library" — that opens Your Books on the
right segment. A page opened from Your Books selects the format that is on loan/reserved.

**Your Books (I5/A2; tab label "Your Books").** Segments: **Loans** | **Reservations**. Cards
(surface colour, radius card): cover, title, "By author", meta line — loans: "Due 27 Oct ·
3 days left" (danger when ≤ 3 days or overdue); reservations: status / rank / pickup (Canning
writes the pickup branch into Status: "Requested pickup location: …" / "Please pickup from: …
by <date>"). Reservation actions are hidden by default: swipe a card left to reveal rounded
**Edit** (accent; sheet: pickup choice, delay until, not required after → `editReservation`)
and **Cancel** (danger; confirm dialog → `cancelReservation`) tiles — a full swipe only opens
the card, one card open at a time; long-press and screen-reader actions offer the same two;
a footnote hints "Swipe left on a reservation to change or cancel it." (I6/A4). Ready-for-pickup
reservations first, highlighted. Pull to refresh. Signed out → sign-in prompt.

**Favourites (I5/A2).** Local list/grid of saved books (cover cards), tap → detail, swipe/long-press
to remove, empty state.

**Profile (I2/A2).** Library card (logo, name, origin, region), account name when signed in,
Sign in / Sign out, Change library, verification status for libraries that needed it ("Verified
on this device" + "Clear verification"), app version, "Not affiliated with Civica or your
library" note.

**Login (I2/A2).** Sheet: library name/logo, "Library card number", "PIN / password" (secure),
Sign in → `signIn`; errors per §5.7. Demo mode hint visible in DEBUG.

**Verification (I2/A2).** Sheet with a web view on the library origin using the core's
`userAgent()`; copy: "{Library} asks you to confirm you're human. Complete the check below."
Watch the web view's cookie store for `aws-waf-token` on the library host; when it appears call
`completeVerification("aws-waf-token=<value>")`, dismiss, and the failed screen retries.
Never inject scripts into or interact with the challenge programmatically.

## 7. Design system

Tokens: `design/tokens.json` (colours, radii, spacing, type, cover sizes, tab bar). The visual
reference (third-party inspiration, do not commit or redistribute) is
`/Users/tsogt/Project/personal/spydus_clone/.private/design/inspiration.jpg` — view it; reading
that one image is allowed (nothing else under `.private/`). Navy screens; white tab bar with five equal items; covers with rounded corners and soft shadow; coral
gradient pill primary button; blue circular bookmark; Borrowed-style cards on `surface`; detail
screen = navy header + white rounded sheet. System fonts (SF Pro / Roboto). Respect Dynamic
Type / font scale reasonably and safe areas; support dark status bar content.

Shared components (I1/A1): `CoverImage` (async + placeholder), `ShelfCover` (cover + caption),
`BookRowCard`, `SectionHeader(title, "View All")`, `SearchField`, `PrimaryPillButton`,
`CircleIconButton`, `GenreTile`, `Chip`, `LoadingView`, `EmptyStateView`, `ErrorStateView(retry)`,
`PlaceholderScreen`, the custom tab bar.

## 8. Build, run & evidence

**Rust targets.** iOS: `aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios`. Android:
`aarch64-linux-android armv7-linux-androideabi x86_64-linux-android` + `cargo-ndk`; SDK
`~/Library/Android/sdk`, NDK `28.2.13676358`; emulator images are 16 KB-page (`ps16k`) → the `.so`
must be 16 KB-aligned. JDK: Android Studio's JBR or `openjdk@17`. Gradle 9.5.0 is in
`~/.gradle/wrapper/dists`. Kotlin compiler (for quick checks): `/Applications/Android
Studio.app/Contents/plugins/Kotlin/kotlinc/bin/kotlinc`.

**Prebuilt core for screen workers (waves 2–3).** Don't rebuild Rust; copy the coordinator's build:
`cp -R $MAIN/ios/LibreSpydusCore/LibreSpydusCore.xcframework ios/LibreSpydusCore/` (and whatever
else `ios/scripts/build-core.sh --help` says is needed), or
`cp -R $MAIN/android/app/src/main/jniLibs android/app/src/main/`.

**Devices.** iOS: create your own simulator and delete it when done —
`xcrun simctl create "LS-<task>" com.apple.CoreSimulator.SimDeviceType.iPhone-17-Pro com.apple.CoreSimulator.SimRuntime.iOS-27-0`.
Never use `E27966EB-7DF4-4267-93D7-8E49EAE994AB` (reserved for the coordinator/user).
Use a per-worker `-derivedDataPath`. Android: use the AVD/port your task names, headless
(`emulator -avd <name> -port <port> -no-window -no-audio -no-boot-anim -no-snapshot-save`), kill
it when done (`adb -s emulator-<port> emu kill`).

**Evidence.** Screenshots (`xcrun simctl io <udid> screenshot x.png`,
`adb -s emulator-<port> exec-out screencap -p > x.png`) of every screen/state you built, saved
under `/private/tmp/librespydus-evidence/<task>/` (not committed). Look at them and compare with
the inspiration before reporting. Build + unit tests must pass:
`xcodebuild -project ios/LibreSpydus.xcodeproj -scheme LibreSpydus -destination 'platform=iOS Simulator,id=<udid>' -derivedDataPath <dir> build test`;
`cd android && ./gradlew :app:assembleDebug :app:testDebugUnitTest`.

**Report** (in `worker_done`): what was built, files changed, commit hash on your branch, the
exact build/test commands + results, screenshot paths, deviations, open issues.
