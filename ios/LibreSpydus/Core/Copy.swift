// GENERATED from design/copy.json by design/scripts/gen_copy.py — do not edit.
// Every word a reader sees or hears (docs/native-apps.md §7.1). Change the JSON, then run the
// script; design/scripts/check_copy.py keeps literals out of the app.

import Foundation

// swiftlint:disable all

enum Copy {
    enum Common {
        static let retry = "Retry"
        static let close = "Close"
        static let done = "Done"
        static let cancel = "Cancel"
        static let save = "Save"
        static let edit = "Edit"
        static let remove = "Remove"
        static let `open` = "Open"
        static let add = "Add"
        static let notNow = "Not now"
        static let clear = "Clear"
        static let back = "Back"
        /// The one title-case exception besides the names (§7.1 style).
        static let viewAll = "View All"
        /// Screen-reader label of a spinner or skeleton with no visible text.
        static let loading = "Loading"
        /// Screen-reader label of the checkmark on a chosen menu item (Android menus).
        static let selected = "Selected"
        /// Android long-press action label on a row with a menu (onLongClickLabel).
        static let moreOptions = "More options"
        /// Action label on a tappable banner or toast (Android onClickLabel).
        static let dismiss = "Dismiss"
        /// Featured collection without sections; verification that keeps failing (Android 'Open the website').
        static let openWebsite = "Open website"
        /// A book's title when the library gives none (BookRef).
        static let untitled = "Untitled"
        /// Between parts of one line: 'Due 27 Oct · 3 days left'. Join parts with this, never a literal.
        static let separator = " · "
        /// Stand-in for {library} when the library's name is unknown. Capitalise the first letter when it starts a sentence.
        static let yourLibrary = "your library"
    }

    enum Account {
        static let signIn = "Sign in"
        static let signOut = "Sign out"
        static let signedIn = "Signed in"
        static let signingIn = "Signing in…"
    }

    enum Tabs {
        static let home = "Home"
        static let yourBooks = "Your Books"
        static let search = "Search"
        /// Was 'Favourites' in both apps' tab bar.
        static let myPicks = "My Picks"
        static let profile = "Profile"
    }

    enum A11y {
        /// "Cover of {title}"
        static func coverOf(title: String) -> String {
            "Cover of \(title)"
        }
        /// "View All {section}"
        /// Screen-reader label of a section's View All button: 'View All New arrivals'.
        static func viewAllSection(section: String) -> String {
            "View All \(section)"
        }
        /// "{label}. {hint}"
        /// Android only. Compose has no hint, and TalkBack reads an onClickLabel as 'Double-tap to <label>', so a third-person hint ('Opens Profile') goes after the label in contentDescription, as FeaturedRow already does. Use onClickLabel / customActions only for imperative action names (Edit reservation, Remove from My Picks, More options).
        static func labelAndHint(label: String, hint: String) -> String {
            "\(label). \(hint)"
        }
        /// Appended to a reservation card's label while its cancel/edit runs.
        static let updating = "Updating"
        /// "{status}. Refreshing"
        /// Your Books status line while a refresh runs: 'Updated 3 min ago. Refreshing'.
        static func refreshing(status: String) -> String {
            "\(status). Refreshing"
        }
        static let dismissHint = "Dismisses this message"
    }

    enum Errors {
        static let cantConnectTitle = "Can’t connect"
        static let verificationNeededTitle = "Verification needed"
        static let libraryUnavailableTitle = "Library unavailable"
        static let signedOutTitle = "Signed out"
        static let signInFailedTitle = "Sign-in failed"
        static let unexpectedTitle = "Something went wrong"
        /// "Can’t reach {library}. Check your connection."
        /// §5.7 / §7.1 glossary. {library} = the library's name, else common.yourLibrary (was 'the library' on both).
        static func network(library: String) -> String {
            "Can’t reach \(library). Check your connection."
        }
        /// "{library} asks you to confirm you’re human."
        static func verification(library: String) -> String {
            "\(library) asks you to confirm you’re human."
        }
        static let signInAgain = "Please sign in again."
        static let credentialsRejected = "Card number or PIN not recognised."
        static let unexpected = "The library’s website returned something unexpected."
        /// "{library} can’t be used from the app right now. Try its website instead."
        static func libraryUnavailable(library: String) -> String {
            "\(library) can’t be used from the app right now. Try its website instead."
        }
        /// The disclosure under an error that shows the core's technical text.
        static let details = "Details"
    }

    enum Home {
        /// "Hello {name}!"
        /// {name} = the borrower's first name.
        static func greeting(name: String) -> String {
            "Hello \(name)!"
        }
        static let greetingSignedOut = "Hello!"
        /// "{library} profile"
        /// Screen-reader label of the library logo button in Home's header.
        static func profileButton(library: String) -> String {
            "\(library) profile"
        }
        static let profileButtonHint = "Opens Profile"
        /// Screen-reader label of Home's search field (a button that opens the Search tab); the visible placeholder is search.placeholder.
        static let searchField = "Search"
        static let searchFieldHint = "Opens Search"
        /// Fallback only: the core's first home shelf title is used when present.
        static let newArrivals = "New arrivals"
        static let nothingNewTitle = "Nothing new right now"
        static let nothingNewMessage = "Check back soon for the latest additions."
        static let popularGenres = "Popular genres"
        /// "Shows {genre} books"
        static func genreHint(genre: String) -> String {
            "Shows \(genre) books"
        }
        /// §7.1: Home's loans shelf (was 'Your borrowed books').
        static let yourLoans = "Your loans"
        static let noLoansTitle = "No loans right now"
        static let noLoansMessage = "Books on loan show up here with their due dates."
        /// Signed-out card in the Your loans section; its button is account.signIn.
        static let signInCardTitle = "See your loans here"
        /// "Sign in with your {library} card."
        static func signInCardMessage(library: String) -> String {
            "Sign in with your \(library) card."
        }
    }

    enum Biometrics {
        /// {method} (§5.14): iOS picks faceId / touchId / opticId from LAContext.biometryType; Android always says fingerprint. The only words that differ between the apps (§7.1).
        static let faceId = "Face ID"
        static let touchId = "Touch ID"
        static let opticId = "Optic ID"
        static let fingerprint = "fingerprint"
        /// "Sign in with {method}"
        /// Sign-in sheet's secondary button and Profile's switch title.
        static func signInWith(method: String) -> String {
            "Sign in with \(method)"
        }
        /// "Sign in with {method} next time?"
        /// Alert after a typed sign-in; buttons use(method) / common.notNow. {library} = the library's name.
        static func offerTitle(method: String) -> String {
            "Sign in with \(method) next time?"
        }
        /// "When {library} signs you out, sign back in with {method} instead of typing your PIN. Your card number and PIN stay on this phone, locked by {method}."
        static func offerMessage(library: String, method: String) -> String {
            "When \(library) signs you out, sign back in with \(method) instead of typing your PIN. Your card number and PIN stay on this phone, locked by \(method)."
        }
        /// "Use {method}"
        static func use(method: String) -> String {
            "Use \(method)"
        }
        /// "No PIN to type when {library} signs you out."
        /// Profile switch subtitle under signInWith.
        static func switchSubtitle(library: String) -> String {
            "No PIN to type when \(library) signs you out."
        }
        /// "Sign in once more to turn on {method}."
        /// Above the sign-in fields when the sheet was opened by turning Profile's switch on.
        static func turnOnNote(method: String) -> String {
            "Sign in once more to turn on \(method)."
        }
        /// "Sign in to {library}"
        /// iOS LAContext localizedReason (shown with Touch ID and after a failed Face ID); Android BiometricPrompt title.
        static func reason(library: String) -> String {
            "Sign in to \(library)"
        }
        /// iOS localizedFallbackTitle; Android BiometricPrompt negative button. Both show the sign-in form.
        static let enterPin = "Enter PIN"
        /// iOS Info.plist NSFaceIDUsageDescription, copied there by hand (gen_copy doesn't write the plist). Swift doesn't use the constant.
        static let faceIdUsage = "Signs you back in to your library without typing your PIN."
    }

    enum MyCard {
        static let button = "My card"
        static let buttonHint = "Shows your library card’s barcode"
        static let title = "My card"
        static let showAtDesk = "Show this at the library desk or self-checkout."
        static let editNumber = "Edit card number"
        static let numberLabel = "Library card number"
        static let numberPlaceholder = "Card number"
        static let numberNote = "Enter the number on your library card or in your library account’s Personal details (e.g. CL114D…)."
        /// Caption under the library's name on the card (Android: the card's title when the library has no name).
        static let libraryCard = "Library card"
        /// "Loading barcode for card {number}"
        static func loadingBarcode(number: String) -> String {
            "Loading barcode for card \(number)"
        }
        /// "Barcode for card {number}"
        /// The barcode is one screen-reader element (§5.9).
        static func barcode(number: String) -> String {
            "Barcode for card \(number)"
        }
        static let cannotEncode = "This number can’t be shown as a barcode."
        /// "Expires {date}"
        /// {date} = cardExpires as the library writes it.
        static func expires(date: String) -> String {
            "Expires \(date)"
        }
        /// Desktop My card dialog while signed out, over account.signIn.
        static let signInMessage = "Sign in to show your library card’s barcode."
        /// Status line next to common.retry when the library's barcode type didn't load: no full stop.
        static let barcodeFailed = "Couldn’t load the barcode"
    }

    enum Featured {
        static let title = "Featured"
        /// Tile hint for .collection and .search links, unless the link has its own description.
        static let collectionHint = "Shows this collection"
        static let websiteHint = "Opens the library’s website"
        /// Collection page without sections; its button is common.openWebsite.
        static let websiteOnly = "This collection is on the library’s website."
        static let emptyTitle = "Nothing here right now"
        static let emptyMessage = "Check back soon, or open the collection on the library’s website."
    }

    enum Search {
        static let title = "Search"
        static let placeholder = "Search by title, author, etc."
        static let clearSearch = "Clear search"
        /// Label of the sort picker (iOS Picker title).
        static let sort = "Sort"
        static let sortResults = "Sort results"
        static let relevance = "Relevance"
        static let newest = "Newest"
        /// "Search {library}"
        static func introTitle(library: String) -> String {
            "Search \(library)"
        }
        static let introMessage = "Find books, eBooks, audiobooks and more by title, author or subject."
        static let recentSearches = "Recent searches"
        /// Hint on a recent search row; its label is the query as shown (was the label 'Search again for {query}', which Voice Control can't match to what's on screen).
        static let recentSearchHint = "Searches again"
        /// Android onClickLabel of a recent search row ('Double-tap to search again'), used instead of recentSearchHint; its long-press action is common.remove.
        static let searchAgain = "Search again"
        /// One: "1 result" · other: "{count} results"
        static func results(count: Int) -> String {
            count == 1 ? "1 result" : "\(copyNumber(count)) results"
        }
        /// "No results for “{query}”"
        static func noResultsTitle(query: String) -> String {
            "No results for “\(query)”"
        }
        static let noResultsMessage = "Check the spelling, or try fewer or different words."
        /// "Nothing in “{title}” right now"
        /// A results list opened from a URL (Home View All, genre tile, Featured search link) that came back empty.
        static func listEmptyTitle(title: String) -> String {
            "Nothing in “\(title)” right now"
        }
        static let listEmptyMessage = "Check back soon."
        static let endOfResults = "End of results"
        static let loadingResults = "Loading results"
        static let loadingMore = "Loading more results"
        static let retryLoadingMoreHint = "Loads more results"
        /// "+{count}"
        /// Chip after the first format chips: '+2'.
        static func moreFormats(count: Int) -> String {
            "+\(copyNumber(count))"
        }
    }

    enum Detail {
        /// "By {author}"
        static func byline(author: String) -> String {
            "By \(author)"
        }
        static let reserve = "Reserve"
        /// "Reserves the {format}"
        /// Hint on the Reserve pill. {format} = the format's label, lower-cased ('book', 'large print'). An edition-level item (no label) uses reserveItemHint.
        static func reserveFormatHint(format: String) -> String {
            "Reserves the \(format)"
        }
        static let reserveItemHint = "Reserves this item"
        /// "Asks you to sign in, then reserves the {format}"
        /// Signed out, the Reserve pill opens sign-in first.
        static func signInToReserveFormatHint(format: String) -> String {
            "Asks you to sign in, then reserves the \(format)"
        }
        static let signInToReserveItemHint = "Asks you to sign in, then reserves this item"
        static let availableOnline = "Available online"
        static let askLibrary = "Ask the library about availability"
        /// "Availability: {status}"
        /// Screen-reader label of the availability line; {status} = the visible text (core summary label or availableOnline / askLibrary).
        static func availability(status: String) -> String {
            "Availability: \(status)"
        }
        static let availabilityFailed = "Couldn’t check availability"
        static let lookupFailed = "Couldn’t look up availability"
        static let checkWebsite = "Check availability on the library’s website"
        /// Hint on the Retry next to availabilityFailed (its label stays common.retry; was the label 'Retry: Couldn’t check availability'). retryLookupHint goes with lookupFailed.
        static let retryAvailabilityHint = "Checks availability again"
        static let retryLookupHint = "Looks up availability again"
        static let findingFormats = "Finding formats…"
        static let checkingAvailability = "Checking availability…"
        static let noRecordLink = "This book has no link to the library’s record."
        static let formats = "Formats"
        static let formatHint = "Shows availability for this format"
        static let synopsis = "Synopsis"
        static let more = "More"
        static let less = "Less"
        /// The synopsis's More / Less button keeps its visible text as its label (was 'Show more of the synopsis' / 'Show less of the synopsis').
        static let moreHint = "Shows the rest of the synopsis"
        static let lessHint = "Shows less of the synopsis"
        static let noMoreDetails = "The library has no more details for this title."
        static let loadingDetails = "Loading details…"
        static let subjects = "Subjects"
        static let subjectHint = "Searches the catalogue for this subject"
        static let details = "Details"
        static let copies = "Copies"
        static let loadingCopies = "Loading copies…"
        static let copiesFailed = "Couldn’t load copies"
        static let retryCopiesHint = "Loads the copies again"
        /// A Copies row whose holding has no location or collection.
        static let copy = "Copy"
        /// "Call number {number}"
        static func callNumber(number: String) -> String {
            "Call number \(number)"
        }
        static let similarBooks = "Similar books"
        static let moreByAuthor = "More by this author"
        static let inThisSeries = "In this series"
        static let otherEditions = "Other editions"
        /// "Loading {panel}"
        /// Screen-reader label of a related carousel while it loads; {panel} = its title, lower-cased.
        static func loadingPanel(panel: String) -> String {
            "Loading \(panel)"
        }
        static let onLoan = "On loan"
        /// "On loan · Due {date}"
        /// Already-yours pill (§6, I8/A6). {date} = '14 Oct' (loans.shortDate style), or the library's text when unreadable.
        static func onLoanDue(date: String) -> String {
            "On loan · Due \(date)"
        }
        static let onLoanOverdue = "On loan · Overdue"
        /// "On loan, due {date}"
        /// Spoken: {date} written out ('14 October').
        static func onLoanDueLabel(date: String) -> String {
            "On loan, due \(date)"
        }
        /// "On loan, overdue, was due {date}"
        static func onLoanOverdueLabel(date: String) -> String {
            "On loan, overdue, was due \(date)"
        }
        static let readyToCollect = "Ready to collect"
        /// "Ready to collect · {branch}"
        /// §7.1: Ready to collect (+ ' at {branch}' when spoken); the visible pill joins with ' · ' (§6).
        static func readyToCollectAt(branch: String) -> String {
            "Ready to collect · \(branch)"
        }
        /// "Ready to collect at {branch}"
        static func readyToCollectAtLabel(branch: String) -> String {
            "Ready to collect at \(branch)"
        }
        static let reserved = "Reserved"
        /// "Reserved · {branch}"
        static func reservedAt(branch: String) -> String {
            "Reserved · \(branch)"
        }
        /// "Reserved, collect from {branch}"
        /// §7.1 glossary: 'collect from' (was 'pick up at').
        static func reservedAtLabel(branch: String) -> String {
            "Reserved, collect from \(branch)"
        }
        /// "Reserved · #{place} in queue"
        /// Numeric queue place.
        static func reservedQueue(place: String) -> String {
            "Reserved · #\(place) in queue"
        }
        /// "Reserved, number {place} in the queue"
        static func reservedQueueLabel(place: String) -> String {
            "Reserved, number \(place) in the queue"
        }
        /// "Reserved · {place} in queue"
        /// A queue place the library writes as text ('1 of 3'). §7.1 bans 'rank' (was 'Reserved · Rank {n}').
        static func reservedQueueText(place: String) -> String {
            "Reserved · \(place) in queue"
        }
        /// "Reserved, {place} in the queue"
        static func reservedQueueTextLabel(place: String) -> String {
            "Reserved, \(place) in the queue"
        }
        /// The already-yours pill opens Your Books on the right list; a renewable loan's pill asks to renew it instead (hint yourBooks.renewHint).
        static let heldHint = "Opens Your Books"
    }

    enum Reserve {
        static let title = "Reserve"
        static let checkingOptions = "Checking reservation options…"
        static let cantReserve = "This title can’t be reserved right now."
        static let notFromApp = "This format can’t be reserved from the app."
        static let confirm = "Confirm reservation"
        /// §7.1 glossary: the branch row (was 'Pick up at').
        static let collectFrom = "Collect from"
        /// Value of the Collect from row when the library doesn't name a branch (§7.1: 'branch' for a branch; was 'Your usual library').
        static let usualBranch = "Your usual branch"
        static let availability = "Availability"
        static let notNeededAfter = "Not needed after"
        /// Android onClickLabel of the Not needed after dropdown.
        static let chooseNotNeededAfter = "Choose when it’s no longer needed"
        /// Result banner when the library's own message is empty. A toast-style confirmation: no full stop (§7.1).
        static let placed = "Reservation placed"
        /// Result banner when the library refused and gave no message.
        static let placeFailed = "The library couldn’t place this reservation."
    }

    enum Loans {
        /// "Due {date}"
        /// {date} = '27 Oct' (day + short month; + year only when it isn't this year). Due-date notifications pass the weekday too: 'Due Thursday 16 Oct'. A date the core can't read is shown as the library wrote it.
        static func dueOn(date: String) -> String {
            "Due \(date)"
        }
        static let dueToday = "Due today"
        static let dueTomorrow = "Due tomorrow"
        /// One: "1 day left" · other: "{count} days left"
        static func daysLeft(count: Int) -> String {
            count == 1 ? "1 day left" : "\(copyNumber(count)) days left"
        }
        /// One: "1 day overdue" · other: "{count} days overdue"
        /// §7.1 glossary (Android's unused Formatting.daysLeftLabel said 'Overdue by {n} days').
        static func daysOverdue(count: Int) -> String {
            count == 1 ? "1 day overdue" : "\(copyNumber(count)) days overdue"
        }
        /// "Due {date} · {days}"
        /// Your Books loan card meta line: 'Due 27 Oct · 3 days left' / 'Due 10 Oct · 2 days overdue'; {days} = daysLeft / daysOverdue.
        static func lineDays(date: String, days: String) -> String {
            "Due \(date) · \(days)"
        }
        /// "Due today · {date}"
        /// Your Books loan due today: 'Due today · 12 Oct'.
        static func lineToday(date: String) -> String {
            "Due today · \(date)"
        }
        /// "Due tomorrow · {date}"
        static func lineTomorrow(date: String) -> String {
            "Due tomorrow · \(date)"
        }
        /// Chip on an overdue loan card and part of its screen-reader label.
        static let overdue = "Overdue"
    }

    enum YourBooks {
        static let title = "Your Books"
        static let loans = "Loans"
        static let reservations = "Reservations"
        /// "Loans ({count})"
        static func loansCount(count: Int) -> String {
            "Loans (\(copyNumber(count)))"
        }
        /// "Reservations ({count})"
        static func reservationsCount(count: Int) -> String {
            "Reservations (\(copyNumber(count)))"
        }
        /// Label of the Loans | Reservations picker (read by screen readers, not shown).
        static let show = "Show"
        static let loadingLoans = "Loading your loans…"
        static let loadingReservations = "Loading your reservations…"
        static let noLoansTitle = "No loans right now"
        /// "Books on loan from {library} show up here with their due dates."
        static func noLoansMessage(library: String) -> String {
            "Books on loan from \(library) show up here with their due dates."
        }
        /// Swipe action / button on a loan, and the Renew dialog's confirming button (cancel button: common.notNow).
        static let renew = "Renew"
        /// Menu item and screen-reader action on a loan row (like editReservation / cancelReservation).
        static let renewLoan = "Renew loan"
        /// Footnote under the Loans list (replaces renewalsSoon); touchAndHold variant where the reservation footnote uses touchAndHoldHint.
        static let loanSwipeHint = "Swipe left on a loan to renew it."
        static let loanTouchAndHoldHint = "Touch and hold a loan to renew it."
        static let renewTitle = "Renew this loan?"
        /// "“{title}” is due {date}. Renewing asks the library for a later due date."
        /// Renew dialog body; {date} as loans.dueOn writes it. renewMessageNoDate when the loan has no due date.
        static func renewMessage(title: String, date: String) -> String {
            "“\(title)” is due \(date). Renewing asks the library for a later due date."
        }
        /// "Renewing “{title}” asks the library for a later due date."
        static func renewMessageNoDate(title: String) -> String {
            "Renewing “\(title)” asks the library for a later due date."
        }
        /// Progress toast while renewLoan runs (it takes a few seconds: it reads the loans list before and after).
        static let renewing = "Renewing…"
        /// "Renewed · Due {date}"
        /// Success toast; {date} = RenewResult.due as loans.dueOn writes it. renewedNoDate when due is missing.
        static func renewed(date: String) -> String {
            "Renewed · Due \(date)"
        }
        static let renewedNoDate = "Renewed"
        /// Error toast when renewLoan throws (not NotLoggedIn). When RenewResult.ok is false, show RenewResult.message (the library's reason or the core's) as the error toast instead.
        static let renewFailed = "Couldn’t renew this loan"
        /// Screen-reader hint on the Renew action, and on the book page's On loan pill of a loan that can be renewed.
        static let renewHint = "Asks the library for a later due date"
        static let noReservationsTitle = "No reservations"
        /// Android: "Tap Reserve on any book and follow it here until it’s ready to collect."
        /// Desktop: "Click Reserve on any book and follow it here until it’s ready to collect."
        static let noReservationsMessage = "Tap Reserve on any book and follow it here until it’s ready to collect."
        /// The ready card's status chip (§7.1; was 'Ready for pickup'). Pending cards show the library's own status, else 'Pending'.
        static let readyToCollect = "Ready to collect"
        static let pending = "Pending"
        /// "Ready to collect ({count})"
        /// Reservations section headers with how many are in each, as loansCount / reservationsCount write the segments: 'Ready to collect (2)', 'Pending (3)'.
        static func readyCount(count: Int) -> String {
            "Ready to collect (\(copyNumber(count)))"
        }
        /// "Pending ({count})"
        static func pendingCount(count: Int) -> String {
            "Pending (\(copyNumber(count)))"
        }
        /// "Collect from {branch}"
        /// Card meta line (§7.1; was 'Pick up at {branch}'). {branch} title-cased when the library shouts it.
        static func collectFrom(branch: String) -> String {
            "Collect from \(branch)"
        }
        /// "Collect from {branch} by {date}"
        static func collectFromBy(branch: String, date: String) -> String {
            "Collect from \(branch) by \(date)"
        }
        /// "Collect by {date}"
        /// Ready card with no branch: 'Collect by 17 Oct' (was 'Pick up by …').
        static func collectBy(date: String) -> String {
            "Collect by \(date)"
        }
        /// "#{place} in queue"
        /// Pending card meta: '#4 in queue · Placed 2 Oct'.
        static func queuePlace(place: String) -> String {
            "#\(place) in queue"
        }
        /// "{place} in queue"
        /// A queue place the library writes as text (was 'Rank {n}'; §7.1 bans 'rank').
        static func queuePlaceText(place: String) -> String {
            "\(place) in queue"
        }
        /// "number {place} in the queue"
        /// The card's screen-reader label says the queue place this way (§7.1 glossary); queuePlaceTextLabel for a place written as text.
        static func queuePlaceLabel(place: String) -> String {
            "number \(place) in the queue"
        }
        /// "{place} in the queue"
        static func queuePlaceTextLabel(place: String) -> String {
            "\(place) in the queue"
        }
        /// "Placed {date}"
        static func placed(date: String) -> String {
            "Placed \(date)"
        }
        /// Footnote under the reservations where cards swipe (iOS 18+, Android). Hidden from screen readers: they get the actions on each card.
        static let swipeHint = "Swipe left on a reservation to edit or cancel it."
        /// The same footnote on iOS 17, where cards don't swipe (long-press menu only).
        static let touchAndHoldHint = "Touch and hold a reservation to edit or cancel it."
        /// Desktop: the footnote under the reservations, whose cards have a '…' menu button and the same right-click menu (docs/desktop-app.md §6). loanRightClickHint goes under the loans.
        static let rightClickHint = "Right-click a reservation to edit or cancel it."
        static let loanRightClickHint = "Right-click a loan to renew it."
        /// Long-press menu item and screen-reader action (Edit / common.cancel are the swipe tiles).
        static let editReservation = "Edit reservation"
        /// Menu item, screen-reader action and the confirmation's destructive button (iOS said 'Cancel Reservation').
        static let cancelReservation = "Cancel reservation"
        /// Android onLongClickLabel of a reservation card.
        static let showActions = "Show reservation actions"
        static let signedOutTitle = "Sign in to see your books"
        /// "Your loans and reservations at {library} appear here."
        static func signedOutMessage(library: String) -> String {
            "Your loans and reservations at \(library) appear here."
        }
        /// Banner after a cancel when the library's message is empty. Toast-style: no full stop.
        static let cancelled = "Reservation cancelled"
        /// Error banner when the library refused with no message.
        static let cancelFailed = "The library didn’t cancel this reservation."
        static let updated = "Reservation updated"
        static let cancelTitle = "Cancel this reservation?"
        /// "“{title}” is waiting for you at {branch}. Cancelling releases it to the next person."
        static func cancelReadyMessage(title: String, branch: String) -> String {
            "“\(title)” is waiting for you at \(branch). Cancelling releases it to the next person."
        }
        /// "“{title}” is waiting for you. Cancelling releases it to the next person."
        static func cancelReadyMessageNoBranch(title: String) -> String {
            "“\(title)” is waiting for you. Cancelling releases it to the next person."
        }
        /// "“{title}” will be removed from your reservations and you’ll lose your place in the queue."
        static func cancelPendingMessage(title: String) -> String {
            "“\(title)” will be removed from your reservations and you’ll lose your place in the queue."
        }
        static let keepReservation = "Keep reservation"
        static let updatedJustNow = "Updated just now"
        /// One: "Updated 1 min ago" · other: "Updated {count} min ago"
        static func updatedMinutesAgo(count: Int) -> String {
            count == 1 ? "Updated 1 min ago" : "Updated \(copyNumber(count)) min ago"
        }
        /// "Updated today at {time}"
        static func updatedToday(time: String) -> String {
            "Updated today at \(time)"
        }
        /// "Updated yesterday at {time}"
        static func updatedYesterday(time: String) -> String {
            "Updated yesterday at \(time)"
        }
        /// "Updated {day} at {time}"
        /// {day} = weekday within a week (Android) or '3 Oct' / '3 Oct 2025'; {time} in the device's 12/24-hour style.
        static func updatedOn(day: String, time: String) -> String {
            "Updated \(day) at \(time)"
        }
        /// §5.2a 'Couldn't refresh' note, with common.retry and refreshHint (was 'Couldn't refresh — showing saved data'; the same words as onboarding.savedList).
        static let refreshFailed = "Couldn’t refresh · Showing the saved list"
        static let refreshHint = "Fetches your loans and reservations again"
    }

    enum EditReservation {
        static let title = "Edit reservation"
        static let loading = "Loading reservation…"
        static let cantEditTitle = "Can’t edit this reservation"
        static let cantEditMessage = "This reservation can’t be edited right now."
        /// §7.1 glossary (was 'Pickup location'); the same words as the Reserve sheet.
        static let collectFrom = "Collect from"
        static let chooseBranch = "Choose a branch"
        static let delayUntil = "Delay until"
        static let delayFooter = "The library won’t fill the reservation before this date."
        /// One term with the Reserve sheet's expiry picker (was 'Not required after').
        static let notNeededAfter = "Not needed after"
        static let notNeededFooter = "The reservation lapses if it isn’t ready by this date."
        /// "Cleared when you save. {footer}"
        static func clearedFooter(footer: String) -> String {
            "Cleared when you save. \(footer)"
        }
        static let saveChanges = "Save changes"
        static let changeFailed = "The library didn’t accept the change."
        /// iOS DatePicker title (hidden; its screen-reader label is dateLabel).
        static let date = "Date"
        /// "{field} date"
        /// Screen-reader label of a date row: 'Delay until date'; Android reads the date as its state.
        static func dateLabel(field: String) -> String {
            "\(field) date"
        }
        static let setDate = "Set date"
        /// "Change {field} date"
        /// Android onClickLabel of a set date; setFieldDate when the date is off; clearField on its clear button. {field} is the row's title lower-cased mid-sentence: 'Change delay until date', 'Clear not needed after'.
        static func changeDate(field: String) -> String {
            "Change \(field) date"
        }
        /// "Set {field} date"
        static func setFieldDate(field: String) -> String {
            "Set \(field) date"
        }
        /// "Clear {field}"
        static func clearField(field: String) -> String {
            "Clear \(field)"
        }
    }

    enum Picks {
        /// §7.1: the tab, the screen title (signed in and out), the bookmark and the toasts. Never Favourites.
        static let title = "My Picks"
        static let subtitleAccount = "Saved on your library account"
        /// Android: "Saved on this phone"
        /// Desktop: "Saved on this computer"
        /// Signed out (was '{n} saved for {library}' / 'Saved on this device for {library}').
        static let subtitlePhone = "Saved on this phone"
        /// Bookmark button's screen-reader label (book page and anywhere else a toggle appears), signed in or out (was 'Add to favourites').
        static let save = "Save to My Picks"
        /// Bookmark label when saved; also the row's remove action / menu item (was 'Remove from Favourites').
        static let remove = "Remove from My Picks"
        /// Toast after a save (§5.11).
        static let saved = "Saved to My Picks"
        static let removed = "Removed from My Picks"
        /// Error toast (§5.11 had a full stop; §7.1: never on toasts).
        static let updateFailed = "Couldn’t update My Picks"
        static let emptyTitle = "No picks yet"
        /// Android: "Tap the bookmark on a book to save it to your library account."
        /// Desktop: "Click the bookmark on a book to save it to your library account."
        static let emptyAccountMessage = "Tap the bookmark on a book to save it to your library account."
        /// Android: "Tap the bookmark on a book to save it on this phone."
        /// Desktop: "Click the bookmark on a book to save it on this computer."
        /// Signed out (was 'Tap the bookmark on any book to save it here.'); its button is findABook.
        static let emptyPhoneMessage = "Tap the bookmark on a book to save it on this phone."
        static let findABook = "Find a book"
        /// Hint on the signed-out list's Edit button (its label is the visible common.edit; was the label 'Edit favourites'); doneEditingHint goes with common.done.
        static let editHint = "Lets you remove books from My Picks"
        static let doneEditingHint = "Finishes editing My Picks"
        /// "Remove {title} from My Picks"
        /// Screen-reader label of a row's remove button in edit mode.
        static func removeTitle(title: String) -> String {
            "Remove \(title) from My Picks"
        }
        /// "Removed {title} from My Picks"
        /// Screen-reader announcement after a remove on the signed-out list.
        static func removedTitle(title: String) -> String {
            "Removed \(title) from My Picks"
        }
        static let loadingMore = "Loading more picks"
        static let retryLoadingMoreHint = "Loads more picks"
        /// Android: "Couldn’t load My Picks from this phone."
        /// Desktop: "Couldn’t load My Picks from this computer."
        /// iOS banner when the phone's list can't be read (was 'Couldn't read favourites: <system error>').
        static let loadFailed = "Couldn’t load My Picks from this phone."
        /// Android: "Couldn’t save My Picks on this phone."
        /// Desktop: "Couldn’t save My Picks on this computer."
        static let saveFailed = "Couldn’t save My Picks on this phone."
        /// Android: "Add this phone’s picks to your library account?"
        /// Desktop: "Add this computer’s picks to your library account?"
        /// §5.11 copy-once alert; buttons common.add / common.notNow.
        static let offerTitle = "Add this phone’s picks to your library account?"
        /// One: "1 book saved on this phone can be added to My Picks on your library account." · other: "{count} books saved on this phone can be added to My Picks on your library account."
        /// Android: One: "1 book saved on this phone can be added to My Picks on your library account." · other: "{count} books saved on this phone can be added to My Picks on your library account."
        /// Desktop: One: "1 book saved on this computer can be added to My Picks on your library account." · other: "{count} books saved on this computer can be added to My Picks on your library account."
        static func offerMessage(count: Int) -> String {
            count == 1 ? "1 book saved on this phone can be added to My Picks on your library account." : "\(copyNumber(count)) books saved on this phone can be added to My Picks on your library account."
        }
        /// One: "Adding 1 book to My Picks…" · other: "Adding {count} books to My Picks…"
        /// Toast while the copy runs (iOS has it; Android should gain it).
        static func adding(count: Int) -> String {
            count == 1 ? "Adding 1 book to My Picks…" : "Adding \(copyNumber(count)) books to My Picks…"
        }
        /// One: "Added 1 book to My Picks" · other: "Added {count} books to My Picks"
        /// Toast when the copy ends; when some failed, join notAdded with common.separator: 'Added 3 books to My Picks · 1 couldn’t be added'.
        static func added(count: Int) -> String {
            count == 1 ? "Added 1 book to My Picks" : "Added \(copyNumber(count)) books to My Picks"
        }
        /// One: "1 couldn’t be added" · other: "{count} couldn’t be added"
        static func notAdded(count: Int) -> String {
            count == 1 ? "1 couldn’t be added" : "\(copyNumber(count)) couldn’t be added"
        }
    }

    enum Profile {
        static let title = "Profile"
        static let noLibraryTitle = "No library selected"
        static let noLibraryMessage = "Choose your library to search its catalogue and sign in."
        static let chooseLibrary = "Choose library"
        static let changeLibrary = "Change library"
        /// Section header; the signed-in row says account.signedIn and its button account.signOut.
        static let account = "Account"
        static let loadingAccount = "Loading your account…"
        /// "Signed in to {library}"
        static func signedInTo(library: String) -> String {
            "Signed in to \(library)"
        }
        /// Inline status line next to common.retry, like detail.copiesFailed: no full stop.
        static let accountFailed = "Couldn’t load your account details"
        /// "Sign out of {library}?"
        /// Confirmation; its destructive button is account.signOut, the other common.cancel.
        static func signOutTitle(library: String) -> String {
            "Sign out of \(library)?"
        }
        static let signOutMessage = "You can sign in again at any time."
        static let notSignedInTitle = "You’re not signed in"
        static let notSignedInMessage = "Sign in with your library card to see your loans and reservations."
        static let humanVerification = "Human verification"
        /// Android: "Verified on this phone"
        /// Desktop: "Verified on this computer"
        static let verified = "Verified on this phone"
        /// "{library} asked you to confirm you’re human. The app sends the library’s verification cookie with its requests."
        static func verifiedMessage(library: String) -> String {
            "\(library) asked you to confirm you’re human. The app sends the library’s verification cookie with its requests."
        }
        static let clearVerification = "Clear verification"
        /// Confirmation; its destructive button is clearVerification, the other common.cancel.
        static let clearVerificationTitle = "Clear verification?"
        /// "{library} will ask you to confirm you’re human again."
        static func clearVerificationMessage(library: String) -> String {
            "\(library) will ask you to confirm you’re human again."
        }
        static let about = "About"
        static let appName = "LibreSpydus"
        /// "Version {version} ({build})"
        static func version(version: String, build: String) -> String {
            "Version \(version) (\(build))"
        }
        /// "Version {version}"
        /// Desktop: the app has a version but no separate build number.
        static func appVersion(version: String) -> String {
            "Version \(version)"
        }
        static let notAffiliated = "LibreSpydus is not affiliated with Civica or your library."
    }

    enum Notifications {
        /// Profile section while signed in (§5.8).
        static let section = "Notifications"
        static let readyTitle = "Ready to collect"
        static let readySubtitle = "Checked each morning and when you open the app"
        static let dueTitle = "Due-date reminders"
        static let dueSubtitle = "2 days before books are due, at 7:45"
        static let blocked = "Notifications are turned off for LibreSpydus."
        /// 'Settings' is the system app's name, so it keeps its capital.
        static let openSettings = "Open Settings"
    }

    enum Alerts {
        /// Local notification (§5.8); the body is readyBody, or just the title when the branch is unknown.
        static let readyTitle = "Ready to collect"
        /// "{title} · {branch}"
        static func readyBody(title: String, branch: String) -> String {
            "\(title) · \(branch)"
        }
        /// Stands in for a missing title in readyBody.
        static let untitledReservation = "A reservation"
        static let untitledLoan = "A loan"
        /// "{first} and {second}"
        /// Due-reminder body for two loans; one loan is just its title; three or more use dueMore. The title is loans.dueOn / dueToday / dueTomorrow with the weekday date ('Due Thursday 16 Oct').
        static func dueTwo(first: String, second: String) -> String {
            "\(first) and \(second)"
        }
        /// One: "{first} and 1 more" · other: "{first} and {count} more"
        static func dueMore(count: Int, first: String) -> String {
            count == 1 ? "\(first) and 1 more" : "\(first) and \(copyNumber(count)) more"
        }
        /// Android notification channel names.
        static let readyChannel = "Ready to collect"
        static let dueChannel = "Due-date reminders"
    }

    enum Onboarding {
        static let title = "Choose your library"
        static let intro = "LibreSpydus works with libraries that use Spydus. You can change this later in Profile."
        /// The picker's title when opened from Profile (profile.changeLibrary is the row).
        static let changeTitle = "Change library"
        static let searchPlaceholder = "Search by library or country"
        static let loading = "Loading libraries…"
        static let emptyTitle = "No libraries listed"
        static let emptyMessage = "Civica’s directory came back empty. Try again later."
        /// "No libraries match “{query}”"
        static func noMatchesTitle(query: String) -> String {
            "No libraries match “\(query)”"
        }
        static let noMatchesMessage = "Try a library name or a country."
        /// "{error} Showing the saved list."
        /// Banner when a refresh failed but a saved list is shown; {error} = directoryNetwork / directoryUnexpected.
        static func savedList(error: String) -> String {
            "\(error) Showing the saved list."
        }
        static let directoryNetwork = "Can’t reach the library directory. Check your connection."
        static let directoryUnexpected = "The library directory returned something unexpected."
        /// One: "1 library" · other: "{count} libraries"
        /// Screen-reader label of a region's count (iOS said '1 libraries').
        static func libraries(count: Int) -> String {
            count == 1 ? "1 library" : "\(copyNumber(count)) libraries"
        }
        /// Screen-reader value of the selected row (and Android's checkmark).
        static let currentLibrary = "Current library"
        /// Hint on the other rows (iOS said 'Select this library'; Android's onClickLabel 'Select this library' / 'Keep this library').
        static let selectHint = "Selects this library"
        /// Section titles, and the Profile place chip when the directory has no country.
        static let regionAU = "Australia"
        static let regionNZ = "New Zealand"
        static let regionUS = "United States"
        static let regionUK = "United Kingdom"
        static let regionSG = "Singapore"
        static let regionOther = "Other"
        /// Row notes (§6).
        static let noteUK = "Asks you to confirm you’re human"
        static let noteSG = "May be unreachable outside school networks"
    }

    enum Login {
        /// The sheet's title; its button is account.signIn (account.signingIn while it runs) and its close button common.close.
        static let title = "Sign in"
        /// Libraries accept either the patron's username or their library card number here.
        static let cardLabel = "Username / library card number"
        static let cardPlaceholder = "Username or card number"
        static let pinLabel = "PIN / password"
        static let pinPlaceholder = "PIN or password"
        /// Desktop: screen-reader label of the PIN field's reveal toggle (hidePin while the PIN shows).
        static let showPin = "Show PIN"
        static let hidePin = "Hide PIN"
        /// "LibreSpydus signs in directly with {library} and doesn’t store your PIN."
        static func privacyNote(library: String) -> String {
            "LibreSpydus signs in directly with \(library) and doesn’t store your PIN."
        }
        /// "LibreSpydus signs in directly with {library}. It keeps your PIN on this phone only if you turn on {method}."
        /// Replaces privacyNote when the phone can use {method} (§5.14), since turning it on keeps the PIN.
        static func privacyNoteBiometrics(library: String, method: String) -> String {
            "LibreSpydus signs in directly with \(library). It keeps your PIN on this phone only if you turn on \(method)."
        }
    }

    enum Verification {
        static let title = "Confirm you’re human"
        /// "{library} asks you to confirm you’re human. Complete the check below."
        /// §6 verification sheet copy.
        static func message(library: String) -> String {
            "\(library) asks you to confirm you’re human. Complete the check below."
        }
        static let closesByItself = "This closes by itself once the library lets you in."
        /// "Opening {library}…"
        static func opening(library: String) -> String {
            "Opening \(library)…"
        }
        /// "{library} didn’t ask for a check this time. Close this and try again."
        static func noCheck(library: String) -> String {
            "\(library) didn’t ask for a check this time. Close this and try again."
        }
        static let reload = "Reload page"
        /// "{library} keeps asking to confirm you’re human after the check. Its own website still works in your browser."
        /// Android shows it with common.openWebsite and common.close when verification keeps failing (iOS shows errors.libraryUnavailable instead).
        static func keepsAsking(library: String) -> String {
            "\(library) keeps asking to confirm you’re human after the check. Its own website still works in your browser."
        }
    }

    enum Desktop {
        /// Desktop only (docs/desktop-app.md §8.2): the app menu and its ⌘Q item. 'LibreSpydus' is the app's name (profile.appName).
        static let quit = "Quit LibreSpydus"
        static let hide = "Hide LibreSpydus"
        /// macOS app-menu items, in sentence case (§7.1), unlike AppKit's own 'Hide Others'.
        static let hideOthers = "Hide others"
        static let showAll = "Show all"
        /// The Window menu and its items.
        static let window = "Window"
        static let minimize = "Minimize"
        static let zoom = "Zoom"
        static let file = "File"
        static let closeWindow = "Close window"
        /// The Go menu: the five sections (tabs.*, ⌘1–⌘5), common.back (⌘[) and search.title (⌘F).
        static let go = "Go"
        /// ⌘R in the Go menu: fetches the account again (§5.5).
        static let refreshAccount = "Refresh account"
        /// "Shows {section}"
        /// Screen-reader hint on a sidebar item; {section} = its label (tabs.*).
        static func sidebarHint(section: String) -> String {
            "Shows \(section)"
        }
        /// "{library} logo"
        /// Screen-reader label of the library's logo in the sidebar header.
        static func libraryLogo(library: String) -> String {
            "\(library) logo"
        }
    }
}

/// A count as the reader's locale writes it ("1,234").
private func copyNumber(_ value: Int) -> String {
    value.formatted()
}
