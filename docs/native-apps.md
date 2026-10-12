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
- `SearchRecord.detailUrl` / `PatronItem.bibUrl` / `RelatedItem.bibUrl` all load with `record(url)`.
  URLs embed a per-search set id that can expire; `RecordDetail.bookmarkUrl` is the best stable
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
`homeGenres() -> [Genre{key,label,query}]` · `homeShelves() -> [Shelf{key,title,query,sort}]`.
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
Five tabs: **Home · Borrowed · Search · Favourites · Profile**; Search is the raised centre
button. Each tab has its own stack. Shared routes: `detail(BookRef)` → Book detail (I4/A3) and
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
card). No bell icon.

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

**Borrowed (I5/A2).** Title "Your Books". Segments: **Loans** | **Reservations**. Cards
(surface colour, radius card): cover, title, "By author", meta line — loans: "Due 27 Oct ·
3 days left" (danger when ≤ 3 days or overdue); reservations: status / rank / pickup; trailing
coral text action: reservations → "Cancel" (confirm dialog → `cancelReservation`) and "Edit"
(sheet: pickup choice, delay until, not required after → `editReservation`). Ready-for-pickup
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
that one image is allowed (nothing else under `.private/`). Navy screens; white tab bar with a
raised white circular centre Search button; covers with rounded corners and soft shadow; coral
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
