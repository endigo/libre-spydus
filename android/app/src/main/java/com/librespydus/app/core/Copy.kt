// GENERATED from design/copy.json by design/scripts/gen_copy.py — do not edit.
// Every word a reader sees or hears (docs/native-apps.md §7.1). Change the JSON, then run the
// script; design/scripts/check_copy.py keeps literals out of the app.
@file:Suppress("ConstPropertyName", "PropertyName", "FunctionName", "unused", "ktlint")

package com.librespydus.app.core

import java.text.NumberFormat

object Copy {
    object Common {
        const val retry = "Retry"
        const val close = "Close"
        const val done = "Done"
        const val cancel = "Cancel"
        const val save = "Save"
        const val edit = "Edit"
        const val remove = "Remove"
        const val open = "Open"
        const val add = "Add"
        const val notNow = "Not now"
        const val clear = "Clear"
        const val back = "Back"
        /** The one title-case exception besides the names (§7.1 style). */
        const val viewAll = "View All"
        /** Screen-reader label of a spinner or skeleton with no visible text. */
        const val loading = "Loading"
        /** Screen-reader label of the checkmark on a chosen menu item (Android menus). */
        const val selected = "Selected"
        /** Android long-press action label on a row with a menu (onLongClickLabel). */
        const val moreOptions = "More options"
        /** Action label on a tappable banner or toast (Android onClickLabel). */
        const val dismiss = "Dismiss"
        /** Featured collection without sections; verification that keeps failing (Android 'Open the website'). */
        const val openWebsite = "Open website"
        /** A book's title when the library gives none (BookRef). */
        const val untitled = "Untitled"
        /** Between parts of one line: 'Due 27 Oct · 3 days left'. Join parts with this, never a literal. */
        const val separator = " · "
        /** Stand-in for {library} when the library's name is unknown. Capitalise the first letter when it starts a sentence. */
        const val yourLibrary = "your library"
    }

    object Account {
        const val signIn = "Sign in"
        const val signOut = "Sign out"
        const val signedIn = "Signed in"
        const val signingIn = "Signing in…"
    }

    object Tabs {
        const val home = "Home"
        const val yourBooks = "Your Books"
        const val search = "Search"
        /** Was 'Favourites' in both apps' tab bar. */
        const val myPicks = "My Picks"
        const val profile = "Profile"
    }

    object A11y {
        /** "Cover of {title}" */
        fun coverOf(title: String): String =
            "Cover of ${title}"
        /** "View All {section}" */
        /** Screen-reader label of a section's View All button: 'View All New arrivals'. */
        fun viewAllSection(section: String): String =
            "View All ${section}"
        /** "{label}. {hint}" */
        /** Android only. Compose has no hint, and TalkBack reads an onClickLabel as 'Double-tap to <label>', so a third-person hint ('Opens Profile') goes after the label in contentDescription, as FeaturedRow already does. Use onClickLabel / customActions only for imperative action names (Edit reservation, Remove from My Picks, More options). */
        fun labelAndHint(label: String, hint: String): String =
            "${label}. ${hint}"
        /** Appended to a reservation card's label while its cancel/edit runs. */
        const val updating = "Updating"
        /** "{status}. Refreshing" */
        /** Your Books status line while a refresh runs: 'Updated 3 min ago. Refreshing'. */
        fun refreshing(status: String): String =
            "${status}. Refreshing"
        const val dismissHint = "Dismisses this message"
    }

    object Errors {
        const val cantConnectTitle = "Can’t connect"
        const val verificationNeededTitle = "Verification needed"
        const val libraryUnavailableTitle = "Library unavailable"
        const val signedOutTitle = "Signed out"
        const val signInFailedTitle = "Sign-in failed"
        const val unexpectedTitle = "Something went wrong"
        /** "Can’t reach {library}. Check your connection." */
        /** §5.7 / §7.1 glossary. {library} = the library's name, else common.yourLibrary (was 'the library' on both). */
        fun network(library: String): String =
            "Can’t reach ${library}. Check your connection."
        /** "{library} asks you to confirm you’re human." */
        fun verification(library: String): String =
            "${library} asks you to confirm you’re human."
        const val signInAgain = "Please sign in again."
        const val credentialsRejected = "Card number or PIN not recognised."
        const val unexpected = "The library site returned something unexpected."
        /** "{library} can’t be used from the app right now. Try its website instead." */
        fun libraryUnavailable(library: String): String =
            "${library} can’t be used from the app right now. Try its website instead."
        /** The disclosure under an error that shows the core's technical text. */
        const val details = "Details"
    }

    object Home {
        /** "Hello {name}!" */
        /** {name} = the borrower's first name. */
        fun greeting(name: String): String =
            "Hello ${name}!"
        const val greetingSignedOut = "Hello!"
        /** "{library} profile" */
        /** Screen-reader label of the library logo button in Home's header. */
        fun profileButton(library: String): String =
            "${library} profile"
        const val profileButtonHint = "Opens Profile"
        /** Screen-reader label of Home's search field (a button that opens the Search tab); the visible placeholder is search.placeholder. */
        const val searchField = "Search"
        const val searchFieldHint = "Opens Search"
        /** Fallback only: the core's first home shelf title is used when present. */
        const val newArrivals = "New arrivals"
        const val nothingNewTitle = "Nothing new right now"
        const val nothingNewMessage = "Check back soon for the latest additions."
        const val popularGenres = "Popular genres"
        /** "Shows {genre} books" */
        fun genreHint(genre: String): String =
            "Shows ${genre} books"
        /** §7.1: Home's loans shelf (was 'Your borrowed books'). */
        const val yourLoans = "Your loans"
        const val noLoansTitle = "No loans right now"
        const val noLoansMessage = "Books on loan show up here with their due dates."
        /** Signed-out card in the Your loans section; its button is account.signIn. */
        const val signInCardTitle = "See your loans here"
        /** "Sign in with your {library} card." */
        fun signInCardMessage(library: String): String =
            "Sign in with your ${library} card."
    }

    object MyCard {
        const val button = "My card"
        const val buttonHint = "Shows your library card’s barcode"
        const val title = "My card"
        const val showAtDesk = "Show this at the library desk or self-checkout."
        const val editNumber = "Edit card number"
        const val numberLabel = "Library card number"
        const val numberPlaceholder = "Card number"
        const val numberNote = "Enter the number on your library card or in your library account’s Personal details (e.g. CL114D…)."
        /** Caption under the library's name on the card (Android: the card's title when the library has no name). */
        const val libraryCard = "Library card"
        /** "Loading barcode for card {number}" */
        fun loadingBarcode(number: String): String =
            "Loading barcode for card ${number}"
        /** "Barcode for card {number}" */
        /** The barcode is one screen-reader element (§5.9). */
        fun barcode(number: String): String =
            "Barcode for card ${number}"
        const val cannotEncode = "This number can’t be shown as a barcode."
        /** "Expires {date}" */
        /** {date} = cardExpires as the library writes it. */
        fun expires(date: String): String =
            "Expires ${date}"
    }

    object Featured {
        const val title = "Featured"
        /** Tile hint for .collection and .search links, unless the link has its own description. */
        const val collectionHint = "Shows this collection"
        const val websiteHint = "Opens the library’s website"
        /** Collection page without sections; its button is common.openWebsite. */
        const val websiteOnly = "This collection is on the library’s website."
        const val emptyTitle = "Nothing here right now"
        const val emptyMessage = "Check back soon, or open the collection on the library’s website."
    }

    object Search {
        const val title = "Search"
        const val placeholder = "Search by title, author, etc."
        const val clearSearch = "Clear search"
        /** Label of the sort picker (iOS Picker title). */
        const val sort = "Sort"
        const val sortResults = "Sort results"
        const val relevance = "Relevance"
        const val newest = "Newest"
        /** "Search {library}" */
        fun introTitle(library: String): String =
            "Search ${library}"
        const val introMessage = "Find books, eBooks, audiobooks and more by title, author or subject."
        const val recentSearches = "Recent searches"
        /** Hint on a recent search row; its label is the query as shown (was the label 'Search again for {query}', which Voice Control can't match to what's on screen). */
        const val recentSearchHint = "Searches again"
        /** Android onClickLabel of a recent search row ('Double-tap to search again'), used instead of recentSearchHint; its long-press action is common.remove. */
        const val searchAgain = "Search again"
        /** One: "1 result" · other: "{count} results" */
        fun results(count: Int): String =
            if (count == 1) "1 result" else "${copyNumber(count)} results"
        /** "No results for “{query}”" */
        fun noResultsTitle(query: String): String =
            "No results for “${query}”"
        const val noResultsMessage = "Check the spelling, or try fewer or different words."
        /** "Nothing in “{title}” right now" */
        /** A results list opened from a URL (Home View All, genre tile, Featured search link) that came back empty. */
        fun listEmptyTitle(title: String): String =
            "Nothing in “${title}” right now"
        const val listEmptyMessage = "Check back soon."
        const val endOfResults = "End of results"
        const val loadingResults = "Loading results"
        const val loadingMore = "Loading more results"
        const val retryLoadingMoreHint = "Loads more results"
        /** "+{count}" */
        /** Chip after the first format chips: '+2'. */
        fun moreFormats(count: Int): String =
            "+${copyNumber(count)}"
    }

    object Detail {
        /** "By {author}" */
        fun byline(author: String): String =
            "By ${author}"
        const val reserve = "Reserve"
        /** "Reserves the {format}" */
        /** Hint on the Reserve pill. {format} = the format's label, lower-cased ('book', 'large print'). An edition-level item (no label) uses reserveItemHint. */
        fun reserveFormatHint(format: String): String =
            "Reserves the ${format}"
        const val reserveItemHint = "Reserves this item"
        /** "Asks you to sign in, then reserves the {format}" */
        /** Signed out, the Reserve pill opens sign-in first. */
        fun signInToReserveFormatHint(format: String): String =
            "Asks you to sign in, then reserves the ${format}"
        const val signInToReserveItemHint = "Asks you to sign in, then reserves this item"
        const val availableOnline = "Available online"
        const val askLibrary = "Ask the library about availability"
        /** "Availability: {status}" */
        /** Screen-reader label of the availability line; {status} = the visible text (core summary label or availableOnline / askLibrary). */
        fun availability(status: String): String =
            "Availability: ${status}"
        const val availabilityFailed = "Couldn’t check availability"
        const val lookupFailed = "Couldn’t look up availability"
        const val checkWebsite = "Check availability on the library’s website"
        /** Hint on the Retry next to availabilityFailed (its label stays common.retry; was the label 'Retry: Couldn’t check availability'). retryLookupHint goes with lookupFailed. */
        const val retryAvailabilityHint = "Checks availability again"
        const val retryLookupHint = "Looks up availability again"
        const val findingFormats = "Finding formats…"
        const val checkingAvailability = "Checking availability…"
        const val noRecordLink = "This book has no link to the library’s record."
        const val formats = "Formats"
        const val formatHint = "Shows availability for this format"
        const val synopsis = "Synopsis"
        const val more = "More"
        const val less = "Less"
        /** The synopsis's More / Less button keeps its visible text as its label (was 'Show more of the synopsis' / 'Show less of the synopsis'). */
        const val moreHint = "Shows the rest of the synopsis"
        const val lessHint = "Shows less of the synopsis"
        const val noMoreDetails = "The library has no more details for this title."
        const val loadingDetails = "Loading details…"
        const val subjects = "Subjects"
        const val subjectHint = "Searches the catalogue for this subject"
        const val details = "Details"
        const val copies = "Copies"
        const val loadingCopies = "Loading copies…"
        const val copiesFailed = "Couldn’t load copies"
        const val retryCopiesHint = "Loads the copies again"
        /** A Copies row whose holding has no location or collection. */
        const val copy = "Copy"
        /** "Call number {number}" */
        fun callNumber(number: String): String =
            "Call number ${number}"
        const val similarBooks = "Similar books"
        const val moreByAuthor = "More by this author"
        const val inThisSeries = "In this series"
        const val otherEditions = "Other editions"
        /** "Loading {panel}" */
        /** Screen-reader label of a related carousel while it loads; {panel} = its title, lower-cased. */
        fun loadingPanel(panel: String): String =
            "Loading ${panel}"
        const val onLoan = "On loan"
        /** "On loan · Due {date}" */
        /** Already-yours pill (§6, I8/A6). {date} = '14 Oct' (loans.shortDate style), or the library's text when unreadable. */
        fun onLoanDue(date: String): String =
            "On loan · Due ${date}"
        const val onLoanOverdue = "On loan · Overdue"
        /** "On loan, due {date}" */
        /** Spoken: {date} written out ('14 October'). */
        fun onLoanDueLabel(date: String): String =
            "On loan, due ${date}"
        /** "On loan, overdue, was due {date}" */
        fun onLoanOverdueLabel(date: String): String =
            "On loan, overdue, was due ${date}"
        const val readyToCollect = "Ready to collect"
        /** "Ready to collect · {branch}" */
        /** §7.1: Ready to collect (+ ' at {branch}' when spoken); the visible pill joins with ' · ' (§6). */
        fun readyToCollectAt(branch: String): String =
            "Ready to collect · ${branch}"
        /** "Ready to collect at {branch}" */
        fun readyToCollectAtLabel(branch: String): String =
            "Ready to collect at ${branch}"
        const val reserved = "Reserved"
        /** "Reserved · {branch}" */
        fun reservedAt(branch: String): String =
            "Reserved · ${branch}"
        /** "Reserved, collect from {branch}" */
        /** §7.1 glossary: 'collect from' (was 'pick up at'). */
        fun reservedAtLabel(branch: String): String =
            "Reserved, collect from ${branch}"
        /** "Reserved · #{place} in queue" */
        /** Numeric queue place. */
        fun reservedQueue(place: String): String =
            "Reserved · #${place} in queue"
        /** "Reserved, number {place} in the queue" */
        fun reservedQueueLabel(place: String): String =
            "Reserved, number ${place} in the queue"
        /** "Reserved · {place} in queue" */
        /** A queue place the library writes as text ('1 of 3'). §7.1 bans 'rank' (was 'Reserved · Rank {n}'). */
        fun reservedQueueText(place: String): String =
            "Reserved · ${place} in queue"
        /** "Reserved, {place} in the queue" */
        fun reservedQueueTextLabel(place: String): String =
            "Reserved, ${place} in the queue"
        /** The already-yours pill opens Your Books on the right list. */
        const val heldHint = "Opens Your Books"
    }

    object Reserve {
        const val title = "Reserve"
        const val checkingOptions = "Checking reservation options…"
        const val cantReserve = "This title can’t be reserved right now."
        const val notFromApp = "This format can’t be reserved from the app."
        const val confirm = "Confirm reservation"
        /** §7.1 glossary: the branch row (was 'Pick up at'). */
        const val collectFrom = "Collect from"
        /** Value of the Collect from row when the library doesn't name a branch (§7.1: 'branch' for a branch; was 'Your usual library'). */
        const val usualBranch = "Your usual branch"
        const val availability = "Availability"
        const val notNeededAfter = "Not needed after"
        /** Android onClickLabel of the Not needed after dropdown. */
        const val chooseNotNeededAfter = "Choose when it’s no longer needed"
        /** Result banner when the library's own message is empty. A toast-style confirmation: no full stop (§7.1). */
        const val placed = "Reservation placed"
        /** Result banner when the library refused and gave no message. */
        const val placeFailed = "The library couldn’t place this reservation."
    }

    object Loans {
        /** "Due {date}" */
        /** {date} = '27 Oct' (day + short month; + year only when it isn't this year). Due-date notifications pass the weekday too: 'Due Thursday 16 Oct'. A date the core can't read is shown as the library wrote it. */
        fun dueOn(date: String): String =
            "Due ${date}"
        const val dueToday = "Due today"
        const val dueTomorrow = "Due tomorrow"
        /** One: "1 day left" · other: "{count} days left" */
        fun daysLeft(count: Int): String =
            if (count == 1) "1 day left" else "${copyNumber(count)} days left"
        /** One: "1 day overdue" · other: "{count} days overdue" */
        /** §7.1 glossary (Android's unused Formatting.daysLeftLabel said 'Overdue by {n} days'). */
        fun daysOverdue(count: Int): String =
            if (count == 1) "1 day overdue" else "${copyNumber(count)} days overdue"
        /** "Due {date} · {days}" */
        /** Your Books loan card meta line: 'Due 27 Oct · 3 days left' / 'Due 10 Oct · 2 days overdue'; {days} = daysLeft / daysOverdue. */
        fun lineDays(date: String, days: String): String =
            "Due ${date} · ${days}"
        /** "Due today · {date}" */
        /** Your Books loan due today: 'Due today · 12 Oct'. */
        fun lineToday(date: String): String =
            "Due today · ${date}"
        /** "Due tomorrow · {date}" */
        fun lineTomorrow(date: String): String =
            "Due tomorrow · ${date}"
        /** Chip on an overdue loan card and part of its screen-reader label. */
        const val overdue = "Overdue"
    }

    object YourBooks {
        const val title = "Your Books"
        const val loans = "Loans"
        const val reservations = "Reservations"
        /** "Loans ({count})" */
        fun loansCount(count: Int): String =
            "Loans (${copyNumber(count)})"
        /** "Reservations ({count})" */
        fun reservationsCount(count: Int): String =
            "Reservations (${copyNumber(count)})"
        /** Label of the Loans | Reservations picker (read by screen readers, not shown). */
        const val show = "Show"
        const val loadingLoans = "Loading your loans…"
        const val loadingReservations = "Loading your reservations…"
        const val noLoansTitle = "No loans right now"
        /** "Books on loan from {library} show up here with their due dates." */
        fun noLoansMessage(library: String): String =
            "Books on loan from ${library} show up here with their due dates."
        const val renewalsSoon = "Renewals coming soon"
        const val noReservationsTitle = "No reservations"
        const val noReservationsMessage = "Tap Reserve on any book and follow it here until it’s ready to collect."
        /** Section header and the ready card's status chip (§7.1; was 'Ready for pickup'). Pending cards show the library's own status, else 'Pending'. */
        const val readyToCollect = "Ready to collect"
        const val pending = "Pending"
        /** "Collect from {branch}" */
        /** Card meta line (§7.1; was 'Pick up at {branch}'). {branch} title-cased when the library shouts it. */
        fun collectFrom(branch: String): String =
            "Collect from ${branch}"
        /** "Collect from {branch} by {date}" */
        fun collectFromBy(branch: String, date: String): String =
            "Collect from ${branch} by ${date}"
        /** "Collect by {date}" */
        /** Ready card with no branch: 'Collect by 17 Oct' (was 'Pick up by …'). */
        fun collectBy(date: String): String =
            "Collect by ${date}"
        /** "#{place} in queue" */
        /** Pending card meta: '#4 in queue · Placed 2 Oct'. */
        fun queuePlace(place: String): String =
            "#${place} in queue"
        /** "{place} in queue" */
        /** A queue place the library writes as text (was 'Rank {n}'; §7.1 bans 'rank'). */
        fun queuePlaceText(place: String): String =
            "${place} in queue"
        /** "number {place} in the queue" */
        /** The card's screen-reader label says the queue place this way (§7.1 glossary); queuePlaceTextLabel for a place written as text. */
        fun queuePlaceLabel(place: String): String =
            "number ${place} in the queue"
        /** "{place} in the queue" */
        fun queuePlaceTextLabel(place: String): String =
            "${place} in the queue"
        /** "Placed {date}" */
        fun placed(date: String): String =
            "Placed ${date}"
        /** Footnote under the reservations where cards swipe (iOS 18+, Android). Hidden from screen readers: they get the actions on each card. */
        const val swipeHint = "Swipe left on a reservation to change or cancel it."
        /** The same footnote on iOS 17, where cards don't swipe (long-press menu only). */
        const val touchAndHoldHint = "Touch and hold a reservation to change or cancel it."
        /** Long-press menu item and screen-reader action (Edit / common.cancel are the swipe tiles). */
        const val editReservation = "Edit reservation"
        /** Menu item, screen-reader action and the confirmation's destructive button (iOS said 'Cancel Reservation'). */
        const val cancelReservation = "Cancel reservation"
        /** Android onLongClickLabel of a reservation card. */
        const val showActions = "Show reservation actions"
        const val signedOutTitle = "Sign in to see your books"
        /** "Your loans and reservations at {library} appear here." */
        fun signedOutMessage(library: String): String =
            "Your loans and reservations at ${library} appear here."
        /** Banner after a cancel when the library's message is empty. Toast-style: no full stop. */
        const val cancelled = "Reservation cancelled"
        /** Error banner when the library refused with no message. */
        const val cancelFailed = "The library didn’t cancel this reservation."
        const val updated = "Reservation updated"
        const val cancelTitle = "Cancel this reservation?"
        /** "“{title}” is waiting for you at {branch}. Cancelling releases it to the next person." */
        fun cancelReadyMessage(title: String, branch: String): String =
            "“${title}” is waiting for you at ${branch}. Cancelling releases it to the next person."
        /** "“{title}” is waiting for you. Cancelling releases it to the next person." */
        fun cancelReadyMessageNoBranch(title: String): String =
            "“${title}” is waiting for you. Cancelling releases it to the next person."
        /** "“{title}” will be removed from your reservations and you’ll lose your place in the queue." */
        fun cancelPendingMessage(title: String): String =
            "“${title}” will be removed from your reservations and you’ll lose your place in the queue."
        const val keepReservation = "Keep reservation"
        const val updatedJustNow = "Updated just now"
        /** One: "Updated 1 min ago" · other: "Updated {count} min ago" */
        fun updatedMinutesAgo(count: Int): String =
            if (count == 1) "Updated 1 min ago" else "Updated ${copyNumber(count)} min ago"
        /** "Updated today at {time}" */
        fun updatedToday(time: String): String =
            "Updated today at ${time}"
        /** "Updated yesterday at {time}" */
        fun updatedYesterday(time: String): String =
            "Updated yesterday at ${time}"
        /** "Updated {day} at {time}" */
        /** {day} = weekday within a week (Android) or '3 Oct' / '3 Oct 2025'; {time} in the device's 12/24-hour style. */
        fun updatedOn(day: String, time: String): String =
            "Updated ${day} at ${time}"
        /** §5.2a 'Couldn't refresh' note, with common.retry and refreshHint (was 'Couldn't refresh — showing saved data'; the same words as onboarding.savedList). */
        const val refreshFailed = "Couldn’t refresh · Showing the saved list"
        const val refreshHint = "Fetches your loans and reservations again"
    }

    object EditReservation {
        const val title = "Edit reservation"
        const val loading = "Loading reservation…"
        const val cantEditTitle = "Can’t edit this reservation"
        const val cantEditMessage = "This reservation can’t be edited right now."
        /** §7.1 glossary (was 'Pickup location'); the same words as the Reserve sheet. */
        const val collectFrom = "Collect from"
        const val chooseBranch = "Choose a branch"
        const val delayUntil = "Delay until"
        const val delayFooter = "The library won’t fill the reservation before this date."
        /** One term with the Reserve sheet's expiry picker (was 'Not required after'). */
        const val notNeededAfter = "Not needed after"
        const val notNeededFooter = "The reservation lapses if it isn’t ready by this date."
        /** "Cleared when you save. {footer}" */
        fun clearedFooter(footer: String): String =
            "Cleared when you save. ${footer}"
        const val saveChanges = "Save changes"
        const val changeFailed = "The library didn’t accept the change."
        /** iOS DatePicker title (hidden; its screen-reader label is dateLabel). */
        const val date = "Date"
        /** "{field} date" */
        /** Screen-reader label of a date row: 'Delay until date'; Android reads the date as its state. */
        fun dateLabel(field: String): String =
            "${field} date"
        const val setDate = "Set date"
        /** "Change {field} date" */
        /** Android onClickLabel of a set date; setFieldDate when the date is off; clearField on its clear button. {field} is the row's title lower-cased mid-sentence: 'Change delay until date', 'Clear not needed after'. */
        fun changeDate(field: String): String =
            "Change ${field} date"
        /** "Set {field} date" */
        fun setFieldDate(field: String): String =
            "Set ${field} date"
        /** "Clear {field}" */
        fun clearField(field: String): String =
            "Clear ${field}"
    }

    object Picks {
        /** §7.1: the tab, the screen title (signed in and out), the bookmark and the toasts. Never Favourites. */
        const val title = "My Picks"
        const val subtitleAccount = "Saved on your library account"
        /** Signed out (was '{n} saved for {library}' / 'Saved on this device for {library}'). */
        const val subtitlePhone = "Saved on this phone"
        /** Bookmark button's screen-reader label (book page and anywhere else a toggle appears), signed in or out (was 'Add to favourites'). */
        const val save = "Save to My Picks"
        /** Bookmark label when saved; also the row's remove action / menu item (was 'Remove from Favourites'). */
        const val remove = "Remove from My Picks"
        /** Toast after a save (§5.11). */
        const val saved = "Saved to My Picks"
        const val removed = "Removed from My Picks"
        /** Error toast (§5.11 had a full stop; §7.1: never on toasts). */
        const val updateFailed = "Couldn’t update My Picks"
        const val emptyTitle = "No picks yet"
        const val emptyAccountMessage = "Tap the bookmark on a book to save it to your library account."
        /** Signed out (was 'Tap the bookmark on any book to save it here.'); its button is findABook. */
        const val emptyPhoneMessage = "Tap the bookmark on a book to save it on this phone."
        const val findABook = "Find a book"
        /** Hint on the signed-out list's Edit button (its label is the visible common.edit; was the label 'Edit favourites'); doneEditingHint goes with common.done. */
        const val editHint = "Lets you remove books from My Picks"
        const val doneEditingHint = "Finishes editing My Picks"
        /** "Remove {title} from My Picks" */
        /** Screen-reader label of a row's remove button in edit mode. */
        fun removeTitle(title: String): String =
            "Remove ${title} from My Picks"
        /** "Removed {title} from My Picks" */
        /** Screen-reader announcement after a remove on the signed-out list. */
        fun removedTitle(title: String): String =
            "Removed ${title} from My Picks"
        const val loadingMore = "Loading more picks"
        const val retryLoadingMoreHint = "Loads more picks"
        /** iOS banner when the phone's list can't be read (was 'Couldn't read favourites: <system error>'). */
        const val loadFailed = "Couldn’t load My Picks from this phone."
        const val saveFailed = "Couldn’t save My Picks on this phone."
        /** §5.11 copy-once alert; buttons common.add / common.notNow. */
        const val offerTitle = "Add this phone’s picks to your library account?"
        /** One: "1 book saved on this phone can be added to My Picks on your library account." · other: "{count} books saved on this phone can be added to My Picks on your library account." */
        fun offerMessage(count: Int): String =
            if (count == 1) "1 book saved on this phone can be added to My Picks on your library account." else "${copyNumber(count)} books saved on this phone can be added to My Picks on your library account."
        /** One: "Adding 1 book to My Picks…" · other: "Adding {count} books to My Picks…" */
        /** Toast while the copy runs (iOS has it; Android should gain it). */
        fun adding(count: Int): String =
            if (count == 1) "Adding 1 book to My Picks…" else "Adding ${copyNumber(count)} books to My Picks…"
        /** One: "Added 1 book to My Picks" · other: "Added {count} books to My Picks" */
        /** Toast when the copy ends; when some failed, join notAdded with common.separator: 'Added 3 books to My Picks · 1 couldn’t be added'. */
        fun added(count: Int): String =
            if (count == 1) "Added 1 book to My Picks" else "Added ${copyNumber(count)} books to My Picks"
        /** One: "1 couldn’t be added" · other: "{count} couldn’t be added" */
        fun notAdded(count: Int): String =
            if (count == 1) "1 couldn’t be added" else "${copyNumber(count)} couldn’t be added"
    }

    object Profile {
        const val title = "Profile"
        const val noLibraryTitle = "No library selected"
        const val noLibraryMessage = "Choose your library to search its catalogue and sign in."
        const val chooseLibrary = "Choose library"
        const val changeLibrary = "Change library"
        /** Section header; the signed-in row says account.signedIn and its button account.signOut. */
        const val account = "Account"
        const val loadingAccount = "Loading your account…"
        /** "Signed in to {library}" */
        fun signedInTo(library: String): String =
            "Signed in to ${library}"
        /** Inline status line next to common.retry, like detail.copiesFailed: no full stop. */
        const val accountFailed = "Couldn’t load your account details"
        /** "Sign out of {library}?" */
        /** Confirmation; its destructive button is account.signOut, the other common.cancel. */
        fun signOutTitle(library: String): String =
            "Sign out of ${library}?"
        const val signOutMessage = "You can sign in again at any time."
        const val notSignedInTitle = "You’re not signed in"
        const val notSignedInMessage = "Sign in with your library card to see your loans and reservations."
        const val humanVerification = "Human verification"
        const val verified = "Verified on this device"
        /** "{library} asked you to confirm you’re human. The app sends the library’s verification cookie with its requests." */
        fun verifiedMessage(library: String): String =
            "${library} asked you to confirm you’re human. The app sends the library’s verification cookie with its requests."
        const val clearVerification = "Clear verification"
        /** Confirmation; its destructive button is clearVerification, the other common.cancel. */
        const val clearVerificationTitle = "Clear verification?"
        /** "{library} will ask you to confirm you’re human again." */
        fun clearVerificationMessage(library: String): String =
            "${library} will ask you to confirm you’re human again."
        const val about = "About"
        const val appName = "LibreSpydus"
        /** "Version {version} ({build})" */
        fun version(version: String, build: String): String =
            "Version ${version} (${build})"
        const val notAffiliated = "LibreSpydus is not affiliated with Civica or your library."
    }

    object Notifications {
        /** Profile section while signed in (§5.8). */
        const val section = "Notifications"
        const val readyTitle = "Ready to collect"
        const val readySubtitle = "Checked each morning and when you open the app"
        const val dueTitle = "Due-date reminders"
        const val dueSubtitle = "2 days before books are due, at 7:45"
        const val blocked = "Notifications are turned off for LibreSpydus."
        /** 'Settings' is the system app's name, so it keeps its capital. */
        const val openSettings = "Open Settings"
    }

    object Alerts {
        /** Local notification (§5.8); the body is readyBody, or just the title when the branch is unknown. */
        const val readyTitle = "Ready to collect"
        /** "{title} · {branch}" */
        fun readyBody(title: String, branch: String): String =
            "${title} · ${branch}"
        /** Stands in for a missing title in readyBody. */
        const val untitledReservation = "A reservation"
        const val untitledLoan = "A loan"
        /** "{first} and {second}" */
        /** Due-reminder body for two loans; one loan is just its title; three or more use dueMore. The title is loans.dueOn / dueToday / dueTomorrow with the weekday date ('Due Thursday 16 Oct'). */
        fun dueTwo(first: String, second: String): String =
            "${first} and ${second}"
        /** One: "{first} and 1 more" · other: "{first} and {count} more" */
        fun dueMore(count: Int, first: String): String =
            if (count == 1) "${first} and 1 more" else "${first} and ${copyNumber(count)} more"
        /** Android notification channel names. */
        const val readyChannel = "Ready to collect"
        const val dueChannel = "Due-date reminders"
    }

    object Onboarding {
        const val title = "Choose your library"
        const val intro = "LibreSpydus works with libraries that use Spydus. You can change this later in Profile."
        /** The picker's title when opened from Profile (profile.changeLibrary is the row). */
        const val changeTitle = "Change library"
        const val searchPlaceholder = "Search by library or country"
        const val loading = "Loading libraries…"
        const val emptyTitle = "No libraries listed"
        const val emptyMessage = "Civica’s directory came back empty. Try again later."
        /** "No libraries match “{query}”" */
        fun noMatchesTitle(query: String): String =
            "No libraries match “${query}”"
        const val noMatchesMessage = "Try a library name or a country."
        /** "{error} Showing the saved list." */
        /** Banner when a refresh failed but a saved list is shown; {error} = directoryNetwork / directoryUnexpected. */
        fun savedList(error: String): String =
            "${error} Showing the saved list."
        const val directoryNetwork = "Can’t reach the library directory. Check your connection."
        const val directoryUnexpected = "The library directory returned something unexpected."
        /** One: "1 library" · other: "{count} libraries" */
        /** Screen-reader label of a region's count (iOS said '1 libraries'). */
        fun libraries(count: Int): String =
            if (count == 1) "1 library" else "${copyNumber(count)} libraries"
        /** Screen-reader value of the selected row (and Android's checkmark). */
        const val currentLibrary = "Current library"
        /** Hint on the other rows (iOS said 'Select this library'; Android's onClickLabel 'Select this library' / 'Keep this library'). */
        const val selectHint = "Selects this library"
        /** Section titles, and the Profile place chip when the directory has no country. */
        const val regionAU = "Australia"
        const val regionNZ = "New Zealand"
        const val regionUS = "United States"
        const val regionUK = "United Kingdom"
        const val regionSG = "Singapore"
        const val regionOther = "Other"
        /** Row notes (§6). */
        const val noteUK = "Asks you to verify you’re human"
        const val noteSG = "May be unreachable outside school networks"
    }

    object Login {
        /** The sheet's title; its button is account.signIn (account.signingIn while it runs) and its close button common.close. */
        const val title = "Sign in"
        const val cardLabel = "Library card number"
        const val cardPlaceholder = "Card number"
        const val pinLabel = "PIN / password"
        const val pinPlaceholder = "PIN or password"
        /** "LibreSpydus signs in directly with {library} and doesn’t store your PIN." */
        fun privacyNote(library: String): String =
            "LibreSpydus signs in directly with ${library} and doesn’t store your PIN."
    }

    object Verification {
        const val title = "Verify you’re human"
        /** "{library} asks you to confirm you’re human. Complete the check below." */
        /** §6 verification sheet copy. */
        fun message(library: String): String =
            "${library} asks you to confirm you’re human. Complete the check below."
        const val closesByItself = "This closes by itself once the library lets you in."
        /** "Opening {library}…" */
        fun opening(library: String): String =
            "Opening ${library}…"
        /** "{library} didn’t ask for a check this time. Close this and try again." */
        fun noCheck(library: String): String =
            "${library} didn’t ask for a check this time. Close this and try again."
        const val reload = "Reload page"
        /** "{library} keeps asking to confirm you’re human after the check. Its own website still works in your browser." */
        /** Android shows it with common.openWebsite and common.close when verification keeps failing (iOS shows errors.libraryUnavailable instead). */
        fun keepsAsking(library: String): String =
            "${library} keeps asking to confirm you’re human after the check. Its own website still works in your browser."
    }
}

/** A count as the reader's locale writes it ("1,234"). */
private fun copyNumber(value: Int): String = NumberFormat.getIntegerInstance().format(value)
