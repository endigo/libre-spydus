//! "Confirm you're human" for libraries behind an AWS WAF check (docs/desktop-app.md §11.2, E2;
//! native-apps.md §6 "Verification"; a port of iOS `VerificationView` and `AppModel.report` /
//! `completeVerification`, laid out for a window).
//!
//! - `AppState::report` on `HumanVerificationRequired` calls `on_required`, which records that this
//!   library needs the check (in this feature's own global entity, `Verification`) and returns the
//!   error the screen shows. After two completed checks that the library still answers with
//!   `HumanVerificationRequired`, it returns iOS's `verificationUnavailable` error instead.
//! - `watch` presents a white dialog: `verification.message` with Open check / Cancel. Open check
//!   starts the helper window (`helper`, the same binary as a child process); on its cookie the app
//!   calls `AppState::complete_verification`, the dialog closes and the failed screen retries
//!   through `reload_key`. Closing the helper window brings the dialog back with a note.
//! - Profile shows "Verified on this computer" + Clear verification (D2); clearing it also
//!   forgets this library's completed checks, as on iOS.
//!
//! Nothing here solves, automates or bypasses the check (native-apps.md §2.3).

mod cookie;
#[cfg(feature = "evidence")]
mod evidence;
mod helper;
mod launcher;

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use gpui_kit::component::WindowExt;
use gpui_kit::{
    App, AppContext, Context, Entity, Global, InteractiveElement, IntoElement, ParentElement, Role, SharedString,
    StatefulInteractiveElement, Styled, Task, TestSupportExt, Window, div, prelude::FluentBuilder, px,
};
use librespydus_core::SpydusError;

use crate::copy::Copy;
use crate::state::formatting::capitalised_first;
use crate::state::{AppState, UserFacingError};
use crate::theme::{self, Font, colors, spacing};
use crate::ui::dialog::{DialogButton, dialog_title, sheet_dialog};
use crate::ui::icons::{Lucide, icon};
use crate::ui::style::FontExt;

pub use self::cookie::{Candidate, domain_matches, header as cookie_header, host_of};
pub use self::helper::{
    FLAG as HELPER_FLAG, HelperArgs, cookie_line, parse_args as parse_helper_args, parse_cookie_line,
};
pub use self::launcher::{HelperLauncher, HelperOutcome, HelperRun, ProcessLauncher};

/// After this many completed checks that still end in `HumanVerificationRequired`, the library is
/// "unavailable" (iOS `AppModel.report`).
pub const MAX_COMPLETIONS: u32 = 2;

/// Where the check stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Nothing to show.
    Idle,
    /// The dialog asks the reader to open the check, with a note after a failed attempt.
    Asking(Option<Note>),
    /// The helper window is open.
    Checking,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Note {
    /// The reader closed the helper window before the library let them in.
    WindowClosed,
    /// The helper couldn't start, or stopped without a token.
    HelperFailed,
}

/// This feature's state: one global entity (docs/desktop-app.md §11.1, "State").
pub struct Verification {
    launcher: Arc<dyn HelperLauncher>,
    /// The library the dialog is for.
    origin: Option<String>,
    phase: Phase,
    /// Completed checks per origin since the last Clear verification (iOS `verificationCompletions`).
    completions: HashMap<String, u32>,
    /// The origin last seen verified, to notice Clear verification.
    verified_origin: Option<String>,
    run_generation: u64,
    stop_run: Option<Box<dyn FnOnce() + Send>>,
    _run: Option<Task<()>>,
    // The dialog (`present`).
    dialog_open: bool,
    dialog_waiting: bool,
}

struct GlobalVerification(Entity<Verification>);

impl Global for GlobalVerification {}

impl Verification {
    /// The app's one, made on first use.
    pub fn global(cx: &mut App) -> Entity<Verification> {
        if let Some(global) = cx.try_global::<GlobalVerification>() {
            return global.0.clone();
        }
        let entity = cx.new(|_| Verification::new(Arc::new(ProcessLauncher)));
        cx.set_global(GlobalVerification(entity.clone()));
        entity
    }

    fn new(launcher: Arc<dyn HelperLauncher>) -> Self {
        Self {
            launcher,
            origin: None,
            phase: Phase::Idle,
            completions: HashMap::new(),
            verified_origin: None,
            run_generation: 0,
            stop_run: None,
            _run: None,
            dialog_open: false,
            dialog_waiting: false,
        }
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// The library the dialog is for.
    pub fn origin(&self) -> Option<&str> {
        self.origin.as_deref()
    }

    /// Completed checks for `origin` since the last Clear verification.
    pub fn completions(&self, origin: &str) -> u32 {
        self.completions.get(origin).copied().unwrap_or(0)
    }

    /// `origin` answered `HumanVerificationRequired`: ask for the check, unless it already failed
    /// `MAX_COMPLETIONS` times. Returns `false` when the library is "unavailable" instead.
    fn require(&mut self, origin: &str, cx: &mut Context<Self>) -> bool {
        if self.completions(origin) >= MAX_COMPLETIONS {
            return false;
        }
        if self.phase == Phase::Idle || self.origin.as_deref() != Some(origin) {
            self.stop();
            self.origin = Some(origin.to_owned());
            self.phase = Phase::Asking(None);
            cx.notify();
        }
        true
    }

    /// Open check: starts the helper window for the library the dialog is for.
    fn start(&mut self, args: HelperArgs, cx: &mut Context<Self>) {
        self.stop();
        self.run_generation += 1;
        let generation = self.run_generation;
        let run = self.launcher.launch(&args);
        self.stop_run = Some(run.stop);
        self.phase = Phase::Checking;
        let outcome = run.outcome;
        self._run = Some(cx.spawn(async move |this, cx| {
            let outcome = outcome.await;
            let _ = this.update(cx, |this, cx| this.finish(generation, outcome, cx));
        }));
        cx.notify();
    }

    fn finish(&mut self, generation: u64, outcome: HelperOutcome, cx: &mut Context<Self>) {
        if generation != self.run_generation {
            return;
        }
        self.stop_run = None;
        match outcome {
            HelperOutcome::Cookie(cookie) => {
                let state = AppState::global(cx);
                let current = state.read(cx).backend().origin();
                let Some(origin) = self.origin.take().filter(|origin| *origin == current) else {
                    self.phase = Phase::Idle;
                    cx.notify();
                    return;
                };
                *self.completions.entry(origin).or_default() += 1;
                self.phase = Phase::Idle;
                cx.notify();
                state.update(cx, |state, cx| state.complete_verification(cookie, cx));
            }
            HelperOutcome::Closed => {
                self.phase = Phase::Asking(Some(Note::WindowClosed));
                cx.notify();
            }
            HelperOutcome::Failed(detail) => {
                log::warn!("verification helper: {detail}");
                self.phase = Phase::Asking(Some(Note::HelperFailed));
                cx.notify();
            }
        }
    }

    /// Cancel (or Escape): closes the helper window, if open, and forgets the request.
    fn cancel(&mut self, cx: &mut Context<Self>) {
        self.stop();
        if self.phase != Phase::Idle {
            self.phase = Phase::Idle;
            self.origin = None;
            cx.notify();
        }
    }

    fn stop(&mut self) {
        self.run_generation += 1;
        self._run = None;
        if let Some(stop) = self.stop_run.take() {
            stop();
        }
    }

    /// AppState changed: a Clear verification forgets the completed checks; another library
    /// drops a request for the old one.
    fn state_changed(&mut self, origin: String, verified: bool, cx: &mut Context<Self>) {
        if verified {
            self.verified_origin = Some(origin.clone());
        } else if self.verified_origin.as_deref() == Some(origin.as_str()) {
            self.verified_origin = None;
            self.completions.remove(&origin);
        }
        if self.origin.as_ref().is_some_and(|asked| *asked != origin) {
            self.cancel(cx);
        }
    }
}

/// Tests: replace how the helper window starts.
pub fn set_launcher(launcher: Arc<dyn HelperLauncher>, cx: &mut App) {
    Verification::global(cx).update(cx, |verification, _| verification.launcher = launcher);
}

/// `AppState::report` on `HumanVerificationRequired`: records that this library needs the check
/// and returns what to show; after `MAX_COMPLETIONS` checks that didn't help, the library is
/// "unavailable" (iOS `verificationUnavailable`).
pub fn on_required(state: &mut AppState, error: &SpydusError, cx: &mut Context<AppState>) -> UserFacingError {
    let origin = state.backend().origin();
    let name = state.library_name();
    let model = Verification::global(cx);
    if model.update(cx, |verification, cx| verification.require(&origin, cx)) {
        UserFacingError::from_core(error, &name)
    } else {
        UserFacingError::verification_unavailable(&name, Some(error.to_string()))
    }
}

/// Called once by the shell at start: present the check when `on_required` asks for it.
pub fn watch(window: &mut Window, cx: &mut App) {
    let model = Verification::global(cx);
    window.observe(&model, cx, |_, window, cx| present(window, cx)).detach();
    let state = AppState::global(cx);
    window
        .observe(&state, cx, move |state, _, cx| {
            let (origin, verified) = {
                let state = state.read(cx);
                (state.backend().origin(), state.is_verified())
            };
            model.update(cx, |verification, cx| verification.state_changed(origin, verified, cx));
        })
        .detach();
    #[cfg(feature = "evidence")]
    evidence::schedule(window, cx);
    present(window, cx);
}

/// Called first thing in `main`: when the process was started as the check's helper window
/// (`librespydus --verify-helper …`), runs it and returns `true` (main then exits).
pub fn run_helper_if_requested() -> bool {
    helper::run_if_requested()
}

// ---- The dialog ----

/// Opens the dialog while the check is asked for, and closes it after. Another dialog on screen
/// (sign in closing, say): look again shortly.
fn present(window: &mut Window, cx: &mut App) {
    let model = Verification::global(cx);
    let (asking, open, waiting) = {
        let verification = model.read(cx);
        (verification.phase != Phase::Idle, verification.dialog_open, verification.dialog_waiting)
    };
    if asking && !open && !waiting {
        if window.has_active_dialog(cx) {
            model.update(cx, |verification, _| verification.dialog_waiting = true);
            window
                .spawn(cx, async move |cx| {
                    cx.background_executor().timer(Duration::from_millis(400)).await;
                    let _ = cx.update(|window, cx| {
                        Verification::global(cx).update(cx, |verification, _| verification.dialog_waiting = false);
                        present(window, cx);
                    });
                })
                .detach();
            return;
        }
        model.update(cx, |verification, _| verification.dialog_open = true);
        open_dialog(window, cx);
    } else if !asking && open {
        model.update(cx, |verification, _| verification.dialog_open = false);
        window.close_dialog(cx);
    }
}

/// Cancel, or Escape (which closes the dialog itself).
fn dismissed(cx: &mut App) {
    Verification::global(cx).update(cx, |verification, cx| {
        verification.dialog_open = false;
        verification.cancel(cx);
    });
}

fn open_check(cx: &mut App) {
    let state = AppState::global(cx).read(cx);
    let args = HelperArgs {
        origin: state.backend().origin(),
        user_agent: librespydus_core::user_agent(),
        title: state.library_name(),
    };
    Verification::global(cx).update(cx, |verification, cx| verification.start(args, cx));
}

fn open_dialog(window: &mut Window, cx: &mut App) {
    let model = Verification::global(cx);
    window.open_dialog(cx, move |dialog, _, cx| {
        let library = AppState::global(cx).read(cx).library_name();
        let phase = model.read(cx).phase;
        sheet_dialog(dialog)
            .w(px(460.))
            .overlay_closable(false)
            .on_cancel(|_, _, cx| {
                dismissed(cx);
                true
            })
            .title(dialog_title(Copy::Verification::title))
            .child(body(&library, phase))
            .footer(footer(phase))
    });
}

fn body(library: &str, phase: Phase) -> impl IntoElement {
    let message = capitalised_first(&Copy::Verification::message(library));
    let note = match phase {
        Phase::Asking(Some(Note::WindowClosed)) => Some(Copy::Verification::windowClosed),
        Phase::Asking(Some(Note::HelperFailed)) => Some(Copy::Verification::helperFailed),
        _ => None,
    };
    let secondary = theme::color(colors::ON_SHEET_SECONDARY);
    div()
        .id("verification")
        .test_support()
        .flex()
        .flex_col()
        .gap(px(spacing::M))
        .child(
            div()
                .id("verification-message")
                .test_support()
                .role(Role::Label)
                .aria_label(SharedString::from(message.clone()))
                .typography(Font::Body)
                .text_color(secondary)
                .child(message),
        )
        .when_some(note, |body, note| {
            body.child(
                div()
                    .id("verification-note")
                    .test_support()
                    .role(Role::Status)
                    .aria_label(SharedString::from(note))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(spacing::S))
                    .px(px(spacing::M))
                    .py(px(spacing::S))
                    .rounded(px(theme::radius::FIELD))
                    .bg(theme::color(colors::CHIP_ON_SHEET))
                    .typography(Font::Caption)
                    .text_color(theme::color(colors::ON_SHEET))
                    .child(icon(Lucide::Info, px(14.), secondary))
                    .child(note),
            )
        })
        .when(phase == Phase::Checking, |body| {
            body.child(
                div()
                    .id("verification-waiting")
                    .test_support()
                    .role(Role::ProgressIndicator)
                    .aria_label(SharedString::from(Copy::Verification::closesByItself))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(spacing::S))
                    .typography(Font::Caption)
                    .text_color(secondary)
                    .child(gpui_kit::component::spinner::Spinner::new().color(secondary))
                    .child(Copy::Verification::closesByItself),
            )
        })
}

fn footer(phase: Phase) -> impl IntoElement {
    div()
        .w_full()
        .flex()
        .flex_row()
        .justify_end()
        .gap(px(spacing::S))
        .child(DialogButton::new(
            "verification-cancel",
            Copy::Common::cancel,
            Rc::new(|window: &mut Window, cx: &mut App| {
                // Before `dismissed`, so `present` doesn't close a second dialog.
                Verification::global(cx).update(cx, |verification, _| verification.dialog_open = false);
                window.close_dialog(cx);
                dismissed(cx);
            }),
        ))
        .when(phase != Phase::Checking, |footer| {
            footer.child(
                DialogButton::new(
                    "verification-open",
                    Copy::Verification::openCheck,
                    Rc::new(|_: &mut Window, cx: &mut App| open_check(cx)),
                )
                .confirming(true),
            )
        })
}
