# LibreSpydus for iOS

SwiftUI app on the shared Rust core (`core/`, UniFFI). The core does every fetch, parse, login
and cache on the device; the app is UI plus local storage. The build contract is
[`docs/native-apps.md`](../docs/native-apps.md) and the core's API is
[`core/README.md`](../core/README.md).

| | |
|---|---|
| Bundle ID / name | `com.librespydus` / LibreSpydus |
| Minimum | iOS 17.0, iPhone portrait (on iPad it runs as an iPhone app) |
| Language | Swift 6 language mode (app, tests and the bindings package). The generated bindings compile cleanly in both Swift 5 and 6 modes. |
| Toolchain | Xcode 27 (Swift 6.4), Rust 1.97, UniFFI 0.32.2 |

## Layout

```
ios/
  scripts/build-core.sh           builds LibreSpydusCore.xcframework (see below)
  LibreSpydusCore/                Swift package, product `LibreSpydusCore`
    Package.swift                 binaryTarget `librespydus_coreFFI` + Swift target `LibreSpydusCore`
    Sources/LibreSpydusCore/librespydus_core.swift  → symlink to core/generated/swift/librespydus_core.swift
    LibreSpydusCore.xcframework   build output (gitignored)
  LibreSpydus.xcodeproj           app + LibreSpydusTests, folder-synchronized groups, shared scheme
  Config/LibreSpydus-Info.plist   launch-screen colour (merged with the generated Info.plist)
  LibreSpydus/
    App/            LibreSpydusApp, RootView (tab shell + root sheets), LaunchOptions (debug hooks)
    Core/           AppModel, LibraryBackend (+ CoreBackend, DemoBackend, DemoData), FavouritesStore,
                    SecretStore (Keychain), Route/AppTab, BookRef, UserFacingError + Loadable, Formatting
    DesignSystem/   Theme (tokens), Appearance, Components/, ComponentsGallery (DEBUG)
    Features/       one folder per screen; wave 1 ships each as a compiling stub
    Resources/      Assets.xcassets (AppIcon, AccentColor, LaunchBackground), DemoData/*.json
  LibreSpydusTests/ Swift Testing; add your own <Feature>Tests.swift files
```

The project uses Xcode folder-synchronized groups (`PBXFileSystemSynchronizedRootGroup`) for
`LibreSpydus/` and `LibreSpydusTests/`. **Add, move or delete files on disk; never edit
`project.pbxproj` for it.** Swift files are compiled and everything else (JSON, asset catalogs)
is bundled. Resources are copied flat into the app bundle, so give them unique names
(e.g. `demo-loans.json` → `Bundle.main.url(forResource: "demo-loans", withExtension: "json")`).

## Prerequisites

- Xcode 27 with an iOS 27 simulator runtime.
- Rust via rustup, plus the iOS targets:
  ```sh
  rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
  ```

## Build the core

```sh
ios/scripts/build-core.sh            # release (default): about 2.5 min from clean on an M-series Mac
ios/scripts/build-core.sh --debug    # faster to compile, much slower at runtime
ios/scripts/build-core.sh --from "$MAIN/ios/LibreSpydusCore/LibreSpydusCore.xcframework"
ios/scripts/build-core.sh --help
```

The script builds the `staticlib` for `aarch64-apple-ios` (device), `aarch64-apple-ios-sim` and
`x86_64-apple-ios` (simulator, combined with `lipo`). It then runs `xcodebuild -create-xcframework`
with the FFI header and a module map that exposes the Clang module `librespydus_coreFFI`. The
generated Swift imports that module. `CARGO_TARGET_DIR` is honoured.

`--from` copies a prebuilt framework instead of building one, which is all a screen worker needs
(docs/native-apps.md §8). If the copied header differs from `core/generated/swift/`, the script
warns. An app built from a framework that doesn't match the bindings traps at launch with
"UniFFI API checksum mismatch".

**Single source of truth for the bindings.** The package compiles
`core/generated/swift/librespydus_core.swift` through a committed symlink, so there's no copy to
keep in sync and nothing generated to gitignore. The header and module map are copied into the
(gitignored) xcframework at build time. After regenerating bindings in `core/`, rerun
`build-core.sh`.

## Build, test, run

```sh
UDID=$(xcrun simctl create "LS-me" com.apple.CoreSimulator.SimDeviceType.iPhone-17-Pro \
  com.apple.CoreSimulator.SimRuntime.iOS-27-0)
xcodebuild -project ios/LibreSpydus.xcodeproj -scheme LibreSpydus \
  -destination "platform=iOS Simulator,id=$UDID" -derivedDataPath ios/build/DerivedData build test
xcrun simctl boot "$UDID"; open -a Simulator
xcrun simctl install "$UDID" ios/build/DerivedData/Build/Products/Debug-iphonesimulator/LibreSpydus.app
xcrun simctl launch "$UDID" com.librespydus -LSLibraryOrigin https://canning.spydus.com -LSDemo YES
xcrun simctl delete "$UDID"   # when done
```

You can also open `ios/LibreSpydus.xcodeproj` in Xcode and run the shared **LibreSpydus** scheme.
The scheme carries the debug launch arguments below, switched off by default.

`LiveCoreSmokeTests` makes one public GET search on Canning: it proves that FFI, async and TLS
work on iOS. To run offline, add `-skip-testing:LibreSpydusTests/LiveCoreSmokeTests`.
`ThemeTokensTests` and the demo-data golden test read `design/tokens.json` and `core/tests/golden/`
from the checkout. The simulator can read them; elsewhere those tests are reported as skipped.

## Debug launch hooks (DEBUG builds only)

Pass them as launch arguments (docs/native-apps.md §5.6):

| argument | effect |
|---|---|
| `-LSLibraryOrigin <origin>` | select a library and skip the picker. The directory entry (name, logo) is looked up with one public GET. |
| `-LSRoute <route>` | `home`, `search:<query>`, `results:<query>`, `detail:<url>`, `borrowed`, `favourites`, `profile`, `libraries`, `login`, `verify`, `gallery` |
| `-LSDemo YES` | demo mode, already signed in as "Test Borrower" |
| `-LSReset YES` | first clear the persisted library, sessions, verification cookies and favourites |
| `-LSGalleryAnchor <section>` | with `-LSRoute gallery`: scroll to `rows`, `chips`, `buttons`, `sheet`, `states` or `type` |
| `-LSPickerQuery <text>` | the library picker starts with this search text |
| `-LSLoginCard <card> -LSLoginPIN <pin>` | with `-LSRoute login`: fill the sign-in form; add `-LSLoginSubmit YES` to sign in once. **Demo mode only** (ignored without `-LSDemo YES`), so a worker can't sign in to a real library by accident |
| `-LSProfileAnchor bottom` | with `-LSRoute profile`: start scrolled to the end of Profile |
| `-LSPersistDemoAccount YES` | with `-LSDemo YES`: save the demo account on the device (in `AccountCache/Demo/`), to show a cold launch with saved data. Off by default: demo mode keeps the account in memory |
| `-LSDemoRefreshFails YES` | with `-LSDemo YES`: every account refresh fails, to show "Couldn't refresh — showing saved data" (with saved data) or the error state (without) |
| `-LSMorningCheckNow YES` | run the alerts' morning check once the launch refresh has settled, instead of waiting for 07:45 (see "Alerts") |
| `-LSDemoNewReady YES` | with `-LSDemo YES`: "Hollow bones" (pending, Riverton) is ready to collect from the demo account's second refresh on, so a "Ready to collect" alert can be shown |
| `-LSAlertTime HH:mm` | due reminders show, and the morning check is scheduled, at this time instead of 07:45, to watch a reminder arrive |
| `-LSMyCard open` / `-LSMyCard edit` | while signed in: Home opens the "My card" sheet (`edit`: on its entry form, filled in) |
| `-LSDemoCardNumber <number>` | with `-LSDemo YES`: the demo card number instead of `DEMO0012345`; `none` leaves no number, so the sheet asks for one |
| `-LSHomeOpen <list>` | Home opens a list as if tapped: `arrivals`, `genre:<key>`, or `featured:<title>` (a Featured tile, e.g. `featured:Library of Things`, once the row has loaded) |
| `-LSHomeScrollTo loans\|featured` | Home scrolls down to "Your borrowed books" / "Featured" |
| `-LSCollectionOpen <section>` | a Featured collection opens that section's "View All" (e.g. `Musical Instruments`) |
| `-LSCollectionItem "<section>\|<words>"` | a Featured collection scrolls to the section and opens its first item whose title has those words |
| `-LSCollectionScrollTo <section>` | a Featured collection scrolls to that section |
| `-LSCollectionOpenWebsite YES` | a collection without sections (eLibrary) taps "Open website" |
| `-LSDetailAction <actions>` | the first book page, once loaded: `format:<code>`, `favourite` (signed out: a local favourite; signed in: saves to My Picks, demo mode only), `related`, `bottom`, `copies` (an edition-level item's Copies list), `open-related`, `reserve`, `confirm` (demo only) |

`gallery` opens the **components gallery**, which shows every DesignSystem component on one
screen. Example:

```sh
xcrun simctl launch --terminate-running-process "$UDID" com.librespydus -LSReset YES \
  -LSLibraryOrigin https://canning.spydus.com -LSDemo YES -LSRoute borrowed
```

## Demo mode

`-LSDemo YES` swaps `CoreBackend` for `DemoBackend`:

- **Public reads** (search, page, availability, record, related) go to the real library. These are
  public GETs. The demo account's own books are the exception: `demo-catalogue.json` (their public
  Canning records, captured 2026-10-12 and trimmed) answers for them offline, the way the OPAC
  does. A loan's or reservation's own `bibUrl` (`…/BIBENQ/78500000?SETLVL=&QRY=IRN(<irn>)&NAVLVL=SET`,
  a set in the patron's session) loads an **empty** record, the session-free
  `…/BIBENQ?SETLVL=&IRN=<irn>` the full one. A search that names one of those works (every word of
  its main title, e.g. "midnight library") returns just that work, with demo availability and
  reserve URLs. "The Midnight Library" has a Book (the ready reservation's bib) and a Large Print
  edition; "The Long Night" has a Book and a Large Print (the overdue loan's bib).
- **Account calls** never touch the network. They're served from `Resources/DemoData/`:
  - `demo-account.json`, `demo-reserve-options.json` and `demo-edit-options.json` are the synthetic
    goldens verbatim (a test checks this);
  - `demo-reservations.json` is `patron-items.json` plus two pending rows built from the public
    `related.json` golden, each pointed at its real Canning bib (`bibIrn` + the session-set
    `bibUrl`; the Baby-sitters club row is the real "graphic novels. 7" edition);
  - `demo-loans.json` holds five loans whose "Due" date is generated from `dueInDays` relative to
    today (−2, 2, 5, 12, 19), linked to their bibs the same way.
- **Sign-in:** `login("demo", "demo")` succeeds and anything else is `CredentialsRejected`. The
  session lives in memory only, never in the Keychain.
- **Mutations** succeed after 0.6 s and update the in-memory lists: cancel removes the row,
  reserve adds a pending row (titled from the search that produced the reserve URL and linked to
  the reserved edition's bib, so the book page then says "Reserved"), and edit changes the pickup
  location. Reserving the same title twice gives `ok: false`.
- **`accountSnapshot`** takes 1.5 s, so the refreshing state is visible, and reflects those
  changes. The account is never saved on the device in demo mode (see `-LSPersistDemoAccount`).
- **My Picks** is an in-memory list seeded with "The Midnight Library" (a work: Book and Large
  Print chips) and "The Love Library" (an edition-level card, as Featured lists show one; its
  record and counts come from the demo catalogue offline). `savePick` (0.6 s) appends a card for
  any IRN the backend has seen (search and page results, records it loaded, the demo works and
  editions); an IRN it knows nothing about is `Upstream("None found")`, a non-numeric one `Parse`.
  `removePick` takes it out; both return the new size, and an emptied list is a genuine
  `noResults`. Pages of 20, like the library's. Relaunching starts from the two seeds again.

Use demo mode to build and screenshot every account screen. Never sign in to a real library
(docs/native-apps.md §2).

## Human verification (UK)

Every `*.spydus.co.uk` library sits behind AWS WAF. Its OPAC answers each request with a "Human
Verification" page, so the core throws `HumanVerificationRequired` (core/README.md "Human
verification"). `AppModel.report` then presents the verification sheet (`VerificationView`):

- The sheet loads `origin + "/"` in a `WKWebView` that uses the default (persistent) website data
  store and `customUserAgent = userAgent()`, the core's own User-Agent, because the WAF token may
  be tied to it.
- The **user** completes the check. A `WKHTTPCookieStoreObserver`, plus a check after every
  finished page load, looks for an `aws-waf-token` cookie whose domain matches the library's host
  (`VerificationCookie.header`, unit-tested: host-only or leading-dot parent domains, not expired,
  not empty). The token is taken only after the last page response has no `x-amzn-waf-action`
  header, which means the web view shows the library's own page again. A token the challenge sets
  before the user finishes is therefore never used.
- `completeVerification(cookie: "aws-waf-token=<value>")` stores the cookie per origin (Keychain)
  and applies it to the client. The sheet closes, and screens rerun their loads through
  `verificationGeneration`. After two completed verifications that the library still rejects, the
  app stops asking and says the library can't be used from the app right now.
- The app never injects scripts into the challenge or interacts with it. Workers may open the
  sheet to check that it renders, and nothing more (docs/native-apps.md §2).
- Profile shows "Verified on this device" while a cookie is stored. **Clear verification**
  removes it from the Keychain, the client and the web view's cookie store.

Shortcut to the sheet: `-LSLibraryOrigin https://aberdeencity.spydus.co.uk -LSRoute verify`.

### Manual end-to-end test (for the user)

Only a person may do this. It needs the Search screen (I3) merged.

1. Install a debug build and launch it with `-LSReset YES`, without `-LSDemo`.
2. In "Choose your library", search `aberdeen` and pick **Aberdeen City Libraries**. Any UK row
   works; they're marked "Asks you to verify you're human".
3. Open the Search tab and search `harry potter`. The "Verify you're human" sheet opens on the
   library's "Let's confirm you are human" page.
4. Tap **Begin** and complete the puzzle.
5. The page reloads into the library's website and the sheet closes by itself within a second
   or two.
6. **Expected:** the search reruns and results load. Profile shows "Verified on this device".
7. If the sheet opens again, the library rejected the token from the app. Complete the check
   once more. If it's rejected again, the screen says the library can't be used from the app
   right now. Report which of these happened, because whether a WAF token works outside a browser
   is still unverified (core/README.md).
8. Profile → **Clear verification** → confirm, then search again. The sheet opens again.

## App core in brief

- **`LibraryBackend`** is the only API screens use: every `SpydusClient` method plus `origin`
  and `setVerificationCookie`.
- **`AppModel`** (`@Observable`, `@MainActor`, read it with `@Environment(AppModel.self)`) holds:
  - the selected `library` (persisted in UserDefaults) and its `backend`, one per origin;
  - the `session` per origin (Keychain), `accountName`, and `favourites`;
  - `selectedTab`, per-tab `paths`, and `pendingSearch` (Home → Search);
  - the root-sheet flags `loginNeeded`, `verificationNeeded` and `libraryPickerPresented`;
  - the reload counters `verificationGeneration` and `accountGeneration`, combined in `reloadKey`
    for `.task(id: model.reloadKey)`.
- **Saved account** (docs/native-apps.md §5.2a): `model.account` is an `AccountStore` holding the
  `accountSnapshot` (name, loans, reservations) for the current origin and session. It's saved
  per origin by `AccountSnapshotCache` (`Application Support/AccountCache/<sha256(origin)>.json`,
  the core's `encodeAccountSnapshot` JSON, written atomically, `.completeUntilFirstUserAuthentication`,
  excluded from backup; a file that doesn't decode is deleted) and loaded synchronously at launch
  and on a library change, so Your Books, Home's shelf, the greeting and Profile show it on the
  first frame. `refresh(force:)` fetches in the background: at launch and on a library change,
  on sign-in, after a reserve / cancel / edit (`performChange`, `accountDidChange()`), on pull to
  refresh (force), and, when the snapshot is more than 2 minutes old or the last refresh failed,
  on returning to the app and on opening Your Books or Home. One fetch runs at a time and callers
  share it; a result for another origin or session, or one that overlaps a change, is dropped. A
  failed refresh keeps the snapshot and sets `lastRefreshError` (Your Books: "Couldn't refresh —
  showing saved data" + Retry). Sign-out and an expired session delete the origin's file; other
  libraries' files stay. Demo mode never saves it.
- **Errors:** `model.report(error)` returns a `UserFacingError` to show (nil for cancellation) and
  performs the side effects. `HumanVerificationRequired` presents the verification sheet. After two
  completed verifications that are still rejected, it stops asking and returns
  `verificationUnavailable` instead. `NotLoggedIn` clears the session and presents login.
- **Verification cookie:** stored per origin in the Keychain and re-applied whenever a client is
  created (`completeVerification(cookie:)`, `clearVerification()`). docs/native-apps.md §5.2
  asks for this persistence. core/README.md's advice to keep the token only in the web view's
  cookie store predates it.
- **Navigation:** `Route.detail(BookRef)`, `Route.results(query:sort:title:)`,
  `Route.collection(FeaturedLink)` and `Route.resultsPage(url:title:)` are handled by every tab's
  stack. Use `NavigationLink(value:)` or `model.push(_:on:)`.
- **`BookRef`** has initialisers from `SearchRecord`, `PatronItem` and `RelatedItem`, plus
  `init(url:)`. `updated(with: RecordDetail)` switches to the stable `bookmarkUrl` and keeps the
  favourite key. A ref from a `PatronItem` loads `patronItemRecordUrl(item:)` (the row's own
  `bibUrl` needs the patron's OPAC session) and keeps the row's bib IRN in its id `bib:<IRN>`
  (`bibIrn`).
- **Book page "Already yours"** (docs/native-apps.md §6): `BookDetailModel.holding(in:)` runs
  `accountHolding` on the saved account with the shown edition's bib IRNs (the selected format's
  `Availability.bibRecordIrn`, else the record's and the opened row's). A holding replaces Reserve
  with a status pill (`HeldStatus`: "On loan · Due 14 Oct", "Ready to collect · Riverton Library",
  "Reserved · #2 in queue") that opens Your Books on that segment (`AppModel.openYourBooks(on:)`).
  A page opened from a loan or reservation selects the format whose `bibRecordIrn` is the row's,
  and a record that comes back empty counts as a failed load.

- **Alerts** (docs/native-apps.md §5.8): local notifications, no server. `AlertCenter`
  (`model.alerts`) holds the two Profile switches ("Ready to collect", "Due-date reminders"; off by
  default, app-wide, in UserDefaults `alerts.readyEnabled` / `alerts.dueEnabled`; demo mode uses
  `alerts.demo.*` so it never changes the real ones) and processes every fetched snapshot:
  `AccountStore.didFetch` runs after each successful `accountSnapshot` (launch, foreground, pull to
  refresh, after a reserve / cancel / edit, the morning check). Ready: `readyAlerts(snapshot,
  savedKeys)` → one notification per new item (`ready:<sha256(origin)>:<patronItemKey>`, "Ready to
  collect" / "<Title> · <Branch>", the branch as Your Books shows it) and the returned keys replace
  `alerts.readyKeys.<sha256(origin)>`. Turning Ready on saves the keys of what's already waiting,
  without a notification; a library with no saved keys (signed in again after a sign-out or an
  expired session, or switched on before the account loaded) is seeded the same way by its next
  snapshot, so nothing already waiting is announced as new. Due: `dueReminders(snapshot, today, 2)` → this library's pending `due:`
  requests are replaced by `UNCalendarNotificationTrigger`s at 07:45 on each `remindOn` (one whose
  07:45 has passed is skipped), "Due Thursday 15 Oct" / "Due today" / "Due tomorrow" and "<A> and
  N more". Sign-out and session expiry delete that library's keys and its pending and delivered
  notifications; a library change keeps them. A tap opens Your Books on Reservations (ready) or
  Loans (due) through `AlertNotificationRouter`, the notification centre's delegate, which also
  shows banners while the app is open. The copy and timing are in `AlertContent`; the notification
  centre and `BGTaskScheduler` sit behind `NotificationScheduling` / `MorningCheckScheduling`, so
  the tests use fakes (`AlertCenterTests`, `AlertContentTests`).
- **My card** (docs/native-apps.md §5.9): Home's "My card" capsule (signed in) opens
  `MyCardSheet`, the library card as a barcode on a white card. `AppModel.cardNumber` is the number
  entered on the sheet (`setCardNumber`, Keychain service `com.librespydus.card`, account = origin;
  in memory in demo mode, where it starts as `DEMO0012345`), else `accountCardNumber`, the
  account's own number, which is nil until the core reads it from the library's Personal details.
  Signing in saves nothing (the sign-in ID may be a username); sign-out deletes the library's
  number, an expired session keeps it. The symbology is `LibraryBackend.cardBarcodeSymbology()`
  (`CoreBackend` and `DemoBackend` ask the library: a public GET, cached in the core; test fakes get
  Code 128), remembered per origin by `AppModel.knownCardSymbology`, so only a library's first open
  shows a spinner. `cardBarcode(cardNumber:symbology:)` gives the modules; `BarcodeView` draws them
  in a `Canvas` without antialiasing, snapped to the pixel grid, with `BarcodeLayout`'s whole device
  pixels per module (floor(pixels / (modules + 20)), at least 1) and 10-module quiet zones, centred
  across the card's full width. The screen goes to full brightness while the sheet is up
  (`BrightnessBoost`, restored on close and when the app leaves the foreground; DEBUG prints
  `🔆 card brightness …`).
- **Featured collections** (docs/native-apps.md §5.10): `LibraryBackend.featured()`,
  `collection(url:)` and `holdings(url:)` (`CoreBackend` forwards to the client, `DemoBackend` to
  the real public backend; test fakes get defaults that return nothing). Home's "Featured" row
  (`FeaturedRow`, `FeaturedRowModel`) loads on its own, keyed on origin + verification, between
  "Popular genres" and "Your borrowed books"; it's hidden while loading, when the list is empty
  (most libraries) and when it fails. Icons map the library's Font Awesome names
  (`FeaturedIcon`, sparkles for anything else). A tile leads where `FeaturedDestination` says:
  `.collection` → `CollectionView`, `.search` → `ResultsPageView` (the results list started from
  `page(url)`: `SearchResultsModel.Request.url`, no sort menu), `.web` → the in-app browser
  (`SFSafariViewController`, `.inAppBrowser($page)`, http(s) only). `CollectionModel` loads the
  page, then one shelf per section from its first results page (≤ 20 covers) as the row comes
  into view, at most 3 at a time (a row scrolled past before its turn waits until it's back);
  empty sections are hidden, a failed one shows Retry in its row, and a page without sections
  (eLibrary) shows "This collection is on the library's website." + **Open website**.
  **Edition-level items** (every card in these lists: one format with an empty code and no label,
  `[FormatRef].isEditionLevel`) get no format chip in results rows, on the book page or in the
  reserve sheet; the book page loads the record from that format's `availabilityUrl` first
  (`DetailRecordSource.candidateURLs`), never re-finds formats, and lists the copies under the
  synopsis (`BookDetailModel.copies` from `Availability.holdingsUrl` → `holdings`, `CopyRow`:
  branch in title case, status green when available, the library's note, call number).
  Favourites store the format with the item, so a saved one opens the same way.
- **My Picks** (docs/native-apps.md §5.11): signed in, Favourites is the Pick list on the library
  account and the book page's bookmark saves to it. `AppModel.picks` is a `PicksStore`, pointed at
  the current origin and session like `AccountStore` (sign-out, an expired session and a library
  change clear it). `refresh()` loads the first page of `LibraryBackend.picks(session:url:)`, then
  the rest in the background up to 10 pages, collecting every record's `workIrn` into `irns`
  (`contains(_:)`); later pages load as the list scrolls. It loads at launch, sign-in, library
  change and verification (RootView's `.task(id: model.reloadKey)` → `loadIfNeeded()`), on pull
  to refresh, and when Favourites comes on screen after a save it doesn't show yet (`isStale`).
  A failed first load is the error state with Retry; a failed refresh keeps the rows and shows
  the error as a notice.
  `setPicked(_:irn:)` flips the IRN at once (a removed row leaves the list at once), calls
  `savePick` / `removePick`, and flips back on failure; a change is laid over any load that
  overlaps it. The IRN is `PickIrn.of(ref, record:)`: a `work:` / `bib:` key's IRN, else the loaded
  record's `bibIrn` (`BookDetailModel.pickIrn`; the bookmark is disabled until it's known).
  Confirmations ("Saved to My Picks", "Removed from My Picks", "Couldn't update My Picks.", the
  copy's progress and summary) are `picks.notice`, shown by `PicksNoticeOverlay`, a toast above
  the tab bar on any screen, and announced to VoiceOver; `NotLoggedIn` takes the usual
  session-expired path instead. `MyPicksView` is a `List` of `SearchResultCard`s (the search row's
  card; a list would give a navigation link a chevron) with swipe-to-delete, a long-press menu and
  a VoiceOver action; signed out, `FavouritesView` shows `LocalFavouritesView`, unchanged.
  **Copy once:** an interactive `signIn` (never a restored session) with local favourites for a
  library not asked before sets `picksCopyOffer`; `PicksCopyOfferAlert` (RootView) asks "Add your
  saved books to My Picks?" once the sign-in sheet has gone. It waits while the app isn't active
  and comes back if something closed it unanswered (iOS's "Save Password?" prompt can, right after
  a sign-in). Either answer is remembered per library (UserDefaults
  `picks.copyOffered.<sha256(origin)>`, demo mode `picks.demo.…`, cleared by `-LSReset`); Add
  (`PicksStore.copy`) saves the local favourites oldest first, one at a time, loading a `brn:` /
  `url:` ref's record for its `bibIrn` (skipped without one), stops on a lost session, a
  verification challenge or a lost connection, shows "Added {k} books to My Picks" (" · {m}
  couldn't be added"; the error tone when none was added), reloads the list and keeps the local
  favourites.
- **Morning check:** the `BGAppRefreshTask` `com.librespydus.morning-check` (Info.plist
  `BGTaskSchedulerPermittedIdentifiers`, `UIBackgroundModes` `fetch`), handled by
  `.backgroundTask(.appRefresh(…))` on the app's scene. `earliestBeginDate` is the next 07:45; it's
  submitted at launch, on going to the background, on sign-in, sign-out and a library change, when
  a switch changes and after each run, while signed in with a switch on, and cancelled otherwise.
  A run (`AppModel.runMorningCheck`) fetches the account with `AccountStore.refreshNow()` (saved
  like any refresh, so the app opens with fresh data, and processed on the way), gives up waiting
  after 25 s or when the task is cancelled, and schedules the next one. iOS decides when it runs
  (often soon after 07:45 on a phone used in the morning; later, or not at all in Low Power Mode or
  after the app was swiped away). Submits are held until the app has finished launching: before
  SwiftUI registers the handler, `submit` raises an Objective-C exception. The simulator has no
  background tasks (`submit` fails with "not available on this platform", logged): use
  `-LSMorningCheckNow YES` there. On a device, pause in the debugger after the app has gone to the
  background and run
  `e -l objc -- (void)[[BGTaskScheduler sharedScheduler] _simulateLaunchForTaskWithIdentifier:@"com.librespydus.morning-check"]`,
  then continue. DEBUG builds print `🔔 alerts …` lines (posted, scheduled, morning check).

  Show the alerts in demo mode: Profile → turn both switches on (allow notifications), then
  relaunch with `-LSLibraryOrigin https://canning.spydus.com -LSDemo YES -LSDemoNewReady YES
  -LSMorningCheckNow YES` for "Ready to collect · Hollow bones · Riverton Library", or with
  `-LSAlertTime <a minute from now>` for "Due Thursday 15 Oct · Whistler" (the demo loan due in
  2 days).

### Core helpers in use

`CoreBackend.searchSorted` → `client.searchSorted(query:sort:)` (`SearchSortOption.core` maps to
the core's `SearchSort`); `SearchResults.isNoResults` → the core's `noResults`;
`AppModel.webViewUserAgent` → `userAgent()`; `FormatRef.isDigital` → `isDigitalFormat(format:)`;
`DueDate.parse` → `parseDueDate(text:)`; `BookRef(PatronItem)` → `patronItemRecordUrl(item:)`;
`BookDetailModel.holding(in:)` → `accountHolding(snapshot:bibIrns:)`; `AlertCenter` →
`readyAlerts(snapshot:alertedKeys:)`, `dueReminders(snapshot:today:daysBefore:)` and
`patronItemKey(item:)`; `MyCardModel` → `cardBarcode(cardNumber:symbology:)` and
`CoreBackend.cardBarcodeSymbology` → `client.cardBarcodeSymbology()`; `CoreBackend.featured` /
`collection` / `holdings` → `client.featured()`, `client.collection(url:)`, `client.holdings(url:)`;
`CoreBackend.picks` / `savePick` / `removePick` → `client.picks(session:url:)`,
`client.savePick(irn:session:)`, `client.removePick(irn:session:)` (never run against a real
library by a worker: demo mode only, docs/native-apps.md §2). Use `availabilitySummary(availability:)` for availability
labels and `homeShelves()` / `homeGenres()` for Home. "Reserve" depends on a format's
`reserveUrl` only — some libraries (Brazoria County) accept holds on ebooks.
