//! The library picker's pure logic (iOS `LibraryDirectory.swift`): regions, search and sections.
//! No I/O; unit-tested below.

use std::cmp::Ordering;

use librespydus_core::LibraryInfo;

use crate::copy::Copy;

/// Picker regions in display order (docs/native-apps.md §6). Unknown region codes are `Other`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Region {
    Au,
    Nz,
    Us,
    Uk,
    Sg,
    Other,
}

impl Region {
    pub const ALL: [Region; 6] = [Self::Au, Self::Nz, Self::Us, Self::Uk, Self::Sg, Self::Other];

    pub fn from_code(code: &str) -> Self {
        match code.trim().to_ascii_uppercase().as_str() {
            "AU" => Self::Au,
            "NZ" => Self::Nz,
            "US" => Self::Us,
            "UK" => Self::Uk,
            "SG" => Self::Sg,
            _ => Self::Other,
        }
    }

    /// `LibraryInfo.region` as the core writes it.
    pub fn code(self) -> &'static str {
        match self {
            Self::Au => "AU",
            Self::Nz => "NZ",
            Self::Us => "US",
            Self::Uk => "UK",
            Self::Sg => "SG",
            Self::Other => "OTHER",
        }
    }

    /// The section header.
    pub fn title(self) -> &'static str {
        match self {
            Self::Au => Copy::Onboarding::regionAU,
            Self::Nz => Copy::Onboarding::regionNZ,
            Self::Us => Copy::Onboarding::regionUS,
            Self::Uk => Copy::Onboarding::regionUK,
            Self::Sg => Copy::Onboarding::regionSG,
            Self::Other => Copy::Onboarding::regionOther,
        }
    }

    /// What a row in this region warns about (core/README.md "Current regional support").
    pub fn caveat(self) -> Option<&'static str> {
        match self {
            Self::Uk => Some(Copy::Onboarding::noteUK),
            Self::Sg => Some(Copy::Onboarding::noteSG),
            Self::Au | Self::Nz | Self::Us | Self::Other => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    pub region: Region,
    pub libraries: Vec<LibraryInfo>,
}

/// `libraries` matching `query`, grouped by region in `Region` order and sorted by name; empty
/// regions are left out.
pub fn sections(libraries: &[LibraryInfo], query: &str) -> Vec<Section> {
    let matching = filter(libraries, query);
    Region::ALL
        .into_iter()
        .filter_map(|region| {
            let members: Vec<LibraryInfo> =
                matching.iter().filter(|library| Region::from_code(&library.region) == region).cloned().collect();
            (!members.is_empty()).then(|| Section { region, libraries: sorted(members) })
        })
        .collect()
}

/// Every whitespace-separated word of `query` must appear in the name, the country or the region's
/// name (case- and accent-insensitive), or be the region code: "city aus", "wellington",
/// "kingdom", "uk".
pub fn filter(libraries: &[LibraryInfo], query: &str) -> Vec<LibraryInfo> {
    let words: Vec<String> = query.split_whitespace().map(fold).collect();
    if words.is_empty() {
        return libraries.to_vec();
    }
    libraries
        .iter()
        .filter(|library| {
            let region = Region::from_code(&library.region);
            let haystack =
                [fold(&library.name), fold(library.country.as_deref().unwrap_or_default()), fold(region.title())];
            words.iter().all(|word| {
                library.region.eq_ignore_ascii_case(word) || haystack.iter().any(|text| text.contains(word.as_str()))
            })
        })
        .cloned()
        .collect()
}

/// By name as a person reads it ("Library 2" before "Library 10"), then by origin.
pub fn sorted(mut libraries: Vec<LibraryInfo>) -> Vec<LibraryInfo> {
    libraries.sort_by(|lhs, rhs| natural_cmp(&lhs.name, &rhs.name).then_with(|| lhs.origin.cmp(&rhs.origin)));
    libraries
}

/// Up to two initials for the logo placeholder: "City of Canning" → "CC".
pub fn monogram(name: &str) -> String {
    const SKIPPED: [&str; 6] = ["of", "the", "and", "&", "-", "–"];
    let letters: String = name
        .split(|c: char| c.is_whitespace() || c == '/')
        .filter(|word| !word.is_empty() && !SKIPPED.contains(&word.to_lowercase().as_str()))
        .take(2)
        .filter_map(|word| word.chars().find(|c| c.is_alphabetic()).or_else(|| word.chars().next()))
        .collect();
    if letters.is_empty() { "?".to_owned() } else { letters.to_uppercase() }
}

/// `https://canning.spydus.com` → `canning.spydus.com`.
pub fn host(origin: &str) -> String {
    let rest = origin.split_once("://").map_or(origin, |(_, rest)| rest);
    let host = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    if host.is_empty() { origin.to_owned() } else { host.to_owned() }
}

/// Lower case without accents, for matching ("Ōtaki" matches "otaki").
pub fn fold(text: &str) -> String {
    text.chars().flat_map(char::to_lowercase).map(strip_accent).collect()
}

fn strip_accent(c: char) -> char {
    match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' => 'a',
        'ç' | 'ć' | 'č' => 'c',
        'ď' => 'd',
        'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ė' | 'ę' | 'ě' => 'e',
        'ì' | 'í' | 'î' | 'ï' | 'ī' | 'į' => 'i',
        'ł' | 'ľ' => 'l',
        'ñ' | 'ń' | 'ň' => 'n',
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ő' => 'o',
        'ř' => 'r',
        'ś' | 'š' | 'ş' => 's',
        'ť' => 't',
        'ù' | 'ú' | 'û' | 'ü' | 'ū' | 'ů' | 'ű' => 'u',
        'ý' | 'ÿ' => 'y',
        'ź' | 'ż' | 'ž' => 'z',
        '’' => '\'',
        other => other,
    }
}

/// Case- and accent-insensitive, with runs of digits compared as numbers (Finder's order).
pub fn natural_cmp(lhs: &str, rhs: &str) -> Ordering {
    let (lhs, rhs) = (fold(lhs), fold(rhs));
    let (mut a, mut b) = (lhs.chars().peekable(), rhs.chars().peekable());
    loop {
        match (a.peek().copied(), b.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let take = |chars: &mut std::iter::Peekable<std::str::Chars<'_>>| {
                    let mut digits = String::new();
                    while let Some(c) = chars.peek().copied().filter(char::is_ascii_digit) {
                        digits.push(c);
                        chars.next();
                    }
                    digits
                };
                let (x, y) = (take(&mut a), take(&mut b));
                let (x, y) = (x.trim_start_matches('0'), y.trim_start_matches('0'));
                let order = x.len().cmp(&y.len()).then_with(|| x.cmp(y));
                if order != Ordering::Equal {
                    return order;
                }
            }
            (Some(x), Some(y)) => {
                if x != y {
                    return x.cmp(&y);
                }
                a.next();
                b.next();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn library(name: &str, region: &str, country: Option<&str>) -> LibraryInfo {
        LibraryInfo {
            id: name.into(),
            name: name.into(),
            origin: format!("https://{}.spydus.com", name.to_lowercase().replace(' ', "")),
            logo_url: None,
            country: country.map(str::to_owned),
            region: region.into(),
            is_test: false,
        }
    }

    fn directory() -> Vec<LibraryInfo> {
        vec![
            library("Wellington City Libraries", "NZ", Some("New Zealand")),
            library("City of Canning", "AU", Some("Australia")),
            library("Aberdeen City Libraries", "UK", Some("United Kingdom")),
            library("Library 10", "US", Some("United States")),
            library("Library 2", "US", Some("United States")),
            library("Nanyang School", "SG", Some("Singapore")),
            library("Niagara", "CA", Some("Canada")),
            library("Ōtaki Library", "NZ", Some("New Zealand")),
        ]
    }

    #[test]
    fn sections_follow_region_order_and_names_sort_naturally() {
        let sections = sections(&directory(), "");
        let regions: Vec<Region> = sections.iter().map(|section| section.region).collect();
        assert_eq!(regions, [Region::Au, Region::Nz, Region::Us, Region::Uk, Region::Sg, Region::Other]);
        let us: Vec<&str> = sections[2].libraries.iter().map(|library| library.name.as_str()).collect();
        assert_eq!(us, ["Library 2", "Library 10"]);
        let nz: Vec<&str> = sections[1].libraries.iter().map(|library| library.name.as_str()).collect();
        assert_eq!(nz, ["Ōtaki Library", "Wellington City Libraries"]);
    }

    #[test]
    fn every_word_must_match_a_name_country_region_or_code() {
        let names = |query: &str| -> Vec<String> {
            sections(&directory(), query)
                .into_iter()
                .flat_map(|section| section.libraries.into_iter().map(|library| library.name))
                .collect()
        };
        assert_eq!(names("wellington"), ["Wellington City Libraries"]);
        assert_eq!(names("city aus"), ["City of Canning"]);
        assert_eq!(names("kingdom"), ["Aberdeen City Libraries"]);
        assert_eq!(names("uk"), ["Aberdeen City Libraries"]);
        assert_eq!(names("otaki"), ["Ōtaki Library"]);
        assert_eq!(names("  CANADA "), ["Niagara"]);
        assert!(names("nowhere").is_empty());
        // Empty regions are left out.
        let only_au = sections(&directory(), "canning");
        assert_eq!(only_au.len(), 1);
        assert_eq!(only_au[0].region, Region::Au);
    }

    #[test]
    fn region_codes_titles_and_caveats() {
        assert_eq!(Region::from_code("uk"), Region::Uk);
        assert_eq!(Region::from_code("CA"), Region::Other);
        assert_eq!(Region::Uk.caveat(), Some("Asks you to confirm you’re human"));
        assert_eq!(Region::Sg.caveat(), Some("May be unreachable outside school networks"));
        assert_eq!(Region::Au.caveat(), None);
        assert_eq!(Region::Nz.title(), "New Zealand");
    }

    #[test]
    fn monograms_and_hosts() {
        assert_eq!(monogram("City of Canning"), "CC");
        assert_eq!(monogram("The Hutt"), "H");
        assert_eq!(monogram("Libraries Tasmania / Libraries"), "LT");
        assert_eq!(monogram("  "), "?");
        assert_eq!(host("https://canning.spydus.com"), "canning.spydus.com");
        assert_eq!(host("https://x.spydus.co.uk/cgi-bin"), "x.spydus.co.uk");
    }
}
