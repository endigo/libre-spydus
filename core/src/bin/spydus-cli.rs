//! Host CLI over the `SpydusClient` facade: prints a public endpoint's result as the same
//! camelCase JSON the apps get (used by `scripts/library-survey.py`).
//!
//!   spydus-cli search [--sort relevance|newest] "<query>"
//!   spydus-cli <page|availability|record|related|holdings|collection> "<url>"
//!   spydus-cli featured                # the home page's Featured collections
//!   spydus-cli barcode-type            # the card barcode symbology
//!   spydus-cli libraries [--all]       # the Spydus library directory; --all adds test instances
//!
//! Public reads only — account/reservation methods are deliberately not exposed here.
//! `SPYDUS_ORIGIN` overrides the library origin (default `https://canning.spydus.com`).
//! `SPYDUS_VERIFICATION_COOKIE` (optional) is passed to `set_verification_cookie`; it is never
//! printed.

use std::process::ExitCode;

use librespydus_core::{SearchSort, SpydusClient, SpydusError, list_libraries};
use serde::Serialize;

const USAGE: &str = "usage: spydus-cli <search|page|availability|record|related|holdings|collection> <query-or-url>\n       spydus-cli search --sort <relevance|newest> <query>\n       spydus-cli featured | barcode-type\n       spydus-cli libraries [--all]";

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let json = match args.as_slice() {
        [command] if command == "libraries" => to_json(list_libraries(false).await),
        [command] if command == "featured" => to_json(client().featured().await),
        [command] if command == "barcode-type" => {
            to_json(Ok::<_, SpydusError>(format!("{:?}", client().card_barcode_symbology().await)))
        }
        [command, all] if command == "libraries" && all == "--all" => {
            to_json(list_libraries(true).await)
        }
        [command, ..] if command == "libraries" => return usage("libraries takes only --all"),
        [command, flag, sort, query] if command == "search" && flag == "--sort" => {
            let sort = match sort.as_str() {
                "relevance" => SearchSort::Relevance,
                "newest" => SearchSort::Newest,
                _ => return usage(&format!("unknown sort {sort:?}")),
            };
            to_json(client().search_sorted(query.clone(), sort).await)
        }
        [command, arg] => {
            let client = client();
            let arg = arg.clone();
            match command.as_str() {
                "search" => to_json(client.search(arg).await),
                "page" => to_json(client.page(arg).await),
                "availability" => to_json(client.availability(arg).await),
                "record" => to_json(client.record(arg).await),
                "related" => to_json(client.related(arg).await),
                "holdings" => to_json(client.holdings(arg).await),
                "collection" => to_json(client.collection(arg).await),
                _ => return usage(&format!("unknown command {command:?}")),
            }
        }
        _ => return usage("wrong arguments"),
    };
    match json {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("spydus-cli {}: {e}", args[0]);
            ExitCode::FAILURE
        }
    }
}

/// The client for `SPYDUS_ORIGIN` (default Canning), with `SPYDUS_VERIFICATION_COOKIE` if set.
fn client() -> std::sync::Arc<SpydusClient> {
    let client = SpydusClient::new(std::env::var("SPYDUS_ORIGIN").ok());
    if let Ok(cookie) = std::env::var("SPYDUS_VERIFICATION_COOKIE") {
        client.set_verification_cookie(Some(cookie));
    }
    client
}

fn usage(problem: &str) -> ExitCode {
    eprintln!("{problem}\n{USAGE}");
    ExitCode::from(2)
}

fn to_json<T: Serialize>(result: Result<T, SpydusError>) -> Result<String, SpydusError> {
    let value = result?;
    serde_json::to_string_pretty(&value).map_err(|e| SpydusError::Parse { message: e.to_string() })
}
