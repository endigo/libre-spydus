//! The Touch ID check (docs/native-apps.md §5.14; iOS `BiometricCheck`): `LocalAuthentication`'s
//! `LAContext` with `.deviceOwnerAuthenticationWithBiometrics` on macOS, a fake in tests and for
//! screenshots (`LS_FAKE_TOUCH_ID`, debug builds).

use futures::FutureExt;
use futures::future::LocalBoxFuture;

use super::vault::{BiometricFailure, Biometry};

/// The computer's biometric check.
pub trait BiometricCheck: Send + Sync {
    /// The check this computer can run now (`canEvaluatePolicy`), or `None`.
    fn biometry(&self) -> Option<Biometry>;
    /// Asks for the check with `reason` ("Sign in to City of Canning"); the fallback button says
    /// `biometrics.enterPin`.
    fn run(&self, reason: String) -> LocalBoxFuture<'static, Result<(), BiometricFailure>>;
}

/// No check on this computer: nothing is offered or kept (the default in tests).
pub struct NoCheck;

impl BiometricCheck for NoCheck {
    fn biometry(&self) -> Option<Biometry> {
        None
    }

    fn run(&self, _reason: String) -> LocalBoxFuture<'static, Result<(), BiometricFailure>> {
        futures::future::ready(Err(BiometricFailure::Unavailable)).boxed_local()
    }
}

/// The system's check (macOS `LAContext`; nothing elsewhere).
pub struct SystemCheck;

#[cfg(target_os = "macos")]
impl BiometricCheck for SystemCheck {
    fn biometry(&self) -> Option<Biometry> {
        macos::biometry()
    }

    fn run(&self, reason: String) -> LocalBoxFuture<'static, Result<(), BiometricFailure>> {
        macos::run(reason)
    }
}

#[cfg(not(target_os = "macos"))]
impl BiometricCheck for SystemCheck {
    fn biometry(&self) -> Option<Biometry> {
        None
    }

    fn run(&self, _reason: String) -> LocalBoxFuture<'static, Result<(), BiometricFailure>> {
        futures::future::ready(Err(BiometricFailure::Unavailable)).boxed_local()
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use std::sync::Mutex;

    use block2::RcBlock;
    use futures::FutureExt;
    use futures::channel::oneshot;
    use futures::future::LocalBoxFuture;
    use objc2::runtime::Bool;
    use objc2_foundation::{NSError, NSString};
    use objc2_local_authentication::{LABiometryType, LAContext, LAError, LAPolicy};

    use super::super::vault::{BiometricFailure, Biometry};
    use crate::copy::Copy;

    const POLICY: LAPolicy = LAPolicy::DeviceOwnerAuthenticationWithBiometrics;

    pub fn biometry() -> Option<Biometry> {
        // SAFETY: a fresh context, used on this thread only.
        unsafe {
            let context = LAContext::new();
            context.canEvaluatePolicy_error(POLICY).ok()?;
            Some(match context.biometryType() {
                LABiometryType::FaceID => Biometry::FaceId,
                LABiometryType::OpticID => Biometry::OpticId,
                _ => Biometry::TouchId,
            })
        }
    }

    pub fn run(reason: String) -> LocalBoxFuture<'static, Result<(), BiometricFailure>> {
        // SAFETY: the context stays alive (moved into the future) until the reply arrives; the
        // reply block only sends on a channel, so it may run on any queue.
        unsafe {
            let context = LAContext::new();
            context.setLocalizedFallbackTitle(Some(&NSString::from_str(Copy::Biometrics::enterPin)));
            if context.canEvaluatePolicy_error(POLICY).is_err() {
                return futures::future::ready(Err(BiometricFailure::Unavailable)).boxed_local();
            }
            let (sender, receiver) = oneshot::channel();
            let sender = Mutex::new(Some(sender));
            let reply = RcBlock::new(move |success: Bool, error: *mut NSError| {
                let result = if success.as_bool() {
                    Ok(())
                } else {
                    Err(failure(if error.is_null() { 0 } else { (*error).code() }))
                };
                if let Some(sender) = sender.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).take() {
                    let _ = sender.send(result);
                }
            });
            context.evaluatePolicy_localizedReason_reply(POLICY, &NSString::from_str(&reason), &reply);
            async move {
                let result = receiver.await.unwrap_or(Err(BiometricFailure::Unavailable));
                drop(context);
                result
            }
            .boxed_local()
        }
    }

    /// An `LAError` code: the reader said no, or the check can't run.
    fn failure(code: isize) -> BiometricFailure {
        let declined = [
            LAError::UserCancel,
            LAError::UserFallback,
            LAError::AuthenticationFailed,
            LAError::SystemCancel,
            LAError::AppCancel,
        ];
        if declined.iter().any(|error| error.0 == code) {
            BiometricFailure::Declined
        } else {
            BiometricFailure::Unavailable
        }
    }
}

/// What a fake check does when it runs.
#[cfg(debug_assertions)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FakeOutcome {
    Pass,
    Fail(BiometricFailure),
    /// Never answers (screenshots of a check under way).
    Hang,
}

/// Tests and screenshots: a check that is available (or not) and passes, fails or hangs, and
/// counts its runs.
#[cfg(debug_assertions)]
pub struct FakeCheck {
    state: std::sync::Mutex<FakeState>,
    delay: Option<std::time::Duration>,
}

#[cfg(debug_assertions)]
struct FakeState {
    biometry: Option<Biometry>,
    outcome: FakeOutcome,
    runs: usize,
    reasons: Vec<String>,
}

#[cfg(debug_assertions)]
impl FakeCheck {
    /// Touch ID that passes.
    pub fn new() -> Self {
        Self::with(Some(Biometry::TouchId), FakeOutcome::Pass)
    }

    pub fn with(biometry: Option<Biometry>, outcome: FakeOutcome) -> Self {
        Self {
            state: std::sync::Mutex::new(FakeState { biometry, outcome, runs: 0, reasons: Vec::new() }),
            delay: None,
        }
    }

    /// `LS_FAKE_TOUCH_ID=available|match|nomatch`: available and hanging, passing or failing,
    /// after a short pause like the real prompt's.
    pub fn from_env() -> Option<Self> {
        let outcome = match std::env::var("LS_FAKE_TOUCH_ID").ok()?.as_str() {
            "available" => FakeOutcome::Hang,
            "match" => FakeOutcome::Pass,
            "nomatch" => FakeOutcome::Fail(BiometricFailure::Declined),
            _ => return None,
        };
        let mut check = Self::with(Some(Biometry::TouchId), outcome);
        check.delay = Some(std::time::Duration::from_millis(800));
        Some(check)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, FakeState> {
        self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn set_biometry(&self, biometry: Option<Biometry>) {
        self.lock().biometry = biometry;
    }

    pub fn set_outcome(&self, outcome: FakeOutcome) {
        self.lock().outcome = outcome;
    }

    /// How many checks ran.
    pub fn runs(&self) -> usize {
        self.lock().runs
    }

    /// The reasons the checks showed.
    pub fn reasons(&self) -> Vec<String> {
        self.lock().reasons.clone()
    }
}

#[cfg(debug_assertions)]
impl Default for FakeCheck {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(debug_assertions)]
impl BiometricCheck for FakeCheck {
    fn biometry(&self) -> Option<Biometry> {
        self.lock().biometry
    }

    fn run(&self, reason: String) -> LocalBoxFuture<'static, Result<(), BiometricFailure>> {
        let outcome = {
            let mut state = self.lock();
            state.runs += 1;
            state.reasons.push(reason);
            if state.biometry.is_none() { FakeOutcome::Fail(BiometricFailure::Unavailable) } else { state.outcome }
        };
        let delay = self.delay;
        async move {
            if let Some(delay) = delay {
                crate::runtime::sleep(delay).await;
            }
            match outcome {
                FakeOutcome::Pass => Ok(()),
                FakeOutcome::Fail(failure) => Err(failure),
                FakeOutcome::Hang => futures::future::pending().await,
            }
        }
        .boxed_local()
    }
}
