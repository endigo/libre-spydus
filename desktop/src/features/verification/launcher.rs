//! Starting the helper window and reading its answer (docs/desktop-app.md §11.2). The app runs
//! `current_exe() --verify-helper …` and reads its stdout on a background thread. Tests replace
//! the launcher with their own (`set_launcher`).

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

use futures::FutureExt;
use futures::channel::oneshot;
use futures::future::BoxFuture;

use super::helper::{self, HelperArgs};

/// How a check ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HelperOutcome {
    /// `aws-waf-token=<value>`.
    Cookie(String),
    /// The reader closed the window.
    Closed,
    /// The helper couldn't start or stopped without a token (the detail is for the log).
    Failed(String),
}

/// One running check: its outcome, and a way to stop it (Cancel in the dialog).
pub struct HelperRun {
    pub outcome: BoxFuture<'static, HelperOutcome>,
    pub stop: Box<dyn FnOnce() + Send>,
}

impl HelperRun {
    /// A run that has already ended.
    pub fn ready(outcome: HelperOutcome) -> Self {
        Self { outcome: futures::future::ready(outcome).boxed(), stop: Box::new(|| {}) }
    }
}

/// Starts the helper window for one library.
pub trait HelperLauncher: Send + Sync {
    fn launch(&self, args: &HelperArgs) -> HelperRun;
}

/// The real one: this binary as a child process.
pub struct ProcessLauncher;

impl HelperLauncher for ProcessLauncher {
    fn launch(&self, args: &HelperArgs) -> HelperRun {
        let exe = match std::env::current_exe() {
            Ok(exe) => exe,
            Err(error) => return HelperRun::ready(HelperOutcome::Failed(error.to_string())),
        };
        let spawned = Command::new(exe)
            .args(args.to_args())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn();
        let mut child = match spawned {
            Ok(child) => child,
            Err(error) => return HelperRun::ready(HelperOutcome::Failed(error.to_string())),
        };
        let Some(stdout) = child.stdout.take() else {
            let _ = child.kill();
            // copy-exempt: log detail
            return HelperRun::ready(HelperOutcome::Failed("no stdout".into()));
        };
        let child = Arc::new(Mutex::new(child));
        let (sender, receiver) = oneshot::channel();
        let waiting = child.clone();
        std::thread::spawn(move || {
            // Reads until the helper exits (no lock is held meanwhile, so Cancel can kill it).
            let cookie =
                BufReader::new(stdout).lines().map_while(Result::ok).find_map(|line| helper::parse_cookie_line(&line));
            let status = waiting.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).wait();
            let _ = sender.send(outcome(cookie, status));
        });
        HelperRun {
            outcome: receiver.map(|answer| answer.unwrap_or(HelperOutcome::Closed)).boxed(),
            stop: Box::new(move || kill(&child)),
        }
    }
}

fn outcome(cookie: Option<String>, status: std::io::Result<std::process::ExitStatus>) -> HelperOutcome {
    if let Some(cookie) = cookie {
        return HelperOutcome::Cookie(cookie);
    }
    match status {
        Ok(status) if status.code() == Some(helper::EXIT_CLOSED) => HelperOutcome::Closed,
        Ok(status) => HelperOutcome::Failed(status.to_string()),
        Err(error) => HelperOutcome::Failed(error.to_string()),
    }
}

fn kill(child: &Mutex<Child>) {
    let _ = child.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).kill();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    fn status(code: i32) -> std::io::Result<std::process::ExitStatus> {
        use std::os::unix::process::ExitStatusExt;
        Ok(std::process::ExitStatus::from_raw(code << 8))
    }

    #[cfg(unix)]
    #[test]
    fn reads_the_outcome_from_stdout_and_the_exit_code() {
        assert_eq!(outcome(Some("aws-waf-token=a".into()), status(0)), HelperOutcome::Cookie("aws-waf-token=a".into()));
        assert_eq!(outcome(None, status(1)), HelperOutcome::Closed);
        assert!(matches!(outcome(None, status(2)), HelperOutcome::Failed(_)));
        assert!(matches!(outcome(None, status(0)), HelperOutcome::Failed(_)));
    }
}
