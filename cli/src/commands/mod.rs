//! The commands (docs/cli.md §2), a module per group, and what they share: the signed-in
//! account's snapshot, the status line over account output, numbered rows as JSON.

pub mod account;
pub mod changes;
pub mod libraries;
pub mod picks;
pub mod search;

use std::sync::Arc;

use clap::CommandFactory;
use librespydus_core::AccountSnapshot;
use serde::Serialize;

use crate::app::Ctx;
use crate::args::{Cli, Command, PicksAction};
use crate::backend::LibraryBackend;
use crate::copy::Copy;
use crate::error::Fail;
use crate::formatting::non_empty;
use crate::output::Tone;
use crate::store::SavedLibrary;
use crate::text;

pub async fn dispatch(ctx: &mut Ctx<'_>, command: Command) -> Result<(), Fail> {
    match command {
        Command::Libraries { words, all } => libraries::libraries(ctx, &words, all).await,
        Command::Use { library } => libraries::use_library(ctx, &library.join(" ")).await,
        Command::Search { query, newest, page } => search::search(ctx, &query.join(" "), newest, page).await,
        Command::Show { target, holdings } => search::show(ctx, &target, holdings).await,
        Command::Login { card, password_stdin } => account::login(ctx, card, password_stdin).await,
        Command::Logout => account::logout(ctx).await,
        Command::Profile => account::profile(ctx).await,
        Command::Books => account::books(ctx, account::Section::Both).await,
        Command::Loans => account::books(ctx, account::Section::Loans).await,
        Command::Reservations => account::books(ctx, account::Section::Reservations).await,
        Command::Renew { numbers, all, yes } => changes::renew(ctx, &numbers, all, yes).await,
        Command::Reserve { target, format, pickup, yes } => {
            changes::reserve(ctx, &target, format.as_deref(), pickup, yes).await
        }
        Command::Cancel { numbers, yes } => changes::cancel(ctx, &numbers, yes).await,
        Command::Edit { number, pickup, delay_until, not_needed_after, clear_delay, clear_not_needed, yes } => {
            let changes = changes::EditChanges { pickup, delay_until, not_needed_after, clear_delay, clear_not_needed };
            changes::edit(ctx, &number, changes, yes).await
        }
        Command::Picks { action } => match action {
            None => picks::list(ctx).await,
            Some(PicksAction::Add { target }) => picks::add(ctx, &target).await,
            Some(PicksAction::Remove { target }) => picks::remove(ctx, &target).await,
        },
        Command::Card => account::card(ctx).await,
        Command::Verify { clear } => account::verify(ctx, clear).await,
        Command::Completions { shell } => {
            let mut command = Cli::command();
            let mut buffer = Vec::new();
            clap_complete::generate(shell, &mut command, "spydus", &mut buffer);
            ctx.out(String::from_utf8_lossy(&buffer).trim_end());
            Ok(())
        }
    }
}

/// The chosen library, its backend and the saved session (`NotSignedIn` without one).
pub async fn signed_in(ctx: &mut Ctx<'_>) -> Result<(SavedLibrary, Arc<dyn LibraryBackend>, String), Fail> {
    let (library, backend) = ctx.library().await?;
    let session = ctx.session()?;
    Ok((library, backend, session))
}

/// A fresh look at the account.
pub struct Account {
    pub library: SavedLibrary,
    pub backend: Arc<dyn LibraryBackend>,
    pub session: String,
    pub snapshot: AccountSnapshot,
}

pub async fn account(ctx: &mut Ctx<'_>) -> Result<Account, Fail> {
    let (library, backend, session) = signed_in(ctx).await?;
    let snapshot = backend.account_snapshot(session.clone()).await?;
    Ok(Account { library, backend, session, snapshot })
}

/// "Signed in to City of Canning · Test Borrower · Updated just now", dimmed, over account output.
pub fn status_line(ctx: &mut Ctx<'_>, library: &SavedLibrary, name: Option<&str>) {
    let mut parts = vec![Copy::Profile::signedInTo(&library.name)];
    if let Some(name) = non_empty(name) {
        parts.push(name);
    }
    parts.push(Copy::YourBooks::updatedJustNow.to_owned());
    if ctx.demo {
        parts.push(text::DEMO_NOTE.to_owned());
    }
    ctx.out_styled(&parts.join(Copy::Common::separator), Tone::Dim);
}

/// A list row as JSON: its number first, then the core type's own keys.
#[derive(Serialize)]
pub struct Numbered<'a, T: Serialize> {
    pub number: usize,
    #[serde(flatten)]
    pub value: &'a T,
}
