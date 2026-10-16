# Copy decisions (C1)

How `design/copy.json` settled every string where the iOS and Android apps worded the same thing
differently, or where a rule in [docs/native-apps.md §7.1](../docs/native-apps.md) changed the
wording. Base: `native-apps` @ 49b114d. The app migrations (iOS I13, Android A11) apply these; the
to-do list is `design/scripts/check_copy.py`'s report.

**Catalogue:** 346 strings in 21 groups: 10 plurals, 0 platform variants, 88 with placeholders.
No string needed a platform variant. The one gesture-dependent string, the swipe hint under
Your Books' reservations, reads the same on both because Android's cards swipe too. iOS 17's
"Touch and hold…" footnote is an OS-version fallback, not an Android/iOS difference, so it is its
own key (`yourBooks.touchAndHoldHint`).

**Rules** (the last column):

| Code | Rule (§7.1 unless noted) |
|---|---|
| G | Glossary: one term per thing (My Picks, Your loans, Collect from / Ready to collect, #n in queue, reservation, …) |
| 1T | One term per thing, for a thing the glossary doesn't list |
| ’ | Curly apostrophes and quotes |
| SC | Sentence case for buttons, titles, alerts, toasts and menus |
| FS | A full stop ends body sentences and error paragraphs, but never a title, button, label, chip or toast |
| PL | Plurals use one/other forms |
| A11Y | Accessibility labels are the visible text or a short noun phrase. Hints start with a third-person verb. Both apps use the same words |
| C | The contract (§5.x / §6) gives exact text that §7.1 doesn't change, so that text is kept or restored |
| D | The apps differed: the clearer of the two (shorter when both were clear) |

Strings that were the same on both apps and that no rule changed are not listed. That is about
220 of them, such as "Retry", "Sign in", "My card", "Popular genres" and every notification text.

## Changed or reconciled strings

| Key | iOS before | Android before | Now | Rule |
|---|---|---|---|---|
| `tabs.myPicks` | Favourites | Favourites | My Picks | G |
| `picks.title` | Favourites (signed out); My Picks (signed in) | Favourites (signed out); My Picks (signed in) | My Picks | G |
| `picks.subtitlePhone` | {count} saved for {library} / Saved on this device for {library} | {count} saved for {library} / Saved on this device for {library} | Saved on this phone | G, C (§5.11) |
| `picks.save` | Add to favourites (signed out); Save to My Picks (signed in) | Add to favourites (signed out); Save to My Picks (signed in) | Save to My Picks | G |
| `picks.remove` | Remove from favourites (bookmark) / Remove from Favourites (menu, signed out) | Remove from favourites / Remove from Favourites (signed out) | Remove from My Picks | G |
| `picks.emptyTitle` | No favourites yet (signed out) | No favourites yet (signed out) | No picks yet | G |
| `picks.emptyPhoneMessage` | Tap the bookmark on any book to save it here. | Tap the bookmark on any book to save it here. | Tap the bookmark on a book to save it on this phone. | G (one wording with the account's empty state) |
| `picks.editHint` | — (label 'Edit favourites' on the 'Edit' button) | — (onClickLabel 'Edit favourites') | Lets you remove books from My Picks | G, A11Y (the label is the visible 'Edit') |
| `picks.doneEditingHint` | — (label 'Done editing favourites' on the 'Done' button) | — (onClickLabel 'Done editing favourites') | Finishes editing My Picks | G, A11Y (the label is the visible 'Done') |
| `picks.retryLoadingMoreHint` | — (label 'Retry loading more picks') | — (contentDescription 'Retry loading more picks') | Loads more picks | A11Y (the label is the visible 'Retry') |
| `picks.removeTitle` | Remove {title} from Favourites | Remove {title} from Favourites | Remove {title} from My Picks | G |
| `picks.removedTitle` | Removed {title} from Favourites | Removed {title} from Favourites | Removed {title} from My Picks | G |
| `picks.loadFailed` | Couldn't read favourites: {system error} | — | Couldn’t load My Picks from this phone. | G, ’, FS (no raw system error) |
| `picks.saveFailed` | Couldn't save favourites: {system error} | — | Couldn’t save My Picks on this phone. | G, ’, FS |
| `picks.updateFailed` | Couldn't update My Picks. | Couldn't update My Picks. | Couldn’t update My Picks | ’, FS (toast: no full stop; §5.11 had one) |
| `picks.offerTitle` | Add your saved books to My Picks? | Add your saved books to My Picks? | Add this phone’s picks to your library account? | C (§5.11's newer text) |
| `picks.offerMessage` | {n} book(s) saved on this phone can be added to your library account. | {n} book(s) saved on this phone can be added to your library account. | 1 book saved on this phone can be added to My Picks on your library account. / {count} books saved on this phone can be added to My Picks on your library account. | C (§5.11), PL |
| `picks.adding` | Adding {n} book(s) to My Picks… | — | Adding 1 book to My Picks… / Adding {count} books to My Picks… | PL; Android gains it |
| `picks.notAdded` |  · {n} couldn't be added |  · {n} couldn't be added | 1 couldn’t be added / {count} couldn’t be added | ’, PL (joined with common.separator) |
| `home.yourLoans` | Your borrowed books | Your borrowed books | Your loans | G |
| `home.noLoansMessage` | Books you borrow will show up here with their due dates. | Books you borrow will show up here with their due dates. | Books on loan show up here with their due dates. | G (no 'borrow'), shorter |
| `yourBooks.noLoansMessage` | Books you borrow from {library} show up here with their due dates. | Books you borrow from {library} show up here with their due dates. | Books on loan from {library} show up here with their due dates. | G (no 'borrow') |
| `yourBooks.readyToCollect` | Ready for pickup (section + chip) | Ready for pickup (section + chip) | Ready to collect | G |
| `yourBooks.collectFrom` | Pick up at {branch} | Pick up at {branch} | Collect from {branch} | G |
| `yourBooks.collectFromBy` | Pick up at {branch} by {date} | Pick up at {branch} by {date} | Collect from {branch} by {date} | G |
| `yourBooks.collectBy` | Pick up by {date} | Pick up by {date} | Collect by {date} | G |
| `yourBooks.queuePlaceText` | Rank {n} | Rank {n} | {place} in queue | G (no 'rank') |
| `yourBooks.queuePlaceLabel` | — (the card read '#4 in queue') | — (the card read '#4 in queue') | number {place} in the queue | G (screen readers: 'number {n} in the queue') |
| `reserve.collectFrom` | Pick up at | Pick up at | Collect from | G, C (§6 'Collect from (branch)') |
| `reserve.usualBranch` | Your usual library | Your usual library | Your usual branch | G ('branch' for a branch) |
| `editReservation.collectFrom` | Pickup location | Pickup location | Collect from | G |
| `editReservation.notNeededAfter` | Not required after | Not required after | Not needed after | 1T (the Reserve sheet's 'Not needed after') |
| `detail.reservedAtLabel` | Reserved, pick up at {branch} | Reserved, pick up at {branch} | Reserved, collect from {branch} | G |
| `detail.reservedQueueText` | Reserved · Rank {n} | Reserved · Rank {n} | Reserved · {place} in queue | G |
| `detail.reservedQueueTextLabel` | Reserved, rank {n} | Reserved, rank {n} | Reserved, {place} in the queue | G |
| `loans.dueOn` | Due 27 Oct; notification 'Due Thursday 16 Oct' (fallback 'Due 16/10') | Due 27 Oct; notification 'Due Thursday 16 Oct' | Due {date} | Date style (§7.1): '15 Oct', the weekday when it helps; no d/M fallback |
| `yourBooks.updatedOn` | Updated 3 Oct at 4:20 pm (any older day) | Updated Monday at 4:20 pm (within a week), else Updated 3 Oct 2026 at 4:20 pm | Updated {day} at {time} | D (same words; {day} = the weekday within a week, as Android; the year only when it isn't this year) |
| `loans.daysOverdue` | {n} days overdue / 1 day overdue | {n} days overdue (cards, Home); Overdue by {n} days (Formatting.daysLeftLabel, unused) | 1 day overdue / {count} days overdue | G, PL |
| `yourBooks.cancelReservation` | Cancel reservation (menu) / Cancel Reservation (confirm button) | Cancel reservation | Cancel reservation | SC, D |
| `yourBooks.keepReservation` | Keep Reservation | Keep reservation | Keep reservation | SC, D |
| `yourBooks.cancelled` | Reservation cancelled. | Reservation cancelled. | Reservation cancelled | FS (toast-style banner) |
| `yourBooks.updated` | Reservation updated. | Reservation updated. | Reservation updated | FS |
| `reserve.placed` | Reservation placed. | Reservation placed. | Reservation placed | FS |
| `home.profileButtonHint` | Opens Profile (hint) | Open Profile (onClickLabel) | Opens Profile | A11Y, D |
| `detail.heldHint` | Opens Your Books (hint) | Open Your Books (onClickLabel) | Opens Your Books | A11Y, D |
| `home.searchField` | Search (label; hint = the placeholder) | Search by title, author, etc. (contentDescription) | Search | A11Y, D (label = short noun; the placeholder stays visible) |
| `home.searchFieldHint` | Search by title, author, etc. (hint) | — | Opens Search | A11Y (hint starts with a verb) |
| `detail.reserveFormatHint` | Reserve the {format} (hint) | Reserve the {format} (contentDescription replaced the label) | Reserves the {format} | A11Y (label stays 'Reserve'; the hint says what it does) |
| `detail.reserveItemHint` | Reserve this item | Reserve this item | Reserves this item | A11Y |
| `detail.signInToReserveFormatHint` | Sign in to reserve the {format} | Sign in to reserve the {format} | Asks you to sign in, then reserves the {format} | A11Y |
| `detail.signInToReserveItemHint` | Sign in to reserve this item | Sign in to reserve this item | Asks you to sign in, then reserves this item | A11Y |
| `search.recentSearchHint` | — (label 'Search again for {query}') | — (contentDescription 'Search again for {query}'; onClickLabel 'Search again') | Searches again | A11Y (the label is the query as shown, so Voice Control can match it; Android keeps the 'Search again' action) |
| `detail.moreHint` | — (label 'Show more of the synopsis' on the 'More' button) | — (contentDescription 'Show more of the synopsis') | Shows the rest of the synopsis | A11Y (the label keeps the visible 'More') |
| `detail.lessHint` | — (label 'Show less of the synopsis' on the 'Less' button) | — (contentDescription 'Show less of the synopsis') | Shows less of the synopsis | A11Y (the label keeps the visible 'Less') |
| `detail.retryAvailabilityHint` | — (label 'Retry: Couldn't check availability') | — (onClickLabel 'Retry: Couldn't check availability') | Checks availability again | A11Y (the label is the visible 'Retry') |
| `detail.retryLookupHint` | — (label 'Retry: Couldn't look up availability') | — (onClickLabel 'Retry: Couldn't look up availability') | Looks up availability again | A11Y |
| `detail.retryCopiesHint` | — (label 'Retry loading copies') | — (onClickLabel 'Retry loading copies') | Loads the copies again | A11Y |
| `search.retryLoadingMoreHint` | — (label 'Retry loading more results') | — (contentDescription 'Retry loading more results') | Loads more results | A11Y |
| `detail.subjectHint` | Search the catalogue for this subject | — | Searches the catalogue for this subject | A11Y; Android gains it |
| `a11y.dismissHint` | Double-tap to dismiss | — (Dismiss as onClickLabel = common.dismiss) | Dismisses this message | A11Y (a hint names the result, not the gesture) |
| `onboarding.selectHint` | Select this library (hint) / Current library (hint) | Select this library / Keep this library (onClickLabel) | Selects this library | A11Y, D ('Current library' is now the row's value) |
| `onboarding.currentLibrary` | Current library (as a hint) | Current library (checkmark contentDescription) | Current library | A11Y (a value, not a hint) |
| `onboarding.libraries` | {n} libraries (also for 1) | 1 library / {n} libraries | 1 library / {count} libraries | PL, D |
| `myCard.loadingBarcode` | Loading barcode for card {number} | Loading barcode | Loading barcode for card {number} | D (iOS: says whose barcode) |
| `editReservation.dateLabel` | {field} date (DatePicker label) | {field}: {date} (contentDescription) | {field} date | D, A11Y (the date is the row's value/state) |
| `common.yourLibrary` | your library (AppModel) / the library (errors) | your library / the library (errors, Search) / Your library (verification) | your library | 1T (one stand-in; capitalise at a sentence start) |
| `common.openWebsite` | Open website | Open website (Featured) / Open the website (verification) | Open website | 1T, D (shorter) |
| `detail.checkWebsite` | Check availability on the library website | Check availability on the library website | Check availability on the library’s website | 1T ('the library’s website' as in Featured) |
| `errors.cantConnectTitle` | Can't connect | Can't connect | Can’t connect | ’ |
| `errors.network` | Can't reach {library}. Check your connection. | Can't reach {library}. Check your connection. | Can’t reach {library}. Check your connection. | ’ |
| `errors.verification` | {library} asks you to confirm you're human. | {library} asks you to confirm you're human. | {library} asks you to confirm you’re human. | ’ |
| `errors.libraryUnavailable` | {library} can't be used from the app right now. Try its website instead. | {library} can't be used from the app right now. Try its website instead. | {library} can’t be used from the app right now. Try its website instead. | ’ |
| `myCard.buttonHint` | Shows your library card's barcode | — | Shows your library card’s barcode | ’; Android gains it |
| `myCard.numberNote` | …your library account's Personal details (e.g. CL114D…). | same | Enter the number on your library card or in your library account’s Personal details (e.g. CL114D…). | ’ |
| `myCard.cannotEncode` | This number can't be shown as a barcode. | same | This number can’t be shown as a barcode. | ’ |
| `featured.websiteHint` | Opens the library's website | same | Opens the library’s website | ’ |
| `featured.websiteOnly` | This collection is on the library's website. | same | This collection is on the library’s website. | ’ |
| `featured.emptyMessage` | Check back soon, or open the collection on the library's website. | same | Check back soon, or open the collection on the library’s website. | ’ |
| `detail.availabilityFailed` | Couldn't check availability | same | Couldn’t check availability | ’ |
| `detail.lookupFailed` | Couldn't look up availability | same | Couldn’t look up availability | ’ |
| `detail.noRecordLink` | This book has no link to the library's record. | same | This book has no link to the library’s record. | ’ |
| `detail.copiesFailed` | Couldn't load copies | same | Couldn’t load copies | ’ |
| `reserve.cantReserve` | This title can't be reserved right now. | same | This title can’t be reserved right now. | ’ |
| `reserve.notFromApp` | This format can't be reserved from the app. | same | This format can’t be reserved from the app. | ’ |
| `reserve.chooseNotNeededAfter` | — | Choose when it's no longer needed | Choose when it’s no longer needed | ’ |
| `reserve.placeFailed` | The library couldn't place this reservation. | same | The library couldn’t place this reservation. | ’ |
| `yourBooks.noReservationsMessage` | Tap Reserve on any book and follow it here until it's ready to collect. | same | Tap Reserve on any book and follow it here until it’s ready to collect. | ’ |
| `yourBooks.cancelFailed` | The library didn't cancel this reservation. | same | The library didn’t cancel this reservation. | ’ |
| `yourBooks.cancelPendingMessage` | “{title}” will be removed from your reservations and you'll lose your place in the queue. | same | “{title}” will be removed from your reservations and you’ll lose your place in the queue. | ’ |
| `yourBooks.refreshFailed` | Couldn't refresh — showing saved data | same | Couldn’t refresh · Showing the saved list | ’, ' · ' between parts, 1T (the same words as onboarding.savedList) |
| `editReservation.cantEditTitle` | Can't edit this reservation | same | Can’t edit this reservation | ’ |
| `editReservation.cantEditMessage` | This reservation can't be edited right now. | same | This reservation can’t be edited right now. | ’ |
| `editReservation.delayFooter` | The library won't fill the reservation before this date. | same | The library won’t fill the reservation before this date. | ’ |
| `editReservation.notNeededFooter` | The reservation lapses if it isn't ready by this date. | same | The reservation lapses if it isn’t ready by this date. | ’ |
| `editReservation.changeFailed` | The library didn't accept the change. | same | The library didn’t accept the change. | ’ |
| `profile.accountFailed` | Couldn't load your account details. | same | Couldn’t load your account details | ’, FS (an inline status line like 'Couldn’t load copies') |
| `profile.notSignedInTitle` | You're not signed in | same | You’re not signed in | ’ |
| `profile.verifiedMessage` | {library} asked you to confirm you're human. The app sends the library's verification cookie with its requests. | same | {library} asked you to confirm you’re human. The app sends the library’s verification cookie with its requests. | ’ |
| `profile.clearVerificationMessage` | {library} will ask you to confirm you're human again. | same | {library} will ask you to confirm you’re human again. | ’ |
| `onboarding.emptyMessage` | Civica's directory came back empty. Try again later. | same | Civica’s directory came back empty. Try again later. | ’ |
| `onboarding.directoryNetwork` | Can't reach the library directory. Check your connection. | same | Can’t reach the library directory. Check your connection. | ’ |
| `onboarding.noteUK` | Asks you to verify you're human | same | Asks you to verify you’re human | ’ |
| `login.privacyNote` | LibreSpydus signs in directly with {library} and doesn't store your PIN. | same | LibreSpydus signs in directly with {library} and doesn’t store your PIN. | ’ |
| `verification.title` | Verify you're human | same | Verify you’re human | ’ |
| `verification.message` | {library} asks you to confirm you're human. Complete the check below. | same | {library} asks you to confirm you’re human. Complete the check below. | ’ |
| `verification.noCheck` | {library} didn't ask for a check this time. Close this and try again. | same | {library} didn’t ask for a check this time. Close this and try again. | ’ |
| `verification.keepsAsking` | — | {library} keeps asking to confirm you're human after the check. Its own website still works in your browser. | {library} keeps asking to confirm you’re human after the check. Its own website still works in your browser. | ’ |

### Accessibility hints on Android

TalkBack reads an `onClickLabel` as "Double-tap to <label>", so a third-person hint ("Opens
Profile") can't be an `onClickLabel`, because that would read "Double-tap to opens Profile". Compose has no hint
property. So on Android a hint goes after the label in `contentDescription`, through
`Copy.A11y.labelAndHint(label, hint)` ("Canning profile. Opens Profile"). `FeaturedRow` already
does this. Use `onClickLabel`, `onLongClickLabel` and `customActions` only for action names in
the imperative, which are the same words iOS uses for its menu items and VoiceOver actions:
`yourBooks.editReservation`, `picks.remove`, `common.moreOptions`, `yourBooks.showActions`,
`search.searchAgain`, `editReservation.changeDate`, …. This affects `home.profileButtonHint`,
`detail.heldHint`, `detail.reserve*Hint`, `onboarding.selectHint`, `home.genreHint`,
`detail.subjectHint`, `detail.formatHint`, `detail.moreHint` / `lessHint`, the `*retry*Hint`s,
`picks.editHint` / `doneEditingHint`,
`myCard.buttonHint`, `featured.*Hint` and `yourBooks.refreshHint`. A recent search row is the
exception: Android keeps its `search.searchAgain` action and doesn't repeat
`search.recentSearchHint`.

## Strings only one app has

| Key | Only in | Should the other app gain it? |
|---|---|---|
| `a11y.viewAllSection` "View All {section}" | iOS | **Yes.** Android's View All buttons only say "View All" |
| `home.genreHint` "Shows {genre} books" | iOS | **Yes** (as `labelAndHint`) |
| `home.searchField` / `home.searchFieldHint` | iOS (label "Search", hint was the placeholder) | **Yes.** Android read the placeholder as the label |
| `myCard.buttonHint` "Shows your library card’s barcode" | iOS | **Yes** |
| `detail.formatHint` "Shows availability for this format" | iOS | **Yes** |
| `detail.subjectHint` "Searches the catalogue for this subject" | iOS | **Yes** |
| `detail.copy` "Copy" (a holding without a location) | iOS | **Yes.** Android shows an empty location |
| `yourBooks.refreshHint` "Fetches your loans and reservations again" | iOS | **Yes** |
| `a11y.refreshing` "{status}. Refreshing" | iOS | **Yes.** Android's live region announces only the text |
| `a11y.updating` "Updating" (a card whose cancel or edit is running) | iOS | **Yes** |
| `a11y.dismissHint` "Dismisses this message" | iOS (was "Double-tap to dismiss") | **Yes**, on Android's dismissable banners and toasts, next to `common.dismiss` |
| `picks.adding` "Adding {n} books to My Picks…" | iOS | **Yes.** Android shows nothing while the copy runs |
| `picks.loadFailed` / `picks.saveFailed` | iOS (a banner over the signed-out list) | Optional. Android's store fails silently |
| `yourBooks.touchAndHoldHint` | iOS 17 only | No. Android cards always swipe |
| `yourBooks.show` "Show" (segmented picker label) | iOS | No. Android's segmented toggle has no label |
| `editReservation.date` "Date" (DatePicker title) | iOS | No. Android uses its own date rows |
| `verification.keepsAsking` + `common.openWebsite` | Android | **Yes.** iOS shows `errors.libraryUnavailable` in the screen that failed, but no way to the website |
| `search.searchAgain` (row action) | Android | No. iOS gives the row `search.recentSearchHint` instead |
| `yourBooks.showActions` "Show reservation actions" | Android (`onLongClickLabel`) | No. VoiceOver lists the actions directly |
| `reserve.chooseNotNeededAfter` | Android (`onClickLabel`) | No. iOS's Picker reads its own label |
| `editReservation.chooseBranch`, `setDate`, `changeDate`, `setFieldDate`, `clearField` | Android (its own date and branch rows) | No. iOS uses Form pickers and toggles |
| `common.selected`, `common.moreOptions`, `common.dismiss`, `common.back` | Android | No. These are system-provided on iOS (menus, context menus, back button) |
| `onboarding.libraries` singular "1 library" | Android | iOS gains it (PL) |

## Not in the catalogue

These are reader-facing only as technical detail, or are data. The checker flags them, and the
migrations mark each line `// copy-exempt: <reason>`:

- Technical detail shown under an error's "Details" disclosure. Examples: "The results page wasn't
  recognised (no records and no "did not find any records" message)." (both), "The Pick list page
  wasn't recognised …" (Android), "The library returned an empty record for {url}" (Android) and
  "empty record from {url}" (iOS). The `NotLoggedIn("not logged in")` placeholders (both) belong
  here too.
- Library labels matched against the record (`DetailLogic` hidden labels "main title", "bookmark
  link", "go to"), and directory data (`LibraryInfo.canning`: "City of Canning", "Australia").
- Debug-only text that isn't in a debug-only file or block: Android's `DeveloperSection` in
  ProfileScreen (Demo mode, On/Off, relaunch hints) and `MyCardLogic.symbologyName` (unused).
- Dead code: Android `Formatting.daysLeftLabel` / `Formatting.dueLine` (unused, and they say
  "Overdue by {n} days"). Delete it rather than exempt it.
- Swift `#if DEBUG` blocks, Kotlin `if (BuildConfig.DEBUG …) { … }` blocks and the debug-only
  files (galleries, debug hooks and actions, demo backend and data, timing log, launch options)
  are skipped by the checker itself.

## Coordinator decisions on the open questions (after C1)

- Android hints as `{label}. {hint}` in `contentDescription`: accepted.
- "your library" as the one fallback name, "Not needed after", "Books on loan": accepted.
- The contract's mixed terms are unified (added to the §7.1 glossary): **edit** a reservation
  (`yourBooks.swipeHint` / `touchAndHoldHint`: "…to edit or cancel it."), **phone**
  (`profile.verified`: "Verified on this phone"), **confirm** you’re human (`verification.title`:
  "Confirm you’re human"; `onboarding.noteUK`: "Asks you to confirm you’re human"), the library’s
  **website** (`errors.unexpected`: "The library’s website returned something unexpected.").
- Error banners keep their full stop; toasts don't.
- Core text: done in `core/src/messages.rs`. The parsers keep the middleware's wording (the goldens
  check it); `SpydusClient` hands the apps the proposed wording below for every core-made reserve,
  cancel and edit message. `SpydusError` Display strings stay developer detail.

## Text the core produces (core/src). Proposed fixes (now applied at the client, see above)

| Where | Now | Breaks | Proposed |
|---|---|---|---|
| `fetch.rs:316` cancel refused | This reservation can't be cancelled right now. | ’ | This reservation can’t be cancelled right now. |
| `fetch.rs:355`, `parse/reserve.rs:302` edit refused | This reservation can't be edited right now. | ’ | This reservation can’t be edited right now. (= `editReservation.cantEditMessage`) |
| `parse/reserve.rs:252` reserve refused | This title can't be reserved right now. | ’ | This title can’t be reserved right now. (= `reserve.cantReserve`) |
| `parse/reserve.rs:362` cancel succeeded | Reservation cancelled. | FS (shown as a toast-style banner) | Reservation cancelled (= `yourBooks.cancelled`) |
| `parse/reserve.rs:362` cancel failed | Couldn't cancel the reservation. | ’ | Couldn’t cancel this reservation. |
| `parse/reserve.rs:363` edit succeeded | Reservation updated. | FS | Reservation updated (= `yourBooks.updated`) |
| `parse/reserve.rs:363` edit failed | Couldn't update the reservation. | ’ | Couldn’t update this reservation. |
| `parse/reserve.rs:389, 399` placed | Reservation placed. | FS | Reservation placed (= `reserve.placed`) |
| `parse/reserve.rs:397` extra step needed | Reservation could not be completed. | Tone (the apps use contractions) | Couldn’t complete this reservation. |
| `error.rs` `SpydusError` Display | network error: …, not logged in, credentials rejected, forbidden origin, upstream error: …, parse error: …, human verification required | Sentence case. These are shown only under an error's "Details" disclosure | Low priority: capitalise the first letter ("Network error: …"), or leave as developer detail |
| `parse/picks.rs:22` | the library sent the Pick list to {url} | Shown only as detail | Leave (developer detail) |

These comply already: `availability_summary` labels ("2 of 3 available", "None available",
"1 copy" / "3 copies", "1 reserved", "No copies listed", joined with " · "), `home_genres` labels,
and `home_shelves` titles ("New arrivals", "New for kids", "New biographies"). The core produces no
alert titles or bodies, and no due-date text. `readyAlerts` and `dueReminders` return items and
keys, and the apps write the words (`alerts.*`, `loans.*`). Expiry choices, pickup names and the
available summary on the Reserve sheet are the library's own words.

## Open questions

1. **Android hints:** is `{label}. {hint}` in `contentDescription` acceptable (see above), or
   should hints become platform variants ("Opens Profile" on iOS, "Open Profile" on Android as an
   `onClickLabel`)? The catalogue keeps one wording, as asked.
2. **Fallback library name:** is "your library" right everywhere? Errors and Search used "the library"
   ("Can’t reach your library. Check your connection.", "Search your library").
3. **"Not needed after" for "Not required after":** Spydus's own form says "Not required after".
   This catalogue uses the Reserve sheet's words for both.
4. **"Books on loan" for "Books you borrow":** the glossary bans "borrowed" and "borrow" as words
   for asking. This extends that to the verb in the two empty states.
5. **The contract's wording is kept where it uses two words for one thing.** "Swipe left on a
   reservation to change or cancel it." (§6) sits next to "Edit reservation". "Verified on this
   device" (§6) sits next to "Saved on this phone". "Asks you to verify you’re human" (§6) sits
   next to "…confirm you’re human". "The library site returned something unexpected." (§5.7)
   sits next to "the library’s website". Should these be unified (edit / phone / confirm /
   website)? This needs the user's call because §7.1 doesn't decide it.
6. **Error banners keep their full stop** ("The library didn’t cancel this reservation.") as error
   sentences. Toasts don't ("Couldn’t update My Picks"). Is that the right reading of FS?

## Desktop (docs/desktop-app.md §8)

`gen_copy.py` writes a third file, `desktop/src/copy.rs` (`Copy::Group::key`: a `pub const` per
plain string, a `pub fn` per string with placeholders or plural forms, `count: i64`). A
**platform variant** lists every platform, `{"ios": …, "android": …, "desktop": …}`; each value is a
string or a `{one, other}` plural, all of one kind with the same placeholders. The phones' files
gain only doc comments for those keys, naming the other platforms' words.

The desktop says **computer** where the phones say "phone", and **Click** where they say "Tap"
(nine variants: `picks.subtitlePhone`, `emptyPhoneMessage`, `emptyAccountMessage`, `loadFailed`,
`saveFailed`, `offerTitle`, `offerMessage`, `profile.verified`, `yourBooks.noReservationsMessage`).
Following the touch-and-hold precedent, the desktop's own gestures are their own keys rather than
variants: `yourBooks.rightClickHint` and `loanRightClickHint`. The `desktop` group holds words only
the desktop shows (the app menu's "Quit LibreSpydus", the Go and Window menus, sidebar hints), in
sentence case like everything else ("Hide others", not AppKit's "Hide Others").

`check_copy.py` also scans `desktop/src/**/*.rs` (GPUI `.child(` / `.label(` / `.title(` /
`.tooltip(` / `.placeholder(` / `Notification::…(` / `aria_label(` and this app's components),
skipping `cfg(debug_assertions)` / `cfg(test)` code, debug-only files (timing log, launch flags,
demo and fake backends, gallery) and lines marked `// copy-exempt: <reason>`.
