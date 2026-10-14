//! What a screen shows for a failed call (docs/native-apps.md §5.7), as iOS `UserFacingError.swift`.
//! Get one from `AppState::report`, which also does the side effects (sign-out, sign-in request).

use librespydus_core::SpydusError;

use crate::copy::Copy;
use crate::state::formatting::capitalised_first;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Network,
    /// The library asked for human verification. Desktop v1 has no web view, so `report` turns
    /// this into `VerificationUnavailable` (docs/desktop-app.md §5.3).
    VerificationRequired,
    /// Verification can't be done from the app: offer the library's website.
    VerificationUnavailable,
    NotLoggedIn,
    CredentialsRejected,
    /// OriginForbidden / Upstream / Parse / anything else.
    Unexpected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserFacingError {
    pub kind: ErrorKind,
    pub message: String,
    /// Technical detail for the "Details" disclosure (the core's error text).
    pub detail: Option<String>,
}

impl UserFacingError {
    pub fn new(kind: ErrorKind, message: impl Into<String>, detail: Option<String>) -> Self {
        Self { kind, message: message.into(), detail }
    }

    /// `error` in the reader's words, naming `library_name` (blank → "your library").
    pub fn from_core(error: &SpydusError, library_name: &str) -> Self {
        let library = library_or_default(library_name);
        let detail = Some(error.to_string());
        match error {
            SpydusError::Network { .. } => Self::new(ErrorKind::Network, Copy::Errors::network(library), detail),
            SpydusError::HumanVerificationRequired => Self::new(
                ErrorKind::VerificationRequired,
                capitalised_first(&Copy::Errors::verification(library)),
                detail,
            ),
            SpydusError::NotLoggedIn => Self::new(ErrorKind::NotLoggedIn, Copy::Errors::signInAgain, detail),
            SpydusError::CredentialsRejected => {
                Self::new(ErrorKind::CredentialsRejected, Copy::Errors::credentialsRejected, detail)
            }
            SpydusError::OriginForbidden | SpydusError::Upstream { .. } | SpydusError::Parse { .. } => {
                Self::unexpected(detail)
            }
        }
    }

    pub fn unexpected(detail: Option<String>) -> Self {
        Self::new(ErrorKind::Unexpected, Copy::Errors::unexpected, detail)
    }

    /// The library keeps asking for verification, which the desktop app can't show: use its
    /// website instead (iOS `verificationUnavailable`).
    pub fn verification_unavailable(library_name: &str, detail: Option<String>) -> Self {
        let message = capitalised_first(&Copy::Errors::libraryUnavailable(library_or_default(library_name)));
        Self::new(ErrorKind::VerificationUnavailable, message, detail)
    }

    pub fn title(&self) -> &'static str {
        match self.kind {
            ErrorKind::Network => Copy::Errors::cantConnectTitle,
            ErrorKind::VerificationRequired => Copy::Errors::verificationNeededTitle,
            ErrorKind::VerificationUnavailable => Copy::Errors::libraryUnavailableTitle,
            ErrorKind::NotLoggedIn => Copy::Errors::signedOutTitle,
            ErrorKind::CredentialsRejected => Copy::Errors::signInFailedTitle,
            ErrorKind::Unexpected => Copy::Errors::unexpectedTitle,
        }
    }

    /// Whether Retry makes sense (sign-in problems are fixed by signing in instead).
    pub fn can_retry(&self) -> bool {
        matches!(self.kind, ErrorKind::Network | ErrorKind::VerificationRequired | ErrorKind::Unexpected)
    }
}

impl std::fmt::Display for UserFacingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for UserFacingError {}

fn library_or_default(name: &str) -> &str {
    if name.trim().is_empty() { Copy::Common::yourLibrary } else { name }
}

/// A screen's state for one load (iOS `Loadable`).
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Loadable<T> {
    #[default]
    Idle,
    Loading,
    Loaded(T),
    Failed(UserFacingError),
}

impl<T> Loadable<T> {
    pub fn value(&self) -> Option<&T> {
        match self {
            Self::Loaded(value) => Some(value),
            _ => None,
        }
    }

    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Loading)
    }

    pub fn error(&self) -> Option<&UserFacingError> {
        match self {
            Self::Failed(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_errors_read_as_the_phones_say_them() {
        let network = UserFacingError::from_core(&SpydusError::Network { message: "timed out".into() }, "Canning");
        assert_eq!(network.kind, ErrorKind::Network);
        assert_eq!(network.message, "Can’t reach Canning. Check your connection.");
        assert!(network.can_retry());
        let unnamed = UserFacingError::from_core(&SpydusError::HumanVerificationRequired, "");
        assert_eq!(unnamed.message, "Your library asks you to confirm you’re human.");
        let parse = UserFacingError::from_core(&SpydusError::Parse { message: "x".into() }, "Canning");
        assert_eq!(parse.kind, ErrorKind::Unexpected);
        assert_eq!(parse.detail.as_deref(), Some("parse error: x"));
        assert!(!UserFacingError::from_core(&SpydusError::NotLoggedIn, "Canning").can_retry());
    }
}
