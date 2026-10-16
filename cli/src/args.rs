//! The command line (docs/cli.md §2), parsed by clap. Help text comes from `text`.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use crate::text;

#[derive(Parser, Debug, PartialEq, Eq)]
#[command(name = "spydus", version, about = text::ABOUT, after_long_help = text::AFTER_HELP, after_help = text::AFTER_HELP)]
pub struct Cli {
    /// Global flags, on every command.
    #[command(flatten)]
    pub global: Global,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Args, Debug, PartialEq, Eq, Default, Clone)]
pub struct Global {
    #[arg(long, global = true, value_name = "NAME OR ORIGIN", help = text::FLAG_LIBRARY)]
    pub library: Option<String>,
    #[arg(long, global = true, help = text::FLAG_JSON)]
    pub json: bool,
    #[arg(long, global = true, help = text::FLAG_DEMO)]
    pub demo: bool,
    #[arg(long, global = true, value_name = "DIR", help = text::FLAG_DATA_DIR)]
    pub data_dir: Option<PathBuf>,
    #[arg(long, global = true, help = text::FLAG_NO_COLOR)]
    pub no_color: bool,
}

#[derive(Subcommand, Debug, PartialEq, Eq)]
pub enum Command {
    #[command(about = text::CMD_LIBRARIES)]
    Libraries {
        #[arg(value_name = "WORDS", help = text::ARG_WORDS)]
        words: Vec<String>,
        #[arg(long, help = text::CMD_LIBRARIES_ALL)]
        all: bool,
    },
    #[command(about = text::CMD_USE)]
    Use {
        #[arg(value_name = "NAME OR ORIGIN", required = true, num_args = 1.., help = text::ARG_LIBRARY)]
        library: Vec<String>,
    },
    #[command(about = text::CMD_SEARCH)]
    Search {
        #[arg(value_name = "QUERY", required = true, num_args = 1.., help = text::ARG_QUERY)]
        query: Vec<String>,
        #[arg(long, help = text::CMD_SEARCH_NEWEST)]
        newest: bool,
        #[arg(long, value_name = "N", default_value_t = 1, help = text::CMD_SEARCH_PAGE)]
        page: usize,
    },
    #[command(about = text::CMD_SHOW)]
    Show {
        #[arg(value_name = "# OR URL", help = text::ARG_NUMBER_OR_URL)]
        target: String,
        #[arg(long, help = text::CMD_SHOW_HOLDINGS)]
        holdings: bool,
    },
    #[command(about = text::CMD_LOGIN)]
    Login {
        #[arg(long, value_name = "NUMBER", help = text::CMD_LOGIN_CARD)]
        card: Option<String>,
        #[arg(long, help = text::CMD_LOGIN_PASSWORD_STDIN)]
        password_stdin: bool,
    },
    #[command(about = text::CMD_LOGOUT)]
    Logout,
    #[command(about = text::CMD_PROFILE)]
    Profile,
    #[command(about = text::CMD_BOOKS)]
    Books,
    #[command(about = text::CMD_LOANS)]
    Loans,
    #[command(about = text::CMD_RESERVATIONS)]
    Reservations,
    #[command(about = text::CMD_RENEW)]
    Renew {
        #[arg(value_name = "#", required_unless_present = "all", help = text::CMD_RENEW_NUMBERS)]
        numbers: Vec<String>,
        #[arg(long, conflicts_with = "numbers", help = text::CMD_RENEW_ALL)]
        all: bool,
        #[arg(short, long, help = text::CMD_YES)]
        yes: bool,
    },
    #[command(about = text::CMD_RESERVE)]
    Reserve {
        #[arg(value_name = "# OR URL", help = text::ARG_NUMBER_OR_URL)]
        target: String,
        #[arg(long, value_name = "NAME", help = text::CMD_RESERVE_FORMAT)]
        format: Option<String>,
        #[arg(long, value_name = "BRANCH", help = text::CMD_RESERVE_PICKUP)]
        pickup: Option<String>,
        #[arg(short, long, help = text::CMD_YES)]
        yes: bool,
    },
    #[command(about = text::CMD_CANCEL)]
    Cancel {
        #[arg(value_name = "#", required = true, num_args = 1.., help = text::CMD_CANCEL_NUMBERS)]
        numbers: Vec<String>,
        #[arg(short, long, help = text::CMD_YES)]
        yes: bool,
    },
    #[command(about = text::CMD_EDIT)]
    Edit {
        #[arg(value_name = "#", help = text::CMD_EDIT_NUMBER)]
        number: String,
        #[arg(long, value_name = "BRANCH", help = text::CMD_EDIT_PICKUP)]
        pickup: Option<String>,
        #[arg(long, value_name = "DATE", help = text::CMD_EDIT_DELAY)]
        delay_until: Option<String>,
        #[arg(long, value_name = "DATE", help = text::CMD_EDIT_NOT_NEEDED)]
        not_needed_after: Option<String>,
        #[arg(long, conflicts_with = "delay_until", help = text::CMD_EDIT_CLEAR_DELAY)]
        clear_delay: bool,
        #[arg(long, conflicts_with = "not_needed_after", help = text::CMD_EDIT_CLEAR_NOT_NEEDED)]
        clear_not_needed: bool,
        #[arg(short, long, help = text::CMD_YES)]
        yes: bool,
    },
    #[command(about = text::CMD_PICKS)]
    Picks {
        #[command(subcommand)]
        action: Option<PicksAction>,
    },
    #[command(about = text::CMD_CARD)]
    Card,
    #[command(about = text::CMD_VERIFY)]
    Verify {
        #[arg(long, help = text::CMD_VERIFY_CLEAR)]
        clear: bool,
    },
    #[command(about = text::CMD_COMPLETIONS)]
    Completions {
        #[arg(value_name = "SHELL")]
        shell: clap_complete::Shell,
    },
}

#[derive(Subcommand, Debug, PartialEq, Eq)]
pub enum PicksAction {
    #[command(about = text::CMD_PICKS_ADD)]
    Add {
        #[arg(value_name = "# OR URL", help = text::ARG_NUMBER_OR_URL)]
        target: String,
    },
    #[command(about = text::CMD_PICKS_REMOVE)]
    Remove {
        #[arg(value_name = "# OR URL", help = text::ARG_NUMBER_OR_URL)]
        target: String,
    },
}

/// What a `<# or url>` argument names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Number(usize),
    Url(String),
}

/// "3" or "#3" → a number; "http://…" / "https://…" → a URL; anything else is a usage error.
pub fn parse_target(argument: &str) -> Result<Target, crate::error::Fail> {
    let trimmed = argument.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return Ok(Target::Url(trimmed.to_owned()));
    }
    parse_number(trimmed)
        .map(Target::Number)
        .ok_or_else(|| crate::error::Fail::Usage(text::not_a_number_or_url(argument)))
}

/// "3" or "#3" → 3 (never 0).
pub fn parse_number(argument: &str) -> Option<usize> {
    argument.trim().trim_start_matches('#').parse::<usize>().ok().filter(|number| *number > 0)
}

/// Every argument as a number, in order, or the first that isn't.
pub fn parse_numbers(arguments: &[String]) -> Result<Vec<usize>, crate::error::Fail> {
    arguments
        .iter()
        .map(|argument| parse_number(argument).ok_or_else(|| crate::error::Fail::Usage(text::not_a_number(argument))))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(std::iter::once("spydus").chain(args.iter().copied()))
    }

    #[test]
    fn global_flags_go_anywhere() {
        let cli = parse(&["--json", "search", "atomic", "habits", "--demo", "--page", "2"]).unwrap();
        assert!(cli.global.json && cli.global.demo);
        assert_eq!(
            cli.command,
            Command::Search { query: vec!["atomic".into(), "habits".into()], newest: false, page: 2 }
        );
        let cli = parse(&["loans", "--library", "canning", "--data-dir", "/tmp/x", "--no-color"]).unwrap();
        assert_eq!(cli.global.library.as_deref(), Some("canning"));
        assert_eq!(cli.global.data_dir.as_deref(), Some(std::path::Path::new("/tmp/x")));
        assert!(cli.global.no_color);
        assert_eq!(cli.command, Command::Loans);
    }

    #[test]
    fn commands_take_their_arguments() {
        assert_eq!(
            parse(&["libraries", "city", "--all"]).unwrap().command,
            Command::Libraries { words: vec!["city".into()], all: true }
        );
        assert_eq!(
            parse(&["use", "city", "of", "canning"]).unwrap().command,
            Command::Use { library: vec!["city".into(), "of".into(), "canning".into()] }
        );
        assert_eq!(
            parse(&["show", "#3", "--holdings"]).unwrap().command,
            Command::Show { target: "#3".into(), holdings: true }
        );
        assert_eq!(
            parse(&["renew", "1", "2", "-y"]).unwrap().command,
            Command::Renew { numbers: vec!["1".into(), "2".into()], all: false, yes: true }
        );
        assert_eq!(
            parse(&["renew", "--all"]).unwrap().command,
            Command::Renew { numbers: vec![], all: true, yes: false }
        );
        assert_eq!(
            parse(&["reserve", "2", "--format", "Book", "--yes"]).unwrap().command,
            Command::Reserve { target: "2".into(), format: Some("Book".into()), pickup: None, yes: true }
        );
        assert_eq!(
            parse(&["cancel", "1", "--yes"]).unwrap().command,
            Command::Cancel { numbers: vec!["1".into()], yes: true }
        );
        assert_eq!(
            parse(&["edit", "1", "--pickup", "Cannington", "--clear-delay"]).unwrap().command,
            Command::Edit {
                number: "1".into(),
                pickup: Some("Cannington".into()),
                delay_until: None,
                not_needed_after: None,
                clear_delay: true,
                clear_not_needed: false,
                yes: false
            }
        );
        assert_eq!(parse(&["picks"]).unwrap().command, Command::Picks { action: None });
        assert_eq!(
            parse(&["picks", "add", "https://x/y"]).unwrap().command,
            Command::Picks { action: Some(PicksAction::Add { target: "https://x/y".into() }) }
        );
        assert_eq!(parse(&["verify", "--clear"]).unwrap().command, Command::Verify { clear: true });
        assert_eq!(
            parse(&["login", "--card", "123", "--password-stdin"]).unwrap().command,
            Command::Login { card: Some("123".into()), password_stdin: true }
        );
        assert!(matches!(parse(&["completions", "zsh"]).unwrap().command, Command::Completions { .. }));
    }

    #[test]
    fn usage_errors_are_clap_errors() {
        assert!(parse(&["search"]).is_err(), "a query is required");
        assert!(parse(&["renew"]).is_err(), "numbers or --all");
        assert!(parse(&["renew", "1", "--all"]).is_err());
        assert!(parse(&["cancel"]).is_err());
        assert!(parse(&["edit", "1", "--delay-until", "2026-11-01", "--clear-delay"]).is_err());
        assert!(parse(&["nope"]).is_err());
        assert!(parse(&[]).is_err());
        assert_eq!(parse(&["--help"]).unwrap_err().kind(), clap::error::ErrorKind::DisplayHelp);
    }

    #[test]
    fn targets_and_numbers() {
        assert_eq!(parse_target("3").unwrap(), Target::Number(3));
        assert_eq!(parse_target("#12").unwrap(), Target::Number(12));
        assert_eq!(
            parse_target(" https://canning.spydus.com/x ").unwrap(),
            Target::Url("https://canning.spydus.com/x".into())
        );
        assert!(matches!(parse_target("0"), Err(crate::error::Fail::Usage(_))));
        assert!(matches!(parse_target("abc"), Err(crate::error::Fail::Usage(_))));
        assert_eq!(parse_numbers(&["1".into(), "#2".into()]).unwrap(), [1, 2]);
        assert!(matches!(parse_numbers(&["1".into(), "x".into()]), Err(crate::error::Fail::Usage(_))));
    }
}
