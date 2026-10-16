//! `LibraryInfo` helpers (iOS `AppModel.swift`'s `LibraryInfo` extension).

use librespydus_core::LibraryInfo;

use crate::copy::Copy;

pub const CANNING_ORIGIN: &str = "https://canning.spydus.com";

/// Canning, the core's default library: its entry in Civica's directory (logo included), so the
/// app shows it correctly before or without a directory fetch.
pub fn canning() -> LibraryInfo {
    LibraryInfo {
        // copy-exempt: Canning's entry in Civica's directory (data, as the directory gives it)
        id: "24fdee8d-ae0f-41b3-b97e-c96059bd06f4".into(),
        // copy-exempt: Canning's name, as the directory gives it (data)
        name: "City of Canning".into(),
        origin: CANNING_ORIGIN.into(),
        logo_url: Some(
            "https://canning.spydus.com/api/maintenance/1.0/imagebrowser/image?blobName=d226b0c4-06d6-44b7-ba98-e42f06b9fd15.png"
                .into(),
        ),
        // copy-exempt: directory data (Canning's country)
        country: Some("Australia".into()),
        region: "AU".into(),
        is_test: false,
    }
}

/// A stand-in built from the origin alone (`--library`), until the directory entry is known.
pub fn placeholder(origin: &str) -> LibraryInfo {
    if origin == CANNING_ORIGIN {
        return canning();
    }
    let host = origin.split("://").nth(1).unwrap_or(origin).split(['/', ':']).next().unwrap_or(origin).to_owned();
    let label = host.split('.').next().unwrap_or(&host).to_owned();
    let (region, country) = if host.ends_with(".uk") {
        ("UK", Some(Copy::Onboarding::regionUK))
    } else if host.ends_with(".nz") {
        ("NZ", Some(Copy::Onboarding::regionNZ))
    } else if host.ends_with(".sg") {
        ("SG", Some(Copy::Onboarding::regionSG))
    } else if host.ends_with(".au") {
        ("AU", Some(Copy::Onboarding::regionAU))
    } else {
        ("OTHER", None)
    };
    let mut name = label.clone();
    if let Some(first) = name.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    LibraryInfo {
        id: format!("placeholder:{origin}"),
        name,
        origin: origin.to_owned(),
        logo_url: None,
        country: country.map(str::to_owned),
        region: region.into(),
        is_test: false,
    }
}

pub fn is_placeholder(info: &LibraryInfo) -> bool {
    info.id.starts_with("placeholder:")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_guess_from_the_host() {
        assert_eq!(placeholder(CANNING_ORIGIN).name, "City of Canning");
        let hutt = placeholder("https://hutt.spydus.co.nz");
        assert_eq!((hutt.name.as_str(), hutt.region.as_str()), ("Hutt", "NZ"));
        assert!(is_placeholder(&hutt));
        assert_eq!(placeholder("https://brazoria.spydus.com").region, "OTHER");
    }
}
