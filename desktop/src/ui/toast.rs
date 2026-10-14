//! Toast helpers (iOS `Toast.swift`): a short line at the window's edge for a confirmation, a
//! failure, or work in progress, through gpui-kit's notifications.

use gpui_kit::component::WindowExt;
use gpui_kit::component::notification::Notification;
use gpui_kit::{App, SharedString, Window};

use crate::state::NoticeTone;

/// All of the app's toasts share this id, so each one replaces the one before (one toast at a
/// time, as on the phones) and a result replaces its "Renewing…".
struct AppNotice;

/// Shows `message` with `tone`'s icon. Progress toasts stay until the next one replaces them.
pub fn show(window: &mut Window, cx: &mut App, message: impl Into<SharedString>, tone: NoticeTone) {
    let message = message.into();
    let note = match tone {
        NoticeTone::Success => Notification::success(message),
        NoticeTone::Info => Notification::info(message),
        NoticeTone::Error => Notification::error(message),
        NoticeTone::Progress => Notification::info(message).autohide(false),
    };
    window.push_notification(note.id::<AppNotice>(), cx);
}

pub fn success(window: &mut Window, cx: &mut App, message: impl Into<SharedString>) {
    show(window, cx, message, NoticeTone::Success);
}

pub fn error(window: &mut Window, cx: &mut App, message: impl Into<SharedString>) {
    show(window, cx, message, NoticeTone::Error);
}
