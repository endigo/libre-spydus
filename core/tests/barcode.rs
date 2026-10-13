//! `barcode`: card numbers as Code 128 / Code 39 / Codabar modules. The golden module strings
//! were decoded back to their text by Apple's Vision barcode reader (VNDetectBarcodesRequest,
//! 2026-10-13); Codabar reads as `A…A` there, with its start/stop characters.

use librespydus_core::barcode::{BarcodeSymbology, card_barcode, symbology_named};

fn bits(text: &str, symbology: BarcodeSymbology) -> (BarcodeSymbology, String, String) {
    let b = card_barcode(text.into(), symbology).expect("encodes");
    (b.symbology, b.text, b.modules.iter().map(|&m| if m { '1' } else { '0' }).collect())
}

const CODE128_CL114D: &str = "1101001000010001000110100011011101001110011010011100110110010011101011000100010011101100100111011001001110011011001110010110010111001100100111011011100100110110011001100011101011";
const CODE128_C_DIGITS: &str = "1101001110011011100100111011011101100010100011011001100111000101101100001010011011110110110011101001100011101011";
const CODE39_CL114D: &str = "10001011101110101110111010001010101110101000111011101000101011101110100010101110101000111010111010101110001011101010001110111010101000111011101011101000101011101011100010101110111011100010101010100011101011101110100011101010100010111011101";
const CODABAR_DIGITS: &str = "101110001000101010001011101010111000101010001011101110001010101011101000101010100011101010100011101010100011101110101000101000101011101000101110101000111010101110100010101010100011101011100010001";

#[test]
fn code128_matches_what_vision_decoded() {
    let (symbology, text, modules) = bits(" CL114D0012345 ", BarcodeSymbology::Code128);
    assert_eq!((symbology, text.as_str()), (BarcodeSymbology::Code128, "CL114D0012345"));
    assert_eq!(modules, CODE128_CL114D);
    // Subset B: start + 13 characters + checksum, 11 modules each, then the 13-module stop.
    assert_eq!(modules.len(), 11 * 15 + 13);
}

#[test]
fn code128_uses_subset_c_for_an_even_run_of_digits() {
    let (_, _, modules) = bits("21234000567890", BarcodeSymbology::Code128);
    assert_eq!(modules, CODE128_C_DIGITS);
    assert_eq!(modules.len(), 11 * (7 + 2) + 13);
    // An odd number of digits stays in subset B.
    assert_eq!(bits("12345", BarcodeSymbology::Code128).2.len(), 11 * 7 + 13);
}

#[test]
fn code39_matches_what_vision_decoded_and_upper_cases() {
    assert_eq!(bits("CL114D0012345", BarcodeSymbology::Code39).2, CODE39_CL114D);
    let (symbology, text, _) = bits("cl114d0012345", BarcodeSymbology::Code39);
    assert_eq!((symbology, text.as_str()), (BarcodeSymbology::Code39, "CL114D0012345"));
    // 15 modules per character with its start/stop, narrow gaps between.
    assert_eq!(CODE39_CL114D.len(), 15 * 15 + 14);
}

#[test]
fn codabar_matches_what_vision_decoded() {
    let (symbology, text, modules) = bits("21234000567890", BarcodeSymbology::Codabar);
    assert_eq!((symbology, text.as_str()), (BarcodeSymbology::Codabar, "21234000567890"));
    assert_eq!(modules, CODABAR_DIGITS);
}

#[test]
fn what_a_symbology_cannot_encode_falls_back_to_code128() {
    // Codabar has no letters; Code 39 no lower-case `@`.
    assert_eq!(bits("CL114D0012345", BarcodeSymbology::Codabar).2, CODE128_CL114D);
    assert_eq!(bits("CL114D0012345", BarcodeSymbology::Codabar).0, BarcodeSymbology::Code128);
    assert_eq!(bits("me@library", BarcodeSymbology::Code39).0, BarcodeSymbology::Code128);
    assert_eq!(bits("*12*", BarcodeSymbology::Code39).0, BarcodeSymbology::Code128);
    assert_eq!(bits("A123A", BarcodeSymbology::Codabar).0, BarcodeSymbology::Code128);
}

#[test]
fn blank_and_non_ascii_numbers_have_no_barcode() {
    assert_eq!(card_barcode("  ".into(), BarcodeSymbology::Code128), None);
    assert_eq!(card_barcode("CL114Ä".into(), BarcodeSymbology::Code39), None);
}

#[test]
fn every_module_run_starts_with_a_bar() {
    for symbology in [BarcodeSymbology::Code128, BarcodeSymbology::Code39, BarcodeSymbology::Codabar] {
        let b = card_barcode("123456".into(), symbology).unwrap();
        assert_eq!((b.modules.first(), b.modules.last()), (Some(&true), Some(&true)), "{symbology:?}");
    }
}

#[test]
fn library_barcode_types_map_to_symbologies() {
    assert_eq!(symbology_named(Some("Code128")), BarcodeSymbology::Code128);
    assert_eq!(symbology_named(Some("Code128B")), BarcodeSymbology::Code128);
    assert_eq!(symbology_named(Some("Code39")), BarcodeSymbology::Code39);
    assert_eq!(symbology_named(Some(" code39extended ")), BarcodeSymbology::Code39);
    assert_eq!(symbology_named(Some("Codabar")), BarcodeSymbology::Codabar);
    assert_eq!(symbology_named(Some("EAN13")), BarcodeSymbology::Code128);
    assert_eq!(symbology_named(None), BarcodeSymbology::Code128);
}
