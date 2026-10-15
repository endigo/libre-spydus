//! Screenshot steps for the human check (cargo feature `evidence` only; docs/desktop-app.md §9):
//! `LIBRESPYDUS_EVIDENCE=verification:<step>`. Ignored without the prefix.
//!
//! - `dialog`: the dialog, as after a `HumanVerificationRequired` (1.5 s after start);
//! - `checking`: the dialog while the helper window is open (a launcher that never answers);
//! - `closed`: the dialog after the reader closed the helper window;
//! - `open`: the dialog, then Open check 1.5 s later with the real launcher. With
//!   `LS_FAKE_VERIFICATION=1` the helper answers with a fake cookie after 2 s (no page loads);
//! - `unavailable`: this library already had two completed checks, so the next
//!   `HumanVerificationRequired` is "unavailable".

use std::sync::Arc;
use std::time::Duration;

use futures::FutureExt;
use gpui_kit::{App, Window};

use super::{HelperArgs, HelperLauncher, HelperRun, MAX_COMPLETIONS, Note, Phase, Verification, open_check};
use crate::state::AppState;

struct NeverAnswers;

impl HelperLauncher for NeverAnswers {
    fn launch(&self, _args: &HelperArgs) -> HelperRun {
        HelperRun { outcome: futures::future::pending().boxed(), stop: Box::new(|| {}) }
    }
}

pub(super) fn schedule(window: &mut Window, cx: &mut App) {
    let Some(step) = crate::features::onboarding::evidence_step("verification") else { return };
    if step == "unavailable" {
        let origin = AppState::global(cx).read(cx).backend().origin();
        Verification::global(cx).update(cx, |verification, _| {
            verification.completions.insert(origin, MAX_COMPLETIONS);
        });
        return;
    }
    window
        .spawn(cx, async move |cx| {
            cx.background_executor().timer(Duration::from_millis(1500)).await;
            let _ = cx.update(|_, cx| ask(cx));
            match step.as_str() {
                "checking" => {
                    let _ = cx.update(|_, cx| {
                        super::set_launcher(Arc::new(NeverAnswers), cx);
                        open_check(cx);
                    });
                }
                "closed" => {
                    let _ = cx.update(|_, cx| {
                        Verification::global(cx).update(cx, |verification, cx| {
                            verification.phase = Phase::Asking(Some(Note::WindowClosed));
                            cx.notify();
                        })
                    });
                }
                "open" => {
                    cx.background_executor().timer(Duration::from_millis(1500)).await;
                    let _ = cx.update(|_, cx| open_check(cx));
                }
                _ => {}
            }
        })
        .detach();
}

fn ask(cx: &mut App) {
    let origin = AppState::global(cx).read(cx).backend().origin();
    Verification::global(cx).update(cx, |verification, cx| {
        verification.require(&origin, cx);
    });
}
