//! The reservation messages the core writes itself, in two wordings. The parsers return the
//! middleware's (the goldens check it, so parity holds); `SpydusClient` hands the apps the wording
//! of docs/native-apps.md §7.1: curly apostrophes, no full stop on a success, "this reservation".
//! A message from the library itself passes through unchanged.

/// (the parsers' wording, the apps' wording)
const MESSAGES: &[(&str, &str)] = &[
    ("Reservation placed.", "Reservation placed"),
    ("Reservation cancelled.", "Reservation cancelled"),
    ("Reservation updated.", "Reservation updated"),
    ("Couldn't cancel the reservation.", "Couldn’t cancel this reservation."),
    ("Couldn't update the reservation.", "Couldn’t update this reservation."),
    ("Reservation could not be completed.", "Couldn’t complete this reservation."),
    ("This reservation can't be cancelled right now.", "This reservation can’t be cancelled right now."),
    ("This reservation can't be edited right now.", "This reservation can’t be edited right now."),
    ("This title can't be reserved right now.", "This title can’t be reserved right now."),
];

/// `message` in the apps' wording when the core wrote it; the library's own words unchanged.
pub(crate) fn for_reader(message: Option<String>) -> Option<String> {
    message.map(|m| MESSAGES.iter().find(|(core, _)| *core == m).map_or(m, |(_, reader)| (*reader).to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_messages_get_the_apps_wording() {
        assert_eq!(for_reader(Some("Reservation placed.".into())).as_deref(), Some("Reservation placed"));
        assert_eq!(
            for_reader(Some("Reservation could not be completed.".into())).as_deref(),
            Some("Couldn’t complete this reservation.")
        );
        // The library's own words, and no message, pass through.
        let library = "You already have a reservation for this title.";
        assert_eq!(for_reader(Some(library.into())).as_deref(), Some(library));
        assert_eq!(for_reader(None), None);
    }

    #[test]
    fn every_core_wording_is_still_written_by_the_parsers() {
        // A message the parsers no longer write would silently keep its old wording in the apps.
        let sources = [include_str!("parse/reserve.rs"), include_str!("fetch.rs")].concat();
        for (core, reader) in MESSAGES {
            assert!(sources.contains(&format!("\"{core}\"")), "{core:?} isn't written by the parsers any more");
            assert!(!reader.contains('\''), "{reader:?} needs a curly apostrophe");
        }
    }
}
