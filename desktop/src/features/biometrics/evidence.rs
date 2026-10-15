//! Screenshots (cargo feature `evidence`, **demo mode only**): `LIBRESPYDUS_EVIDENCE=biometrics:<step>`,
//! usually with `LS_FAKE_TOUCH_ID=available|match|nomatch` (mini-knit has no Touch ID).
//! - `offer`: signs out and in again with demo / demo, so the offer shows;
//! - `details`: keeps demo / demo for the library (Profile's switch on);
//! - `turn-on`: Profile's switch turned on, so the sign-in dialog shows `turnOnNote`;
//! - `sign-in-button`: signed out with details kept, the sign-in dialog without the automatic
//!   check (its Touch ID button and the biometric privacy note);
//! - `sign-in-checking`: the same with the automatic check under way (`available` hangs);
//! - `expired`: details kept, then Profile › Developer › Expire session, so the app signs back in.

use std::time::Duration;

use gpui_kit::{App, Window};

use super::{SignInDetails, with};
use crate::state::AppState;

pub fn start(window: &mut Window, cx: &mut App) {
    if !AppState::global(cx).read(cx).is_demo() {
        return;
    }
    let Some(step) = crate::features::onboarding::evidence_step("biometrics") else { return };
    window
        .spawn(cx, async move |cx| {
            cx.background_executor().timer(Duration::from_millis(1500)).await;
            let _ = cx.update(|_, cx| run(&step, cx));
        })
        .detach();
}

fn keep_demo_details(state: &mut AppState, cx: &mut App) {
    let origin = state.backend.origin();
    // copy-exempt: the demo account's sign-in details (screenshots)
    let details = SignInDetails::new("demo", "demo");
    with(state, cx, |biometrics| {
        if biometrics.vault.store(&origin, &details).is_ok() {
            biometrics.username = Some(details.username.clone());
        }
    });
}

fn run(step: &str, cx: &mut App) {
    AppState::global(cx).update(cx, |state, cx| {
        match step {
            "offer" => {
                state.sign_out(cx);
                // copy-exempt: the demo account's sign-in details (screenshots)
                state.sign_in("demo".into(), "demo".into(), cx).detach();
            }
            "details" => keep_demo_details(state, cx),
            "turn-on" => state.set_biometric_sign_in(true, cx),
            "sign-in-button" | "sign-in-checking" => {
                state.sign_out(cx);
                keep_demo_details(state, cx);
                let skip = step == "sign-in-button";
                with(state, cx, |biometrics| biometrics.skips_automatic_check = skip);
                state.request_sign_in(cx);
            }
            "expired" => {
                keep_demo_details(state, cx);
                state.expire_session_for_debugging(cx);
            }
            _ => {}
        }
        cx.notify();
    });
}
