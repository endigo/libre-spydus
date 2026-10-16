//! Running one command: the world it runs in (clock, terminal, backends, keychain), the
//! per-run context every command shares (the chosen library, its backend, the saved session,
//! where files go) and how a failure is reported (docs/cli.md §7).

use std::io::{BufRead, Write};
use std::path::Path;
use std::sync::Arc;

use chrono::{DateTime, Local, NaiveDate, Utc};
use clap::Parser;
use librespydus_core::{LibraryInfo, SpydusClient};
use serde::Serialize;

use crate::args::{Cli, Global};
use crate::backend::{BackendFuture, CoreBackend, DemoBackend, LibraryBackend};
use crate::copy::Copy;
use crate::error::{EXIT_OK, EXIT_USAGE, Fail};
use crate::output::{Tone, styled};
use crate::store::{
    Config, FileSecretStore, LastList, MemorySecretStore, Paths, SESSION_SERVICE, SavedLibrary, SecretStore,
    VERIFICATION_SERVICE,
};
use crate::text;
use crate::{backend, commands};

/// Civica's directory, fetched when a library is named rather than given as an origin.
pub type DirectoryFetch = Arc<dyn Fn(bool) -> BackendFuture<Vec<LibraryInfo>> + Send + Sync>;
/// The public backend for an origin (the demo wraps it).
pub type BackendFactory = Arc<dyn Fn(&str) -> Arc<dyn LibraryBackend> + Send + Sync>;
/// The OS keychain; `None` keeps secrets in memory (tests).
pub type KeychainFactory = Arc<dyn Fn() -> Box<dyn SecretStore> + Send + Sync>;
/// A no-echo prompt for a PIN or a cookie, given the prompt text.
pub type SecretPrompt = Box<dyn FnMut(&str) -> std::io::Result<String> + Send>;

/// Everything a run touches outside its arguments. Tests build one with canned input, a fixed
/// clock, a fake backend and no keychain.
pub struct World {
    pub today: NaiveDate,
    pub now: DateTime<Utc>,
    /// Terminal columns for wrapping.
    pub width: usize,
    /// Colour on stdout.
    pub color: bool,
    /// Prompts and confirmations can be asked (stdin is a terminal).
    pub interactive: bool,
    pub stdin: Box<dyn BufRead + Send>,
    pub stdout: Box<dyn Write + Send>,
    pub stderr: Box<dyn Write + Send>,
    pub read_secret: SecretPrompt,
    pub backends: BackendFactory,
    pub directory: DirectoryFetch,
    pub keychain: Option<KeychainFactory>,
}

impl World {
    /// The real world: the terminal, the clock, the core and the OS keychain.
    pub fn real() -> Self {
        use std::io::IsTerminal;
        let stdout = std::io::stdout();
        let color = std::env::var_os("NO_COLOR").is_none_or(|v| v.is_empty())
            && stdout.is_terminal()
            && anstream::AutoStream::choice(&stdout) != anstream::ColorChoice::Never;
        let width = terminal_size::terminal_size().map_or(80, |(w, _)| usize::from(w.0)).max(40);
        let now = Utc::now();
        Self {
            today: now.with_timezone(&Local).date_naive(),
            now,
            width,
            color,
            interactive: std::io::stdin().is_terminal(),
            stdin: Box::new(std::io::BufReader::new(std::io::stdin())),
            stdout: Box::new(stdout),
            stderr: Box::new(std::io::stderr()),
            read_secret: Box::new(|prompt| rpassword::prompt_password(prompt)),
            backends: Arc::new(|origin| Arc::new(CoreBackend::new(Some(origin)))),
            directory: Arc::new(|all| Box::pin(librespydus_core::list_libraries(all))),
            keychain: Some(Arc::new(|| Box::new(crate::store::KeychainSecretStore))),
        }
    }
}

/// Parses `argv` and runs the command; the process exit code.
pub async fn run(argv: Vec<String>, world: &mut World) -> i32 {
    let cli = match Cli::try_parse_from(argv) {
        Ok(cli) => cli,
        Err(error) => {
            let text = error.render().to_string();
            return if error.use_stderr() {
                let _ = write!(world.stderr, "{text}");
                EXIT_USAGE
            } else {
                let _ = write!(world.stdout, "{text}");
                EXIT_OK
            };
        }
    };
    if cli.global.no_color {
        world.color = false;
    }
    let json = cli.global.json;
    let mut ctx = Ctx::new(world, cli.global);
    let library_name = |ctx: &Ctx| ctx.library.as_ref().map(|l| l.name.clone()).unwrap_or_default();
    match commands::dispatch(&mut ctx, cli.command).await {
        Ok(()) => EXIT_OK,
        Err(Fail::AlreadyReported(exit)) => exit,
        Err(fail) => {
            let name = library_name(&ctx);
            if fail.ends_session() {
                ctx.forget_session();
            }
            let report = fail.report(&name);
            if json {
                let error = ErrorJson {
                    kind: report.kind,
                    message: &report.message,
                    detail: report.detail.as_deref(),
                    hint: report.hint.as_deref(),
                };
                let text = serde_json::to_string(&ErrorEnvelope { error }).unwrap_or_default();
                let _ = writeln!(ctx.world.stderr, "{text}");
            } else {
                let _ = writeln!(ctx.world.stderr, "{}", styled(&report.message, Tone::Error, false));
                if let Some(detail) = &report.detail {
                    let _ = writeln!(ctx.world.stderr, "{detail}");
                }
                if let Some(hint) = &report.hint {
                    let _ = writeln!(ctx.world.stderr, "{hint}");
                }
            }
            report.exit
        }
    }
}

/// `{"error": {...}}` on stderr under `--json` (docs/cli.md §4): the core's error variant (or
/// the CLI's own kind), the apps' wording, technical detail and a hint when there is one.
#[derive(Serialize)]
struct ErrorJson<'a> {
    kind: &'a str,
    message: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hint: Option<&'a str>,
}

#[derive(Serialize)]
struct ErrorEnvelope<'a> {
    error: ErrorJson<'a>,
}

/// What every command shares for one run.
pub struct Ctx<'w> {
    pub world: &'w mut World,
    pub json: bool,
    pub demo: bool,
    pub library_flag: Option<String>,
    pub paths: Paths,
    pub secrets: Box<dyn SecretStore>,
    /// The chosen library, once `library()` resolved it.
    pub library: Option<SavedLibrary>,
    backend: Option<Arc<dyn LibraryBackend>>,
}

impl<'w> Ctx<'w> {
    fn new(world: &'w mut World, global: Global) -> Self {
        let paths = Paths::new(global.data_dir.as_deref());
        let secrets: Box<dyn SecretStore> = if global.demo {
            Box::new(MemorySecretStore::default())
        } else if let Some(file) = &paths.secrets_file {
            Box::new(FileSecretStore::new(file.clone()))
        } else if let Some(keychain) = &world.keychain {
            keychain()
        } else {
            Box::new(MemorySecretStore::default())
        };
        Self {
            world,
            json: global.json,
            demo: global.demo,
            library_flag: global.library,
            paths,
            secrets,
            library: None,
            backend: None,
        }
    }

    // ---- The library and its backend ----

    /// The chosen library (`--library`, else the saved one, else Canning in demo mode) and a
    /// backend for it, with this library's verification cookie set.
    pub async fn library(&mut self) -> Result<(SavedLibrary, Arc<dyn LibraryBackend>), Fail> {
        if let (Some(library), Some(backend)) = (&self.library, &self.backend) {
            return Ok((library.clone(), backend.clone()));
        }
        let library = match self.library_flag.clone() {
            Some(flag) => self.resolve_library(&flag, false).await?,
            None => match Config::load(&self.paths)?.library {
                Some(saved) => saved,
                None if self.demo => canning(),
                None => return Err(Fail::NoLibrary),
            },
        };
        let public = (self.world.backends)(&library.origin);
        let backend: Arc<dyn LibraryBackend> =
            if self.demo { Arc::new(DemoBackend::live(public, self.world.today)) } else { public };
        if let Some(cookie) = self.secrets.get(VERIFICATION_SERVICE, &library.origin)? {
            backend.set_verification_cookie(Some(format!("aws-waf-token={cookie}")));
        }
        self.library = Some(library.clone());
        self.backend = Some(backend.clone());
        Ok((library, backend))
    }

    /// `query` as a library: an origin as given, or the one directory entry whose name holds
    /// every word. With `lookup_origin`, an origin is also looked up in the directory for its name.
    pub async fn resolve_library(&mut self, query: &str, lookup_origin: bool) -> Result<SavedLibrary, Fail> {
        let query = query.trim();
        if query.starts_with("http://") || query.starts_with("https://") {
            let origin = SpydusClient::new(Some(query.to_owned())).origin();
            if lookup_origin
                && let Ok(libraries) = (self.world.directory)(true).await
                && let Some(info) = libraries.iter().find(|info| info.origin.eq_ignore_ascii_case(&origin))
            {
                return Ok(SavedLibrary { origin: info.origin.clone(), name: info.name.clone() });
            }
            return Ok(SavedLibrary { origin: origin.clone(), name: placeholder_name(&origin) });
        }
        let libraries = (self.world.directory)(false).await.map_err(Fail::Directory)?;
        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        let exact: Vec<&LibraryInfo> = libraries.iter().filter(|info| info.name.eq_ignore_ascii_case(query)).collect();
        let matches: Vec<&LibraryInfo> = if exact.is_empty() {
            libraries
                .iter()
                .filter(|info| {
                    let haystack = format!("{} {}", info.name, info.origin).to_lowercase();
                    words.iter().all(|word| haystack.contains(word.as_str()))
                })
                .collect()
        } else {
            exact
        };
        match matches.as_slice() {
            [] => Err(Fail::NoMatch(text::no_library_matches(query))),
            [one] => Ok(SavedLibrary { origin: one.origin.clone(), name: one.name.clone() }),
            several => {
                let infos: Vec<LibraryInfo> = several.iter().map(|info| (*info).clone()).collect();
                if self.json {
                    self.json(&infos);
                } else {
                    commands::libraries::print_libraries(self, &infos)?;
                }
                Err(Fail::NoMatch(text::several_libraries(several.len())))
            }
        }
    }

    // ---- The session ----

    /// The saved session for the chosen library (the demo's in demo mode).
    pub fn session(&mut self) -> Result<String, Fail> {
        if self.demo {
            return Ok(backend::demo::SESSION.to_owned());
        }
        let origin = self.library.as_ref().map(|l| l.origin.clone()).ok_or(Fail::NoLibrary)?;
        self.secrets.get(SESSION_SERVICE, &origin)?.ok_or(Fail::NotSignedIn)
    }

    /// Whether a session is saved (nothing is fetched).
    pub fn has_session(&mut self) -> bool {
        self.session().is_ok()
    }

    pub fn save_session(&mut self, session: &str) -> Result<(), Fail> {
        if self.demo {
            return Ok(());
        }
        let origin = self.library.as_ref().map(|l| l.origin.clone()).ok_or(Fail::NoLibrary)?;
        self.secrets.set(SESSION_SERVICE, &origin, Some(session))
    }

    /// Deletes the saved session (the library ended it, or `spydus logout`).
    pub fn forget_session(&mut self) {
        if self.demo {
            return;
        }
        if let Some(origin) = self.library.as_ref().map(|l| l.origin.clone()) {
            let _ = self.secrets.set(SESSION_SERVICE, &origin, None);
        }
    }

    // ---- Lists ----

    /// The last numbered list, which must exist.
    pub fn last_list(&self, hint: &str) -> Result<LastList, Fail> {
        LastList::load(&self.paths, self.demo)?.ok_or_else(|| Fail::List(hint.to_owned()))
    }

    pub fn save_list(&self, list: &LastList) -> Result<(), Fail> {
        list.save(&self.paths, self.demo)
    }

    // ---- Output ----

    pub fn out(&mut self, text: &str) {
        let _ = writeln!(self.world.stdout, "{text}");
    }

    pub fn out_styled(&mut self, text: &str, tone: Tone) {
        let styled = styled(text, tone, self.world.color);
        let _ = writeln!(self.world.stdout, "{styled}");
    }

    pub fn note(&mut self, text: &str) {
        let _ = writeln!(self.world.stderr, "{text}");
    }

    /// One JSON document on stdout.
    pub fn json<T: Serialize>(&mut self, value: &T) {
        let text = serde_json::to_string_pretty(value).unwrap_or_default();
        let _ = writeln!(self.world.stdout, "{text}");
    }

    /// Whether this run prints for people (not `--json`).
    pub fn human(&self) -> bool {
        !self.json
    }

    pub fn width(&self) -> usize {
        self.world.width
    }

    pub fn color(&self) -> bool {
        self.world.color
    }

    /// The table in `columns` and `rows`, written to stdout.
    pub fn table(&mut self, columns: &[crate::output::Column], rows: &[Vec<crate::output::Cell>]) {
        let text = crate::output::render_table(columns, rows, self.world.width, self.world.color);
        let _ = write!(self.world.stdout, "{text}");
    }

    // ---- Prompts (on stderr, so stdout stays the data) ----

    /// A line from stdin after `label: ` on stderr, trimmed; empty is a usage error.
    pub fn ask(&mut self, label: &str) -> Result<String, Fail> {
        let _ = write!(self.world.stderr, "{}", text::prompt(label));
        let _ = self.world.stderr.flush();
        let line = self.read_line()?;
        if line.is_empty() { Err(Fail::Usage(text::no_input().to_owned())) } else { Ok(line) }
    }

    /// The next line of stdin, trimmed (empty at the end of input).
    pub fn read_line(&mut self) -> Result<String, Fail> {
        let mut line = String::new();
        self.world.stdin.read_line(&mut line).map_err(|error| Fail::Io(error.to_string()))?;
        Ok(line.trim().to_owned())
    }

    /// A secret without echo on a terminal, else the next line of stdin.
    pub fn read_secret(&mut self, prompt: &str) -> Result<String, Fail> {
        if self.world.interactive {
            let _ = self.world.stderr.flush();
            return (self.world.read_secret)(prompt)
                .map(|s| s.trim().to_owned())
                .map_err(|error| Fail::Io(error.to_string()));
        }
        self.read_line()
    }

    /// Asks `title` / `message` and waits for yes, unless `yes`. Off a terminal without `yes`
    /// it's a usage error (add `--yes`); a no is `Declined`.
    pub fn confirm(&mut self, yes: bool, title: &str, message: &str) -> Result<(), Fail> {
        if yes {
            return Ok(());
        }
        if !self.world.interactive {
            return Err(Fail::Usage(text::HINT_YES.to_owned()));
        }
        let _ = writeln!(self.world.stderr, "{}", styled(title, Tone::Bold, self.world.color));
        let _ = writeln!(self.world.stderr, "{message}");
        let _ = write!(self.world.stderr, "{}", text::PROMPT_YES_NO);
        let _ = self.world.stderr.flush();
        let answer = self.read_line()?.to_lowercase();
        if answer == "y" || answer == "yes" { Ok(()) } else { Err(Fail::Declined) }
    }

    /// Lists `choices` numbered on stderr and reads one number; `usage` is the error off a terminal.
    pub fn choose(&mut self, title: &str, choices: &[String], usage: String) -> Result<usize, Fail> {
        if !self.world.interactive {
            return Err(Fail::Usage(usage));
        }
        let _ = writeln!(self.world.stderr, "{}", styled(title, Tone::Bold, self.world.color));
        for (index, choice) in choices.iter().enumerate() {
            let _ = writeln!(self.world.stderr, "{:>3}  {choice}", format!("#{}", index + 1));
        }
        let _ = write!(self.world.stderr, "{}", text::PROMPT_CHOOSE);
        let _ = self.world.stderr.flush();
        let answer = self.read_line()?;
        if answer.is_empty() {
            return Err(Fail::Usage(text::no_input().to_owned()));
        }
        crate::args::parse_number(&answer)
            .filter(|number| *number <= choices.len())
            .map(|number| number - 1)
            .ok_or_else(|| {
                Fail::Usage(text::no_such_number(crate::args::parse_number(&answer).unwrap_or(0), choices.len()))
            })
    }

    /// `now` in the computer's time zone, for "Updated …" lines.
    pub fn now_local(&self) -> DateTime<Local> {
        self.world.now.with_timezone(&Local)
    }
}

/// Canning, the core's default library, as the directory names it.
pub fn canning() -> SavedLibrary {
    // copy-exempt: Canning's name, as the directory gives it (data)
    SavedLibrary { origin: librespydus_core::config::ORIGIN.to_owned(), name: "City of Canning".to_owned() }
}

/// A stand-in name built from the origin alone ("https://hutt.spydus.co.nz" → "Hutt"), until the
/// directory entry is known.
pub fn placeholder_name(origin: &str) -> String {
    if origin == librespydus_core::config::ORIGIN {
        return canning().name;
    }
    let host = origin.split("://").nth(1).unwrap_or(origin).split(['/', ':']).next().unwrap_or(origin);
    let label = host.split('.').next().unwrap_or(host);
    let mut name = label.to_owned();
    if let Some(first) = name.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    if name.is_empty() { Copy::Common::yourLibrary.to_owned() } else { name }
}

/// `path` as text for messages.
pub fn display_path(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_guess_from_the_host() {
        assert_eq!(placeholder_name("https://canning.spydus.com"), "City of Canning");
        assert_eq!(placeholder_name("https://hutt.spydus.co.nz"), "Hutt");
        assert_eq!(placeholder_name("https://brazoria.spydus.com/"), "Brazoria");
    }
}
