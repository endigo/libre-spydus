//! Featured collections, edition-level result cards and holdings, on Canning's public pages
//! (fetched 2026-10-13): the OPAC home page's "Featured" list, the Library of Things and eLibrary
//! collection pages, two saved-search result lists and one holdings modal.

mod common;

use librespydus_core::config::ORIGIN;
use librespydus_core::parse::availability::parse_holdings;
use librespydus_core::parse::featured::{list_url, parse_collection, parse_featured};
use librespydus_core::parse::search::parse_search_results;
use librespydus_core::types::{FeaturedKind, Holding};

const BASE: &str = "https://canning.spydus.com/cgi-bin/spydus.exe";

#[test]
fn the_home_pages_featured_list() {
    let featured = parse_featured(&common::fixture("featured/home.html"), ORIGIN);
    let titles: Vec<&str> = featured.iter().map(|f| f.title.as_str()).collect();
    assert_eq!(
        titles,
        [
            "What's New", "Coming Soon", "eLibrary", "First Nations", "Languages Other Than English",
            "Library of Things", "Local History", "More to Celebrate",
        ]
    );
    let icons: Vec<Option<&str>> = featured.iter().map(|f| f.icon.as_deref()).collect();
    assert_eq!(
        icons,
        [
            Some("book"), Some("balloons"), Some("computer"), Some("circle"), Some("globe"),
            Some("game-board"), Some("map-location-dot"), Some("user-circle"),
        ]
    );
    let things = &featured[5];
    assert_eq!(things.kind, FeaturedKind::Collection);
    assert_eq!(things.url, format!("{BASE}/MSGTRNGEN/OPAC/LIBRARYOFTHINGS?HOMEPRMS=UD_LIBRARYOFTHINGS"));
    // A single saved search becomes a results list for `page`.
    let coming = &featured[1];
    assert_eq!(coming.kind, FeaturedKind::Search);
    assert_eq!(coming.url, format!("{BASE}/RUNSQRY/OPAC/BIBENQ?SQRYIRN=4050924&QRYTEXT=Coming%20Soon&NRECS=20"));
    assert_eq!(featured[0].description.as_deref(), Some("New Library items available in the last 2 months"));
    // The account links and outside services in the other lists aren't featured.
    assert!(featured.iter().all(|f| !matches!(f.title.as_str(), "Login" | "BorrowBox" | "Libby by OverDrive")));
}

#[test]
fn a_page_without_a_featured_list_has_none() {
    assert!(parse_featured(&common::fixture("featured/collection-elibrary.html"), ORIGIN).is_empty());
    assert!(parse_featured("<html><h2>Featured</h2></html>", ORIGIN).is_empty());
}

#[test]
fn the_library_of_things_collection() {
    let collection = parse_collection(&common::fixture("featured/collection-library-of-things.html"), ORIGIN);
    assert_eq!(collection.title.as_deref(), Some("Library of Things"));
    let titles: Vec<&str> = collection.sections.iter().map(|s| s.title.as_str()).collect();
    assert_eq!(
        titles,
        [
            "Active Minds", "Book Packs", "Games", "Hobbies", "Learning Resources", "Memory Bags",
            "Musical Instruments", "Sporting Equipment", "Technology",
        ]
    );
    assert_eq!(
        collection.sections[6].url,
        format!("{BASE}/RUNSQRY/OPAC/BIBENQ?SQRYIRN=4050308&QRYTEXT=Musical%20Instruments&SETLVL=SET&ISGLB=0&NRECS=20")
    );
    // Hobbies' link has a line break (`&#xA;`) in it.
    assert!(collection.sections[3].url.starts_with(&format!("{BASE}/RUNSQRY/OPAC/BIBENQ?SQRYIRN=4050482&")));
    for section in &collection.sections {
        assert!(!section.url.contains("XSLT") && !section.url.contains("SQL_RANDOM") && !section.url.contains("FMT=GD"));
        assert!(section.url.ends_with("&NRECS=20"), "{}", section.url);
    }
}

#[test]
fn a_page_of_outside_links_has_no_sections() {
    let collection = parse_collection(&common::fixture("featured/collection-elibrary.html"), ORIGIN);
    assert_eq!(collection.title.as_deref(), Some("eLibrary"));
    assert!(collection.sections.is_empty());
}

#[test]
fn list_urls_drop_the_grid_view_and_random_order() {
    assert_eq!(
        list_url("/cgi-bin/spydus.exe/RUNSQRY/OPAC/BIBENQ?SQRYIRN=1&amp;SORTS=SQL_RANDOM&amp;QRYTEXT=A b&amp;XSLT=XGridView.xsl&amp;FMT=GD&amp;NRECS=8&amp;GCN=BESTSELLERS", ORIGIN)
            .as_deref(),
        Some("https://canning.spydus.com/cgi-bin/spydus.exe/RUNSQRY/OPAC/BIBENQ?SQRYIRN=1&QRYTEXT=A%20b&NRECS=20")
    );
    // Another order stays.
    assert_eq!(
        list_url("/x/RUNSQRY/OPAC/BIBENQ?SQRYIRN=2&SORTS=TITLE", ORIGIN).as_deref(),
        Some("https://canning.spydus.com/x/RUNSQRY/OPAC/BIBENQ?SQRYIRN=2&SORTS=TITLE&NRECS=20")
    );
}

#[test]
fn edition_level_cards_get_their_author_blurb_and_a_format() {
    let results = parse_search_results(&common::fixture("featured/results-coming-soon.html"), ORIGIN, None);
    assert_eq!(results.records.len(), 20);
    let first = &results.records[0];
    assert_eq!(first.title.as_deref(), Some("American hagwon"));
    assert_eq!(first.author.as_deref(), Some("Lee, Min Jin"));
    assert_eq!(first.author_role, None);
    assert_eq!(first.year.as_deref(), Some("2026"));
    assert!(first.summary.as_deref().is_some_and(|s| s.starts_with("John and Helen Koh")));
    assert_eq!(first.formats.len(), 1);
    let format = &first.formats[0];
    assert_eq!((format.code.as_str(), format.label.as_deref()), ("", None));
    assert_eq!(
        format.availability_url.as_deref(),
        Some(format!("{BASE}/ENQ/OPAC/BIBENQ?SETLVL=&IRN=19244197&FMT=RECFMT").as_str())
    );
    assert!(format.reserve_url.as_deref().is_some_and(|u| u.contains("/PGM/OPAC/CCOPT/") && u.contains("SVL=19244197")));
    // A card with only a year has no author.
    assert!(results.records.iter().any(|r| r.author.is_none() && r.year.is_some()));
    assert!(results.next_page_url.is_some());
}

#[test]
fn library_of_things_items_have_a_blurb_but_no_author() {
    let results = parse_search_results(&common::fixture("featured/results-musical-instruments.html"), ORIGIN, None);
    let bell = &results.records[0];
    assert_eq!(bell.title.as_deref(), Some("Cage Bell : Musical Instruments"));
    assert_eq!(bell.author, None);
    assert!(bell.summary.as_deref().is_some_and(|s| s.starts_with("A fun bell for small hands!")));
    assert_eq!(bell.formats.len(), 1);
    assert!(bell.cover_urls.len() == 1);
}

#[test]
fn holdings_rows_become_copies() {
    let holdings = parse_holdings(&common::fixture("featured/holdings-cage-bell.html"));
    assert_eq!(
        holdings,
        [
            Holding {
                location: Some("RIVERTON LIBRARY".into()),
                collection: Some("Library of Things".into()),
                call_number: Some("LOT MUS011".into()),
                status: Some("On loan - Due: 29 Oct 2026".into()),
                description: Some("Pink body, teal handle".into()),
                available: false,
            },
            Holding {
                location: Some("WILLETTON LIBRARY".into()),
                collection: Some("Library of Things".into()),
                call_number: Some("LOT MUS010".into()),
                status: Some("On loan - Due: 26 Oct 2026".into()),
                description: Some("Blue body, green handle".into()),
                available: false,
            },
        ]
    );
}

#[test]
fn an_available_copy_is_available() {
    let html = r#"<table><tr><th>Location</th></tr><tr>
        <td data-caption="Location"><span class="d-block"><a href="/x"><span>CANNINGTON LIBRARY</span></a></span></td>
        <td data-caption="Status/Desc"><span class="d-block">Available</span></td></tr></table>"#;
    let holdings = parse_holdings(html);
    assert_eq!(holdings.len(), 1);
    assert_eq!(holdings[0].location.as_deref(), Some("CANNINGTON LIBRARY"));
    assert_eq!((holdings[0].status.as_deref(), holdings[0].description.as_deref()), (Some("Available"), None));
    assert!(holdings[0].available);
    assert!(parse_holdings("<div>Loading...</div>").is_empty());
}
