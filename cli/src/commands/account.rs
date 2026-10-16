//! Signing in and looking at the account (docs/cli.md §2–3): `login`, `logout`, `profile`,
//! `books`, `loans`, `reservations`, `card`, `verify`.

use chrono::NaiveDate;
use librespydus_core::PatronItem;
use serde::Serialize;

use super::{account, status_line};
use crate::app::Ctx;
use crate::backend::demo;
use crate::copy::Copy;
use crate::error::Fail;
use crate::formatting::{author_name, non_empty};
use crate::output::{Cell, Column, Tone};
use crate::patron::{LoanRow, ReservationKind, ReservationRow, due_line, reservation_rows};
use crate::store::{SavedLibrary, VERIFICATION_SERVICE};
use crate::text;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LoginJson<'a> {
    library: &'a SavedLibrary,
    name: Option<String>,
    demo: bool,
}

/// Signs in and keeps the session (never the password) for later runs.
pub async fn login(ctx: &mut Ctx<'_>, card: Option<String>, password_stdin: bool) -> Result<(), Fail> {
    let (library, backend) = ctx.library().await?;
    let session = if ctx.demo {
        demo::SESSION.to_owned()
    } else {
        let card = match non_empty(card.as_deref()) {
            Some(card) => card,
            None => ctx.ask(Copy::Login::cardLabel)?,
        };
        let password =
            if password_stdin { ctx.read_line()? } else { ctx.read_secret(&text::prompt(Copy::Login::pinLabel))? };
        if password.is_empty() {
            return Err(Fail::Usage(text::no_input().to_owned()));
        }
        backend.login(card, password).await?
    };
    ctx.save_session(&session)?;
    let name = backend.account(session).await.ok().and_then(|links| non_empty(links.name.as_deref()));
    if ctx.json {
        ctx.json(&LoginJson { library: &library, name, demo: ctx.demo });
        return Ok(());
    }
    ctx.out(&Copy::Profile::signedInTo(&library.name));
    if let Some(name) = &name {
        ctx.out(&text::signed_in_as(name));
    }
    if ctx.demo {
        ctx.out_styled(text::DEMO_NOTE, Tone::Dim);
    }
    Ok(())
}

/// Forgets this library's session.
pub async fn logout(ctx: &mut Ctx<'_>) -> Result<(), Fail> {
    let (library, _) = ctx.library().await?;
    ctx.forget_session();
    if ctx.json {
        ctx.json(&serde_json::json!({ "library": library, "signedOut": true }));
        return Ok(());
    }
    ctx.out(&text::signed_out_of(&library.name));
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProfileJson<'a> {
    library: &'a SavedLibrary,
    name: Option<&'a str>,
    card_number: Option<&'a str>,
    card_expires: Option<&'a str>,
    loans: usize,
    reservations: usize,
    ready: usize,
    demo: bool,
}

/// Name, library, card number and the counts.
pub async fn profile(ctx: &mut Ctx<'_>) -> Result<(), Fail> {
    let account = account(ctx).await?;
    let snapshot = &account.snapshot;
    let ready = snapshot.reservations.ready.items.len();
    let reservations = ready + snapshot.reservations.pending.items.len();
    let loans = snapshot.loans.items.len();
    if ctx.json {
        ctx.json(&ProfileJson {
            library: &account.library,
            name: snapshot.account_name.as_deref(),
            card_number: snapshot.card_number.as_deref(),
            card_expires: snapshot.card_expires.as_deref(),
            loans,
            reservations,
            ready,
            demo: ctx.demo,
        });
        return Ok(());
    }
    status_line(ctx, &account.library, snapshot.account_name.as_deref());
    let mut rows: Vec<(String, String)> = Vec::new();
    rows.push((text::ACCOUNT_NAME.to_owned(), snapshot.account_name.clone().unwrap_or_default()));
    rows.push((Copy::Onboarding::currentLibrary.to_owned(), account.library.name.clone()));
    if let Some(number) = non_empty(snapshot.card_number.as_deref()) {
        let expires = non_empty(snapshot.card_expires.as_deref()).map(|date| Copy::MyCard::expires(&date));
        let value = match expires {
            Some(expires) => format!("{number}{}{expires}", Copy::Common::separator),
            None => number,
        };
        rows.push((Copy::MyCard::numberLabel.to_owned(), value));
    }
    rows.push((Copy::YourBooks::loans.to_owned(), loans.to_string()));
    rows.push((Copy::YourBooks::reservations.to_owned(), reservations.to_string()));
    rows.push((Copy::YourBooks::readyToCollect.to_owned(), ready.to_string()));
    let label_width = rows.iter().map(|(label, _)| label.chars().count()).max().unwrap_or(0);
    for (label, value) in rows {
        ctx.out(&format!("{label:<label_width$}  {value}"));
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Loans,
    Reservations,
    Both,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LoanJson<'a> {
    number: usize,
    #[serde(flatten)]
    item: &'a PatronItem,
    due_date: Option<NaiveDate>,
    days_left: Option<i64>,
    overdue: bool,
    can_renew: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReservationJson<'a> {
    number: usize,
    kind: &'static str,
    #[serde(flatten)]
    item: &'a PatronItem,
    status: String,
    pickup: Option<&'a str>,
    pickup_by: Option<&'a str>,
    can_cancel: bool,
    can_edit: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BooksJson<'a> {
    library: &'a SavedLibrary,
    name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    loans: Option<Vec<LoanJson<'a>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reservations: Option<Vec<ReservationJson<'a>>>,
    demo: bool,
}

/// Your Books: the loans table, the reservations table, or both.
pub async fn books(ctx: &mut Ctx<'_>, section: Section) -> Result<(), Fail> {
    let account = account(ctx).await?;
    let today = ctx.world.today;
    let loans = LoanRow::rows(&account.snapshot.loans.items, today);
    let reservations = reservation_rows(&account.snapshot.reservations, today);
    let name = account.snapshot.account_name.as_deref();
    if ctx.json {
        let loans_json = (section != Section::Reservations).then(|| {
            loans
                .iter()
                .enumerate()
                .map(|(index, row)| LoanJson {
                    number: index + 1,
                    item: &row.item,
                    due_date: row.due.as_ref().map(|due| due.due_date),
                    days_left: row.due.as_ref().map(|due| due.days_left),
                    overdue: row.is_overdue(),
                    can_renew: row.can_renew(),
                })
                .collect()
        });
        let reservations_json = (section != Section::Loans).then(|| {
            reservations
                .iter()
                .enumerate()
                .map(|(index, row)| ReservationJson {
                    number: index + 1,
                    kind: kind_name(row.kind),
                    item: &row.item,
                    status: row.status_line(),
                    pickup: row.pickup.as_deref(),
                    pickup_by: row.pickup_by.as_deref(),
                    can_cancel: row.cancel_href.is_some(),
                    can_edit: row.edit_href.is_some(),
                })
                .collect()
        });
        ctx.json(&BooksJson {
            library: &account.library,
            name,
            loans: loans_json,
            reservations: reservations_json,
            demo: ctx.demo,
        });
        return Ok(());
    }
    status_line(ctx, &account.library, name);
    if section != Section::Reservations {
        if section == Section::Both {
            ctx.out_styled(&Copy::YourBooks::loansCount(loans.len() as i64), Tone::Bold);
        }
        print_loans(ctx, &loans, &account.library, today);
    }
    if section != Section::Loans {
        if section == Section::Both {
            ctx.out("");
            ctx.out_styled(&Copy::YourBooks::reservationsCount(reservations.len() as i64), Tone::Bold);
        }
        print_reservations(ctx, &reservations);
    }
    Ok(())
}

fn kind_name(kind: ReservationKind) -> &'static str {
    match kind {
        ReservationKind::Ready => "ready",
        ReservationKind::Pending => "pending",
    }
}

/// #, title, author, the due line ("Due 17 Oct · 2 days left", red when overdue) and "Renew"
/// when the library offers a renewal.
pub fn print_loans(ctx: &mut Ctx<'_>, loans: &[LoanRow], library: &SavedLibrary, today: NaiveDate) {
    if loans.is_empty() {
        ctx.out(Copy::YourBooks::noLoansTitle);
        ctx.out(&Copy::YourBooks::noLoansMessage(&library.name));
        return;
    }
    let columns = [
        Column::new("#").right(),
        Column::new(text::COL_TITLE).wrap(),
        Column::new(text::COL_AUTHOR).max(24).wrap(),
        Column::new(text::COL_DUE).wrap(),
        Column::new(""),
    ];
    let rows: Vec<Vec<Cell>> = loans
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let due = match &row.due {
                Some(due) => {
                    Cell::toned(due_line(due, today), if due.is_overdue() { Tone::Error } else { Tone::Plain })
                }
                None => Cell::new(row.due_text.clone().unwrap_or_default()),
            };
            vec![
                Cell::new((index + 1).to_string()),
                Cell::new(row.book.title.clone()),
                Cell::new(author_name(row.book.author.as_deref()).unwrap_or_default()),
                due,
                Cell::toned(if row.can_renew() { text::RENEWABLE } else { text::NOT_RENEWABLE }, Tone::Dim),
            ]
        })
        .collect();
    ctx.table(&columns, &rows);
}

/// #, title, status ("Ready to collect", green; "Pending · #4 in queue · Placed 2 Oct") and
/// where to collect.
pub fn print_reservations(ctx: &mut Ctx<'_>, reservations: &[ReservationRow]) {
    if reservations.is_empty() {
        ctx.out(Copy::YourBooks::noReservationsTitle);
        ctx.out(Copy::YourBooks::noReservationsMessage);
        return;
    }
    let columns = [
        Column::new("#").right(),
        Column::new(text::COL_TITLE).wrap(),
        Column::new(text::COL_STATUS).max(28).wrap(),
        Column::new(Copy::Reserve::collectFrom).max(28).wrap(),
    ];
    let rows: Vec<Vec<Cell>> = reservations
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let tone = if row.is_ready() { Tone::Success } else { Tone::Plain };
            let collect = match (&row.pickup, &row.pickup_by) {
                (Some(branch), Some(by)) => {
                    format!("{branch}{}{}", Copy::Common::separator, Copy::YourBooks::collectBy(&library_date(by, row)))
                }
                (Some(branch), None) => branch.clone(),
                (None, Some(by)) => Copy::YourBooks::collectBy(&library_date(by, row)),
                (None, None) => String::new(),
            };
            vec![
                Cell::new((index + 1).to_string()),
                Cell::new(row.book.title.clone()),
                Cell::toned(row.status_line(), tone),
                Cell::new(collect),
            ]
        })
        .collect();
    ctx.table(&columns, &rows);
}

fn library_date(text: &str, row: &ReservationRow) -> String {
    crate::formatting::due_date::parse(text)
        .map_or_else(|| text.to_owned(), |date| crate::formatting::short_date(date, row.today))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CardJson<'a> {
    library: &'a SavedLibrary,
    card_number: Option<&'a str>,
    card_expires: Option<&'a str>,
}

/// The library card number (the barcode the apps draw).
pub async fn card(ctx: &mut Ctx<'_>) -> Result<(), Fail> {
    let account = account(ctx).await?;
    let number = non_empty(account.snapshot.card_number.as_deref());
    let expires = non_empty(account.snapshot.card_expires.as_deref());
    if ctx.json {
        ctx.json(&CardJson {
            library: &account.library,
            card_number: number.as_deref(),
            card_expires: expires.as_deref(),
        });
        return Ok(());
    }
    let Some(number) = number else {
        ctx.out(text::NO_CARD_NUMBER);
        return Ok(());
    };
    ctx.out(&format!("{}  {number}", Copy::MyCard::numberLabel));
    if let Some(expires) = expires {
        ctx.out_styled(&Copy::MyCard::expires(&expires), Tone::Dim);
    }
    if ctx.demo {
        ctx.out_styled(text::DEMO_NOTE, Tone::Dim);
    }
    Ok(())
}

/// Keeps (or clears) the library's human-verification cookie. Nothing is automated: the reader
/// completes the check in a browser and pastes the cookie's value.
pub async fn verify(ctx: &mut Ctx<'_>, clear: bool) -> Result<(), Fail> {
    let (library, backend) = ctx.library().await?;
    if clear {
        ctx.secrets.set(VERIFICATION_SERVICE, &library.origin, None)?;
        backend.set_verification_cookie(None);
        if ctx.json {
            ctx.json(&serde_json::json!({ "library": library, "verified": false }));
        } else {
            ctx.out(text::VERIFICATION_CLEARED);
        }
        return Ok(());
    }
    ctx.note(&crate::output::styled(Copy::Verification::title, Tone::Bold, ctx.color()));
    ctx.note(&text::verify_steps(&library.name, &library.origin));
    let cookie = ctx.read_secret(text::PROMPT_COOKIE)?;
    let cookie = cookie.trim().trim_start_matches("aws-waf-token=").trim_matches(';').to_owned();
    if cookie.is_empty() {
        return Err(Fail::Usage(text::no_input().to_owned()));
    }
    ctx.secrets.set(VERIFICATION_SERVICE, &library.origin, Some(&cookie))?;
    if ctx.json {
        ctx.json(&serde_json::json!({ "library": library, "verified": true }));
    } else {
        ctx.out(text::VERIFIED);
        if ctx.demo {
            ctx.out_styled(text::DEMO_NOTE, Tone::Dim);
        }
    }
    Ok(())
}
