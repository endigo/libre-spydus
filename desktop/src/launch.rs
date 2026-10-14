//! Debug launch flags (docs/desktop-app.md §5.7; native-apps.md §5.6). Release builds ignore
//! every flag and start as a reader would.
//!
//! ```text
//! --library <origin>        select a library, skip the picker
//! --demo                    demo mode, signed in as "Test Borrower"
//! --demo-refresh-fails      demo refreshes fail (the saved account stays)
//! --reset                   delete saved state first
//! --data-dir <dir>          keep every file under <dir> and secrets in memory
//! --route <route>           home | search:<q> | results:<q> | detail:<url> | borrowed |
//!                           borrowed:reservations | favourites | profile | libraries | login |
//!                           card | gallery
//! --window <W>x<H>          window size (default 1280x820, at least 960x640)
//! --evidence                no activation or focus; a fixed window position
//! --screenshot <png>        render the window to <png> and quit (cargo feature `evidence`)
//! --screenshot-after <s>    …after <s> seconds (default 6)
//! ```

use std::path::PathBuf;

pub const DEFAULT_WINDOW: (f32, f32) = (1280., 820.);
pub const MIN_WINDOW: (f32, f32) = (960., 640.);
pub const DEFAULT_SCREENSHOT_AFTER: f64 = 6.;

#[derive(Debug, Clone, PartialEq)]
pub enum LaunchRoute {
    Home,
    Search(String),
    Results(String),
    Detail(String),
    Borrowed,
    BorrowedReservations,
    Favourites,
    Profile,
    Libraries,
    Login,
    Card,
    Gallery,
}

impl LaunchRoute {
    pub fn parse(text: &str) -> Result<Self, String> {
        let (name, argument) = match text.split_once(':') {
            Some((name, argument)) => (name, Some(argument)),
            None => (text, None),
        };
        Ok(match (name, argument) {
            ("home", None) => Self::Home,
            ("search", argument) => Self::Search(argument.unwrap_or_default().to_owned()),
            ("results", Some(query)) => Self::Results(query.to_owned()),
            ("detail", Some(url)) => Self::Detail(url.to_owned()),
            ("borrowed", None) => Self::Borrowed,
            ("borrowed", Some("reservations")) => Self::BorrowedReservations,
            ("favourites", None) => Self::Favourites,
            ("profile", None) => Self::Profile,
            ("libraries", None) => Self::Libraries,
            ("login", None) => Self::Login,
            ("card", None) => Self::Card,
            ("gallery", None) => Self::Gallery,
            _ => return Err(format!("unknown --route {text}")),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LaunchOptions {
    pub library: Option<String>,
    pub demo: bool,
    pub demo_refresh_fails: bool,
    pub reset: bool,
    pub data_dir: Option<PathBuf>,
    pub route: Option<LaunchRoute>,
    pub window: (f32, f32),
    pub evidence: bool,
    pub screenshot: Option<PathBuf>,
    pub screenshot_after: f64,
}

impl Default for LaunchOptions {
    fn default() -> Self {
        Self {
            library: None,
            demo: false,
            demo_refresh_fails: false,
            reset: false,
            data_dir: None,
            route: None,
            window: DEFAULT_WINDOW,
            evidence: false,
            screenshot: None,
            screenshot_after: DEFAULT_SCREENSHOT_AFTER,
        }
    }
}

impl LaunchOptions {
    /// Secrets stay in memory: demo mode, or a `--data-dir`.
    pub fn secrets_in_memory(&self) -> bool {
        self.demo || self.data_dir.is_some()
    }

    /// The flags after the program name. Release builds ignore them.
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        if cfg!(debug_assertions) { Self::parse_flags(args) } else { Ok(Self::default()) }
    }

    pub fn parse_flags(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut options = Self::default();
        let mut args = args.into_iter();
        while let Some(flag) = args.next() {
            let mut value = || args.next().ok_or_else(|| format!("{flag} needs a value"));
            match flag.as_str() {
                "--library" => options.library = Some(value()?.trim_end_matches('/').to_owned()),
                "--demo" => options.demo = true,
                "--demo-refresh-fails" => options.demo_refresh_fails = true,
                "--reset" => options.reset = true,
                "--data-dir" => options.data_dir = Some(PathBuf::from(value()?)),
                "--route" => options.route = Some(LaunchRoute::parse(&value()?)?),
                "--window" => {
                    let text = value()?;
                    let (w, h) = text.split_once('x').ok_or_else(|| format!("--window {text}: expected <W>x<H>"))?;
                    let parse = |n: &str| n.parse::<f32>().map_err(|_| format!("--window {text}: expected <W>x<H>"));
                    options.window = (parse(w)?.max(MIN_WINDOW.0), parse(h)?.max(MIN_WINDOW.1));
                }
                "--evidence" => options.evidence = true,
                "--screenshot" => options.screenshot = Some(PathBuf::from(value()?)),
                "--screenshot-after" => {
                    let text = value()?;
                    options.screenshot_after = text
                        .parse::<f64>()
                        .ok()
                        .filter(|s| *s >= 0.)
                        .ok_or_else(|| format!("--screenshot-after {text}: seconds"))?;
                }
                other => return Err(format!("unknown flag {other}")),
            }
        }
        Ok(options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<LaunchOptions, String> {
        LaunchOptions::parse_flags(args.iter().map(|arg| (*arg).to_owned()))
    }

    #[test]
    fn every_flag_parses() {
        let options = parse(&[
            "--library",
            "https://canning.spydus.com/",
            "--demo",
            "--demo-refresh-fails",
            "--reset",
            "--data-dir",
            "/tmp/x",
            "--route",
            "borrowed:reservations",
            "--window",
            "800x600",
            "--evidence",
            "--screenshot",
            "/tmp/x/a.png",
            "--screenshot-after",
            "2.5",
        ])
        .unwrap();
        assert_eq!(options.library.as_deref(), Some("https://canning.spydus.com"));
        assert!(options.demo && options.demo_refresh_fails && options.reset && options.evidence);
        assert_eq!(options.route, Some(LaunchRoute::BorrowedReservations));
        assert_eq!(options.window, MIN_WINDOW);
        assert_eq!(options.screenshot_after, 2.5);
        assert!(options.secrets_in_memory());
        assert_eq!(
            parse(&["--route", "detail:https://x/y?a=b:c"]).unwrap().route,
            Some(LaunchRoute::Detail("https://x/y?a=b:c".into()))
        );
        assert_eq!(parse(&["--route", "search"]).unwrap().route, Some(LaunchRoute::Search(String::new())));
        assert!(parse(&["--route", "nope"]).is_err());
        assert!(parse(&["--window"]).is_err());
        assert!(parse(&["--bogus"]).is_err());
        assert_eq!(parse(&[]).unwrap(), LaunchOptions::default());
    }
}
