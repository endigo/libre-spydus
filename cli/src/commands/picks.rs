//! `spydus picks` (docs/cli.md §2): My Picks on the library account when signed in, else on this
//! computer (the desktop app's `state/local_picks.rs`).

use librespydus_core::SearchRecord;
use serde::Serialize;

use super::{Numbered, search};
use crate::app::Ctx;
use crate::args::{Target, parse_target};
use crate::book_ref::BookRef;
use crate::copy::Copy;
use crate::error::Fail;
use crate::formatting::non_empty;
use crate::output::Tone;
use crate::store::{LastList, ListKind, ListRow, LocalPicks, SavedLibrary};
use crate::text;

/// At most this many pages of the account's Pick list.
const MAX_PAGES: usize = 10;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PicksJson<'a> {
    library: &'a SavedLibrary,
    on_account: bool,
    picks: Vec<Numbered<'a, SearchRecord>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PickChangeJson<'a> {
    title: &'a str,
    id: &'a str,
    on_account: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    count: Option<i64>,
    ok: bool,
}

/// The list, numbered and saved for `picks remove` and `show`.
pub async fn list(ctx: &mut Ctx<'_>) -> Result<(), Fail> {
    let (library, backend) = ctx.library().await?;
    let session = ctx.session().ok();
    let (rows, on_account) = match session {
        Some(session) => {
            let mut rows = Vec::new();
            let mut url = None;
            for _ in 0..MAX_PAGES {
                let results = backend.picks(session.clone(), url).await?;
                let start = rows.len();
                rows.extend(results.records.iter().enumerate().map(|(index, record)| ListRow {
                    number: start + index + 1,
                    record: record.clone(),
                    book: Some(BookRef::from_search_record(record)),
                }));
                url = results.next_page_url;
                if url.is_none() {
                    break;
                }
            }
            (rows, true)
        }
        None => {
            let picks = LocalPicks::load(&ctx.paths, &library.origin)?;
            let rows = picks
                .items
                .iter()
                .enumerate()
                .map(|(index, book)| ListRow {
                    number: index + 1,
                    record: book.to_search_record(),
                    book: Some(book.clone()),
                })
                .collect();
            (rows, false)
        }
    };
    let list = LastList {
        kind: ListKind::Picks,
        origin: library.origin.clone(),
        library: library.name.clone(),
        query: Copy::Picks::title.to_owned(),
        saved_at: ctx.world.now,
        local: !on_account,
        rows,
    };
    ctx.save_list(&list)?;
    if ctx.json {
        let picks = list.rows.iter().map(|row| Numbered { number: row.number, value: &row.record }).collect();
        ctx.json(&PicksJson { library: &library, on_account, picks });
        return Ok(());
    }
    let subtitle = if on_account { Copy::Picks::subtitleAccount } else { Copy::Picks::subtitlePhone };
    ctx.out_styled(&format!("{}{}{subtitle}", Copy::Picks::title, Copy::Common::separator), Tone::Bold);
    if list.rows.is_empty() {
        ctx.out(Copy::Picks::emptyTitle);
        ctx.out(if on_account { Copy::Picks::emptyAccountMessage } else { Copy::Picks::emptyPhoneMessage });
        return Ok(());
    }
    search::print_records(ctx, &list.rows);
    ctx.out_styled(&text::pick_count(list.rows.len() as i64), Tone::Dim);
    Ok(())
}

/// The book a `<# or url>` names: from the last list, or loaded from the library.
async fn book_for(ctx: &mut Ctx<'_>, target: &str, hint: &str) -> Result<(BookRef, Option<SearchRecord>), Fail> {
    let (library, backend) = ctx.library().await?;
    match parse_target(target)? {
        Target::Number(number) => {
            let list = ctx.last_list(hint)?;
            let row = list.row(number, &library.origin, ctx.world.now)?.clone();
            backend.note_records(std::slice::from_ref(&row.record));
            let book = row.book.clone().unwrap_or_else(|| BookRef::from_search_record(&row.record));
            Ok((book, Some(row.record)))
        }
        Target::Url(url) => {
            let detail = backend.record(url.clone()).await?;
            Ok((BookRef::from_record(&url, &detail), None))
        }
    }
}

/// The IRN the account's Pick list takes: the work's from a search card, else the edition's.
fn pick_irn(book: &BookRef) -> Option<String> {
    book.id
        .strip_prefix("work:")
        .or_else(|| book.id.strip_prefix("bib:"))
        .map(str::to_owned)
        .filter(|irn| !irn.is_empty() && irn.bytes().all(|b| b.is_ascii_digit()))
}

/// Saves a book to My Picks.
pub async fn add(ctx: &mut Ctx<'_>, target: &str) -> Result<(), Fail> {
    let (library, backend) = ctx.library().await?;
    let (book, _) = book_for(ctx, target, text::HINT_SEARCH_FIRST).await?;
    let (count, on_account) = match ctx.session().ok() {
        Some(session) => {
            let irn = pick_irn(&book).ok_or_else(|| Fail::Usage(text::no_irn(&book.title)))?;
            (Some(backend.save_pick(irn, session).await?), true)
        }
        None => {
            let mut picks = LocalPicks::load(&ctx.paths, &library.origin)?;
            picks.add(&book, ctx.world.now);
            picks.save(&ctx.paths, &library.origin)?;
            (None, false)
        }
    };
    if ctx.json {
        ctx.json(&PickChangeJson { title: &book.title, id: &book.id, on_account, count, ok: true });
        return Ok(());
    }
    ctx.out_styled(Copy::Picks::saved, Tone::Success);
    ctx.out(&book.title);
    ctx.out_styled(if on_account { Copy::Picks::subtitleAccount } else { Copy::Picks::subtitlePhone }, Tone::Dim);
    Ok(())
}

/// Removes a book from My Picks (by number from the picks list, or by URL).
pub async fn remove(ctx: &mut Ctx<'_>, target: &str) -> Result<(), Fail> {
    let (library, backend) = ctx.library().await?;
    if let Target::Number(_) = parse_target(target)? {
        let list = ctx.last_list(text::HINT_PICKS_FIRST)?;
        if list.kind != ListKind::Picks {
            return Err(Fail::List(text::HINT_PICKS_FIRST.to_owned()));
        }
    }
    let (book, _) = book_for(ctx, target, text::HINT_PICKS_FIRST).await?;
    let (count, on_account) = match ctx.session().ok() {
        Some(session) => {
            let irn = pick_irn(&book).ok_or_else(|| Fail::Usage(text::no_irn(&book.title)))?;
            (Some(backend.remove_pick(irn, session).await?), true)
        }
        None => {
            let mut picks = LocalPicks::load(&ctx.paths, &library.origin)?;
            let id = book.id.clone();
            let by_url = non_empty(book.url.as_deref()).and_then(|url| {
                picks.items.iter().find(|item| item.url.as_deref() == Some(url.as_str())).map(|i| i.id.clone())
            });
            picks.remove(&by_url.unwrap_or(id));
            picks.save(&ctx.paths, &library.origin)?;
            (None, false)
        }
    };
    if ctx.json {
        ctx.json(&PickChangeJson { title: &book.title, id: &book.id, on_account, count, ok: true });
        return Ok(());
    }
    ctx.out_styled(&Copy::Picks::removedTitle(&book.title), Tone::Success);
    Ok(())
}
