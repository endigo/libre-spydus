//! The helper window (docs/desktop-app.md §11.2): `librespydus --verify-helper <origin>
//! <user-agent> [<library name>]`, started by the app as a child process so its tao event loop
//! never meets GPUI's. It shows the library's page in a wry web view, with the core's User-Agent,
//! for the **reader** to complete the check. About every 500 ms it reads the web view's cookies;
//! once the library's `aws-waf-token` is there and no page is loading, it prints
//! `aws-waf-token=<value>` on one stdout line and exits 0. Closing the window exits 1.
//!
//! It never injects scripts into the page and never interacts with the check
//! (native-apps.md §2.3). The web view is private (incognito), so every check starts afresh and a
//! token the library rejected can't come back.

/// The flag that turns the binary into the helper.
pub const FLAG: &str = "--verify-helper";

/// Exit codes the app reads.
pub const EXIT_VERIFIED: i32 = 0;
pub const EXIT_CLOSED: i32 = 1;
pub const EXIT_FAILED: i32 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelperArgs {
    /// The library's origin, without a trailing slash.
    pub origin: String,
    pub user_agent: String,
    /// The window's title: the library's name (else its host).
    pub title: String,
}

impl HelperArgs {
    /// The page the web view opens: `origin + "/"` (core/README.md "Human verification").
    pub fn page_url(&self) -> String {
        format!("{}/", self.origin)
    }

    pub fn host(&self) -> String {
        super::cookie::host_of(&self.origin)
    }

    /// The arguments after the binary that start this helper.
    pub fn to_args(&self) -> Vec<String> {
        vec![FLAG.to_owned(), self.origin.clone(), self.user_agent.clone(), self.title.clone()]
    }
}

/// `None` when the process wasn't started as the helper (`args` without the binary's name);
/// otherwise its arguments, or what's wrong with them.
pub fn parse_args(args: &[String]) -> Option<Result<HelperArgs, String>> {
    let (first, rest) = args.split_first()?;
    if first != FLAG {
        return None;
    }
    Some(parse_rest(rest))
}

fn parse_rest(rest: &[String]) -> Result<HelperArgs, String> {
    // copy-exempt: developer-facing usage errors on stderr
    let usage = || format!("usage: librespydus {FLAG} <origin> <user-agent> [<library name>]");
    let (origin, user_agent, title) = match rest {
        [origin, user_agent] => (origin, user_agent, None),
        [origin, user_agent, title] => (origin, user_agent, Some(title)),
        _ => return Err(usage()),
    };
    let origin = origin.trim().trim_end_matches('/').to_owned();
    let is_web = origin.starts_with("https://") || origin.starts_with("http://");
    if !is_web || super::cookie::host_of(&origin).is_empty() {
        return Err(usage());
    }
    if user_agent.trim().is_empty() {
        return Err(usage());
    }
    let title = title.map(|t| t.trim().to_owned()).filter(|t| !t.is_empty());
    let title = title.unwrap_or_else(|| super::cookie::host_of(&origin));
    Ok(HelperArgs { origin, user_agent: user_agent.clone(), title })
}

/// The one stdout line the helper prints for `cookie` (`aws-waf-token=<value>`).
pub fn cookie_line(cookie: &str) -> String {
    format!("{cookie}\n")
}

/// The cookie in one line of the helper's stdout, if it is one.
pub fn parse_cookie_line(line: &str) -> Option<String> {
    let line = line.trim();
    let value = line.strip_prefix(super::cookie::NAME)?.strip_prefix('=')?;
    (!value.is_empty() && !value.contains(char::is_whitespace)).then(|| line.to_owned())
}

/// Runs the helper when the process was started as one, then exits; `false` otherwise.
pub fn run_if_requested() -> bool {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse_args(&args) {
        None => false,
        Some(Err(usage)) => {
            eprintln!("{usage}");
            std::process::exit(EXIT_FAILED);
        }
        Some(Ok(args)) => std::process::exit(run(&args)),
    }
}

fn print_cookie(cookie: &str) -> ! {
    use std::io::Write;
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(cookie_line(cookie).as_bytes());
    let _ = out.flush();
    std::process::exit(EXIT_VERIFIED);
}

fn run(args: &HelperArgs) -> i32 {
    #[cfg(debug_assertions)]
    if std::env::var("LS_FAKE_VERIFICATION").is_ok_and(|value| value == "1") {
        // Demo evidence and tests: a fake token after 2 s, no window, no page loaded.
        let _ = args;
        std::thread::sleep(std::time::Duration::from_secs(2));
        // copy-exempt: a fake cookie value (debug only)
        print_cookie(&format!("{}=fake-verification-token", super::cookie::NAME));
    }
    platform::run(args)
}

#[cfg(target_os = "macos")]
mod platform {
    use std::cell::Cell;
    use std::rc::Rc;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    use tao::dpi::LogicalSize;
    use tao::event::{Event, StartCause, WindowEvent};
    use tao::event_loop::{ControlFlow, EventLoop};
    use tao::window::WindowBuilder;
    use wry::{PageLoadEvent, WebViewBuilder};

    use super::super::cookie::{self, Candidate};
    use super::{EXIT_CLOSED, EXIT_FAILED, HelperArgs, print_cookie};

    const POLL: Duration = Duration::from_millis(500);

    pub(super) fn run(args: &HelperArgs) -> i32 {
        let event_loop = EventLoop::new();
        let window = match WindowBuilder::new()
            .with_title(args.title.clone())
            .with_inner_size(LogicalSize::new(900.0, 760.0))
            .with_min_inner_size(LogicalSize::new(480.0, 480.0))
            .build(&event_loop)
        {
            Ok(window) => window,
            Err(error) => {
                eprintln!("verify-helper: {error}");
                return EXIT_FAILED;
            }
        };
        let loading = Rc::new(Cell::new(true));
        let loaded_once = Rc::new(Cell::new(false));
        let webview = {
            let loading = loading.clone();
            let loaded_once = loaded_once.clone();
            WebViewBuilder::new()
                .with_url(args.page_url())
                .with_user_agent(args.user_agent.clone())
                .with_incognito(true)
                .with_on_page_load_handler(move |event, _url| {
                    loading.set(matches!(event, PageLoadEvent::Started));
                    if matches!(event, PageLoadEvent::Finished) {
                        loaded_once.set(true);
                    }
                })
                .build(&window)
        };
        let webview = match webview {
            Ok(webview) => webview,
            Err(error) => {
                eprintln!("verify-helper: {error}");
                return EXIT_FAILED;
            }
        };
        window.set_focus();
        let host = args.host();
        let parent = std::os::unix::process::parent_id();
        #[cfg(feature = "evidence")]
        let mut snapshot = super::evidence::Snapshot::from_env();

        event_loop.run(move |event, _, control_flow| match event {
            Event::NewEvents(StartCause::Init) => *control_flow = ControlFlow::WaitUntil(Instant::now() + POLL),
            Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
                *control_flow = ControlFlow::WaitUntil(Instant::now() + POLL);
                // The app quit: nobody is waiting for the token.
                if std::os::unix::process::parent_id() != parent {
                    std::process::exit(EXIT_CLOSED);
                }
                #[cfg(feature = "evidence")]
                if let Some(snapshot) = snapshot.as_mut() {
                    snapshot.tick(&webview, loaded_once.get() && !loading.get());
                    return;
                }
                // Only between page loads: the check's own redirect isn't the library's page yet.
                if loading.get() || !loaded_once.get() {
                    return;
                }
                let Ok(cookies) = webview.cookies() else { return };
                let candidates: Vec<Candidate> = cookies
                    .iter()
                    .map(|cookie| Candidate {
                        name: cookie.name().to_owned(),
                        value: cookie.value().to_owned(),
                        domain: cookie.domain().unwrap_or_default().to_owned(),
                        expires: cookie.expires_datetime().map(|at| at.unix_timestamp()),
                    })
                    .collect();
                if let Some(header) = cookie::header(&candidates, &host, now()) {
                    print_cookie(&header);
                }
            }
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => std::process::exit(EXIT_CLOSED),
            _ => {}
        })
    }

    fn now() -> i64 {
        SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |elapsed| elapsed.as_secs() as i64)
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    use super::{EXIT_FAILED, HelperArgs};

    /// Windows and Linux builds come later (docs/desktop-app.md §11).
    pub(super) fn run(_args: &HelperArgs) -> i32 {
        EXIT_FAILED
    }
}

/// Evidence only (cargo feature `evidence`): `LS_VERIFY_HELPER_SNAPSHOT=<png>` saves the web
/// view's own rendering 4 s after the page first finished loading, then closes the helper (exit
/// 1). It only looks: no input, no script. `screencapture` doesn't work over SSH.
#[cfg(all(feature = "evidence", target_os = "macos"))]
mod evidence {
    use std::ffi::CString;
    use std::time::{Duration, Instant};

    use block2::RcBlock;
    use objc2::rc::Retained;
    use objc2::runtime::{AnyObject, Bool};
    use objc2::{class, msg_send};
    use wry::{WebView, WebViewExtMacOS};

    use super::EXIT_CLOSED;

    pub(super) struct Snapshot {
        path: String,
        ready_at: Option<Instant>,
        taken: bool,
    }

    impl Snapshot {
        pub(super) fn from_env() -> Option<Self> {
            let path = std::env::var("LS_VERIFY_HELPER_SNAPSHOT").ok().filter(|path| !path.is_empty())?;
            Some(Self { path, ready_at: None, taken: false })
        }

        pub(super) fn tick(&mut self, webview: &WebView, loaded: bool) {
            if self.taken {
                return;
            }
            if loaded && self.ready_at.is_none() {
                self.ready_at = Some(Instant::now() + Duration::from_secs(4));
            }
            if self.ready_at.is_some_and(|at| Instant::now() >= at) {
                self.taken = true;
                save(&webview.webview(), &self.path);
            }
        }
    }

    fn quit() {
        std::process::exit(EXIT_CLOSED);
    }

    /// `takeSnapshotWithConfiguration:` → TIFF → PNG on disk, then exit.
    fn save<T: objc2::Message>(view: &Retained<T>, path: &str) {
        let view = Retained::as_ptr(view) as *mut AnyObject;
        let path = CString::new(path).unwrap_or_default();
        let handler = RcBlock::new(move |image: *mut AnyObject, _error: *mut AnyObject| {
            if !image.is_null() {
                unsafe {
                    let tiff: *mut AnyObject = msg_send![image, TIFFRepresentation];
                    let rep: *mut AnyObject = msg_send![class!(NSBitmapImageRep), imageRepWithData: tiff];
                    let properties: *mut AnyObject = msg_send![class!(NSDictionary), dictionary];
                    // NSBitmapImageFileTypePNG
                    let png: *mut AnyObject = msg_send![rep, representationUsingType: 4usize, properties: properties];
                    let file: *mut AnyObject = msg_send![class!(NSString), stringWithUTF8String: path.as_ptr()];
                    let written: Bool = msg_send![png, writeToFile: file, atomically: Bool::YES];
                    eprintln!("verify-helper: snapshot written: {}", written.as_bool());
                }
            }
            quit();
        });
        unsafe {
            let _: () = msg_send![
                view,
                takeSnapshotWithConfiguration: std::ptr::null_mut::<AnyObject>(),
                completionHandler: &*handler
            ];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|arg| (*arg).to_owned()).collect()
    }

    #[test]
    fn not_a_helper_without_the_flag() {
        assert_eq!(parse_args(&[]), None);
        assert_eq!(parse_args(&args(&["--demo", "--verify-helper"])), None);
    }

    #[test]
    fn parses_origin_user_agent_and_title() {
        let parsed = parse_args(&args(&[FLAG, "https://camden.spydus.co.uk/", "UA/1.0", "Camden Libraries"]));
        let parsed = parsed.expect("helper").expect("valid");
        assert_eq!(parsed.origin, "https://camden.spydus.co.uk");
        assert_eq!(parsed.user_agent, "UA/1.0");
        assert_eq!(parsed.title, "Camden Libraries");
        assert_eq!(parsed.page_url(), "https://camden.spydus.co.uk/");
        assert_eq!(parsed.host(), "camden.spydus.co.uk");
        assert_eq!(parse_args(&parsed.to_args()[..]), Some(Ok(parsed)));
    }

    #[test]
    fn the_title_defaults_to_the_host() {
        let parsed = parse_args(&args(&[FLAG, "https://camden.spydus.co.uk", "UA"])).unwrap().unwrap();
        assert_eq!(parsed.title, "camden.spydus.co.uk");
    }

    #[test]
    fn rejects_bad_arguments() {
        for bad in [
            args(&[FLAG]),
            args(&[FLAG, "https://camden.spydus.co.uk"]),
            args(&[FLAG, "camden.spydus.co.uk", "UA"]),
            args(&[FLAG, "https://", "UA"]),
            args(&[FLAG, "https://camden.spydus.co.uk", "  "]),
            args(&[FLAG, "https://a", "UA", "T", "extra"]),
        ] {
            assert!(matches!(parse_args(&bad), Some(Err(_))), "{bad:?}");
        }
    }

    #[test]
    fn the_cookie_line_round_trips() {
        let line = cookie_line("aws-waf-token=abc:def/1==");
        assert_eq!(line, "aws-waf-token=abc:def/1==\n");
        assert_eq!(parse_cookie_line(&line).as_deref(), Some("aws-waf-token=abc:def/1=="));
    }

    #[test]
    fn other_lines_are_not_cookies() {
        for line in ["", "aws-waf-token=", "aws-waf-token", "other=1", "aws-waf-tokenx=1", "aws-waf-token=a b"] {
            assert_eq!(parse_cookie_line(line), None, "{line:?}");
        }
    }
}
