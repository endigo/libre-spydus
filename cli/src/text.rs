//! The words only the CLI says (docs/cli.md §1): command help, prompts, column headings and
//! hints. Everything the apps also say comes from `Copy` instead. Terms follow
//! docs/native-apps.md §7.1: Your Books, loans, reservations, My Picks, Reserve, Renew, Cancel
//! reservation, library card, Ready to collect, pickup branch — never "borrowed", "holds",
//! "checkouts", "favourites" or "patron".

use crate::copy::Copy;

// ---- The program ----

pub const ABOUT: &str = "LibreSpydus for the terminal: search a Spydus library’s catalogue, look at a book, sign in to see Your Books, renew, reserve and cancel, and keep My Picks.";
pub const AFTER_HELP: &str = "Lists print #1, #2, … and later commands take those numbers: show, reserve and picks add from the last search (or picks) list; renew, cancel and edit from a fresh look at the account.\n\nExit codes: 0 done · 1 the library, network or account failed · 2 usage · 3 sign in needed · 4 confirm you’re human (spydus verify).";

// ---- Global flags ----

pub const FLAG_LIBRARY: &str =
    "The library for this run only (a name or an origin); otherwise the saved one (spydus use)";
pub const FLAG_JSON: &str = "Machine output: one JSON document on stdout";
pub const FLAG_DEMO: &str = "The demo account (Test Borrower) at the chosen library; saves nothing";
pub const FLAG_DATA_DIR: &str = "Keep config, lists and secrets in this folder instead of the user’s config folder and keychain. Secrets go in <dir>/secrets.json in plain text: for tests only";
pub const FLAG_NO_COLOR: &str = "Plain output (so does NO_COLOR, or stdout not being a terminal)";

// ---- Commands ----

pub const CMD_LIBRARIES: &str = "The Spydus directory: name, region, origin; filtered by words";
pub const CMD_LIBRARIES_ALL: &str = "Also list Civica’s test, training and demo instances";
pub const CMD_USE: &str = "Choose the library (saved) by name or origin";
pub const CMD_SEARCH: &str = "Search the catalogue: numbered results with title, author, year and formats";
pub const CMD_SEARCH_NEWEST: &str = "Newest additions first instead of relevance";
pub const CMD_SEARCH_PAGE: &str = "Which page of results (1 is the first)";
pub const CMD_SHOW: &str = "The book page: formats and availability, synopsis, details";
pub const CMD_SHOW_HOLDINGS: &str = "Also list the branches holding each format";
pub const CMD_LOGIN: &str = "Sign in to the chosen library with your library card";
pub const CMD_LOGIN_CARD: &str = "Username or library card number (asked for when missing)";
pub const CMD_LOGIN_PASSWORD_STDIN: &str =
    "Read the PIN or password from the first line of stdin instead of a no-echo prompt";
pub const CMD_LOGOUT: &str = "Forget this library’s session";
pub const CMD_PROFILE: &str = "Name, library, card number, and counts of loans, reservations and ready to collect";
pub const CMD_BOOKS: &str = "Your Books: loans, then reservations";
pub const CMD_LOANS: &str = "Numbered loans: title, author, due line, and whether each can be renewed";
pub const CMD_RESERVATIONS: &str = "Numbered reservations: title, status, and where to collect";
pub const CMD_RENEW: &str = "Renew loans by number and print each result";
pub const CMD_RENEW_NUMBERS: &str = "Loan numbers, as spydus loans prints them";
pub const CMD_RENEW_ALL: &str = "Every loan the library lets you renew";
pub const CMD_RESERVE: &str = "Reserve a book from the last list (by number) or a format’s reserve link";
pub const CMD_RESERVE_FORMAT: &str = "Which format to reserve when the book has several (its name, e.g. Book)";
pub const CMD_RESERVE_PICKUP: &str = "Pickup branch, when the library lets you choose";
pub const CMD_CANCEL: &str = "Cancel reservations by number";
pub const CMD_CANCEL_NUMBERS: &str = "Reservation numbers, as spydus reservations prints them";
pub const CMD_EDIT: &str =
    "Edit a reservation: its pickup branch and dates. Without a change, prints what can be changed";
pub const CMD_EDIT_NUMBER: &str = "The reservation’s number, as spydus reservations prints it";
pub const CMD_EDIT_PICKUP: &str = "Collect from this branch (its name or code)";
pub const CMD_EDIT_DELAY: &str = "Delay until this date (YYYY-MM-DD)";
pub const CMD_EDIT_NOT_NEEDED: &str = "Not needed after this date (YYYY-MM-DD)";
pub const CMD_EDIT_CLEAR_DELAY: &str = "Remove the delay";
pub const CMD_EDIT_CLEAR_NOT_NEEDED: &str = "Remove the not-needed-after date";
pub const CMD_PICKS: &str = "My Picks: on your library account when signed in, else saved on this computer";
pub const CMD_PICKS_ADD: &str = "Save a book to My Picks, by number from the last search list or by URL";
pub const CMD_PICKS_REMOVE: &str = "Remove a book from My Picks, by number from the picks list or by URL";
pub const CMD_CARD: &str = "Your library card number";
pub const CMD_VERIFY: &str = "Confirm you’re human for a library that asks (UK libraries): open its website in a browser, then paste the cookie here";
pub const CMD_VERIFY_CLEAR: &str = "Forget this library’s verification";
pub const CMD_COMPLETIONS: &str = "Shell completions";
pub const CMD_YES: &str = "Don’t ask first";
pub const ARG_NUMBER_OR_URL: &str = "A number from the last list (#3 or 3), or a URL";
pub const ARG_WORDS: &str = "Words a library’s name, country or address must contain";
pub const ARG_LIBRARY: &str = "A library’s name (or part of it), or its origin such as https://canning.spydus.com";
pub const ARG_QUERY: &str = "What to search for";

// ---- Column headings ----

pub const COL_NAME: &str = "Name";
pub const COL_REGION: &str = "Region";
pub const COL_ORIGIN: &str = "Origin";
pub const COL_TITLE: &str = "Title";
pub const COL_AUTHOR: &str = "Author";
pub const COL_YEAR: &str = "Year";
pub const COL_FORMATS: &str = "Formats";
pub const COL_DUE: &str = "Due";
pub const COL_STATUS: &str = "Status";
pub const COL_BRANCH: &str = "Branch";
pub const COL_COLLECTION: &str = "Collection";
pub const COL_CALL_NUMBER: &str = "Call number";
pub const COL_CODE: &str = "Code";

// ---- Lines ----

/// "Library  City of Canning · https://canning.spydus.com"
pub fn library_line(name: &str, origin: &str) -> String {
    format!("Library  {name} · {origin}")
}

pub fn libraries_count(count: usize) -> String {
    if count == 1 { "1 library".to_owned() } else { format!("{count} libraries") }
}

pub fn test_instance() -> &'static str {
    "test instance"
}

pub fn next_page(command: &str) -> String {
    format!("Next page: {command}")
}

pub fn page_of(page: usize) -> String {
    format!("Page {page}")
}

pub fn signed_in_as(name: &str) -> String {
    format!("Signed in as {name}")
}

pub fn signed_out_of(library: &str) -> String {
    format!("Signed out of {library}")
}

pub const DEMO_NOTE: &str = "Demo account · nothing is saved";
pub const NOTHING_CHANGED: &str = "Nothing changed.";
pub const CARD_NUMBER: &str = "Card number";
pub const ACCOUNT_NAME: &str = "Name";
pub const PICKUP_CHOICES: &str = "Branches";
pub const CURRENT: &str = "current";
pub const RENEWABLE: &str = "Renew";
pub const NOT_RENEWABLE: &str = "–";
pub const NO_FORMATS: &str = "No formats listed";
pub const NO_HOLDINGS: &str = "No copies listed";
pub const VERIFIED: &str = "Verification saved for this library.";
pub const VERIFICATION_CLEARED: &str = "Verification cleared.";

pub fn reserve_summary(title: &str, format: &str) -> String {
    format!("Reserve “{title}” · {format}")
}

pub fn reserve_summary_no_format(title: &str) -> String {
    format!("Reserve “{title}”")
}

pub fn edit_summary(title: &str) -> String {
    format!("Edit the reservation for “{title}”")
}

pub fn changes_line(changes: &[String]) -> String {
    changes.join(" · ")
}

pub fn result_line(number: usize, title: &str, result: &str) -> String {
    format!("#{number} {title}: {result}")
}

// ---- Prompts ----

pub const PROMPT_YES_NO: &str = "[y/N] ";
pub const PROMPT_CHOOSE: &str = "Number: ";
pub const PROMPT_COOKIE: &str = "aws-waf-token cookie value: ";

pub fn prompt(label: &str) -> String {
    format!("{label}: ")
}

pub fn verify_steps(library: &str, address: &str) -> String {
    format!(
        "{library} asks you to confirm you’re human.\n1. Open {address} in a browser and complete the check.\n2. In the browser’s developer tools, copy the value of the aws-waf-token cookie for that site.\n3. Paste it below. It is kept for this library only and never printed."
    )
}

// ---- Hints and refusals ----

pub const HINT_CHOOSE_LIBRARY: &str = "Choose one with spydus use <name>, or look with spydus libraries.";
pub const HINT_SIGN_IN: &str = "Sign in with spydus login.";
pub const HINT_VERIFY: &str = "Complete the check with spydus verify.";
pub const HINT_SEARCH_FIRST: &str = "Run spydus search first.";
pub const HINT_PICKS_FIRST: &str = "Run spydus picks first.";
pub const HINT_YES: &str = "Not a terminal: add --yes to confirm.";
pub const HINT_PAGE: &str = "There is no such page.";

pub fn several_libraries(count: usize) -> String {
    format!("{count} libraries match. Narrow it down with more words, or give the origin.")
}

pub fn no_library_matches(query: &str) -> String {
    format!("No library matches “{query}”.")
}

pub fn list_for_other_library(library: &str) -> String {
    format!("That number is from a list for {library}. Search again at this library.")
}

pub fn list_stale(query: &str) -> String {
    format!("The list for “{query}” is more than a day old. Run that search again.")
}

pub fn no_such_number(number: usize, count: usize) -> String {
    if count == 1 {
        format!("There is no #{number}: the list has 1 entry.")
    } else {
        format!("There is no #{number}: the list has {count} entries.")
    }
}

pub fn not_a_number_or_url(argument: &str) -> String {
    format!("“{argument}” is not a number from a list or a URL.")
}

pub fn not_a_number(argument: &str) -> String {
    format!("“{argument}” is not a number from a list.")
}

pub fn not_a_page(argument: &str) -> String {
    format!("“{argument}” is not a page number.")
}

pub fn not_renewable(number: usize, title: &str) -> String {
    format!("#{number} “{title}” can’t be renewed: the library offers no renewal for it.")
}

pub fn no_cancel_link(number: usize, title: &str) -> String {
    format!("#{number} “{title}” can’t be cancelled from here.")
}

pub fn no_edit_link(number: usize, title: &str) -> String {
    format!("#{number} “{title}” can’t be edited from here.")
}

pub fn no_reserve_link(title: &str) -> String {
    format!("“{title}” has no format that can be reserved from here.")
}

pub fn choose_format(choices: &[String]) -> String {
    format!("Which format? Add --format <name>: {}", choices.join(", "))
}

pub fn unknown_format(name: &str, choices: &[String]) -> String {
    format!("No format called “{name}”. Choose from: {}", choices.join(", "))
}

pub fn unknown_branch(name: &str, choices: &[String]) -> String {
    format!("No branch called “{name}”. Choose from: {}", choices.join(", "))
}

pub fn not_a_date(argument: &str) -> String {
    format!("“{argument}” is not a date (use YYYY-MM-DD).")
}

pub fn no_irn(title: &str) -> String {
    format!("“{title}” has no record number to save.")
}

pub fn nothing_to_renew() -> &'static str {
    "No loan can be renewed right now."
}

pub fn couldnt_write(path: &str, error: &str) -> String {
    format!("Couldn’t write {path}: {error}")
}

pub fn couldnt_read(path: &str, error: &str) -> String {
    format!("Couldn’t read {path}: {error}")
}

pub fn keychain_failed(error: &str) -> String {
    format!("Couldn’t use the keychain: {error}")
}

pub fn no_input() -> &'static str {
    "Nothing was entered."
}

// ---- Added with the commands ----

pub fn renew_count(count: usize) -> String {
    if count == 1 { Copy::YourBooks::renewTitle.to_owned() } else { format!("Renew {count} loans?") }
}

pub fn cancel_count(count: usize) -> String {
    if count == 1 { Copy::YourBooks::cancelTitle.to_owned() } else { format!("Cancel {count} reservations?") }
}

/// "#3 The long night · Due 14 Oct" in a confirmation that lists several items.
pub fn item_line(number: usize, title: &str, detail: Option<&str>) -> String {
    match detail {
        Some(detail) => format!("#{number} {title}{}{detail}", Copy::Common::separator),
        None => format!("#{number} {title}"),
    }
}

pub const NO_CARD_NUMBER: &str = "No card number is listed on this account.";
pub const NO_PICKUP: &str = "–";

pub fn pickup_choice(name: &str, code: &str, current: bool) -> String {
    if current { format!("{name} ({code}, {CURRENT})") } else { format!("{name} ({code})") }
}

pub fn count_line(label: &str, count: usize) -> String {
    format!("{label}  {count}")
}

pub fn pick_count(count: i64) -> String {
    if count == 1 { "1 pick".to_owned() } else { format!("{count} picks") }
}

pub fn nothing_to_do() -> &'static str {
    "Nothing to do."
}

pub fn page_too_far(max: usize) -> String {
    format!(
        "Only the first {max} pages can be reached from here (the library gives one next-page link at a time). Narrow the search instead."
    )
}

/// The last list has no entries at all.
pub fn list_empty(query: &str) -> String {
    format!("The last list (“{query}”) is empty. {HINT_SEARCH_FIRST}")
}

/// The last list's numbers run from `first` to `last` (a later page starts past #1).
pub fn no_such_number_in(number: usize, first: usize, last: usize) -> String {
    if first == last {
        format!("There is no #{number}: the list has only #{first}.")
    } else {
        format!("There is no #{number}: the list runs from #{first} to #{last}.")
    }
}
