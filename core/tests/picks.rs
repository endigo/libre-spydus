//! The Pick list ("My Picks"), on Canning's pages for a guest's list (fetched 2026-10-13: one work
//! saved from keyword results and one edition saved from the "Coming Soon" saved search, then the
//! same list emptied) and the website's save/remove replies.

mod common;

use librespydus_core::config::ORIGIN;
use librespydus_core::error::SpydusError;
use librespydus_core::parse::picks::parse_pick_reply;
use librespydus_core::parse::search::parse_search_results;

const BASE: &str = "https://canning.spydus.com/cgi-bin/spydus.exe";

#[test]
fn a_pick_list_holds_works_and_editions() {
    let list = parse_search_results(&common::fixture("picks/list-guest-two.html"), ORIGIN, None);
    assert!(!list.no_results);
    assert_eq!(list.total_results, Some(2));
    assert_eq!(list.records.len(), 2);

    // A work saved from search results keeps its format tabs.
    let wolf = &list.records[0];
    assert_eq!(wolf.title.as_deref(), Some("100% wolf"));
    assert_eq!(wolf.author.as_deref(), Some("Lyons, Jayne"));
    assert_eq!(wolf.work_irn.as_deref(), Some("15724969"));
    assert_eq!(wolf.formats.iter().map(|f| f.code.as_str()).collect::<Vec<_>>(), ["EBK"]);

    // An edition saved from a saved-search list is edition-level, like the list it came from.
    let hagwon = &list.records[1];
    assert_eq!(hagwon.title.as_deref(), Some("American hagwon"));
    assert_eq!(hagwon.author.as_deref(), Some("Lee, Min Jin"));
    assert_eq!(hagwon.year.as_deref(), Some("2026"));
    assert_eq!(hagwon.work_irn.as_deref(), Some("19244197"));
    assert!(hagwon.summary.as_deref().is_some_and(|s| s.starts_with("John and Helen Koh")));
    assert_eq!(hagwon.formats.len(), 1);
    let format = &hagwon.formats[0];
    assert_eq!((format.code.as_str(), format.label.as_deref()), ("", None));
    assert_eq!(
        format.availability_url.as_deref(),
        Some(format!("{BASE}/ENQ/OPAC/BIBENQ?SETLVL=&IRN=19244197&FMT=RECFMT").as_str())
    );
    assert!(format.reserve_url.as_deref().is_some_and(|u| u.contains("/PGM/OPAC/CCOPT/") && u.contains("SVL=19244197")));
}

#[test]
fn an_empty_pick_list_is_no_results() {
    let list = parse_search_results(&common::fixture("picks/list-guest-empty.html"), ORIGIN, None);
    assert!(list.records.is_empty());
    assert!(list.no_results);
}

#[test]
fn save_and_remove_replies_carry_the_list_size() {
    // A guest's first save also names the list it made (a cookie the website keeps).
    let first = r#"<?xml version="1.0"?><AJAXRES><XSVL COUNT="1"><COOKIE NAME="SVLID" VALUE="78607578"/></XSVL></AJAXRES>"#;
    assert_eq!(parse_pick_reply(first).ok(), Some(1));
    assert_eq!(parse_pick_reply(r#"<?xml version="1.0"?><AJAXRES><XSVL COUNT="2"/></AJAXRES>"#).ok(), Some(2));
    assert_eq!(parse_pick_reply(r#"<?xml version="1.0"?><AJAXRES><XSVL COUNT="0"/></AJAXRES>"#).ok(), Some(0));
}

#[test]
fn a_reply_that_isnt_a_count_is_an_error() {
    // A result set the library no longer has (`XSVL/OPAC/BIBENQ/1?SVL=…`).
    let none_found = r#"<p class="xerror" xmlns="http://www.w3.org/1999/xhtml" xmlns:civica="http://www.civica.com">None found</p>"#;
    assert!(matches!(parse_pick_reply(none_found), Err(SpydusError::Upstream { message }) if message == "None found"));
    let redirect = r#"<AJAXRES><REDIRECT URL="/cgi-bin/spydus.exe/MSGTRN/OPAC/LOGINB"/></AJAXRES>"#;
    assert!(matches!(parse_pick_reply(redirect), Err(SpydusError::Upstream { message }) if message.contains("LOGINB")));
    assert!(matches!(parse_pick_reply("<html>maintenance</html>"), Err(SpydusError::Parse { .. })));
}
