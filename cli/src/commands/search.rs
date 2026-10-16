//! `spydus search` and `spydus show` (docs/cli.md §2–3): the catalogue, numbered, and one book's
//! page. The book page's availability line and "already yours" status follow the desktop app's
//! `features/detail/logic.rs` and `held.rs`.

use librespydus_core::app::{AccountHolding, HoldingKind, account_holding, availability_summary, is_digital_format};
use librespydus_core::{
    Availability, Holding, PatronItem, RecordDetail, RecordFormat, SearchRecord, SearchResults, SearchSort,
};
use serde::Serialize;

use super::Numbered;
use crate::app::Ctx;
use crate::args::{Target, parse_target};
use crate::book_ref::BookRef;
use crate::copy::Copy;
use crate::error::Fail;
use crate::formatting::{DueStatus, author_name, non_empty, short_date};
use crate::output::{Cell, Column, Tone, wrap_paragraph};
use crate::patron::{DUE, PICKUP_LOCATION, display_name};
use crate::store::{LastList, ListKind, ListRow};
use crate::text;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchJson<'a> {
    query: &'a str,
    page: usize,
    total_results: Option<i64>,
    has_next_page: bool,
    records: Vec<Numbered<'a, SearchRecord>>,
}

/// How far `--page` walks: Spydus gives no way to jump to a page, only a link to the next one,
/// so page 10 is ten GETs. Further pages are refused before any request (be gentle).
pub const MAX_PAGE: usize = 10;

/// Searches, saves the numbered list for later commands, and prints it.
pub async fn search(ctx: &mut Ctx<'_>, query: &str, newest: bool, page: usize) -> Result<(), Fail> {
    let (library, backend) = ctx.library().await?;
    if page == 0 {
        return Err(Fail::Usage(text::not_a_page("0")));
    }
    if page > MAX_PAGE {
        return Err(Fail::Usage(text::page_too_far(MAX_PAGE)));
    }
    let sort = if newest { SearchSort::Newest } else { SearchSort::Relevance };
    let mut results = backend.search_sorted(query.to_owned(), sort).await?;
    for _ in 1..page {
        let Some(url) = results.next_page_url.clone() else {
            return Err(Fail::Usage(text::HINT_PAGE.to_owned()));
        };
        results = backend.page(url).await?;
    }
    let rows = numbered_rows(&results);
    let list = LastList {
        kind: ListKind::Search,
        origin: library.origin.clone(),
        library: library.name.clone(),
        query: query.to_owned(),
        saved_at: ctx.world.now,
        local: false,
        rows: rows.clone(),
    };
    ctx.save_list(&list)?;
    if ctx.json {
        let records = rows.iter().map(|row| Numbered { number: row.number, value: &row.record }).collect();
        ctx.json(&SearchJson {
            query,
            page,
            total_results: results.total_results,
            has_next_page: results.next_page_url.is_some(),
            records,
        });
        return Ok(());
    }
    if rows.is_empty() {
        ctx.out(&Copy::Search::noResultsTitle(query));
        ctx.out(Copy::Search::noResultsMessage);
        return Ok(());
    }
    let mut header = Vec::new();
    if let Some(total) = results.total_results {
        header.push(Copy::Search::results(total));
    }
    if page > 1 {
        header.push(text::page_of(page));
    }
    if !header.is_empty() {
        ctx.out_styled(&header.join(Copy::Common::separator), Tone::Bold);
    }
    print_records(ctx, &rows);
    if results.next_page_url.is_some() {
        let command = format!("spydus search {query} --page {}", page + 1);
        ctx.out_styled(&text::next_page(&command), Tone::Dim);
    } else {
        ctx.out_styled(Copy::Search::endOfResults, Tone::Dim);
    }
    Ok(())
}

/// Rows numbered by the library's rank (absolute across pages), else by position from
/// `showing.from`.
pub fn numbered_rows(results: &SearchResults) -> Vec<ListRow> {
    let base = results.showing.as_ref().map_or(1, |showing| showing.from.max(1)) as usize;
    results
        .records
        .iter()
        .enumerate()
        .map(|(index, record)| ListRow {
            number: record.rank.filter(|rank| *rank > 0).map_or(base + index, |rank| rank as usize),
            record: record.clone(),
            book: None,
        })
        .collect()
}

/// The results table: #, title, author, year, formats.
pub fn print_records(ctx: &mut Ctx<'_>, rows: &[ListRow]) {
    let columns = [
        Column::new("#").right(),
        Column::new(text::COL_TITLE).wrap(),
        Column::new(text::COL_AUTHOR).max(24).wrap(),
        Column::new(text::COL_YEAR),
        Column::new(text::COL_FORMATS).max(24).wrap(),
    ];
    let cells: Vec<Vec<Cell>> = rows
        .iter()
        .map(|row| {
            let book = BookRef::from_search_record(&row.record);
            vec![
                Cell::new(row.number.to_string()),
                Cell::new(book.title),
                Cell::new(author_name(book.author.as_deref()).unwrap_or_default()),
                Cell::toned(book.year.unwrap_or_default(), Tone::Dim),
                Cell::toned(format_names(&row.record.formats).join(", "), Tone::Dim),
            ]
        })
        .collect();
    ctx.table(&columns, &cells);
}

/// The one format the core gives an edition-level item (a pick, a loan): no code and no label.
pub fn is_edition_level(format: &RecordFormat) -> bool {
    format.code.trim().is_empty() && non_empty(format.label.as_deref()).is_none()
}

/// Each format's label, else its code.
pub fn format_names(formats: &[RecordFormat]) -> Vec<String> {
    formats.iter().map(format_name).collect()
}

pub fn format_name(format: &RecordFormat) -> String {
    non_empty(format.label.as_deref()).unwrap_or_else(|| format.code.clone())
}

// ---- show ----

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FormatJson<'a> {
    #[serde(flatten)]
    format: &'a RecordFormat,
    availability: &'a Option<Availability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    holdings: &'a Option<Vec<Holding>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HeldJson<'a> {
    kind: &'static str,
    text: &'a str,
    item: &'a PatronItem,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ShowJson<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    number: Option<usize>,
    title: &'a str,
    author: Option<String>,
    year: Option<String>,
    url: Option<String>,
    held: Option<HeldJson<'a>>,
    formats: Vec<FormatJson<'a>>,
    record: &'a RecordDetail,
}

struct FormatView {
    format: RecordFormat,
    availability: Option<Availability>,
    holdings: Option<Vec<Holding>>,
}

/// The book page: formats with availability (branches with `--holdings`), synopsis, details,
/// and "On loan" / "Reserved" when the account has it.
pub async fn show(ctx: &mut Ctx<'_>, target: &str, holdings: bool) -> Result<(), Fail> {
    let (library, backend) = ctx.library().await?;
    let (number, record) = match parse_target(target)? {
        Target::Number(number) => {
            let list = ctx.last_list(text::HINT_SEARCH_FIRST)?;
            let row = list.row(number, &library.origin, ctx.world.now)?.clone();
            backend.note_records(std::slice::from_ref(&row.record));
            (Some(number), row.record)
        }
        Target::Url(url) => (None, SearchRecord { detail_url: Some(url), ..Default::default() }),
    };
    let mut formats = Vec::new();
    for format in &record.formats {
        let availability = match &format.availability_url {
            Some(url) => backend.availability(url.clone()).await.ok(),
            None => None,
        };
        let copies = match availability.as_ref().and_then(|a| non_empty(a.holdings_url.as_deref())) {
            Some(url) if holdings => Some(backend.holdings(url).await?),
            _ => None,
        };
        formats.push(FormatView { format: format.clone(), availability, holdings: copies });
    }
    let preferred = record
        .formats
        .iter()
        .position(|format| !is_digital_format(format.clone()) && format.availability_url.is_some())
        .or_else(|| record.formats.iter().position(|format| format.availability_url.is_some()));
    let detail_url = preferred
        .and_then(|index| record.formats[index].availability_url.clone())
        .or_else(|| record.detail_url.clone())
        .ok_or_else(|| Fail::Usage(Copy::Detail::noRecordLink.to_owned()))?;
    let detail = backend.record(detail_url.clone()).await?;
    let from_record = BookRef::from_record(&detail_url, &detail);
    let title = non_empty(record.title.as_deref()).unwrap_or_else(|| from_record.title.clone());
    let author = author_name(record.author.as_deref()).or_else(|| author_name(from_record.author.as_deref()));
    let year = non_empty(record.year.as_deref());
    let held = match ctx.session() {
        Ok(session) => match backend.account_snapshot(session).await {
            Ok(snapshot) => {
                let mut irns: Vec<String> = formats
                    .iter()
                    .filter_map(|view| view.availability.as_ref())
                    .filter_map(|a| non_empty(a.bib_record_irn.as_deref()))
                    .collect();
                irns.extend(non_empty(detail.bib_irn.as_deref()));
                account_holding(snapshot, irns).map(|holding| (held_text(&holding, ctx.world.today), holding))
            }
            Err(_) => None,
        },
        Err(_) => None,
    };
    if ctx.json {
        ctx.json(&ShowJson {
            number,
            title: &title,
            author,
            year,
            url: non_empty(detail.bookmark_url.as_deref()).or_else(|| Some(detail_url.clone())),
            held: held.as_ref().map(|((kind, text), holding)| HeldJson { kind, text, item: &holding.item }),
            formats: formats
                .iter()
                .map(|view| FormatJson {
                    format: &view.format,
                    availability: &view.availability,
                    holdings: &view.holdings,
                })
                .collect(),
            record: &detail,
        });
        return Ok(());
    }
    let width = ctx.width();
    ctx.out_styled(&title, Tone::Bold);
    let mut byline = Vec::new();
    if let Some(author) = &author {
        byline.push(Copy::Detail::byline(author));
    }
    if let Some(year) = &year {
        byline.push(year.clone());
    }
    if !byline.is_empty() {
        ctx.out(&byline.join(Copy::Common::separator));
    }
    if let Some(((kind, text), _)) = &held {
        let tone = if *kind == "overdue" { Tone::Error } else { Tone::Success };
        ctx.out_styled(text, tone);
    }
    if let [edition] = formats.as_slice()
        && is_edition_level(&edition.format)
    {
        // An edition-level card (a pick, a loan): its one line of availability under the byline.
        ctx.out(&availability_line(&edition.format, edition.availability.as_ref()));
    } else if !formats.is_empty() {
        ctx.out("");
        ctx.out_styled(Copy::Detail::formats, Tone::Bold);
        let label_width = formats.iter().map(|view| format_name(&view.format).chars().count()).max().unwrap_or(0);
        for view in &formats {
            let line = availability_line(&view.format, view.availability.as_ref());
            ctx.out(&format!("  {:<label_width$}  {line}", format_name(&view.format)));
        }
    }
    if let Some(summary) = non_empty(detail.summary.as_deref()) {
        ctx.out("");
        ctx.out_styled(Copy::Detail::synopsis, Tone::Bold);
        ctx.out(&wrap_paragraph(&summary, width, "  "));
    }
    if !detail.details.is_empty() {
        ctx.out("");
        ctx.out_styled(Copy::Detail::details, Tone::Bold);
        let label_width = detail.details.iter().map(|row| row.label.chars().count()).max().unwrap_or(0).min(20);
        for row in &detail.details {
            let values: Vec<&str> = row.values.iter().map(|value| value.text.as_str()).collect();
            let indent = " ".repeat(label_width + 4);
            let text = wrap_paragraph(&values.join("; "), width, &indent);
            ctx.out(&format!("  {:<label_width$}  {}", row.label, text.trim_start()));
        }
    }
    if holdings {
        for view in &formats {
            ctx.out("");
            let heading = if is_edition_level(&view.format) {
                Copy::Detail::copies.to_owned()
            } else {
                format!("{}{}{}", Copy::Detail::copies, Copy::Common::separator, format_name(&view.format))
            };
            ctx.out_styled(&heading, Tone::Bold);
            print_holdings(ctx, view.holdings.as_deref().unwrap_or_default());
        }
    }
    Ok(())
}

/// "2 of 3 available", "Available online", or "Ask the library about availability".
pub fn availability_line(format: &RecordFormat, availability: Option<&Availability>) -> String {
    let summary = availability.cloned().map(availability_summary);
    let has_counts = summary
        .as_ref()
        .is_some_and(|summary| summary.available.is_some() || summary.copies.is_some() || summary.reserved.is_some());
    if is_digital_format(format.clone()) && !has_counts {
        return Copy::Detail::availableOnline.to_owned();
    }
    match summary {
        Some(summary) => summary.label,
        None => Copy::Detail::askLibrary.to_owned(),
    }
}

fn print_holdings(ctx: &mut Ctx<'_>, holdings: &[Holding]) {
    if holdings.is_empty() {
        ctx.out(&format!("  {}", text::NO_HOLDINGS));
        return;
    }
    let columns = [
        Column::new(text::COL_BRANCH).wrap(),
        Column::new(text::COL_COLLECTION).max(20).wrap(),
        Column::new(text::COL_CALL_NUMBER).max(16).wrap(),
        Column::new(text::COL_STATUS).wrap(),
    ];
    let rows: Vec<Vec<Cell>> = holdings
        .iter()
        .map(|holding| {
            let branch = display_name(holding.location.as_deref())
                .or_else(|| non_empty(holding.collection.as_deref()))
                .unwrap_or_else(|| Copy::Detail::copy.to_owned());
            let status = non_empty(holding.status.as_deref()).unwrap_or_default();
            vec![
                Cell::new(branch),
                Cell::new(non_empty(holding.collection.as_deref()).unwrap_or_default()),
                Cell::new(non_empty(holding.call_number.as_deref()).unwrap_or_default()),
                Cell::toned(status, if holding.available { Tone::Success } else { Tone::Plain }),
            ]
        })
        .collect();
    ctx.table(&columns, &rows);
}

/// The "already yours" line (desktop `held.rs`): its kind for JSON and its text.
pub fn held_text(holding: &AccountHolding, today: chrono::NaiveDate) -> (&'static str, String) {
    let item = &holding.item;
    match holding.kind {
        HoldingKind::Loan => {
            let due_text = non_empty(item.fields.get(DUE).map(String::as_str));
            match DueStatus::from_due_text(due_text.as_deref(), today) {
                Some(due) if due.is_overdue() => ("overdue", Copy::Detail::onLoanOverdue.to_owned()),
                Some(due) => ("loan", Copy::Detail::onLoanDue(&short_date(due.due_date, today))),
                None => match due_text {
                    Some(text) => ("loan", Copy::Detail::onLoanDue(&text)),
                    None => ("loan", Copy::Detail::onLoan.to_owned()),
                },
            }
        }
        HoldingKind::ReservationReady => {
            let pickup = display_name(item.fields.get(PICKUP_LOCATION).map(String::as_str));
            (
                "ready",
                pickup
                    .as_deref()
                    .map_or_else(|| Copy::Detail::readyToCollect.to_owned(), Copy::Detail::readyToCollectAt),
            )
        }
        HoldingKind::ReservationPending => {
            // copy-exempt: the reservations list's field name (a key)
            let rank = non_empty(item.fields.get("Rank").map(String::as_str));
            let text = if let Some(branch) = display_name(item.fields.get(PICKUP_LOCATION).map(String::as_str)) {
                Copy::Detail::reservedAt(&branch)
            } else if let Some(rank) = rank {
                if rank.parse::<i64>().is_ok() {
                    Copy::Detail::reservedQueue(&rank)
                } else {
                    Copy::Detail::reservedQueueText(&rank)
                }
            } else {
                Copy::Detail::reserved.to_owned()
            };
            ("reserved", text)
        }
    }
}
