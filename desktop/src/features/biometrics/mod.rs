//! Sign back in with Touch ID (docs/desktop-app.md §11, E4; native-apps.md §5.14). Placeholder
//! hooks with their final signatures; E4 fills them.

use gpui_kit::{AnyElement, App, Context, Window};

use crate::state::AppState;

/// `AppState::report` on `NotLoggedIn`: `true` when a Touch ID sign-in started instead of
/// clearing the session (the session and saved account stay on screen until it ends).
pub fn sign_back_in(_state: &mut AppState, _cx: &mut Context<AppState>) -> bool {
    false
}

/// `AppState::sign_in` succeeded with these details (a typed sign-in, or E4's own).
pub fn did_sign_in(_state: &mut AppState, _username: &str, _pin: &str, _cx: &mut Context<AppState>) {}

/// `AppState::sign_out` for `origin`.
pub fn did_sign_out(_state: &mut AppState, _origin: &str, _cx: &mut Context<AppState>) {}

/// Profile: the Touch ID switch, or nothing.
pub fn profile_section(_window: &mut Window, _cx: &mut App) -> Option<AnyElement> {
    None
}

/// Called once by the shell at start (the offer after a typed sign-in, …).
pub fn watch(_window: &mut Window, _cx: &mut App) {}

/// `--reset`: deletes every library's saved details (`secrets_in_memory`: nothing to delete).
pub fn reset(_secrets_in_memory: bool) {}
