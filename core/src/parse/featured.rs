//! The library's own featured collections (new; no TS counterpart): the "Featured" list on the
//! OPAC home page and the collection pages it links to (Canning, 2026-10-13: What's New, Coming
//! Soon, eLibrary, First Nations, Languages Other Than English, Library of Things, Local History,
//! More to Celebrate). Each collection page is a set of titled sections, each one a saved search
//! (`RUNSQRY`) whose results `parse_search_results` reads.

use std::sync::LazyLock;

use regex::Regex;

use crate::parse::decode::{absolute, collapse, decode};
use crate::types::{FeaturedCollection, FeaturedSection, FeaturedKind, FeaturedLink};

/// `<h2>Featured</h2>` heading a `tab-container` on the home page.
static FEATURED_HEADING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<h2[^>]*>\s*Featured\s*</h2>").unwrap());
static LIST_GROUP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?s)<div[^>]*class="[^"]*\blist-group\b[^"]*"[^>]*>(.*?)</div>"#).unwrap());
static LIST_ITEM: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<a\b([^>]*class="[^"]*\blist-group-item\b[^"]*"[^>]*)>(.*?)</a>"#).unwrap()
});
static HREF: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"\bhref="([^"]*)""#).unwrap());
static TITLE_ATTR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"\btitle="([^"]*)""#).unwrap());
static ICON_CLASSES: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"<span[^>]*class="([^"]*\bfa-[^"]*)""#).unwrap());
static TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]*>").unwrap());

/// The start of a collection page section: `<div id="tabX" class="js-tab-container …">`, which
/// holds `<h2><span class="col">Title</span></h2>` and the hidden link `<a id="urltabX" href=…>`.
static SECTION_START: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"<div id="(tab[^"]+)" class="[^"]*\bjs-tab-container\b[^"]*""#).unwrap()
});
static SECTION_HEADING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)<h2[^>]*>(.*?)</h2>").unwrap());
static SECTION_LINK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"<a id="url(tab[^"]+)" href="([^"]*)""#).unwrap());
static PAGE_TITLE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)<title>(.*?)</title>").unwrap());

/// Font Awesome classes that style an icon rather than name it.
const ICON_STYLES: &[&str] = &[
    "fa", "fas", "far", "fal", "fab", "fa-fw", "fa-solid", "fa-regular", "fa-light", "fa-thin",
    "fa-duotone", "fa-brands", "fa-sharp", "fa-lg", "fa-xs", "fa-sm", "fa-2x", "fa-3x", "fa-pulse",
    "fa-spin",
];

/// The home page's "Featured" list → its links, in order. Empty when the page has none.
pub fn parse_featured(html: &str, origin: &str) -> Vec<FeaturedLink> {
    let Some(heading) = FEATURED_HEADING.find(html) else { return Vec::new() };
    let Some(group) = LIST_GROUP.captures(&html[heading.end()..]) else { return Vec::new() };
    LIST_ITEM
        .captures_iter(&group[1])
        .filter_map(|item| {
            let attrs = &item[1];
            let title = text(&item[2]);
            let href = decode(HREF.captures(attrs)?.get(1)?.as_str());
            if title.is_empty() || href.trim().is_empty() {
                return None;
            }
            let description = TITLE_ATTR
                .captures(attrs)
                .map(|t| collapse(&decode(&t[1])))
                .filter(|d| !d.is_empty() && *d != title);
            let icon = ICON_CLASSES.captures(&item[2]).and_then(|c| {
                c[1].split_whitespace()
                    .find(|class| class.starts_with("fa-") && !ICON_STYLES.contains(class))
                    .map(|class| class.trim_start_matches("fa-").to_owned())
            });
            let (kind, url) = if href.contains("/RUNSQRY/") {
                (FeaturedKind::Search, list_url(&href, origin)?)
            } else if href.contains("/MSGTRNGEN/") {
                (FeaturedKind::Collection, absolute(origin, Some(href.trim()))?)
            } else {
                (FeaturedKind::Web, absolute(origin, Some(href.trim()))?)
            };
            Some(FeaturedLink { title, description, icon, kind, url })
        })
        .collect()
}

/// A collection page (`/MSGTRNGEN/OPAC/…`) → its title and its saved-search sections, in order.
/// A page without sections (eLibrary: links to outside services) has none; open it on the web.
pub fn parse_collection(html: &str, origin: &str) -> FeaturedCollection {
    let title = PAGE_TITLE
        .captures(html)
        .map(|t| collapse(&decode(&t[1])))
        .and_then(|t| t.split(" | ").next().map(str::to_owned))
        .filter(|t| !t.is_empty());
    let starts: Vec<(usize, String)> =
        SECTION_START.captures_iter(html).map(|c| (c.get(0).map_or(0, |m| m.start()), c[1].to_owned())).collect();
    let sections = starts
        .iter()
        .enumerate()
        .filter_map(|(i, (start, id))| {
            let end = starts.get(i + 1).map_or(html.len(), |(next, _)| *next);
            let chunk = &html[*start..end];
            let link = SECTION_LINK.captures(chunk).filter(|l| &l[1] == id)?;
            let href = decode(&link[2]);
            if !href.contains("/RUNSQRY/") {
                return None;
            }
            let title = text(&SECTION_HEADING.captures(chunk)?[1]);
            Some(FeaturedSection { title: (!title.is_empty()).then_some(title)?, url: list_url(&href, origin)? })
        })
        .collect();
    FeaturedCollection { title, sections }
}

/// A saved-search link as a results list `SpydusClient::page` reads: absolute, without the
/// home-page grid view (`XSLT=XGridView.xsl`, `FMT=GD`), the random order (`SORTS=SQL_RANDOM`)
/// and `GCN`, 20 records a page (`NRECS=20`). Line breaks inside the link are dropped.
pub fn list_url(href: &str, origin: &str) -> Option<String> {
    let href: String = decode(href).chars().filter(|c| !c.is_control()).collect();
    let url = absolute(origin, Some(href.trim()))?;
    let (path, query) = url.split_once('?').unwrap_or((&url, ""));
    let mut params: Vec<String> = query
        .split('&')
        .filter(|p| !p.is_empty())
        .filter(|p| {
            let (key, value) = p.split_once('=').unwrap_or((p, ""));
            match key.to_ascii_uppercase().as_str() {
                "XSLT" | "GCN" | "NRECS" => false,
                "FMT" => !value.eq_ignore_ascii_case("GD"),
                "SORTS" => !value.eq_ignore_ascii_case("SQL_RANDOM"),
                _ => true,
            }
        })
        .map(|p| p.replace(' ', "%20"))
        .collect();
    params.push("NRECS=20".to_owned());
    Some(format!("{path}?{}", params.join("&")))
}

fn text(fragment: &str) -> String {
    collapse(&decode(&TAG.replace_all(fragment, " ")))
}
