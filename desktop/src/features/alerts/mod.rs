//! Ready-to-collect alerts and due-date reminders as desktop notifications (docs/desktop-app.md
//! §11, E3; native-apps.md §5.8). Placeholder hooks with their final signatures; E3 fills them.

use gpui_kit::{AnyElement, App, Context, Window};

use crate::state::AppState;

/// A fresh account snapshot was just applied (`AppState::account().snapshot()`).
pub fn account_refreshed(_state: &mut AppState, _cx: &mut Context<AppState>) {}

/// `AppState::sign_out` for `origin`.
pub fn did_sign_out(_state: &mut AppState, _origin: &str, _cx: &mut Context<AppState>) {}

/// Profile: the Notifications section, or nothing.
pub fn profile_section(_window: &mut Window, _cx: &mut App) -> Option<AnyElement> {
    None
}

/// Called once by the shell at start (the morning check timer, …).
pub fn start(_window: &mut Window, _cx: &mut App) {}
