//! `spydus libraries` and `spydus use`: Civica's directory (docs/cli.md §2).

use librespydus_core::LibraryInfo;
use serde::Serialize;

use crate::app::Ctx;
use crate::copy::Copy;
use crate::error::Fail;
use crate::output::{Cell, Column, Tone};
use crate::store::{Config, SavedLibrary};
use crate::text;

/// The directory, filtered to entries whose name, country, region or origin holds every word.
pub async fn libraries(ctx: &mut Ctx<'_>, words: &[String], all: bool) -> Result<(), Fail> {
    let libraries = (ctx.world.directory)(all).await.map_err(Fail::Directory)?;
    let words: Vec<String> = words.iter().map(|word| word.to_lowercase()).collect();
    let matches: Vec<&LibraryInfo> = libraries
        .iter()
        .filter(|info| {
            let haystack = format!(
                "{} {} {} {}",
                info.name,
                info.country.as_deref().unwrap_or_default(),
                region_name(&info.region),
                info.origin
            )
            .to_lowercase();
            words.iter().all(|word| haystack.contains(word.as_str()))
        })
        .collect();
    if ctx.json {
        ctx.json(&matches);
        return Ok(());
    }
    if matches.is_empty() {
        let query = words.join(" ");
        if query.is_empty() {
            ctx.out(Copy::Onboarding::emptyTitle);
        } else {
            ctx.out(&Copy::Onboarding::noMatchesTitle(&query));
            ctx.out(Copy::Onboarding::noMatchesMessage);
        }
        return Ok(());
    }
    let infos: Vec<LibraryInfo> = matches.into_iter().cloned().collect();
    print_libraries(ctx, &infos)?;
    ctx.out_styled(&Copy::Onboarding::libraries(infos.len() as i64), Tone::Dim);
    Ok(())
}

/// The table: name, region, origin (test instances say so).
pub fn print_libraries(ctx: &mut Ctx<'_>, libraries: &[LibraryInfo]) -> Result<(), Fail> {
    let columns =
        [Column::new(text::COL_NAME).wrap(), Column::new(text::COL_REGION), Column::new(text::COL_ORIGIN).wrap()];
    let rows: Vec<Vec<Cell>> = libraries
        .iter()
        .map(|info| {
            let name = if info.is_test {
                format!("{}{}{}", info.name, Copy::Common::separator, text::test_instance())
            } else {
                info.name.clone()
            };
            vec![Cell::new(name), Cell::new(region_name(&info.region)), Cell::toned(info.origin.clone(), Tone::Dim)]
        })
        .collect();
    ctx.table(&columns, &rows);
    Ok(())
}

/// The directory's region code as the apps name it.
pub fn region_name(region: &str) -> &str {
    match region {
        "AU" => Copy::Onboarding::regionAU,
        "NZ" => Copy::Onboarding::regionNZ,
        "US" => Copy::Onboarding::regionUS,
        "UK" => Copy::Onboarding::regionUK,
        "SG" => Copy::Onboarding::regionSG,
        "OTHER" | "" => Copy::Onboarding::regionOther,
        other => other,
    }
}

#[derive(Serialize)]
struct UseJson<'a> {
    library: &'a SavedLibrary,
    saved: bool,
}

/// Chooses the library and saves it (not in demo mode, which saves nothing).
pub async fn use_library(ctx: &mut Ctx<'_>, query: &str) -> Result<(), Fail> {
    let library = ctx.resolve_library(query, true).await?;
    let saved = !ctx.demo;
    if saved {
        let mut config = Config::load(&ctx.paths)?;
        config.library = Some(library.clone());
        config.save(&ctx.paths)?;
    }
    if ctx.json {
        ctx.json(&UseJson { library: &library, saved });
        return Ok(());
    }
    ctx.out_styled(Copy::Onboarding::currentLibrary, Tone::Bold);
    ctx.out(&text::library_line(&library.name, &library.origin));
    if !saved {
        ctx.out_styled(text::DEMO_NOTE, Tone::Dim);
    }
    Ok(())
}
