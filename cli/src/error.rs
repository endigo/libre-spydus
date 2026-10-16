//! What a failed command prints and which exit code it gives (docs/cli.md §7). The core's
//! errors read as the apps say them (ported from the desktop app's `state/error.rs`, iOS
//! `UserFacingError.swift`); the CLI's own failures (no library, not signed in, a bad number,
//! a usage error) carry a hint on what to run.

use librespydus_core::SpydusError;

use crate::copy::Copy;
use crate::formatting::capitalised_first;
use crate::text;

/// Success.
pub const EXIT_OK: i32 = 0;
/// The library, network or account failed.
pub const EXIT_FAILED: i32 = 1;
/// A usage error (clap's, and the CLI's own).
pub const EXIT_USAGE: i32 = 2;
/// Sign-in is needed, or the session expired.
pub const EXIT_SIGN_IN: i32 = 3;
/// A "confirm you're human" check is needed (`spydus verify`).
pub const EXIT_VERIFY: i32 = 4;

/// Why a command stopped.
#[derive(Debug)]
pub enum Fail {
    /// The core failed.
    Core(SpydusError),
    /// A command that needs a library ran without one.
    NoLibrary,
    /// An account command ran without a saved session.
    NotSignedIn,
    /// Wrong arguments (beyond what clap checks).
    Usage(String),
    /// A list number that can't be used: no list, another library's, stale, or out of range.
    List(String),
    /// The library refused a change (a `ReserveResult` / `RenewResult` with `ok: false`).
    Refused(String),
    /// A file or the keychain couldn't be read or written.
    Io(String),
    /// The reader answered no at a confirmation.
    Declined,
    /// Civica's library directory couldn't be read.
    Directory(SpydusError),
    /// No library (or no choice) matches what was asked for.
    NoMatch(String),
    /// The command already printed its outcome: just exit with this code.
    AlreadyReported(i32),
}

impl From<SpydusError> for Fail {
    fn from(error: SpydusError) -> Self {
        Self::Core(error)
    }
}

/// A failure in the reader's words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// The `SpydusError` variant, or the CLI's own kind (`--json`'s `error.kind`).
    pub kind: &'static str,
    pub message: String,
    /// Technical detail (the core's error text), shown under the message.
    pub detail: Option<String>,
    /// What to run next.
    pub hint: Option<String>,
    pub exit: i32,
}

impl Fail {
    /// `self` in the reader's words, naming `library_name` (blank → "your library").
    pub fn report(&self, library_name: &str) -> Report {
        let library = library_or_default(library_name);
        let report = |kind, message: String, detail, hint: Option<&str>, exit| Report {
            kind,
            message,
            detail,
            hint: hint.map(str::to_owned),
            exit,
        };
        match self {
            Self::Core(error) => {
                let detail = Some(error.to_string());
                match error {
                    SpydusError::Network { .. } => {
                        report("Network", Copy::Errors::network(library), detail, None, EXIT_FAILED)
                    }
                    SpydusError::HumanVerificationRequired => report(
                        "HumanVerificationRequired",
                        capitalised_first(&Copy::Errors::verification(library)),
                        None,
                        Some(text::HINT_VERIFY),
                        EXIT_VERIFY,
                    ),
                    SpydusError::NotLoggedIn => report(
                        "NotLoggedIn",
                        Copy::Errors::signInAgain.to_owned(),
                        None,
                        Some(text::HINT_SIGN_IN),
                        EXIT_SIGN_IN,
                    ),
                    SpydusError::CredentialsRejected => report(
                        "CredentialsRejected",
                        Copy::Errors::credentialsRejected.to_owned(),
                        None,
                        None,
                        EXIT_FAILED,
                    ),
                    SpydusError::OriginForbidden => {
                        report("OriginForbidden", Copy::Errors::unexpected.to_owned(), detail, None, EXIT_FAILED)
                    }
                    SpydusError::Upstream { .. } => {
                        report("Upstream", Copy::Errors::unexpected.to_owned(), detail, None, EXIT_FAILED)
                    }
                    SpydusError::Parse { .. } => {
                        report("Parse", Copy::Errors::unexpected.to_owned(), detail, None, EXIT_FAILED)
                    }
                }
            }
            Self::NoLibrary => report(
                "NoLibrary",
                Copy::Profile::noLibraryTitle.to_owned(),
                None,
                Some(text::HINT_CHOOSE_LIBRARY),
                EXIT_FAILED,
            ),
            Self::NotSignedIn => report(
                "NotSignedIn",
                Copy::Profile::notSignedInTitle.to_owned(),
                None,
                Some(text::HINT_SIGN_IN),
                EXIT_SIGN_IN,
            ),
            Self::Usage(message) => report("Usage", message.clone(), None, None, EXIT_USAGE),
            Self::List(message) => report("List", message.clone(), None, None, EXIT_FAILED),
            Self::Refused(message) => report("Refused", message.clone(), None, None, EXIT_FAILED),
            Self::Io(message) => report("Io", message.clone(), None, None, EXIT_FAILED),
            Self::Declined => report("Declined", text::NOTHING_CHANGED.to_owned(), None, None, EXIT_FAILED),
            Self::Directory(error) => {
                let message = match error {
                    SpydusError::Network { .. } => Copy::Onboarding::directoryNetwork,
                    _ => Copy::Onboarding::directoryUnexpected,
                };
                report("Directory", message.to_owned(), Some(error.to_string()), None, EXIT_FAILED)
            }
            Self::NoMatch(message) => report("NoMatch", message.clone(), None, None, EXIT_FAILED),
            Self::AlreadyReported(exit) => report("Failed", String::new(), None, None, *exit),
        }
    }

    /// Whether the saved session should be forgotten: the library says it has ended.
    pub fn ends_session(&self) -> bool {
        matches!(self, Self::Core(SpydusError::NotLoggedIn))
    }
}

fn library_or_default(name: &str) -> &str {
    if name.trim().is_empty() { Copy::Common::yourLibrary } else { name }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_errors_read_as_the_apps_say_them() {
        let network = Fail::from(SpydusError::Network { message: "timed out".into() }).report("City of Canning");
        assert_eq!(network.kind, "Network");
        assert_eq!(network.message, "Can’t reach City of Canning. Check your connection.");
        assert_eq!(network.detail.as_deref(), Some("network error: timed out"));
        assert_eq!(network.exit, EXIT_FAILED);

        let unnamed = Fail::from(SpydusError::HumanVerificationRequired).report("");
        assert_eq!(unnamed.message, "Your library asks you to confirm you’re human.");
        assert_eq!(unnamed.hint.as_deref(), Some(text::HINT_VERIFY));
        assert_eq!(unnamed.exit, EXIT_VERIFY);

        let expired = Fail::from(SpydusError::NotLoggedIn).report("Canning");
        assert_eq!(
            (expired.kind, expired.message.as_str(), expired.exit),
            ("NotLoggedIn", "Please sign in again.", EXIT_SIGN_IN)
        );
        assert_eq!(expired.hint.as_deref(), Some(text::HINT_SIGN_IN));
        assert!(Fail::from(SpydusError::NotLoggedIn).ends_session());

        let rejected = Fail::from(SpydusError::CredentialsRejected).report("Canning");
        assert_eq!((rejected.message.as_str(), rejected.exit), ("Card number or PIN not recognised.", EXIT_FAILED));

        for error in [
            SpydusError::Parse { message: "x".into() },
            SpydusError::Upstream { message: "x".into() },
            SpydusError::OriginForbidden,
        ] {
            let report = Fail::from(error).report("Canning");
            assert_eq!(report.message, "The library’s website returned something unexpected.");
            assert!(report.detail.is_some());
            assert_eq!(report.exit, EXIT_FAILED);
        }
    }

    #[test]
    fn cli_failures_carry_hints_and_codes() {
        let no_library = Fail::NoLibrary.report("");
        assert_eq!(
            (no_library.kind, no_library.message.as_str(), no_library.exit),
            ("NoLibrary", "No library selected", EXIT_FAILED)
        );
        assert_eq!(no_library.hint.as_deref(), Some(text::HINT_CHOOSE_LIBRARY));
        let signed_out = Fail::NotSignedIn.report("");
        assert_eq!((signed_out.message.as_str(), signed_out.exit), ("You’re not signed in", EXIT_SIGN_IN));
        assert_eq!(Fail::Usage("bad".into()).report("").exit, EXIT_USAGE);
        assert_eq!(Fail::List("bad".into()).report("").exit, EXIT_FAILED);
        assert_eq!(Fail::Refused("no".into()).report("").exit, EXIT_FAILED);
        assert_eq!(Fail::Declined.report("").message, "Nothing changed.");
        let directory = Fail::Directory(SpydusError::Network { message: "x".into() }).report("");
        assert_eq!(directory.message, "Can’t reach the library directory. Check your connection.");
        assert_eq!(Fail::NoMatch("none".into()).report("").kind, "NoMatch");
        assert_eq!(Fail::AlreadyReported(EXIT_FAILED).report("").exit, EXIT_FAILED);
    }
}
