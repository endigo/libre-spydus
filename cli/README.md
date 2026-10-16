# spydus — LibreSpydus for the terminal

`spydus` is a command-line client for a Spydus library: search the catalogue, look at a book,
sign in with your library card to see Your Books, renew, reserve and cancel, and keep My Picks.
It calls the shared Rust core (`core/`) directly, as the desktop app does, and says the same
things the apps say. The build contract is [docs/cli.md](../docs/cli.md).

## Install

```sh
cargo install --path cli          # → ~/.cargo/bin/spydus
# or
cd cli && cargo build --release   # → cli/target/release/spydus
```

Shell completions: `spydus completions zsh > ~/.zfunc/_spydus` (also `bash`, `fish`,
`powershell`, `elvish`).

## Try it without an account

```sh
spydus --demo loans               # the demo account's loans at City of Canning
spydus --demo search the midnight library
spydus --demo show 1
spydus --demo renew 1 --yes
spydus --demo reserve 1 --format Book --yes
spydus --demo picks
```

`--demo` signs you in as "Test Borrower". Changes succeed after a moment and last for the one
run; public reads (`libraries`, `search`, `show`) go to the live library. Nothing is saved but
the demo's own last list, so numbers work from one command to the next.

## Choose a library and sign in

```sh
spydus libraries canning          # the Spydus directory, filtered by words (--all adds test instances)
spydus use city of canning        # saved; or an origin: spydus use https://canning.spydus.com
spydus login                      # asks for your card number and PIN (no echo); keeps the session
spydus profile
spydus logout
```

`--library <name or origin>` picks a library for one run instead. The session is kept in the OS
keychain (`com.librespydus.cli.session`, account = origin); the PIN is never stored.

## Commands

| Command | What it does |
|---|---|
| `spydus libraries [<words>] [--all]` | the directory: name, region, origin |
| `spydus use <name or origin>` | choose the library (saved) |
| `spydus search <words…> [--newest] [--page <n>]` | numbered results: title, author, year, formats; then `Next page: …` |
| `spydus show <# or url> [--holdings]` | the book page: formats with availability, synopsis, details, branches holding it; "On loan" / "Reserved" when it's yours |
| `spydus login [--card <number>] [--password-stdin]` | sign in |
| `spydus logout` | forget this library's session |
| `spydus profile` | name, library, card number, counts |
| `spydus books` / `loans` / `reservations` | Your Books, numbered |
| `spydus renew <#…> \| --all [--yes]` | renew loans, one result line each |
| `spydus reserve <# or url> [--format <name>] [--pickup <branch>] [--yes]` | reserve a format of a book from the last list (or a reserve link) |
| `spydus cancel <#…> [--yes]` | cancel reservations |
| `spydus edit <#> [--pickup <branch>] [--delay-until YYYY-MM-DD] [--not-needed-after YYYY-MM-DD] [--clear-delay] [--clear-not-needed] [--yes]` | change a reservation; with no change, prints what can be changed |
| `spydus picks [add <# or url> \| remove <# or url>]` | My Picks: on your library account when signed in, else on this computer |
| `spydus card` | your library card number |
| `spydus verify [--clear]` | UK libraries' "confirm you're human": complete the check in a browser, paste the `aws-waf-token` cookie value |
| `spydus completions <shell>` | shell completions |

Global flags: `--library`, `--json`, `--demo`, `--data-dir <dir>`, `--no-color` (also `NO_COLOR`
and a non-terminal stdout).

### Numbers

Lists print `#1, #2, …` and later commands take those numbers. `show`, `reserve` and `picks add`
use the last search (or picks) list, saved with its library and query; a number from another
library's list, or from a list more than a day old, is refused with a hint. `renew`, `cancel` and
`edit` look at the account fresh and number loans and reservations as `loans` / `reservations`
print them. A URL always works where a number does.

### Confirmations

Renew, reserve, cancel and edit ask first on a terminal (`[y/N]`). Off a terminal, add `--yes`.
Prompts go to stderr; results go to stdout.

### `--json`

One JSON document on stdout, the core's types as they serialise (camelCase) with the list number
added; errors are one object on stderr:

```json
{"error":{"kind":"NotSignedIn","message":"You’re not signed in","hint":"Sign in with spydus login."}}
```

## Where things are kept

| What | Where |
|---|---|
| config (the chosen library) | `dirs::config_dir()/LibreSpydus/cli.json` (macOS: `~/Library/Application Support/LibreSpydus/cli.json`) |
| the last list, My Picks on this computer | `dirs::cache_dir()/LibreSpydus/cli/` |
| session, verification cookie | the OS keychain: `com.librespydus.cli.session`, `com.librespydus.cli.verification` (account = origin) |

`--data-dir <dir>` keeps everything under `<dir>` instead — secrets in `<dir>/secrets.json` in
plain text, for tests — and never touches the keychain.

## Exit codes

| Code | Meaning |
|---|---|
| 0 | done |
| 1 | the library, network or account failed (the message says what) |
| 2 | usage error |
| 3 | sign in needed, or the session expired |
| 4 | the library asks you to confirm you're human (`spydus verify`) |

## Development

```sh
cd cli
cargo test                                    # no network
cargo test -- --ignored live_canning_search   # one live public search
cargo clippy --all-targets -- -D warnings
UPDATE_GOLDEN=1 cargo test --lib golden       # rewrite tests/golden/*.txt
```

`src/copy.rs` is generated by `design/scripts/gen_copy.py` from `design/copy.json` (the same
words as the apps); don't edit it. CLI-only words live in `src/text.rs`.
