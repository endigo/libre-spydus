//! Screenshot steps for the alerts (cargo feature `evidence` only; docs/desktop-app.md §9): with
//! `LIBRESPYDUS_EVIDENCE=alerts:<step>` the app sets up a state the launch flags can't reach.
//! Ignored without the prefix.
//!
//! Steps (with `--demo`, after the launch refresh):
//! - `on`: both switches on (nothing is posted).
//! - `ask-ready`: turns "Ready to collect" on the way a click does. With
//!   `LS_ALERTS_PERMISSION=denied` that's the blocked state.
//! - `post`: both switches on, then a refresh: with `LS_DEMO_NEW_READY=1` a "Ready to collect"
//!   alert, and from 07:45 the day's due reminders. Unbundled, they show as in-app toasts; from a
//!   bundle, as macOS notifications (the delivered ones are logged).
//! - `tap`: `post`, then the "Ready to collect" alert clicked: Your Books › Reservations › the book.

use std::time::Duration;

use gpui_kit::test::TestWindowExt;
use gpui_kit::{App, AsyncApp, ScrollDelta, Window, point, px};

use super::content::AlertKind;
use crate::state::AppState;

const PREFIX: &str = "alerts:";

pub(super) fn schedule(_window: &mut Window, cx: &mut App) {
    let Some(step) = std::env::var("LIBRESPYDUS_EVIDENCE").ok().and_then(|v| v.strip_prefix(PREFIX).map(str::to_owned))
    else {
        return;
    };
    cx.spawn(async move |cx| {
        settle(cx).await;
        match step.as_str() {
            "on" => {
                cx.update(|cx| {
                    let state = AppState::global(cx);
                    let (origin, saved) = {
                        let state = state.read(cx);
                        (state.backend().origin(), state.account().snapshot().cloned())
                    };
                    super::center(cx).update(cx, |center, _| {
                        center.apply_switch(AlertKind::Ready, true, Some(&origin), saved.as_ref());
                        center.apply_switch(AlertKind::Due, true, Some(&origin), saved.as_ref());
                    });
                    cx.refresh_windows();
                });
                pause(cx, 300).await;
                scroll_to_section(cx);
            }
            "ask-ready" => {
                cx.update(|cx| super::set_switch(AlertKind::Ready, true, cx));
                pause(cx, 500).await;
                scroll_to_section(cx);
            }
            "post" | "tap" => {
                cx.update(|cx| {
                    super::center(cx).update(cx, |center, _| center.set_allow_demo(true));
                    super::set_switch(AlertKind::Ready, true, cx);
                });
                // One permission request at a time.
                pause(cx, 1000).await;
                cx.update(|cx| super::set_switch(AlertKind::Due, true, cx));
                pause(cx, 1500).await;
                cx.update(|cx| AppState::global(cx).update(cx, |state, cx| state.refresh_account(true, cx)).detach());
                settle(cx).await;
                pause(cx, 800).await;
                cx.update(|cx| {
                    let center = super::center(cx);
                    let center = center.read(cx);
                    log::warn!("alerts evidence: permission {:?}", center.permission());
                    for posted in center.posted() {
                        log::warn!(
                            "alerts evidence: posted {} ({}) “{}” / “{}”",
                            posted.note.id,
                            if posted.to_system { "notification center" } else { "in-app toast" }, // copy-exempt: a log line
                            posted.note.title,
                            posted.note.body
                        );
                    }
                });
                log_delivered(cx).await;
                if step == "tap" {
                    cx.update(|cx| {
                        let tap = super::center(cx)
                            .read(cx)
                            .posted()
                            .iter()
                            .rev()
                            .find(|posted| posted.note.tap.kind == AlertKind::Ready)
                            .map(|posted| posted.note.tap.clone());
                        if let Some(tap) = tap {
                            log::warn!("alerts evidence: clicking {:?}", tap);
                            super::open_alert(&tap, cx);
                        }
                    });
                }
            }
            _ => log::warn!("alerts evidence: unknown step {step}"),
        }
    })
    .detach();
}

/// Profile: scrolled so the Notifications section shows in a small window.
fn scroll_to_section(cx: &mut AsyncApp) {
    cx.update(|cx| {
        let Some(window) = super::center(cx).read(cx).window else { return };
        let _ = window.update(cx, |_, window, cx| {
            window.render_frame(cx);
            if window.try_find("profile-screen").is_some() {
                window.scroll("profile-screen", ScrollDelta::Pixels(point(px(0.), px(-260.))), cx);
            }
        });
    });
}

/// Waits until the account has loaded and no refresh is running (at most 20 s).
async fn settle(cx: &mut AsyncApp) {
    pause(cx, 500).await;
    for _ in 0..80 {
        let settled = cx.update(|cx| {
            let state = AppState::global(cx);
            let state = state.read(cx);
            state.account().snapshot().is_some() && !state.account().is_refreshing()
        });
        if settled {
            return;
        }
        pause(cx, 250).await;
    }
}

async fn pause(cx: &mut AsyncApp, ms: u64) {
    cx.background_executor().timer(Duration::from_millis(ms)).await;
}

#[cfg(target_os = "macos")]
async fn log_delivered(cx: &mut AsyncApp) {
    let system = super::system::macos::MacSystem::new();
    if !system_has_center(&system) {
        log::warn!("alerts evidence: not running from a bundle, so no notification center");
        return;
    }
    pause(cx, 1500).await;
    let ids = super::system::macos::delivered_ids(&system).await;
    log::warn!("alerts evidence: {} delivered in Notification Centre: {ids:?}", ids.len());
}

#[cfg(target_os = "macos")]
fn system_has_center(system: &super::system::macos::MacSystem) -> bool {
    use super::system::AlertSystem;
    system.has_notification_center()
}

#[cfg(not(target_os = "macos"))]
async fn log_delivered(_: &mut AsyncApp) {}
