//! The commands that change the account (docs/cli.md §2, §9): `renew`, `reserve`, `cancel`,
//! `edit`. Each asks first unless `--yes`, numbers items as `loans` / `reservations` print them
//! (a fresh look at the account), and reports what the library said.

use chrono::NaiveDate;
use librespydus_core::{EditOptions, EditOverrides, RecordFormat, ReserveOptions};
use serde::Serialize;

use super::{account, search};
use crate::app::Ctx;
use crate::args::{Target, parse_numbers, parse_target};
use crate::book_ref::BookRef;
use crate::copy::Copy;
use crate::error::{EXIT_FAILED, Fail};
use crate::formatting::{non_empty, short_date};
use crate::output::Tone;
use crate::patron::{
    LoanRow, ReservationRow, confirm_renew_message, display_name, due_date_text, renew_result_text, reservation_rows,
};
use crate::text;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ChangeJson {
    number: usize,
    title: String,
    ok: bool,
    message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    due: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ChangesJson {
    results: Vec<ChangeJson>,
    ok: bool,
}

/// Prints each result and exits 1 when the library refused any.
fn finish(ctx: &mut Ctx<'_>, results: Vec<ChangeJson>) -> Result<(), Fail> {
    let ok = results.iter().all(|result| result.ok);
    if ctx.json {
        ctx.json(&ChangesJson { results, ok });
    } else {
        for result in &results {
            let text = result.message.clone().unwrap_or_default();
            let line = text::result_line(result.number, &result.title, &text);
            ctx.out_styled(&line, if result.ok { Tone::Success } else { Tone::Error });
        }
    }
    if ok { Ok(()) } else { Err(Fail::AlreadyReported(EXIT_FAILED)) }
}

// ---- renew ----

/// Renews loans by number (or every renewable one with `--all`).
pub async fn renew(ctx: &mut Ctx<'_>, numbers: &[String], all: bool, yes: bool) -> Result<(), Fail> {
    let numbers = parse_numbers(numbers)?;
    let account = account(ctx).await?;
    let today = ctx.world.today;
    let loans = LoanRow::rows(&account.snapshot.loans.items, today);
    let chosen: Vec<(usize, &LoanRow)> = if all {
        loans.iter().enumerate().filter(|(_, row)| row.can_renew()).map(|(i, row)| (i + 1, row)).collect()
    } else {
        let mut chosen = Vec::new();
        for number in numbers {
            let row = loans.get(number - 1).ok_or_else(|| Fail::Usage(text::no_such_number(number, loans.len())))?;
            if !row.can_renew() {
                return Err(Fail::Usage(text::not_renewable(number, &row.book.title)));
            }
            chosen.push((number, row));
        }
        chosen
    };
    if chosen.is_empty() {
        ctx.out(text::nothing_to_renew());
        return Ok(());
    }
    let message = if let [(_, row)] = chosen.as_slice() {
        confirm_renew_message(&row.item, today)
    } else {
        chosen
            .iter()
            .map(|(number, row)| text::item_line(*number, &row.book.title, due_date_text(&row.item, today).as_deref()))
            .collect::<Vec<_>>()
            .join("\n")
    };
    ctx.confirm(yes, &text::renew_count(chosen.len()), &message)?;
    let mut results = Vec::new();
    for (number, row) in chosen {
        let href = row.renew_href.clone().unwrap_or_default();
        let result = account.backend.renew_loan(href, account.session.clone()).await?;
        results.push(ChangeJson {
            number,
            title: row.book.title.clone(),
            ok: result.ok,
            message: Some(renew_result_text(&result, today)),
            due: result.due.clone(),
        });
    }
    finish(ctx, results)
}

// ---- reserve ----

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReserveJson<'a> {
    title: &'a str,
    format: Option<String>,
    pickup: Option<String>,
    options: &'a ReserveOptions,
    ok: bool,
    message: Option<String>,
}

/// Reserves a format of a book from the last list (or a format's reserve link).
pub async fn reserve(
    ctx: &mut Ctx<'_>,
    target: &str,
    format_flag: Option<&str>,
    pickup: Option<String>,
    yes: bool,
) -> Result<(), Fail> {
    let (library, backend) = ctx.library().await?;
    let session = ctx.session()?;
    let (title, format, url) = match parse_target(target)? {
        Target::Url(url) => (url.clone(), None, url),
        Target::Number(number) => {
            let list = ctx.last_list(text::HINT_SEARCH_FIRST)?;
            let row = list.row(number, &library.origin, ctx.world.now)?.clone();
            backend.note_records(std::slice::from_ref(&row.record));
            let title = BookRef::from_search_record(&row.record).title;
            let reservable: Vec<&RecordFormat> =
                row.record.formats.iter().filter(|f| f.reserve_url.is_some()).collect();
            let names: Vec<String> = reservable.iter().map(|f| search::format_name(f)).collect();
            let format = match format_flag {
                Some(wanted) => reservable
                    .iter()
                    .find(|f| {
                        search::format_name(f).eq_ignore_ascii_case(wanted) || f.code.eq_ignore_ascii_case(wanted)
                    })
                    .copied()
                    .ok_or_else(|| Fail::Usage(text::unknown_format(wanted, &names)))?,
                None => match reservable.as_slice() {
                    [] => return Err(Fail::Usage(text::no_reserve_link(&title))),
                    [one] => *one,
                    several => {
                        let index = ctx.choose(Copy::Detail::formats, &names, text::choose_format(&names))?;
                        several[index]
                    }
                },
            };
            let url = format.reserve_url.clone().unwrap_or_default();
            (title, Some(search::format_name(format)), url)
        }
    };
    let options = backend.reserve_options(url.clone(), session.clone()).await?;
    if !options.can_reserve {
        let reason = non_empty(options.message.as_deref()).unwrap_or_else(|| Copy::Reserve::cantReserve.to_owned());
        return Err(Fail::Refused(reason));
    }
    let pickup = pickup.or_else(|| non_empty(options.pickup_location.as_deref()));
    let summary = match &format {
        Some(format) => text::reserve_summary(&title, format),
        None => text::reserve_summary_no_format(&title),
    };
    let mut lines = vec![summary.clone()];
    let branch = pickup
        .as_deref()
        .and_then(|branch| display_name(Some(branch)))
        .unwrap_or_else(|| Copy::Reserve::usualBranch.to_owned());
    lines.push(format!("{}: {branch}", Copy::Reserve::collectFrom));
    if let Some(available) = non_empty(options.available_summary.as_deref()) {
        lines.push(format!("{}: {available}", Copy::Reserve::availability));
    }
    ctx.confirm(yes, Copy::Reserve::confirm, &lines.join("\n"))?;
    let result = backend.place_reservation(url, session, None, pickup.clone()).await?;
    if ctx.json {
        ctx.json(&ReserveJson {
            title: &title,
            format: format.clone(),
            pickup,
            options: &options,
            ok: result.ok,
            message: result.message.clone(),
        });
        return if result.ok { Ok(()) } else { Err(Fail::AlreadyReported(EXIT_FAILED)) };
    }
    if !result.ok {
        let reason = non_empty(result.message.as_deref()).unwrap_or_else(|| Copy::Reserve::placeFailed.to_owned());
        return Err(Fail::Refused(reason));
    }
    ctx.out_styled(Copy::Reserve::placed, Tone::Success);
    ctx.out(&summary);
    ctx.out(&format!("{}: {branch}", Copy::Reserve::collectFrom));
    Ok(())
}

// ---- cancel ----

/// Cancels reservations by number.
pub async fn cancel(ctx: &mut Ctx<'_>, numbers: &[String], yes: bool) -> Result<(), Fail> {
    let numbers = parse_numbers(numbers)?;
    let account = account(ctx).await?;
    let rows = reservation_rows(&account.snapshot.reservations, ctx.world.today);
    let mut chosen: Vec<(usize, &ReservationRow)> = Vec::new();
    for number in numbers {
        let row = rows.get(number - 1).ok_or_else(|| Fail::Usage(text::no_such_number(number, rows.len())))?;
        if row.cancel_href.is_none() {
            return Err(Fail::Usage(text::no_cancel_link(number, &row.book.title)));
        }
        chosen.push((number, row));
    }
    let message = if let [(_, row)] = chosen.as_slice() {
        row.cancel_message()
    } else {
        chosen
            .iter()
            .map(|(number, row)| text::item_line(*number, &row.book.title, Some(&row.status_line())))
            .collect::<Vec<_>>()
            .join("\n")
    };
    ctx.confirm(yes, &text::cancel_count(chosen.len()), &message)?;
    let mut results = Vec::new();
    for (number, row) in chosen {
        let href = row.cancel_href.clone().unwrap_or_default();
        let result = account.backend.cancel_reservation(href, account.session.clone()).await?;
        let message = if result.ok {
            Copy::YourBooks::cancelled.to_owned()
        } else {
            non_empty(result.message.as_deref()).unwrap_or_else(|| Copy::YourBooks::cancelFailed.to_owned())
        };
        results.push(ChangeJson {
            number,
            title: row.book.title.clone(),
            ok: result.ok,
            message: Some(message),
            due: None,
        });
    }
    finish(ctx, results)
}

// ---- edit ----

/// What `spydus edit` was asked to change.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct EditChanges {
    pub pickup: Option<String>,
    pub delay_until: Option<String>,
    pub not_needed_after: Option<String>,
    pub clear_delay: bool,
    pub clear_not_needed: bool,
}

impl EditChanges {
    fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EditOptionsJson<'a> {
    number: usize,
    title: &'a str,
    options: &'a EditOptions,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EditJson<'a> {
    number: usize,
    title: &'a str,
    changes: &'a EditOverrides,
    ok: bool,
    message: Option<String>,
}

/// Changes a reservation's pickup branch and dates; with no change asked for, prints what can
/// be changed.
pub async fn edit(ctx: &mut Ctx<'_>, number: &str, changes: EditChanges, yes: bool) -> Result<(), Fail> {
    let number = parse_numbers(std::slice::from_ref(&number.to_owned()))?[0];
    let account = account(ctx).await?;
    let today = ctx.world.today;
    let rows = reservation_rows(&account.snapshot.reservations, today);
    let row = rows.get(number - 1).ok_or_else(|| Fail::Usage(text::no_such_number(number, rows.len())))?;
    let href = row.edit_href.clone().ok_or_else(|| Fail::Usage(text::no_edit_link(number, &row.book.title)))?;
    let options = account.backend.edit_options(href.clone(), account.session.clone()).await?;
    if !options.can_edit {
        let reason =
            non_empty(options.message.as_deref()).unwrap_or_else(|| Copy::EditReservation::cantEditMessage.to_owned());
        return Err(Fail::Refused(reason));
    }
    let choice_names: Vec<String> =
        options.pickup_choices.iter().map(|choice| display_name(Some(&choice.name)).unwrap_or_default()).collect();
    if changes.is_empty() {
        if ctx.json {
            ctx.json(&EditOptionsJson { number, title: &row.book.title, options: &options });
            return Ok(());
        }
        ctx.out_styled(&text::edit_summary(&row.book.title), Tone::Bold);
        let current = options.pickup_selected.as_deref();
        let current_name = options
            .pickup_choices
            .iter()
            .find(|choice| Some(choice.code.as_str()) == current)
            .map(|choice| choice.name.as_str())
            .or(row.pickup.as_deref());
        ctx.out(&format!(
            "{}  {}",
            Copy::EditReservation::collectFrom,
            display_name(current_name).unwrap_or_else(|| text::NO_PICKUP.to_owned())
        ));
        if !options.pickup_choices.is_empty() {
            ctx.out(&format!("{}:", text::PICKUP_CHOICES));
            for choice in &options.pickup_choices {
                let name = display_name(Some(&choice.name)).unwrap_or_default();
                ctx.out(&format!(
                    "  {}",
                    text::pickup_choice(&name, &choice.code, Some(choice.code.as_str()) == current)
                ));
            }
        }
        ctx.out(&format!(
            "{}  {}",
            Copy::EditReservation::delayUntil,
            date_text(options.delay_until.as_deref(), today)
        ));
        ctx.out(&format!(
            "{}  {}",
            Copy::EditReservation::notNeededAfter,
            date_text(options.not_required_after.as_deref(), today)
        ));
        return Ok(());
    }
    let pickup = match &changes.pickup {
        Some(wanted) => Some(
            branch_code(&options, wanted).ok_or_else(|| Fail::Usage(text::unknown_branch(wanted, &choice_names)))?,
        ),
        None => None,
    };
    let overrides = EditOverrides {
        pickup,
        delay_until: changes.delay_until.as_deref().map(parse_date).transpose()?,
        not_required_after: changes.not_needed_after.as_deref().map(parse_date).transpose()?,
        clear_delay: changes.clear_delay,
        clear_expiry: changes.clear_not_needed,
    };
    let mut lines = Vec::new();
    if let Some(code) = &overrides.pickup {
        let name = options.pickup_choices.iter().find(|c| &c.code == code).map(|c| c.name.clone()).unwrap_or_default();
        lines.push(format!("{}: {}", Copy::EditReservation::collectFrom, display_name(Some(&name)).unwrap_or(name)));
    }
    if let Some(date) = &overrides.delay_until {
        lines.push(format!("{}: {}", Copy::EditReservation::delayUntil, date_text(Some(date), today)));
    }
    if overrides.clear_delay {
        lines.push(format!("{}: {}", Copy::EditReservation::delayUntil, text::NO_PICKUP));
    }
    if let Some(date) = &overrides.not_required_after {
        lines.push(format!("{}: {}", Copy::EditReservation::notNeededAfter, date_text(Some(date), today)));
    }
    if overrides.clear_expiry {
        lines.push(format!("{}: {}", Copy::EditReservation::notNeededAfter, text::NO_PICKUP));
    }
    let message = format!("{}\n{}", text::edit_summary(&row.book.title), lines.join("\n"));
    ctx.confirm(yes, Copy::EditReservation::title, &message)?;
    let result = account.backend.edit_reservation(href, account.session.clone(), overrides.clone()).await?;
    if ctx.json {
        ctx.json(&EditJson {
            number,
            title: &row.book.title,
            changes: &overrides,
            ok: result.ok,
            message: result.message.clone(),
        });
        return if result.ok { Ok(()) } else { Err(Fail::AlreadyReported(EXIT_FAILED)) };
    }
    if !result.ok {
        let reason =
            non_empty(result.message.as_deref()).unwrap_or_else(|| Copy::EditReservation::changeFailed.to_owned());
        return Err(Fail::Refused(reason));
    }
    ctx.out_styled(Copy::YourBooks::updated, Tone::Success);
    ctx.out(&text::result_line(number, &row.book.title, &text::changes_line(&lines)));
    Ok(())
}

/// The pickup choice `wanted` names: its code, its full name, or the one choice whose name
/// contains the words ("cannington" → CANNINGTON LIBRARY), ignoring case.
fn branch_code(options: &EditOptions, wanted: &str) -> Option<String> {
    let wanted = wanted.trim().to_lowercase();
    if let Some(exact) = options
        .pickup_choices
        .iter()
        .find(|choice| choice.code.to_lowercase() == wanted || choice.name.to_lowercase() == wanted)
    {
        return Some(exact.code.clone());
    }
    let words: Vec<&str> = wanted.split_whitespace().collect();
    let mut matches = options
        .pickup_choices
        .iter()
        .filter(|choice| {
            let name = choice.name.to_lowercase();
            !words.is_empty() && words.iter().all(|word| name.contains(word))
        })
        .map(|choice| choice.code.clone());
    match (matches.next(), matches.next()) {
        (Some(code), None) => Some(code),
        _ => None,
    }
}

/// A `YYYY-MM-DD` argument, as the core's edit form takes it.
fn parse_date(argument: &str) -> Result<String, Fail> {
    NaiveDate::parse_from_str(argument.trim(), "%Y-%m-%d")
        .map(|date| date.format("%Y-%m-%d").to_string())
        .map_err(|_| Fail::Usage(text::not_a_date(argument)))
}

/// A date the library gave ("2026-11-01") as the apps print it ("1 Nov"), or "–".
fn date_text(date: Option<&str>, today: NaiveDate) -> String {
    match non_empty(date) {
        Some(text) => NaiveDate::parse_from_str(&text, "%Y-%m-%d")
            .ok()
            .or_else(|| crate::formatting::due_date::parse(&text))
            .map_or(text, |date| short_date(date, today)),
        None => text::NO_PICKUP.to_owned(),
    }
}
