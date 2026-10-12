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

`gallery` opens the **components gallery**, which shows every DesignSystem component on one
screen. Example:

```sh
xcrun simctl launch --terminate-running-process "$UDID" com.librespydus -LSReset YES \
  -LSLibraryOrigin https://canning.spydus.com -LSDemo YES -LSRoute borrowed
```

## Demo mode

`-LSDemo YES` swaps `CoreBackend` for `DemoBackend`:

- **Public reads** (search, page, availability, record, related) go to the real library. These are
  public GETs.
- **Account calls** never touch the network. They're served from `Resources/DemoData/`:
  - `demo-account.json`, `demo-reserve-options.json` and `demo-edit-options.json` are the synthetic
    goldens verbatim (a test checks this);
  - `demo-reservations.json` is `patron-items.json` plus two pending rows built from the public
    `related.json` golden;
  - `demo-loans.json` holds five loans whose "Due" date is generated from `dueInDays` relative to
    today (−2, 2, 5, 12, 19).
- **Sign-in:** `login("demo", "demo")` succeeds and anything else is `CredentialsRejected`. The
  session lives in memory only, never in the Keychain.
- **Mutations** succeed after 0.6 s and update the in-memory lists: cancel removes the row,
  reserve adds a pending row (titled from the search that produced the reserve URL), and edit
  changes the pickup location. Reserving the same title twice gives `ok: false`.

Use demo mode to build and screenshot every account screen. Never sign in to a real library
(docs/native-apps.md §2).

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
- **Errors:** `model.report(error)` returns a `UserFacingError` to show (nil for cancellation) and
  performs the side effects. `HumanVerificationRequired` presents the verification sheet. After two
  completed verifications that are still rejected, it stops asking and returns
  `verificationUnavailable` instead. `NotLoggedIn` clears the session and presents login.
- **Verification cookie:** stored per origin in the Keychain and re-applied whenever a client is
  created (`completeVerification(cookie:)`, `clearVerification()`). docs/native-apps.md §5.2
  asks for this persistence. core/README.md's advice to keep the token only in the web view's
  cookie store predates it.
- **Navigation:** `Route.detail(BookRef)` and `Route.results(query:sort:title:)` are handled by
  every tab's stack. Use `NavigationLink(value:)` or `model.push(_:on:)`.
- **`BookRef`** has initialisers from `SearchRecord`, `PatronItem` and `RelatedItem`, plus
  `init(url:)`. `updated(with: RecordDetail)` switches to the stable `bookmarkUrl` and keeps the
  favourite key.

### Pending core additions (`TODO(P)`)

The P task adds `noResults`, `SearchSort` + `searchSorted`, `userAgent()` and the §4.1 helpers.
Until they're merged, the app uses local fallbacks, each marked `TODO(P)`
(`grep -rn "TODO(P)" ios/LibreSpydus`):

| fallback | after P merges |
|---|---|
| `CoreBackend.searchSorted` calls `search` | call `client.searchSorted(query:sort:)` |
| `SearchSortOption` (app enum) | map it to the core's `SearchSort` |
| `SearchResults.isNoResults` | return `noResults` |
| `AppModel.webViewUserAgent` (a copy of `config::USER_AGENT`) | return `userAgent()` |
| `FormatRef.isDigital` heuristic | call `isDigitalFormat(format:)` |
| `DueDate.parse` | call `parseDueDate(text:)` |
