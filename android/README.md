# LibreSpydus for Android

Jetpack Compose app on the shared Rust core (`../core`, UniFFI). The core does every fetch, parse,
login and cache on the device; this app is UI plus local storage. The contract (layout,
architecture, screens, design, evidence) is [`docs/native-apps.md`](../docs/native-apps.md).

- applicationId / namespace `com.librespydus.app`, label **LibreSpydus**
- minSdk 26, compileSdk / targetSdk 36
- Kotlin 2.4, AGP 9.3 (built-in Kotlin), Gradle 9.5.0 wrapper, JDK 17 toolchain

## Prerequisites

| | |
|---|---|
| Android SDK | `~/Library/Android/sdk` with platform `android-36`, build-tools 36+, NDK `28.2.13676358` |
| JDK | 17 or newer to run Gradle (`openjdk@17`, or Android Studio's JBR). The build compiles for 17. |
| Rust | rustup + `rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android` |
| cargo-ndk | `cargo install cargo-ndk` (4.x) |

Gradle needs to find the SDK: either export `ANDROID_HOME=~/Library/Android/sdk`, or create
`android/local.properties` (gitignored) with `sdk.dir=/Users/<you>/Library/Android/sdk`.

## 1. Build the core library

```sh
android/scripts/build-core.sh            # release, arm64-v8a + x86_64
android/scripts/build-core.sh --debug    # debug build (bigger .so, faster compile)
android/scripts/build-core.sh --all      # adds armeabi-v7a
android/scripts/build-core.sh --abi arm64-v8a
android/scripts/build-core.sh --from "$MAIN/android/app/src/main/jniLibs"   # copy a prebuilt set
android/scripts/build-core.sh --help
```

The script runs `cargo ndk --platform 26` in `core/` and writes
`app/src/main/jniLibs/<abi>/liblibrespydus_core.so` (gitignored). It sets `ANDROID_HOME` /
`ANDROID_NDK_HOME` itself (defaults above) and honours `CARGO_TARGET_DIR`.

**16 KB pages.** The emulator images are `ps16k`, and Android 15+ devices may use 16 KB pages, so
every `PT_LOAD` segment must be 16 KB-aligned. NDK r28 links that way by default; the script also
passes `-z max-page-size=16384` and then checks each `.so` with `llvm-readelf -lW` (fails the
build otherwise). AGP stores the libraries uncompressed and 16 KB zip-aligned. To check an APK:

```sh
~/Library/Android/sdk/build-tools/37.0.0/zipalign -c -P 16 -v 4 app/build/outputs/apk/debug/app-debug.apk
```

The app packages only the ABIs that have a core library in `jniLibs` (JNA ships its own
`libjnidispatch.so` for many more), so build every ABI you want to ship.

**Bindings.** The generated Kotlin is not copied: `app/build.gradle.kts` adds
`../core/generated/kotlin` as a Kotlin source dir. Rebuild the `.so` whenever the core (and so its
committed bindings) changes, or the two get out of step.

## 2. Build, test, run

```sh
cd android
./gradlew :app:assembleDebug
./gradlew :app:testDebugUnitTest                      # JVM unit tests
ANDROID_SERIAL=emulator-5554 ./gradlew :app:connectedDebugAndroidTest   # on a device/emulator
adb -s emulator-5554 install -r app/build/outputs/apk/debug/app-debug.apk
```

The instrumented `CoreSmokeTest` calls the real core on the device
(`SpydusClient("https://canning.spydus.com").search("harry potter")`), which proves the `.so`
loads (16 KB pages), JNA, the coroutine bridge and TLS. It's a public GET only.

**Release / R8.** `assembleRelease` minifies with `proguard-rules.pro` (keep rules for JNA and
`uniffi.librespydus_core`). It's signed with the debug key until a release keystore exists. To run
the instrumented tests against the minified build:

```sh
ANDROID_SERIAL=emulator-5554 ./gradlew :app:connectedReleaseAndroidTest -Plibrespydus.testBuildType=release
```

**Emulator.** Headless, per the contract:

```sh
~/Library/Android/sdk/emulator/emulator -avd Pixel_9 -port 5554 -no-window -no-audio -no-boot-anim -no-snapshot-save
adb -s emulator-5554 emu kill
```

On this machine the `Pixel_9` AVD's own image (`android-37.0/google_apis_playstore_ps16k`) has no
`system.img`. Booting it with `-sysdir ~/Library/Android/sdk/system-images/android-37.2/google_apis_playstore_ps16k/arm64-v8a/`
(also 16 KB pages) works without changing the AVD.

### Dependency versions

`gradle/libs.versions.toml` pins AndroidX and Coil to the newest releases that still compile
against android-36. Compose 1.12 (BOM 2026.08+), core 1.19, lifecycle 2.11, navigation 2.10 and
Coil 3.6 declare `minCompileSdk 37` in their AAR metadata. Raise them together with `compileSdk`
once android-37 is installed and the contract allows it. AGP 9.4 needs Gradle 9.6, so AGP stays on 9.3.

## Debug launch options (DEBUG builds only)

Intent extras read by `MainActivity` (`core/LaunchOptions.kt`, docs §5.6):

| extra | meaning |
|---|---|
| `--es ls_origin <origin>` | select that library (directory entry if found, else a stand-in), skip the picker |
| `--es ls_route <route>` | `home`, `search:<query>`, `results:<query>`, `detail:<url>`, `borrowed`, `favourites`, `profile`, `libraries`, `login`, `verify`, `gallery` |
| `--ez ls_demo true` | demo mode, already signed in as "Test Borrower" |
| `--ez ls_reset true` | clear persisted library, sessions, verification cookies, favourites and saved accounts first |
| `--ez ls_demo_persist true` | with `ls_demo`: save the demo account on the device like a real one (under its own key), so a cold relaunch shows it at once |
| `--ez ls_demo_refresh_fails true` | with `ls_demo`: every demo account refresh fails like an unreachable library ("Couldn't refresh — showing saved data" over a saved demo account) |
| `--ez ls_morning_check_now true` | run the 07:45 morning check (docs §5.8) through WorkManager now, once the launch's own account refresh has settled |
| `--ez ls_demo_new_ready true` | with `ls_demo`: the "In transit" reservation (Baby-sitters club) becomes ready at Riverton Library from the demo account's second refresh on, so a "Ready to collect" alert can be shown |
| `--ez ls_open_card true` | with `ls_demo`: Home opens My card once (`feature/mycard/MyCardDebugHooks.kt`) |
| `--es ls_demo_card <number>` | with `ls_demo`: the demo card number instead of `DEMO0012345`; `none` → no number (the card asks for one). Quote non-ASCII values: `"--es ls_demo_card 'CL114Ä'"` |

```sh
adb -s emulator-5554 shell am start -n com.librespydus.app/.MainActivity \
  --es ls_origin https://canning.spydus.com --ez ls_demo true --es ls_route borrowed
# adb shell re-splits arguments: quote values that contain spaces or '&' twice
adb -s emulator-5554 shell am start -S -n com.librespydus.app/.MainActivity --ez ls_demo true \
  --es ls_route "'search:harry potter'"
```

`ls_route gallery` opens the components gallery: every shared component on one screen, for visual
checks. Add `-S` to `am start` to force-stop first (launch options apply on a fresh start or
`onNewIntent`).

## Demo mode

`ls_demo` swaps the backend for `DemoBackend` (docs §5.4):

- Public reads (search, page, availability, record, related) go to the real library through
  `CoreBackend`. Those are public GETs. The exception is the demo account's own books
  (`core/DemoCatalogue.kt`, `assets/demo/catalogue.json`): their records, a search that names
  their title and their formats' availability are answered offline, so a loan's or reservation's
  page works without a network. Like the real OPAC, a demo row's own `bibUrl` (a set in the
  patron's session) loads an empty record; the session-free `…/BIBENQ?SETLVL=&IRN=…` loads the
  full one. The Midnight Library has Book, Large Print and eBook editions (the Book is on loan,
  overdue) and The Thursday Murder Club a Book and a Large Print (the Large Print is reserved).
- Account calls are served from `app/src/main/assets/demo/*.json`, which is built from the core's
  synthetic goldens ("Test Borrower") plus four generated loans on Canning records.
  `dueInDays` becomes a `Due` date relative to today. Fixture URLs are rewritten to the selected
  library's origin.
- `login("demo", "demo")` succeeds. Anything else gives `CredentialsRejected`.
- Reserve, cancel and edit succeed after about 0.6 s and change the in-memory lists. Nothing is
  ever posted. A reserved row carries the reserved edition's bib IRN, so the book page then shows
  "Reserved · …" in place of Reserve.
- `accountSnapshot` takes about 1.5 s (so the refreshing state shows) and reflects those changes.
- The demo session is kept in memory only, never in the secure store, and the demo account is never
  saved on the device (unless `ls_demo_persist`, for screenshots).

## Saved account (docs §5.2a)

`AppState.account` (`core/AccountStore.kt`) holds the signed-in borrower's `AccountSnapshot` (name,
loans, both reservation lists). Your Books, Home's "Your borrowed books" shelf, the Home greeting
and Profile's name all read it; no screen calls `loans()` / `reservations()` any more.

- Saved per library in `noBackupFilesDir/account-cache/<sha256(origin)>.json`
  (`core/AccountCache.kt`): the core's `encodeAccountSnapshot` JSON sealed with AES-GCM under the
  same Keystore key as the session store, the origin bound in. A file that doesn't decrypt or
  decode (another `ACCOUNT_SNAPSHOT_VERSION`) is deleted. Restored before the library is shown, so
  the first frame already has it.
- Refreshed in the background (one at a time; a result for an older library/session is dropped):
  at launch with a session, on sign-in, on `onStart` and when Your Books or Home opens if it is over
  two minutes old, on pull to refresh, and after a reserve / cancel / edit.
- A failed refresh keeps the data ("Couldn't refresh — showing saved data" + Retry on Your Books).
  Sign-out and an expired session delete that library's file; other libraries keep theirs.

Never sign in with real credentials while developing (docs §2). Account screens are built and
screenshotted in demo mode.

## Alerts (docs §5.8)

Profile → Notifications (signed in) has two app-wide switches, both off by default: "Ready to
collect" and "Due-date reminders". Turning one on asks for `POST_NOTIFICATIONS` on Android 13+;
refused or blocked → the switch stays off and "Notifications are turned off for LibreSpydus." +
Open Settings shows.

- `core/Alerts.kt` is the one processor. `AccountStore` hands it every fresh `accountSnapshot`
  (launch, foreground, pull to refresh, after a reserve / cancel / edit, the morning check):
  `readyAlerts` → one notification per new ready reservation → the returned keys are saved.
  Turning Ready on seeds the keys from the saved account (no notification). State lives in the
  `alerts` SharedPreferences: `readyEnabled`, `dueEnabled`, `readyKeys.<sha256(origin)>`,
  `dueLastPosted.<sha256(origin)>.<due:YYYY-MM-DD>`; demo data is filed under `demo:<origin>`.
- `core/MorningCheckWorker.kt`: WorkManager unique one-time work `morning-check`, delayed to the
  next 07:45 local (`MorningCheckTime`, java.time, DST-safe), no constraints. The app keeps it
  scheduled (KEEP) while signed in with a switch on and cancels it otherwise; each run fetches
  the account like a foreground refresh (`AppState.morningCheck`), posts the due reminders whose
  `remindOn` is today (once per key per day, from the saved account if the fetch failed) and
  replaces itself with the next 07:45 (REPLACE).
- `core/AlertsAndroid.kt`: channels `ready` / `due`, notification tag = the §5.8 id
  (`ready:<originHash>:<key>`, `due:<originHash>:<YYYY-MM-DD>`), tap → `MainActivity` with
  `ls_open_borrowed` → `AppState.openFromAlert` → Your Books on Reservations / Loans.
- Sign-out and an expired session forget that library's keys and remove its notifications.

```sh
# Demo: turn both switches on in Profile first, then:
adb -s emulator-5556 shell am start -S -n com.librespydus.app/.MainActivity --es ls_origin https://canning.spydus.com \
  --ez ls_demo true --ez ls_demo_new_ready true --ez ls_morning_check_now true
adb -s emulator-5556 shell am broadcast -a androidx.work.diagnostics.REQUEST_DIAGNOSTICS -p com.librespydus.app
adb -s emulator-5556 logcat -s MorningCheck Alerts WM-DiagnosticsWrkr
```

## My card (docs §5.9)

Home shows a "My card" capsule right of the greeting while signed in; it opens a sheet with the
library card as a barcode for the desk or self-checkout (`feature/mycard/`).

- **The number** isn't the sign-in ID (that may be a username): the patron enters it on the card
  (field "Library card number"), and can edit it. `AppState.cardNumber` = the number saved on the
  device, else the account's own (`accountCardNumber`, a stub returning null until the core reads
  Personal details). Saved trimmed in the secure store under `card.<origin>`; deleted on sign-out,
  kept when the session expires. Demo mode: `DEMO0012345`, in memory only.
- **Symbology:** `LibraryBackend.cardBarcodeSymbology()` (the library's mobile config
  `barcodeType`: Canning Code 128, Cumberland Code 39; demo mode asks the real library, a public
  GET). `CardSymbologies` remembers the last answer per library, so only the first open shows a
  spinner. The core's `cardBarcode` gives the modules (Code 128 fallback; null for non-ASCII →
  "This number can't be shown as a barcode.").
- **Drawing:** `BarcodeView` (Compose `Canvas`) gives every module the same whole number of device
  pixels, `floor(width / (modules + 20))`, at least 1, bars centred with ≥ 10 white modules each
  side, on a white card in any theme. Screen brightness is full while the sheet is up (activity
  window `screenBrightness`), restored when it closes or the app leaves the foreground.

```sh
adb -s emulator-5556 shell am start -S -n com.librespydus.app/.MainActivity \
  --es ls_origin https://cumberlandlibraries.spydus.com --ez ls_demo true --ez ls_open_card true
```

## Featured collections (docs §5.10)

Home shows a "Featured" row between "Popular genres" and "Your borrowed books" for libraries
whose OPAC home page has a Featured list (Canning: What's New, Coming Soon, eLibrary, First Nations,
Languages Other Than English, Library of Things, Local History, More to Celebrate). It loads on its
own (`HomeFeaturedModel`, once per library and verification, again on pull to refresh) and is
hidden when the list is empty or fails: most libraries (Hutt, Geelong, Cumberland) have none.

- **Tiles** (`FeaturedRow.kt`): a `surface` card with the library's Font Awesome icon on a tinted
  circle (`FeaturedIcon.forName`: book, balloons → celebration, computer, circle, globe, game-board
  → puzzle piece, map-location-dot → map, user-circle → person in a circle, anything else →
  sparkles) and the title in two lines.
- **Tap** (`AppNavigator.openFeatured`): `COLLECTION` → `Route.Collection(title, url)`;
  `SEARCH` → `Route.ListResults(url, title)`; `WEB` → the in-app browser (`InAppBrowser`, a Custom
  Tab in the app's navy via `androidx.browser`, else the default browser).
- **Collection screen** (`CollectionScreen`, `CollectionModel`): `collection(url)`, then one shelf per
  section (title, "View All" → `ListResults`, covers from the first `page(section.url)`, up to 20).
  A shelf loads when its row is composed (scrolled into view), at most three at a time (a
  `Semaphore`); an empty section is hidden, a failed one shows Retry in its row. A page without
  sections (eLibrary), or whose sections all came back empty, shows "This collection is on the
  library's website." / "Nothing here right now" and **Open website**.
- **Results by URL** (`ListResultsScreen`): the Search results list started from `page(url)`
  (`SearchResultsModel.Request.url`), paged by `nextPageUrl`, without the sort menu.
- **Edition-level items** (every item in those lists): one format with an empty code and no label
  (`EditionLevel`). Rows, the book page and the reserve sheet show no format chip; the book page
  loads the record through `format.availabilityUrl` first, never re-finds formats, and adds a
  **Copies** list from `holdings(availability.holdingsUrl)` (branch title-cased, status green when
  available, the copy's note, the call number). Favourites keep the format, so a saved item opens
  the same way. Reserve works as for any format (demo mode only while developing).

`LibraryBackend.featured()` / `collection(url)` / `holdings(url)` default to empty (test fakes);
`CoreBackend` forwards to the core, `DemoBackend` to the real library (public GETs).

```sh
# Canning's Featured row; Hutt has none
adb -s emulator-5556 shell am start -S -n com.librespydus.app/.MainActivity --es ls_origin https://canning.spydus.com --ez ls_demo true
adb -s emulator-5556 shell am start -S -n com.librespydus.app/.MainActivity --es ls_origin https://hutt.spydus.co.nz
```

## My Picks (docs §5.11)

Signed in, Favourites is the reader's Pick list on the library account ("My Picks" on the website)
and the bookmark saves to it; signed out, both are this phone's own favourites, unchanged.

- **`AppState.picks`** (`core/PicksStore.kt`), bound to the library and session next to the account:
  the picked set (`irns`, the `workIrn` of every record on the first 10 pages), loaded after sign-in,
  at launch while signed in and when Favourites refreshes; cleared on sign-out, session expiry and
  library switch. `PickIrn`: a results card's `workIrn`; a ref keyed `work:` / `bib:` → that IRN;
  otherwise the loaded record's `bibIrn` (the bookmark is disabled until then).
- **Bookmark** (book page): flips at once, then `savePick` / `removePick`; a failure flips back with
  "Couldn't update My Picks.", `NotLoggedIn` signs out as anywhere else. "Saved to My Picks" /
  "Removed from My Picks" show in a pill above the tab bar (`feature/favourites/PicksOverlays.kt`,
  hosted by `LibreSpydusRoot`).
- **Favourites** (`MyPicksScreen`): "My Picks" / "Saved on your library account", the search results
  row (`SearchResultRow` with a filled bookmark, a long-press menu and a TalkBack action, each of which
  removes the row at once), pull to refresh, paging past 10 pages, "No picks yet".
- **Copy once:** after an interactive sign-in with local favourites, "Add your saved books to My
  Picks?" (Add / Not now), remembered per library in the `picks` SharedPreferences
  (`copyOffered.<sha256(origin)>`, demo under `demo:<origin>`; `ls_reset` clears it). Add saves them
  oldest first (`brn:` / `url:` refs load their record for the bib IRN) and says "Added N books to My
  Picks" (+ " · M couldn't be added").
- **Demo:** `core/DemoPicks.kt`, in memory, seeded with The Midnight Library (a work, three chips) and
  The Thursday Murder Club's Book (edition-level, no chip). Never call these against a real library
  while developing (docs §2).

```sh
# Demo My Picks; with local favourites seeded for the copy offer (sign out, then sign in demo / demo)
adb -s emulator-5556 shell am start -S -n com.librespydus.app/.MainActivity --ez ls_reset true \
  --es ls_origin https://canning.spydus.com --ez ls_demo true --ez ls_seed_favourites true --es ls_route favourites
```

## Code map (`app/src/main/java/com/librespydus/app/`)

| path | owner | what |
|---|---|---|
| `MainActivity.kt`, `LibreSpydusApplication.kt` | A1 | edge-to-edge activity; Application holds `AppState` and the Coil loader |
| `core/AppState.kt` | A1 | single source of truth (StateFlows): library, backend, session, account name, demo, tabs, pending search/navigation, verification/login flags, generation counters, favourites |
| `core/LibraryBackend.kt` | A1 | `LibraryBackend` interface (every `SpydusClient` method + `origin`) and `CoreBackend` |
| `core/DemoBackend.kt`, `core/DemoCatalogue.kt` | A1, A6 | `DemoBackend` + `DemoData` (assets/demo); the demo account's books offline |
| `core/FavouritesStore.kt` | A1 | per-origin favourites, `files/favourites.json` |
| `core/SecureStore.kt` | A1 | AES-256-GCM over SharedPreferences with an Android Keystore key (sessions, verification cookies); `AesGcmSealer`, `KeystoreKeys` |
| `core/AccountStore.kt`, `core/AccountCache.kt` | A5 | the saved account: background refresh, per-origin sealed snapshot files |
| `core/LibraryStore.kt` | A1 | selected library in Preferences DataStore |
| `core/BookRef.kt`, `core/Route.kt` | A1 | `BookRef` (+ constructors from `SearchRecord` / `PatronItem` / `RelatedItem` / `RecordDetail`), `Tab`, `Route`, `ResultsSort` |
| `core/UserFacingError.kt`, `core/Formatting.kt` | A1 | §5.7 messages; author, due date and greeting helpers |
| `core/CoreHelpers.kt` | A1 | seams for the P helpers (`userAgent`, `isDigitalFormat`, `availabilitySummary`, `homeGenres`, `homeShelves`) and `LibraryDirectory` |
| `core/LaunchOptions.kt` | A1 | DEBUG intent extras |
| `core/PicksStore.kt`, `core/DemoPicks.kt` | A10 | My Picks (§5.11): `PicksStore` (set, list, save/remove, copy offer, notices), `PickIrn`, `PicksText`, `PicksOfferStore`; the demo Pick list |
| `core/Alerts.kt`, `core/AlertsContent.kt`, `core/AlertsAndroid.kt`, `core/MorningCheckWorker.kt` | A7 | ready-to-collect alerts and due-date reminders (§5.8): processor + stores, wording and 07:45 maths, channels/permission/tap intent, the WorkManager morning check |
| `ui/theme/` | A1 | `design/tokens.json` as `LsColors`, `LsType`, `LsRadius`, `LsSpacing`, `LsCover`, `LsTabBar`; `LsIcons` |
| `ui/components/` | A1 | `CoverImage`, `ShelfCover`, `BookRowCard`, `SectionHeader`, `SearchField`, `PrimaryPillButton`, `SecondaryPillButton`, `CircleIconButton`, `LsTopBar`, `GenreTile`, `Chip`, `SegmentedToggle`, `LoadingView`, `EmptyStateView`, `ErrorStateView`, `PlaceholderScreen`, `LsTabBar`, `ComponentGallery` |
| `ui/nav/` | A1 | `LibreSpydusRoot` (splash, onboarding, tabs, AppState-driven sheets), one `NavHost` per tab, `AppNavigator`, `LocalAppState` |
| `feature/onboarding/`, `feature/profile/`, `feature/borrowed/`, `feature/favourites/` | A2 (+ A10) | `LibraryPickerScreen`, `LoginSheet`, `VerificationScreen`, `ProfileScreen` (+ A7 `NotificationsSection`), `BorrowedScreen`, `FavouritesScreen` (+ A10 `MyPicksScreen`, `MyPicksLogic`, `PicksOverlays`) |
| `feature/home/`, `feature/search/`, `feature/detail/` | A3 (+ A9) | `HomeScreen`, `SearchScreen`, `SearchResultsScreen` (+ `ListResultsScreen`), `BookDetailScreen` (+ the edition-level "Copies" list) |
| `feature/featured/` | A9 | Featured collections (§5.10): `FeaturedRow` (Home section + tiles), `HomeFeaturedModel`, `CollectionScreen` / `CollectionModel`, `FeaturedLogic` (icon mapping, routing, `EditionLevel`, `CopyRow`), `FeaturedIcons`, `InAppBrowser` |
| `feature/mycard/` | A8 | My card (§5.9): `MyCardButton` (Home header), `MyCardSheet` (card, entry form, `BarcodeView`, brightness), `MyCardLogic` (content, `BarcodeLayout`, `CardSymbologies`, `BrightnessBoost`) |

### Notes for screen authors (waves 2–3)

- Every screen gets `(appState: AppState, navigator: AppNavigator, modifier)`. Detail also gets the
  `BookRef`, and results get the `Route.Results`. The NavHost in `ui/nav` calls these signatures,
  so keep them. Use `navigator.openBook(BookRef.from(record))`,
  `navigator.openResults(query, sort, title)`, `navigator.back()` and `navigator.openSearch(query)`.
- Get data from `appState.backend.value` (a `LibraryBackend`) and the session from
  `appState.session.value`. Catch `SpydusException` and show `appState.report(e)`. That returns
  the `UserFacingError` for `ErrorStateView`. It also presents verification or login when needed.
  Re-run loads when `verificationGeneration` or `accountGeneration` changes (use them as
  `LaunchedEffect` keys).
- Tab content is laid out above the tab bar. Each screen handles its own top inset
  (`statusBarsPadding()`), because the detail screen's navy header extends under the status bar.
- Mutations (`placeReservation`, `cancelReservation`, `editReservation`) run only after the user
  confirms. `ReserveResult.ok == false` is data to show, not an error.
- Use `CoreHelpers.*` (`userAgent`, `isDigitalFormat`, `availabilitySummary`, `homeGenres`,
  `homeShelves`) and `Formatting.parseDueDate`: they call the core and map to app types.
  `LibraryBackend.searchSorted` calls the core's `searchSorted`. "Reserve" depends on a
  format's `reserveUrl` only; some libraries (Brazoria County) accept holds on ebooks.
- Tests: JVM tests in `app/src/test/` (see `FakeBackend`), device tests in `app/src/androidTest/`.
  JVM tests call the real core on the host: Gradle's `buildHostCore` task runs
  `cargo build --lib` in `core/` (first run ~1–2 min) and points JNA at `core/target/debug`
  (or `$CARGO_TARGET_DIR/debug`).
