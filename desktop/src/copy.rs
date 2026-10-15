// GENERATED from design/copy.json by design/scripts/gen_copy.py — do not edit.
// Every word a reader sees or hears (docs/native-apps.md §7.1, docs/desktop-app.md §8). Change
// the JSON, then run the script; design/scripts/check_copy.py keeps literals out of the app.

#![allow(non_snake_case, non_upper_case_globals, dead_code, clippy::all)]

/// `Copy::Common::retry`, `Copy::Search::results(count)`: one module per group, a `const` per
/// plain string and a function per string with placeholders or plural forms.
pub mod Copy {
    pub mod Common {
        pub const retry: &str = "Retry";
        pub const close: &str = "Close";
        pub const done: &str = "Done";
        pub const cancel: &str = "Cancel";
        pub const save: &str = "Save";
        pub const edit: &str = "Edit";
        pub const remove: &str = "Remove";
        pub const open: &str = "Open";
        pub const add: &str = "Add";
        pub const notNow: &str = "Not now";
        pub const clear: &str = "Clear";
        pub const back: &str = "Back";
        /// The one title-case exception besides the names (§7.1 style).
        pub const viewAll: &str = "View All";
        /// Screen-reader label of a spinner or skeleton with no visible text.
        pub const loading: &str = "Loading";
        /// Screen-reader label of the checkmark on a chosen menu item (Android menus).
        pub const selected: &str = "Selected";
        /// Android long-press action label on a row with a menu (onLongClickLabel).
        pub const moreOptions: &str = "More options";
        /// Action label on a tappable banner or toast (Android onClickLabel).
        pub const dismiss: &str = "Dismiss";
        /// Featured collection without sections; verification that keeps failing (Android 'Open the website').
        pub const openWebsite: &str = "Open website";
        /// A book's title when the library gives none (BookRef).
        pub const untitled: &str = "Untitled";
        /// Between parts of one line: 'Due 27 Oct · 3 days left'. Join parts with this, never a literal.
        pub const separator: &str = " · ";
        /// Stand-in for {library} when the library's name is unknown. Capitalise the first letter when it starts a sentence.
        pub const yourLibrary: &str = "your library";
    }

    pub mod Account {
        pub const signIn: &str = "Sign in";
        pub const signOut: &str = "Sign out";
        pub const signedIn: &str = "Signed in";
        pub const signingIn: &str = "Signing in…";
    }

    pub mod Tabs {
        pub const home: &str = "Home";
        pub const yourBooks: &str = "Your Books";
        pub const search: &str = "Search";
        /// Was 'Favourites' in both apps' tab bar.
        pub const myPicks: &str = "My Picks";
        pub const profile: &str = "Profile";
    }

    pub mod A11y {
        /// "Cover of {title}"
        pub fn coverOf(title: &str) -> String {
            format!("Cover of {}", title)
        }
        /// "View All {section}"
        /// Screen-reader label of a section's View All button: 'View All New arrivals'.
        pub fn viewAllSection(section: &str) -> String {
            format!("View All {}", section)
        }
        /// "{label}. {hint}"
        /// Android only. Compose has no hint, and TalkBack reads an onClickLabel as 'Double-tap to <label>', so a third-person hint ('Opens Profile') goes after the label in contentDescription, as FeaturedRow already does. Use onClickLabel / customActions only for imperative action names (Edit reservation, Remove from My Picks, More options).
        pub fn labelAndHint(label: &str, hint: &str) -> String {
            format!("{}. {}", label, hint)
        }
        /// Appended to a reservation card's label while its cancel/edit runs.
        pub const updating: &str = "Updating";
        /// "{status}. Refreshing"
        /// Your Books status line while a refresh runs: 'Updated 3 min ago. Refreshing'.
        pub fn refreshing(status: &str) -> String {
            format!("{}. Refreshing", status)
        }
        pub const dismissHint: &str = "Dismisses this message";
    }

    pub mod Errors {
        pub const cantConnectTitle: &str = "Can’t connect";
        pub const verificationNeededTitle: &str = "Verification needed";
        pub const libraryUnavailableTitle: &str = "Library unavailable";
        pub const signedOutTitle: &str = "Signed out";
        pub const signInFailedTitle: &str = "Sign-in failed";
        pub const unexpectedTitle: &str = "Something went wrong";
        /// "Can’t reach {library}. Check your connection."
        /// §5.7 / §7.1 glossary. {library} = the library's name, else common.yourLibrary (was 'the library' on both).
        pub fn network(library: &str) -> String {
            format!("Can’t reach {}. Check your connection.", library)
        }
        /// "{library} asks you to confirm you’re human."
        pub fn verification(library: &str) -> String {
            format!("{} asks you to confirm you’re human.", library)
        }
        pub const signInAgain: &str = "Please sign in again.";
        pub const credentialsRejected: &str = "Card number or PIN not recognised.";
        pub const unexpected: &str = "The library’s website returned something unexpected.";
        /// "{library} can’t be used from the app right now. Try its website instead."
        pub fn libraryUnavailable(library: &str) -> String {
            format!("{} can’t be used from the app right now. Try its website instead.", library)
        }
        /// The disclosure under an error that shows the core's technical text.
        pub const details: &str = "Details";
    }

    pub mod Home {
        /// "Hello {name}!"
        /// {name} = the borrower's first name.
        pub fn greeting(name: &str) -> String {
            format!("Hello {}!", name)
        }
        pub const greetingSignedOut: &str = "Hello!";
        /// "{library} profile"
        /// Screen-reader label of the library logo button in Home's header.
        pub fn profileButton(library: &str) -> String {
            format!("{} profile", library)
        }
        pub const profileButtonHint: &str = "Opens Profile";
        /// Screen-reader label of Home's search field (a button that opens the Search tab); the visible placeholder is search.placeholder.
        pub const searchField: &str = "Search";
        pub const searchFieldHint: &str = "Opens Search";
        /// Fallback only: the core's first home shelf title is used when present.
        pub const newArrivals: &str = "New arrivals";
        pub const nothingNewTitle: &str = "Nothing new right now";
        pub const nothingNewMessage: &str = "Check back soon for the latest additions.";
        pub const popularGenres: &str = "Popular genres";
        /// "Shows {genre} books"
        pub fn genreHint(genre: &str) -> String {
            format!("Shows {} books", genre)
        }
        /// §7.1: Home's loans shelf (was 'Your borrowed books').
        pub const yourLoans: &str = "Your loans";
        pub const noLoansTitle: &str = "No loans right now";
        pub const noLoansMessage: &str = "Books on loan show up here with their due dates.";
        /// Signed-out card in the Your loans section; its button is account.signIn.
        pub const signInCardTitle: &str = "See your loans here";
        /// "Sign in with your {library} card."
        pub fn signInCardMessage(library: &str) -> String {
            format!("Sign in with your {} card.", library)
        }
    }

    pub mod Biometrics {
        /// {method} (§5.14): iOS picks faceId / touchId / opticId from LAContext.biometryType; Android always says fingerprint. The only words that differ between the apps (§7.1).
        pub const faceId: &str = "Face ID";
        pub const touchId: &str = "Touch ID";
        pub const opticId: &str = "Optic ID";
        pub const fingerprint: &str = "fingerprint";
        /// "Sign in with {method}"
        /// Sign-in sheet's secondary button and Profile's switch title.
        pub fn signInWith(method: &str) -> String {
            format!("Sign in with {}", method)
        }
        /// "Sign in with {method} next time?"
        /// Alert after a typed sign-in; buttons use(method) / common.notNow. {library} = the library's name.
        pub fn offerTitle(method: &str) -> String {
            format!("Sign in with {} next time?", method)
        }
        /// "When {library} signs you out, sign back in with {method} instead of typing your PIN. Your card number and PIN stay on this computer, locked by {method}."
        /// iOS: "When {library} signs you out, sign back in with {method} instead of typing your PIN. Your card number and PIN stay on this phone, locked by {method}."
        /// Android: "When {library} signs you out, sign back in with {method} instead of typing your PIN. Your card number and PIN stay on this phone, locked by {method}."
        pub fn offerMessage(library: &str, method: &str) -> String {
            format!("When {} signs you out, sign back in with {} instead of typing your PIN. Your card number and PIN stay on this computer, locked by {}.", library, method, method)
        }
        /// "Use {method}"
        pub fn r#use(method: &str) -> String {
            format!("Use {}", method)
        }
        /// "No PIN to type when {library} signs you out."
        /// Profile switch subtitle under signInWith.
        pub fn switchSubtitle(library: &str) -> String {
            format!("No PIN to type when {} signs you out.", library)
        }
        /// "Sign in once more to turn on {method}."
        /// Above the sign-in fields when the sheet was opened by turning Profile's switch on.
        pub fn turnOnNote(method: &str) -> String {
            format!("Sign in once more to turn on {}.", method)
        }
        /// "Sign in to {library}"
        /// iOS LAContext localizedReason (shown with Touch ID and after a failed Face ID); Android BiometricPrompt title.
        pub fn reason(library: &str) -> String {
            format!("Sign in to {}", library)
        }
        /// iOS localizedFallbackTitle; Android BiometricPrompt negative button. Both show the sign-in form.
        pub const enterPin: &str = "Enter PIN";
        /// iOS Info.plist NSFaceIDUsageDescription, copied there by hand (gen_copy doesn't write the plist). Swift doesn't use the constant.
        pub const faceIdUsage: &str = "Signs you back in to your library without typing your PIN.";
    }

    pub mod MyCard {
        pub const button: &str = "My card";
        pub const buttonHint: &str = "Shows your library card’s barcode";
        pub const title: &str = "My card";
        pub const showAtDesk: &str = "Show this at the library desk or self-checkout.";
        pub const editNumber: &str = "Edit card number";
        pub const numberLabel: &str = "Library card number";
        pub const numberPlaceholder: &str = "Card number";
        pub const numberNote: &str = "Enter the number on your library card or in your library account’s Personal details (e.g. CL114D…).";
        /// Caption under the library's name on the card (Android: the card's title when the library has no name).
        pub const libraryCard: &str = "Library card";
        /// "Loading barcode for card {number}"
        pub fn loadingBarcode(number: &str) -> String {
            format!("Loading barcode for card {}", number)
        }
        /// "Barcode for card {number}"
        /// The barcode is one screen-reader element (§5.9).
        pub fn barcode(number: &str) -> String {
            format!("Barcode for card {}", number)
        }
        pub const cannotEncode: &str = "This number can’t be shown as a barcode.";
        /// "Expires {date}"
        /// {date} = cardExpires as the library writes it.
        pub fn expires(date: &str) -> String {
            format!("Expires {}", date)
        }
        /// Desktop My card dialog while signed out, over account.signIn.
        pub const signInMessage: &str = "Sign in to show your library card’s barcode.";
        /// Status line next to common.retry when the library's barcode type didn't load: no full stop.
        pub const barcodeFailed: &str = "Couldn’t load the barcode";
    }

    pub mod Featured {
        pub const title: &str = "Featured";
        /// Tile hint for .collection and .search links, unless the link has its own description.
        pub const collectionHint: &str = "Shows this collection";
        pub const websiteHint: &str = "Opens the library’s website";
        /// Collection page without sections; its button is common.openWebsite.
        pub const websiteOnly: &str = "This collection is on the library’s website.";
        pub const emptyTitle: &str = "Nothing here right now";
        pub const emptyMessage: &str = "Check back soon, or open the collection on the library’s website.";
    }

    pub mod Search {
        pub const title: &str = "Search";
        pub const placeholder: &str = "Search by title, author, etc.";
        pub const clearSearch: &str = "Clear search";
        /// Label of the sort picker (iOS Picker title).
        pub const sort: &str = "Sort";
        pub const sortResults: &str = "Sort results";
        pub const relevance: &str = "Relevance";
        pub const newest: &str = "Newest";
        /// "Search {library}"
        pub fn introTitle(library: &str) -> String {
            format!("Search {}", library)
        }
        pub const introMessage: &str = "Find books, eBooks, audiobooks and more by title, author or subject.";
        pub const recentSearches: &str = "Recent searches";
        /// Hint on a recent search row; its label is the query as shown (was the label 'Search again for {query}', which Voice Control can't match to what's on screen).
        pub const recentSearchHint: &str = "Searches again";
        /// Android onClickLabel of a recent search row ('Double-tap to search again'), used instead of recentSearchHint; its long-press action is common.remove.
        pub const searchAgain: &str = "Search again";
        /// One: "1 result" · other: "{count} results"
        pub fn results(count: i64) -> String {
            if count == 1 {
                "1 result".to_owned()
            } else {
                format!("{} results", super::super::copy_number(count))
            }
        }
        /// "No results for “{query}”"
        pub fn noResultsTitle(query: &str) -> String {
            format!("No results for “{}”", query)
        }
        pub const noResultsMessage: &str = "Check the spelling, or try fewer or different words.";
        /// "Nothing in “{title}” right now"
        /// A results list opened from a URL (Home View All, genre tile, Featured search link) that came back empty.
        pub fn listEmptyTitle(title_: &str) -> String {
            format!("Nothing in “{}” right now", title_)
        }
        pub const listEmptyMessage: &str = "Check back soon.";
        pub const endOfResults: &str = "End of results";
        pub const loadingResults: &str = "Loading results";
        pub const loadingMore: &str = "Loading more results";
        pub const retryLoadingMoreHint: &str = "Loads more results";
        /// "+{count}"
        /// Chip after the first format chips: '+2'.
        pub fn moreFormats(count: i64) -> String {
            format!("+{}", super::super::copy_number(count))
        }
    }

    pub mod Detail {
        /// "By {author}"
        pub fn byline(author: &str) -> String {
            format!("By {}", author)
        }
        pub const reserve: &str = "Reserve";
        /// "Reserves the {format}"
        /// Hint on the Reserve pill. {format} = the format's label, lower-cased ('book', 'large print'). An edition-level item (no label) uses reserveItemHint.
        pub fn reserveFormatHint(format: &str) -> String {
            format!("Reserves the {}", format)
        }
        pub const reserveItemHint: &str = "Reserves this item";
        /// "Asks you to sign in, then reserves the {format}"
        /// Signed out, the Reserve pill opens sign-in first.
        pub fn signInToReserveFormatHint(format: &str) -> String {
            format!("Asks you to sign in, then reserves the {}", format)
        }
        pub const signInToReserveItemHint: &str = "Asks you to sign in, then reserves this item";
        pub const availableOnline: &str = "Available online";
        pub const askLibrary: &str = "Ask the library about availability";
        /// "Availability: {status}"
        /// Screen-reader label of the availability line; {status} = the visible text (core summary label or availableOnline / askLibrary).
        pub fn availability(status: &str) -> String {
            format!("Availability: {}", status)
        }
        pub const availabilityFailed: &str = "Couldn’t check availability";
        pub const lookupFailed: &str = "Couldn’t look up availability";
        pub const checkWebsite: &str = "Check availability on the library’s website";
        /// Hint on the Retry next to availabilityFailed (its label stays common.retry; was the label 'Retry: Couldn’t check availability'). retryLookupHint goes with lookupFailed.
        pub const retryAvailabilityHint: &str = "Checks availability again";
        pub const retryLookupHint: &str = "Looks up availability again";
        pub const findingFormats: &str = "Finding formats…";
        pub const checkingAvailability: &str = "Checking availability…";
        pub const noRecordLink: &str = "This book has no link to the library’s record.";
        pub const formats: &str = "Formats";
        pub const formatHint: &str = "Shows availability for this format";
        pub const synopsis: &str = "Synopsis";
        pub const more: &str = "More";
        pub const less: &str = "Less";
        /// The synopsis's More / Less button keeps its visible text as its label (was 'Show more of the synopsis' / 'Show less of the synopsis').
        pub const moreHint: &str = "Shows the rest of the synopsis";
        pub const lessHint: &str = "Shows less of the synopsis";
        pub const noMoreDetails: &str = "The library has no more details for this title.";
        pub const loadingDetails: &str = "Loading details…";
        pub const subjects: &str = "Subjects";
        pub const subjectHint: &str = "Searches the catalogue for this subject";
        pub const details: &str = "Details";
        pub const copies: &str = "Copies";
        pub const loadingCopies: &str = "Loading copies…";
        pub const copiesFailed: &str = "Couldn’t load copies";
        pub const retryCopiesHint: &str = "Loads the copies again";
        /// A Copies row whose holding has no location or collection.
        pub const copy: &str = "Copy";
        /// "Call number {number}"
        pub fn callNumber(number: &str) -> String {
            format!("Call number {}", number)
        }
        pub const similarBooks: &str = "Similar books";
        pub const moreByAuthor: &str = "More by this author";
        pub const inThisSeries: &str = "In this series";
        pub const otherEditions: &str = "Other editions";
        /// "Loading {panel}"
        /// Screen-reader label of a related carousel while it loads; {panel} = its title, lower-cased.
        pub fn loadingPanel(panel: &str) -> String {
            format!("Loading {}", panel)
        }
        pub const onLoan: &str = "On loan";
        /// "On loan · Due {date}"
        /// Already-yours pill (§6, I8/A6). {date} = '14 Oct' (loans.shortDate style), or the library's text when unreadable.
        pub fn onLoanDue(date: &str) -> String {
            format!("On loan · Due {}", date)
        }
        pub const onLoanOverdue: &str = "On loan · Overdue";
        /// "On loan, due {date}"
        /// Spoken: {date} written out ('14 October').
        pub fn onLoanDueLabel(date: &str) -> String {
            format!("On loan, due {}", date)
        }
        /// "On loan, overdue, was due {date}"
        pub fn onLoanOverdueLabel(date: &str) -> String {
            format!("On loan, overdue, was due {}", date)
        }
        pub const readyToCollect: &str = "Ready to collect";
        /// "Ready to collect · {branch}"
        /// §7.1: Ready to collect (+ ' at {branch}' when spoken); the visible pill joins with ' · ' (§6).
        pub fn readyToCollectAt(branch: &str) -> String {
            format!("Ready to collect · {}", branch)
        }
        /// "Ready to collect at {branch}"
        pub fn readyToCollectAtLabel(branch: &str) -> String {
            format!("Ready to collect at {}", branch)
        }
        pub const reserved: &str = "Reserved";
        /// "Reserved · {branch}"
        pub fn reservedAt(branch: &str) -> String {
            format!("Reserved · {}", branch)
        }
        /// "Reserved, collect from {branch}"
        /// §7.1 glossary: 'collect from' (was 'pick up at').
        pub fn reservedAtLabel(branch: &str) -> String {
            format!("Reserved, collect from {}", branch)
        }
        /// "Reserved · #{place} in queue"
        /// Numeric queue place.
        pub fn reservedQueue(place: &str) -> String {
            format!("Reserved · #{} in queue", place)
        }
        /// "Reserved, number {place} in the queue"
        pub fn reservedQueueLabel(place: &str) -> String {
            format!("Reserved, number {} in the queue", place)
        }
        /// "Reserved · {place} in queue"
        /// A queue place the library writes as text ('1 of 3'). §7.1 bans 'rank' (was 'Reserved · Rank {n}').
        pub fn reservedQueueText(place: &str) -> String {
            format!("Reserved · {} in queue", place)
        }
        /// "Reserved, {place} in the queue"
        pub fn reservedQueueTextLabel(place: &str) -> String {
            format!("Reserved, {} in the queue", place)
        }
        /// The already-yours pill opens Your Books on the right list; a renewable loan's pill asks to renew it instead (hint yourBooks.renewHint).
        pub const heldHint: &str = "Opens Your Books";
    }

    pub mod Reserve {
        pub const title: &str = "Reserve";
        pub const checkingOptions: &str = "Checking reservation options…";
        pub const cantReserve: &str = "This title can’t be reserved right now.";
        pub const notFromApp: &str = "This format can’t be reserved from the app.";
        pub const confirm: &str = "Confirm reservation";
        /// §7.1 glossary: the branch row (was 'Pick up at').
        pub const collectFrom: &str = "Collect from";
        /// Value of the Collect from row when the library doesn't name a branch (§7.1: 'branch' for a branch; was 'Your usual library').
        pub const usualBranch: &str = "Your usual branch";
        pub const availability: &str = "Availability";
        pub const notNeededAfter: &str = "Not needed after";
        /// Android onClickLabel of the Not needed after dropdown.
        pub const chooseNotNeededAfter: &str = "Choose when it’s no longer needed";
        /// Result banner when the library's own message is empty. A toast-style confirmation: no full stop (§7.1).
        pub const placed: &str = "Reservation placed";
        /// Result banner when the library refused and gave no message.
        pub const placeFailed: &str = "The library couldn’t place this reservation.";
    }

    pub mod Loans {
        /// "Due {date}"
        /// {date} = '27 Oct' (day + short month; + year only when it isn't this year). Due-date notifications pass the weekday too: 'Due Thursday 16 Oct'. A date the core can't read is shown as the library wrote it.
        pub fn dueOn(date: &str) -> String {
            format!("Due {}", date)
        }
        pub const dueToday: &str = "Due today";
        pub const dueTomorrow: &str = "Due tomorrow";
        /// One: "1 day left" · other: "{count} days left"
        pub fn daysLeft(count: i64) -> String {
            if count == 1 {
                "1 day left".to_owned()
            } else {
                format!("{} days left", super::super::copy_number(count))
            }
        }
        /// One: "1 day overdue" · other: "{count} days overdue"
        /// §7.1 glossary (Android's unused Formatting.daysLeftLabel said 'Overdue by {n} days').
        pub fn daysOverdue(count: i64) -> String {
            if count == 1 {
                "1 day overdue".to_owned()
            } else {
                format!("{} days overdue", super::super::copy_number(count))
            }
        }
        /// "Due {date} · {days}"
        /// Your Books loan card meta line: 'Due 27 Oct · 3 days left' / 'Due 10 Oct · 2 days overdue'; {days} = daysLeft / daysOverdue.
        pub fn lineDays(date: &str, days: &str) -> String {
            format!("Due {} · {}", date, days)
        }
        /// "Due today · {date}"
        /// Your Books loan due today: 'Due today · 12 Oct'.
        pub fn lineToday(date: &str) -> String {
            format!("Due today · {}", date)
        }
        /// "Due tomorrow · {date}"
        pub fn lineTomorrow(date: &str) -> String {
            format!("Due tomorrow · {}", date)
        }
        /// Chip on an overdue loan card and part of its screen-reader label.
        pub const overdue: &str = "Overdue";
    }

    pub mod YourBooks {
        pub const title: &str = "Your Books";
        pub const loans: &str = "Loans";
        pub const reservations: &str = "Reservations";
        /// "Loans ({count})"
        pub fn loansCount(count: i64) -> String {
            format!("Loans ({})", super::super::copy_number(count))
        }
        /// "Reservations ({count})"
        pub fn reservationsCount(count: i64) -> String {
            format!("Reservations ({})", super::super::copy_number(count))
        }
        /// Label of the Loans | Reservations picker (read by screen readers, not shown).
        pub const show: &str = "Show";
        pub const loadingLoans: &str = "Loading your loans…";
        pub const loadingReservations: &str = "Loading your reservations…";
        pub const noLoansTitle: &str = "No loans right now";
        /// "Books on loan from {library} show up here with their due dates."
        pub fn noLoansMessage(library: &str) -> String {
            format!("Books on loan from {} show up here with their due dates.", library)
        }
        /// Swipe action / button on a loan, and the Renew dialog's confirming button (cancel button: common.notNow).
        pub const renew: &str = "Renew";
        /// Menu item and screen-reader action on a loan row (like editReservation / cancelReservation).
        pub const renewLoan: &str = "Renew loan";
        /// Footnote under the Loans list (replaces renewalsSoon); touchAndHold variant where the reservation footnote uses touchAndHoldHint.
        pub const loanSwipeHint: &str = "Swipe left on a loan to renew it.";
        pub const loanTouchAndHoldHint: &str = "Touch and hold a loan to renew it.";
        pub const renewTitle: &str = "Renew this loan?";
        /// "“{title}” is due {date}. Renewing asks the library for a later due date."
        /// Renew dialog body; {date} as loans.dueOn writes it. renewMessageNoDate when the loan has no due date.
        pub fn renewMessage(title_: &str, date: &str) -> String {
            format!("“{}” is due {}. Renewing asks the library for a later due date.", title_, date)
        }
        /// "Renewing “{title}” asks the library for a later due date."
        pub fn renewMessageNoDate(title_: &str) -> String {
            format!("Renewing “{}” asks the library for a later due date.", title_)
        }
        /// Progress toast while renewLoan runs (it takes a few seconds: it reads the loans list before and after).
        pub const renewing: &str = "Renewing…";
        /// "Renewed · Due {date}"
        /// Success toast; {date} = RenewResult.due as loans.dueOn writes it. renewedNoDate when due is missing.
        pub fn renewed(date: &str) -> String {
            format!("Renewed · Due {}", date)
        }
        pub const renewedNoDate: &str = "Renewed";
        /// Error toast when renewLoan throws (not NotLoggedIn). When RenewResult.ok is false, show RenewResult.message (the library's reason or the core's) as the error toast instead.
        pub const renewFailed: &str = "Couldn’t renew this loan";
        /// Screen-reader hint on the Renew action, and on the book page's On loan pill of a loan that can be renewed.
        pub const renewHint: &str = "Asks the library for a later due date";
        pub const noReservationsTitle: &str = "No reservations";
        /// iOS: "Tap Reserve on any book and follow it here until it’s ready to collect."
        /// Android: "Tap Reserve on any book and follow it here until it’s ready to collect."
        pub const noReservationsMessage: &str = "Click Reserve on any book and follow it here until it’s ready to collect.";
        /// The ready card's status chip (§7.1; was 'Ready for pickup'). Pending cards show the library's own status, else 'Pending'.
        pub const readyToCollect: &str = "Ready to collect";
        pub const pending: &str = "Pending";
        /// "Ready to collect ({count})"
        /// Reservations section headers with how many are in each, as loansCount / reservationsCount write the segments: 'Ready to collect (2)', 'Pending (3)'.
        pub fn readyCount(count: i64) -> String {
            format!("Ready to collect ({})", super::super::copy_number(count))
        }
        /// "Pending ({count})"
        pub fn pendingCount(count: i64) -> String {
            format!("Pending ({})", super::super::copy_number(count))
        }
        /// "Collect from {branch}"
        /// Card meta line (§7.1; was 'Pick up at {branch}'). {branch} title-cased when the library shouts it.
        pub fn collectFrom(branch: &str) -> String {
            format!("Collect from {}", branch)
        }
        /// "Collect from {branch} by {date}"
        pub fn collectFromBy(branch: &str, date: &str) -> String {
            format!("Collect from {} by {}", branch, date)
        }
        /// "Collect by {date}"
        /// Ready card with no branch: 'Collect by 17 Oct' (was 'Pick up by …').
        pub fn collectBy(date: &str) -> String {
            format!("Collect by {}", date)
        }
        /// "#{place} in queue"
        /// Pending card meta: '#4 in queue · Placed 2 Oct'.
        pub fn queuePlace(place: &str) -> String {
            format!("#{} in queue", place)
        }
        /// "{place} in queue"
        /// A queue place the library writes as text (was 'Rank {n}'; §7.1 bans 'rank').
        pub fn queuePlaceText(place: &str) -> String {
            format!("{} in queue", place)
        }
        /// "number {place} in the queue"
        /// The card's screen-reader label says the queue place this way (§7.1 glossary); queuePlaceTextLabel for a place written as text.
        pub fn queuePlaceLabel(place: &str) -> String {
            format!("number {} in the queue", place)
        }
        /// "{place} in the queue"
        pub fn queuePlaceTextLabel(place: &str) -> String {
            format!("{} in the queue", place)
        }
        /// "Placed {date}"
        pub fn placed(date: &str) -> String {
            format!("Placed {}", date)
        }
        /// Footnote under the reservations where cards swipe (iOS 18+, Android). Hidden from screen readers: they get the actions on each card.
        pub const swipeHint: &str = "Swipe left on a reservation to edit or cancel it.";
        /// The same footnote on iOS 17, where cards don't swipe (long-press menu only).
        pub const touchAndHoldHint: &str = "Touch and hold a reservation to edit or cancel it.";
        /// Desktop: the footnote under the reservations, whose cards have a '…' menu button and the same right-click menu (docs/desktop-app.md §6). loanRightClickHint goes under the loans.
        pub const rightClickHint: &str = "Right-click a reservation to edit or cancel it.";
        pub const loanRightClickHint: &str = "Right-click a loan to renew it.";
        /// Long-press menu item and screen-reader action (Edit / common.cancel are the swipe tiles).
        pub const editReservation: &str = "Edit reservation";
        /// Menu item, screen-reader action and the confirmation's destructive button (iOS said 'Cancel Reservation').
        pub const cancelReservation: &str = "Cancel reservation";
        /// Android onLongClickLabel of a reservation card.
        pub const showActions: &str = "Show reservation actions";
        pub const signedOutTitle: &str = "Sign in to see your books";
        /// "Your loans and reservations at {library} appear here."
        pub fn signedOutMessage(library: &str) -> String {
            format!("Your loans and reservations at {} appear here.", library)
        }
        /// Banner after a cancel when the library's message is empty. Toast-style: no full stop.
        pub const cancelled: &str = "Reservation cancelled";
        /// Error banner when the library refused with no message.
        pub const cancelFailed: &str = "The library didn’t cancel this reservation.";
        pub const updated: &str = "Reservation updated";
        pub const cancelTitle: &str = "Cancel this reservation?";
        /// "“{title}” is waiting for you at {branch}. Cancelling releases it to the next person."
        pub fn cancelReadyMessage(title_: &str, branch: &str) -> String {
            format!("“{}” is waiting for you at {}. Cancelling releases it to the next person.", title_, branch)
        }
        /// "“{title}” is waiting for you. Cancelling releases it to the next person."
        pub fn cancelReadyMessageNoBranch(title_: &str) -> String {
            format!("“{}” is waiting for you. Cancelling releases it to the next person.", title_)
        }
        /// "“{title}” will be removed from your reservations and you’ll lose your place in the queue."
        pub fn cancelPendingMessage(title_: &str) -> String {
            format!("“{}” will be removed from your reservations and you’ll lose your place in the queue.", title_)
        }
        pub const keepReservation: &str = "Keep reservation";
        pub const updatedJustNow: &str = "Updated just now";
        /// One: "Updated 1 min ago" · other: "Updated {count} min ago"
        pub fn updatedMinutesAgo(count: i64) -> String {
            if count == 1 {
                "Updated 1 min ago".to_owned()
            } else {
                format!("Updated {} min ago", super::super::copy_number(count))
            }
        }
        /// "Updated today at {time}"
        pub fn updatedToday(time: &str) -> String {
            format!("Updated today at {}", time)
        }
        /// "Updated yesterday at {time}"
        pub fn updatedYesterday(time: &str) -> String {
            format!("Updated yesterday at {}", time)
        }
        /// "Updated {day} at {time}"
        /// {day} = weekday within a week (Android) or '3 Oct' / '3 Oct 2025'; {time} in the device's 12/24-hour style.
        pub fn updatedOn(day: &str, time: &str) -> String {
            format!("Updated {} at {}", day, time)
        }
        /// §5.2a 'Couldn't refresh' note, with common.retry and refreshHint (was 'Couldn't refresh — showing saved data'; the same words as onboarding.savedList).
        pub const refreshFailed: &str = "Couldn’t refresh · Showing the saved list";
        pub const refreshHint: &str = "Fetches your loans and reservations again";
    }

    pub mod EditReservation {
        pub const title: &str = "Edit reservation";
        pub const loading: &str = "Loading reservation…";
        pub const cantEditTitle: &str = "Can’t edit this reservation";
        pub const cantEditMessage: &str = "This reservation can’t be edited right now.";
        /// §7.1 glossary (was 'Pickup location'); the same words as the Reserve sheet.
        pub const collectFrom: &str = "Collect from";
        pub const chooseBranch: &str = "Choose a branch";
        pub const delayUntil: &str = "Delay until";
        pub const delayFooter: &str = "The library won’t fill the reservation before this date.";
        /// One term with the Reserve sheet's expiry picker (was 'Not required after').
        pub const notNeededAfter: &str = "Not needed after";
        pub const notNeededFooter: &str = "The reservation lapses if it isn’t ready by this date.";
        /// "Cleared when you save. {footer}"
        pub fn clearedFooter(footer: &str) -> String {
            format!("Cleared when you save. {}", footer)
        }
        pub const saveChanges: &str = "Save changes";
        pub const changeFailed: &str = "The library didn’t accept the change.";
        /// iOS DatePicker title (hidden; its screen-reader label is dateLabel).
        pub const date: &str = "Date";
        /// "{field} date"
        /// Screen-reader label of a date row: 'Delay until date'; Android reads the date as its state.
        pub fn dateLabel(field: &str) -> String {
            format!("{} date", field)
        }
        pub const setDate: &str = "Set date";
        /// "Change {field} date"
        /// Android onClickLabel of a set date; setFieldDate when the date is off; clearField on its clear button. {field} is the row's title lower-cased mid-sentence: 'Change delay until date', 'Clear not needed after'.
        pub fn changeDate(field: &str) -> String {
            format!("Change {} date", field)
        }
        /// "Set {field} date"
        pub fn setFieldDate(field: &str) -> String {
            format!("Set {} date", field)
        }
        /// "Clear {field}"
        pub fn clearField(field: &str) -> String {
            format!("Clear {}", field)
        }
    }

    pub mod Picks {
        /// §7.1: the tab, the screen title (signed in and out), the bookmark and the toasts. Never Favourites.
        pub const title: &str = "My Picks";
        pub const subtitleAccount: &str = "Saved on your library account";
        /// iOS: "Saved on this phone"
        /// Android: "Saved on this phone"
        /// Signed out (was '{n} saved for {library}' / 'Saved on this device for {library}').
        pub const subtitlePhone: &str = "Saved on this computer";
        /// Bookmark button's screen-reader label (book page and anywhere else a toggle appears), signed in or out (was 'Add to favourites').
        pub const save: &str = "Save to My Picks";
        /// Bookmark label when saved; also the row's remove action / menu item (was 'Remove from Favourites').
        pub const remove: &str = "Remove from My Picks";
        /// Toast after a save (§5.11).
        pub const saved: &str = "Saved to My Picks";
        pub const removed: &str = "Removed from My Picks";
        /// Error toast (§5.11 had a full stop; §7.1: never on toasts).
        pub const updateFailed: &str = "Couldn’t update My Picks";
        pub const emptyTitle: &str = "No picks yet";
        /// iOS: "Tap the bookmark on a book to save it to your library account."
        /// Android: "Tap the bookmark on a book to save it to your library account."
        pub const emptyAccountMessage: &str = "Click the bookmark on a book to save it to your library account.";
        /// iOS: "Tap the bookmark on a book to save it on this phone."
        /// Android: "Tap the bookmark on a book to save it on this phone."
        /// Signed out (was 'Tap the bookmark on any book to save it here.'); its button is findABook.
        pub const emptyPhoneMessage: &str = "Click the bookmark on a book to save it on this computer.";
        pub const findABook: &str = "Find a book";
        /// Hint on the signed-out list's Edit button (its label is the visible common.edit; was the label 'Edit favourites'); doneEditingHint goes with common.done.
        pub const editHint: &str = "Lets you remove books from My Picks";
        pub const doneEditingHint: &str = "Finishes editing My Picks";
        /// "Remove {title} from My Picks"
        /// Screen-reader label of a row's remove button in edit mode.
        pub fn removeTitle(title_: &str) -> String {
            format!("Remove {} from My Picks", title_)
        }
        /// "Removed {title} from My Picks"
        /// Screen-reader announcement after a remove on the signed-out list.
        pub fn removedTitle(title_: &str) -> String {
            format!("Removed {} from My Picks", title_)
        }
        pub const loadingMore: &str = "Loading more picks";
        pub const retryLoadingMoreHint: &str = "Loads more picks";
        /// iOS: "Couldn’t load My Picks from this phone."
        /// Android: "Couldn’t load My Picks from this phone."
        /// iOS banner when the phone's list can't be read (was 'Couldn't read favourites: <system error>').
        pub const loadFailed: &str = "Couldn’t load My Picks from this computer.";
        /// iOS: "Couldn’t save My Picks on this phone."
        /// Android: "Couldn’t save My Picks on this phone."
        pub const saveFailed: &str = "Couldn’t save My Picks on this computer.";
        /// iOS: "Add this phone’s picks to your library account?"
        /// Android: "Add this phone’s picks to your library account?"
        /// §5.11 copy-once alert; buttons common.add / common.notNow.
        pub const offerTitle: &str = "Add this computer’s picks to your library account?";
        /// One: "1 book saved on this computer can be added to My Picks on your library account." · other: "{count} books saved on this computer can be added to My Picks on your library account."
        /// iOS: One: "1 book saved on this phone can be added to My Picks on your library account." · other: "{count} books saved on this phone can be added to My Picks on your library account."
        /// Android: One: "1 book saved on this phone can be added to My Picks on your library account." · other: "{count} books saved on this phone can be added to My Picks on your library account."
        pub fn offerMessage(count: i64) -> String {
            if count == 1 {
                "1 book saved on this computer can be added to My Picks on your library account.".to_owned()
            } else {
                format!("{} books saved on this computer can be added to My Picks on your library account.", super::super::copy_number(count))
            }
        }
        /// One: "Adding 1 book to My Picks…" · other: "Adding {count} books to My Picks…"
        /// Toast while the copy runs (iOS has it; Android should gain it).
        pub fn adding(count: i64) -> String {
            if count == 1 {
                "Adding 1 book to My Picks…".to_owned()
            } else {
                format!("Adding {} books to My Picks…", super::super::copy_number(count))
            }
        }
        /// One: "Added 1 book to My Picks" · other: "Added {count} books to My Picks"
        /// Toast when the copy ends; when some failed, join notAdded with common.separator: 'Added 3 books to My Picks · 1 couldn’t be added'.
        pub fn added(count: i64) -> String {
            if count == 1 {
                "Added 1 book to My Picks".to_owned()
            } else {
                format!("Added {} books to My Picks", super::super::copy_number(count))
            }
        }
        /// One: "1 couldn’t be added" · other: "{count} couldn’t be added"
        pub fn notAdded(count: i64) -> String {
            if count == 1 {
                "1 couldn’t be added".to_owned()
            } else {
                format!("{} couldn’t be added", super::super::copy_number(count))
            }
        }
    }

    pub mod Profile {
        pub const title: &str = "Profile";
        pub const noLibraryTitle: &str = "No library selected";
        pub const noLibraryMessage: &str = "Choose your library to search its catalogue and sign in.";
        pub const chooseLibrary: &str = "Choose library";
        pub const changeLibrary: &str = "Change library";
        /// Section header; the signed-in row says account.signedIn and its button account.signOut.
        pub const account: &str = "Account";
        pub const loadingAccount: &str = "Loading your account…";
        /// "Signed in to {library}"
        pub fn signedInTo(library: &str) -> String {
            format!("Signed in to {}", library)
        }
        /// Inline status line next to common.retry, like detail.copiesFailed: no full stop.
        pub const accountFailed: &str = "Couldn’t load your account details";
        /// "Sign out of {library}?"
        /// Confirmation; its destructive button is account.signOut, the other common.cancel.
        pub fn signOutTitle(library: &str) -> String {
            format!("Sign out of {}?", library)
        }
        pub const signOutMessage: &str = "You can sign in again at any time.";
        pub const notSignedInTitle: &str = "You’re not signed in";
        pub const notSignedInMessage: &str = "Sign in with your library card to see your loans and reservations.";
        pub const humanVerification: &str = "Human verification";
        /// iOS: "Verified on this phone"
        /// Android: "Verified on this phone"
        pub const verified: &str = "Verified on this computer";
        /// "{library} asked you to confirm you’re human. The app sends the library’s verification cookie with its requests."
        pub fn verifiedMessage(library: &str) -> String {
            format!("{} asked you to confirm you’re human. The app sends the library’s verification cookie with its requests.", library)
        }
        pub const clearVerification: &str = "Clear verification";
        /// Confirmation; its destructive button is clearVerification, the other common.cancel.
        pub const clearVerificationTitle: &str = "Clear verification?";
        /// "{library} will ask you to confirm you’re human again."
        pub fn clearVerificationMessage(library: &str) -> String {
            format!("{} will ask you to confirm you’re human again.", library)
        }
        pub const about: &str = "About";
        pub const appName: &str = "LibreSpydus";
        /// "Version {version} ({build})"
        pub fn version(version: &str, build: &str) -> String {
            format!("Version {} ({})", version, build)
        }
        /// "Version {version}"
        /// Desktop: the app has a version but no separate build number.
        pub fn appVersion(version: &str) -> String {
            format!("Version {}", version)
        }
        pub const notAffiliated: &str = "LibreSpydus is not affiliated with Civica or your library.";
    }

    pub mod Notifications {
        /// Profile section while signed in (§5.8).
        pub const section: &str = "Notifications";
        pub const readyTitle: &str = "Ready to collect";
        pub const readySubtitle: &str = "Checked each morning and when you open the app";
        pub const dueTitle: &str = "Due-date reminders";
        pub const dueSubtitle: &str = "2 days before books are due, at 7:45";
        pub const blocked: &str = "Notifications are turned off for LibreSpydus.";
        /// 'Settings' is the system app's name, so it keeps its capital.
        pub const openSettings: &str = "Open Settings";
    }

    pub mod Alerts {
        /// Local notification (§5.8); the body is readyBody, or just the title when the branch is unknown.
        pub const readyTitle: &str = "Ready to collect";
        /// "{title} · {branch}"
        pub fn readyBody(title: &str, branch: &str) -> String {
            format!("{} · {}", title, branch)
        }
        /// Stands in for a missing title in readyBody.
        pub const untitledReservation: &str = "A reservation";
        pub const untitledLoan: &str = "A loan";
        /// "{first} and {second}"
        /// Due-reminder body for two loans; one loan is just its title; three or more use dueMore. The title is loans.dueOn / dueToday / dueTomorrow with the weekday date ('Due Thursday 16 Oct').
        pub fn dueTwo(first: &str, second: &str) -> String {
            format!("{} and {}", first, second)
        }
        /// One: "{first} and 1 more" · other: "{first} and {count} more"
        pub fn dueMore(count: i64, first: &str) -> String {
            if count == 1 {
                format!("{} and 1 more", first)
            } else {
                format!("{} and {} more", first, super::super::copy_number(count))
            }
        }
        /// Android notification channel names.
        pub const readyChannel: &str = "Ready to collect";
        pub const dueChannel: &str = "Due-date reminders";
    }

    pub mod Onboarding {
        pub const title: &str = "Choose your library";
        pub const intro: &str = "LibreSpydus works with libraries that use Spydus. You can change this later in Profile.";
        /// The picker's title when opened from Profile (profile.changeLibrary is the row).
        pub const changeTitle: &str = "Change library";
        pub const searchPlaceholder: &str = "Search by library or country";
        pub const loading: &str = "Loading libraries…";
        pub const emptyTitle: &str = "No libraries listed";
        pub const emptyMessage: &str = "Civica’s directory came back empty. Try again later.";
        /// "No libraries match “{query}”"
        pub fn noMatchesTitle(query: &str) -> String {
            format!("No libraries match “{}”", query)
        }
        pub const noMatchesMessage: &str = "Try a library name or a country.";
        /// "{error} Showing the saved list."
        /// Banner when a refresh failed but a saved list is shown; {error} = directoryNetwork / directoryUnexpected.
        pub fn savedList(error: &str) -> String {
            format!("{} Showing the saved list.", error)
        }
        pub const directoryNetwork: &str = "Can’t reach the library directory. Check your connection.";
        pub const directoryUnexpected: &str = "The library directory returned something unexpected.";
        /// One: "1 library" · other: "{count} libraries"
        /// Screen-reader label of a region's count (iOS said '1 libraries').
        pub fn libraries(count: i64) -> String {
            if count == 1 {
                "1 library".to_owned()
            } else {
                format!("{} libraries", super::super::copy_number(count))
            }
        }
        /// Screen-reader value of the selected row (and Android's checkmark).
        pub const currentLibrary: &str = "Current library";
        /// Hint on the other rows (iOS said 'Select this library'; Android's onClickLabel 'Select this library' / 'Keep this library').
        pub const selectHint: &str = "Selects this library";
        /// Section titles, and the Profile place chip when the directory has no country.
        pub const regionAU: &str = "Australia";
        pub const regionNZ: &str = "New Zealand";
        pub const regionUS: &str = "United States";
        pub const regionUK: &str = "United Kingdom";
        pub const regionSG: &str = "Singapore";
        pub const regionOther: &str = "Other";
        /// Row notes (§6).
        pub const noteUK: &str = "Asks you to confirm you’re human";
        pub const noteSG: &str = "May be unreachable outside school networks";
    }

    pub mod Login {
        /// The sheet's title; its button is account.signIn (account.signingIn while it runs) and its close button common.close.
        pub const title: &str = "Sign in";
        /// Libraries accept either the patron's username or their library card number here.
        pub const cardLabel: &str = "Username / library card number";
        pub const cardPlaceholder: &str = "Username or card number";
        pub const pinLabel: &str = "PIN / password";
        pub const pinPlaceholder: &str = "PIN or password";
        /// Desktop: screen-reader label of the PIN field's reveal toggle (hidePin while the PIN shows).
        pub const showPin: &str = "Show PIN";
        pub const hidePin: &str = "Hide PIN";
        /// "LibreSpydus signs in directly with {library} and doesn’t store your PIN."
        pub fn privacyNote(library: &str) -> String {
            format!("LibreSpydus signs in directly with {} and doesn’t store your PIN.", library)
        }
        /// "LibreSpydus signs in directly with {library}. It keeps your PIN on this computer only if you turn on {method}."
        /// iOS: "LibreSpydus signs in directly with {library}. It keeps your PIN on this phone only if you turn on {method}."
        /// Android: "LibreSpydus signs in directly with {library}. It keeps your PIN on this phone only if you turn on {method}."
        /// Replaces privacyNote when the phone can use {method} (§5.14), since turning it on keeps the PIN.
        pub fn privacyNoteBiometrics(library: &str, method: &str) -> String {
            format!("LibreSpydus signs in directly with {}. It keeps your PIN on this computer only if you turn on {}.", library, method)
        }
    }

    pub mod Verification {
        pub const title: &str = "Confirm you’re human";
        /// "{library} asks you to confirm you’re human. Complete the check below."
        /// §6 verification sheet copy.
        pub fn message(library: &str) -> String {
            format!("{} asks you to confirm you’re human. Complete the check below.", library)
        }
        pub const closesByItself: &str = "This closes by itself once the library lets you in.";
        /// "Opening {library}…"
        pub fn opening(library: &str) -> String {
            format!("Opening {}…", library)
        }
        /// "{library} didn’t ask for a check this time. Close this and try again."
        pub fn noCheck(library: &str) -> String {
            format!("{} didn’t ask for a check this time. Close this and try again.", library)
        }
        pub const reload: &str = "Reload page";
        /// "{library} keeps asking to confirm you’re human after the check. Its own website still works in your browser."
        /// Android shows it with common.openWebsite and common.close when verification keeps failing (iOS shows errors.libraryUnavailable instead).
        pub fn keepsAsking(library: &str) -> String {
            format!("{} keeps asking to confirm you’re human after the check. Its own website still works in your browser.", library)
        }
    }

    pub mod Desktop {
        /// Desktop only (docs/desktop-app.md §8.2): the app menu and its ⌘Q item. 'LibreSpydus' is the app's name (profile.appName).
        pub const quit: &str = "Quit LibreSpydus";
        pub const hide: &str = "Hide LibreSpydus";
        /// macOS app-menu items, in sentence case (§7.1), unlike AppKit's own 'Hide Others'.
        pub const hideOthers: &str = "Hide others";
        pub const showAll: &str = "Show all";
        /// The Window menu and its items.
        pub const window: &str = "Window";
        pub const minimize: &str = "Minimize";
        pub const zoom: &str = "Zoom";
        pub const file: &str = "File";
        pub const closeWindow: &str = "Close window";
        /// The Go menu: the five sections (tabs.*, ⌘1–⌘5), common.back (⌘[) and search.title (⌘F).
        pub const go: &str = "Go";
        /// ⌘R in the Go menu: fetches the account again (§5.5).
        pub const refreshAccount: &str = "Refresh account";
        /// "Shows {section}"
        /// Screen-reader hint on a sidebar item; {section} = its label (tabs.*).
        pub fn sidebarHint(section: &str) -> String {
            format!("Shows {}", section)
        }
        /// "{library} logo"
        /// Screen-reader label of the library's logo in the sidebar header.
        pub fn libraryLogo(library: &str) -> String {
            format!("{} logo", library)
        }
    }
}

/// A count with digit grouping ("1,234").
fn copy_number(value: i64) -> String {
    let digits = value.unsigned_abs().to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3 + 1);
    if value < 0 {
        grouped.push('-');
    }
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped
}
