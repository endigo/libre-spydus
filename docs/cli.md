# LibreSpydus CLI (`spydus`): build contract

A command-line client for a Spydus library, on the shared Rust core (`core/`), which it calls
directly as the desktop app does. It's for people who live in a terminal, and for scripts:
- pick a library, then search its catalogue and look at a book;
- sign in, see Your Books, and renew, reserve or cancel;
- keep My Picks.

It says what the apps say (`docs/native-apps.md` §7.1) wherever the concept is the same.

`core/src/bin/spydus-cli.rs` stays as it is. It's a developer tool that prints a public page's JSON
for `scripts/library-survey.py`. This contract is about the user's CLI.

## 1. Decisions
- **Crate:** `cli/`, a standalone Cargo package `librespydus-cli` (edition 2024, `publish = false`).
  - The binary is `spydus`.
  - It has its own committed `Cargo.lock`, and `cli/target/` is gitignored.
  - It is not a workspace with `core/` or `desktop/`, so their builds stay untouched.
  - It must not depend on `librespydus-desktop`, which pulls in GPUI. Port the few pure helpers it
    needs (formatting, error wording, the demo account) and say in each file where it came from.
- **Dependencies:**
  - core and runtime: `librespydus-core` (path `../core`), `tokio` (rt-multi-thread, macros, time);
  - command line: `clap` 4 (derive; `clap_complete` for `spydus completions`), `rpassword` (no-echo
    password prompt);
  - data: `serde` and `serde_json`, `chrono`, `sha2`;
  - storage: `keyring` 3 (`apple-native`, `windows-native`, `linux-native-sync-persistent`), `dirs`;
  - errors: `anyhow`;
  - output: `anstream` and `anstyle` for colour, honouring `NO_COLOR` and a non-TTY stdout;
  - tests: `assert_cmd`, `predicates`, `tempfile`.
  - Nothing heavier: no TUI framework.
- **Platforms:** macOS, Linux and Windows. It is built and checked on macOS.
- **Words.** `design/scripts/gen_copy.py` writes its Rust module to `cli/src/copy.rs` as well as
  `desktop/src/copy.rs`; the two files are identical (desktop forms of platform variants: "Saved on
  this computer"), and `gen_copy.py --check` covers both.
  - Everything the apps also say comes from `Copy::…`: status and due lines, errors, empty states,
    section names, confirmations and results.
  - CLI-only words go in one module, `cli/src/text.rs`, using the glossary's terms. That means
    command help, prompts and column headings.
  - The terms are: Your Books, loans, reservations, My Picks, Reserve, Renew, Cancel reservation,
    library card, Ready to collect, pickup branch.
  - Never "borrowed", "holds", "checkouts", "favourites" or "patron".

## 2. Commands
Global flags, on every command:

| Flag | Meaning |
|---|---|
| `--library <name or origin>` | this run only; otherwise the saved library (`spydus use`) |
| `--json` | machine output (§4) |
| `--demo` | demo account (§6); saves nothing |
| `--data-dir <dir>` | config, lists and secrets in this folder instead of the user's config folder and keychain; for tests and workers (§5) |
| `--no-color` | plain output (so does `NO_COLOR`, or stdout not being a terminal) |

| Command | Does |
|---|---|
| `spydus libraries [<words>] [--all]` | the Spydus directory: name, region, origin; filtered by words; `--all` adds test instances |
| `spydus use <name or origin>` | choose the library (saved). One match is chosen; several are listed for the user to narrow down |
| `spydus search <query…> [--newest] [--page <n>]` | numbered results: title, author, year and formats, plus the result count and the next page's command. *Note (C1):* Spydus gives one next-page link at a time, so `--page <n>` follows n−1 links; pages beyond 10 are refused before any request |
| `spydus show <# or url>` | the book page: title, author, year; each format with its availability; synopsis; details; the branches holding it (`--holdings`); and, when signed in, the held status ("On loan", "Reserved"). *Note (C1):* `--holdings` lists copies only where the library's availability fragment links to its copies list (an XHLD link: edition-level cards and some formats); for a format without one it says "No copies listed", as the apps show no copies there either |
| `spydus login [--card <number>] [--password-stdin]` | sign in to the chosen library. It prompts for the card number if missing and for the password without echo, then prints who's signed in |
| `spydus logout` | forget this library's session |
| `spydus profile` | name, library, card number, and counts of loans, reservations and ready to collect |
| `spydus books` | Your Books: loans then reservations (what `loans` and `reservations` print, under their headings) |
| `spydus loans` | numbered loans: title, author, due line ("Due 17 Oct · 2 days left", "Overdue"), and whether it can be renewed |
| `spydus reservations` | numbered reservations: title, status line ("Ready to collect", "In transit", queue place), and pickup branch |
| `spydus renew <#…> \| --all [--yes]` | renew loans and print each result (new due date, or why not); asks first on a terminal unless `--yes` |
| `spydus reserve <# or url> [--format <name>] [--pickup <branch>] [--yes]` | Reserve. With several formats or branches and no flag, it asks on a terminal and otherwise fails, listing the choices |
| `spydus cancel <#…> [--yes]` | Cancel reservation, asking first |
| `spydus edit <#> [--pickup <branch>] [...]` | change a reservation (what `edit_options` offers: pickup branch, and any other field the core exposes) |
| `spydus picks [add <# or url> \| remove <# or url>]` | My Picks: on the account when signed in; otherwise "Saved on this computer" in the data folder |
| `spydus card` | the library card number |
| `spydus verify [--clear]` | UK libraries' "confirm you're human": print the library's address to open in a browser, then read the `aws-waf-token` cookie's value from a prompt and keep it for this library. Never automates, solves or bypasses the check |
| `spydus completions <shell>` | shell completions |

**Numbers.** Lists print `#1`, `#2`, … and later commands take those numbers:
- **`show`, `reserve` and `picks add`** take the number from the last `search` (or `picks`) list,
  which is saved with its library and query.
- **`renew`, `cancel` and `edit`** fetch the account fresh and number it the way `loans` and
  `reservations` print it (the apps' order), so `spydus renew 2` works without running
  `spydus loans` first.

A number from a list for another library or a stale query is refused with a hint. A URL always works.

**Without a library.** A command that needs one says how to choose: `spydus use <name>`, or
`spydus libraries` to look.

**Not signed in.** An account command says how to sign in: `spydus login`. An expired session gets
the apps' wording plus that hint.

## 3. Output (humans)
- **Columns.** Aligned, and wrapped to the terminal's width. Titles keep their words; authors are
  shortened as the apps shorten them.
- **Colour** only on a terminal: overdue in the error colour, ready to collect in the success
  colour, headings bold. It must stay readable with colour off.
- **Dates and due lines** are formatted as the apps format them. Port the desktop app's
  `state/formatting.rs`.
- **Library and status.** One line at the top of account output: the library, who's signed in, and
  when it was fetched.
- **Empty states** use the apps' words ("No loans", and so on, from `Copy`).

## 4. Output (`--json`)
- **Data.** stdout gets one JSON document: the core's types, as serde serialises them (camelCase,
  as `spydus-cli` prints), with the list number added where the human output numbers rows.
- **Errors.** On failure, stderr gets `{"error": {"kind": "<SpydusError variant>", "message":
  "<the apps' wording>"}}`, with a non-zero exit code.
- **Stability.** Keys are stable; adding keys is fine, but renaming one is a breaking change.

## 5. Where things are kept
- **Config:** `dirs::config_dir()/LibreSpydus/cli.json` holds the library (origin and name).
- **Lists:** `dirs::cache_dir()/LibreSpydus/cli/` holds the last `search` and `picks` lists, and My
  Picks on the computer when signed out (one file per library, named by the SHA-256 of its origin).
- **Secrets:** the session and the verification cookie go in the OS keychain through `keyring`,
  with service `com.librespydus.cli.session` or `com.librespydus.cli.verification` and account =
  origin.
  - They are never printed, logged, or written to a file.
  - The password is never stored. The CLI has no sign-back-in: run `spydus login` again.
- **`--data-dir <dir>`** puts config and lists under `<dir>`. Secrets go in `<dir>/secrets.json`
  (plain text, for tests only, and said so in `--help`). It never touches the keychain.

## 6. Demo mode (`--demo`)
- **Account.** You are signed in as "Test Borrower" at the chosen library (Canning by default).
  Loans, reservations, edit and reserve options, and My Picks come from the iOS demo fixtures
  `ios/LibreSpydus/Resources/DemoData/*.json`, compiled in with `include_str!` (one copy, as the
  desktop app does).
- **Changes.** Renew, reserve, cancel, edit and picks succeed after 300 ms, following the demo's
  rules. Port them from the desktop app's `backend/demo.rs`. They last for the one run.
- **Public reads** (`libraries`, `search`, `show`) go to the live library, as the apps' demo mode
  does.
- **Nothing is saved:** no config, no lists and no secrets.
  - *Deviation (C1):* the demo keeps its own last list (public search results or the demo's
    picks, under `lists/demo/`) so `spydus --demo show 1` and `--demo reserve 1` work across runs,
    which numbering needs; it is separate from the reader's list. Still no config, no secrets and
    no local picks. `spydus --demo use <name>` prints the match and saves nothing.

## 7. Errors and exit codes
`0` means success. `1` means the library, network or account failed, with the apps' wording (port
the desktop app's `state/error.rs`). `2` means a usage error (clap's). `3` means sign-in is needed
or the session expired. `4` means a "confirm you're human" check is needed (see `spydus verify`).

## 8. Checks
- **Build and lint.** In `cli/`, `cargo test` and `cargo clippy --all-targets -- -D warnings` pass,
  with `cargo fmt` applied. Never run rustfmt on `core/`.
- **No network in tests.** Account commands run with `--demo --data-dir <tmp>`. Public reads use a
  fake backend behind the same trait the demo uses. One `#[ignore]`d test may search Canning live.
- **Unit tests:**
  - argument parsing;
  - numbering, including that stale and other-library numbers are refused;
  - golden human output for `loans`, `reservations`, `books`, `profile` and `search` (demo or fake
    data, colour off, fixed width);
  - the JSON shapes;
  - mapping each error to its message and exit code.
- **Words.** `python3 design/scripts/gen_copy.py --check` and `python3 design/scripts/check_copy.py`
  pass.
- **Evidence** goes in `/private/tmp/librespydus-evidence/C1/`:
  - a transcript of every command in demo mode;
  - `libraries`, `search` and `show` against Canning, live (public reads only);
  - `--json` samples;
  - the error cases.

## 9. Safety (as `docs/native-apps.md` §2)
- **Account commands:** workers and agents use `--demo` only. They never sign in with real
  credentials and never send a POST to a library.
- **Real libraries:** public GETs only. Never automate, solve or bypass a "confirm you're human"
  check.
- **Data:** always pass `--data-dir` under `/private/tmp/librespydus-evidence/<task>/`. Never
  create, read or delete the user's keychain items or config.
- **Who tests for real:** the user does the first real sign-in, renew, reserve and cancel.

## 10. Docs
- `cli/README.md`: install (`cargo install --path cli`), a tour with the demo, every command with an
  example, where things are kept, and exit codes.
- A line about the CLI in the top-level `README.md`.
- The §7.1 glossary already covers the words.
