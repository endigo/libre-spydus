//! "Confirm you're human" for libraries behind an AWS WAF check (docs/desktop-app.md §11, E2;
//! native-apps.md §6 "Verification"). Placeholder hooks with their final signatures; E2 fills them.

use gpui_kit::{App, Context, Window};
use librespydus_core::SpydusError;

use crate::state::{AppState, UserFacingError};

/// `AppState::report` on `HumanVerificationRequired`: returns what to show. Until E2, the
/// library is "unavailable" (no web view).
pub fn on_required(state: &mut AppState, error: &SpydusError, _cx: &mut Context<AppState>) -> UserFacingError {
    UserFacingError::verification_unavailable(&state.library_name(), Some(error.to_string()))
}

/// Called once by the shell at start: present the check when `AppState` asks for it.
pub fn watch(_window: &mut Window, _cx: &mut App) {}

/// Called first thing in `main`: when the process was started as the check's helper window
/// (`librespydus --verify-helper …`), runs it and returns `true` (main then exits).
pub fn run_helper_if_requested() -> bool {
    false
}
