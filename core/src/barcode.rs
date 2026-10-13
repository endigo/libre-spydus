//! The library card as a barcode the library's scanners read: the apps' "My card" screen draws
//! the modules [`card_barcode`] returns, so iOS and Android show identical bars. No I/O; the
//! library's symbology comes from [`crate::SpydusClient::card_barcode_symbology`].

/// A 1D barcode symbology a library's card scanners read. Spydus libraries configure one in the
/// mobile app config (`barcodeType`: Canning `Code128`, Cumberland `Code39`); Codabar is the
/// classic library-card symbology.
#[derive(uniffi::Enum, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BarcodeSymbology {
    /// Code 128 (subset C for an even run of digits, else subset B). The default.
    #[default]
    Code128,
    /// Code 39 (upper-case letters, digits, space and `-.$/+%`), wide:narrow 3:1.
    Code39,
    /// Codabar (digits and `-$:/.+`), start/stop `A`, wide:narrow 3:1.
    Codabar,
}

/// A card number ready to draw.
#[derive(uniffi::Record, Debug, Clone, PartialEq, Eq)]
pub struct CardBarcode {
    /// The symbology the bars are in: the one asked for, or Code 128 when the number has
    /// characters it can't encode.
    pub symbology: BarcodeSymbology,
    /// One entry per module (the narrowest bar or space), left to right: `true` is black. Quiet
    /// zones are not included: leave at least 10 white modules on each side.
    pub modules: Vec<bool>,
    /// The number as encoded (trimmed; upper-case for Code 39), to print under the bars.
    pub text: String,
}

/// `card_number` as a barcode in `symbology`, falling back to Code 128 when it has characters that
/// symbology can't encode. `None` for a blank number or one Code 128 can't encode either
/// (characters outside printable ASCII).
#[uniffi::export]
pub fn card_barcode(card_number: String, symbology: BarcodeSymbology) -> Option<CardBarcode> {
    let text = card_number.trim();
    if text.is_empty() {
        return None;
    }
    let preferred = match symbology {
        BarcodeSymbology::Code128 => None,
        BarcodeSymbology::Code39 => {
            let upper = text.to_ascii_uppercase();
            code39(&upper).map(|modules| (modules, upper))
        }
        BarcodeSymbology::Codabar => codabar(text).map(|modules| (modules, text.to_owned())),
    };
    match preferred {
        Some((modules, text)) => Some(CardBarcode { symbology, modules, text }),
        None => code128(text).map(|modules| CardBarcode {
            symbology: BarcodeSymbology::Code128,
            modules,
            text: text.to_owned(),
        }),
    }
}

/// The library's `barcodeType` (Spydus mobile config) as a symbology: Code 39 and Codabar by
/// name, Code 128 for everything else (including `Code128A/B/C` and anything unknown or missing).
pub fn symbology_named(barcode_type: Option<&str>) -> BarcodeSymbology {
    match barcode_type.map(|t| t.trim().to_ascii_lowercase()).as_deref() {
        Some("code39" | "code39extended") => BarcodeSymbology::Code39,
        Some("codabar") => BarcodeSymbology::Codabar,
        _ => BarcodeSymbology::Code128,
    }
}

/// Appends `widths` as alternating bar/space runs, starting with a bar.
fn push_widths(modules: &mut Vec<bool>, widths: impl IntoIterator<Item = u8>) {
    for (i, width) in widths.into_iter().enumerate() {
        modules.extend(std::iter::repeat_n(i % 2 == 0, usize::from(width)));
    }
}

// ---- Code 128 ---------------------------------------------------------------------------------

/// Bar/space widths of symbol values 0–105 (6 elements, 11 modules each), then STOP (7, 13).
const CODE128: [&str; 107] = [
    "212222", "222122", "222221", "121223", "121322", "131222", "122213", "122312", "132212",
    "221213", "221312", "231212", "112232", "122132", "122231", "113222", "123122", "123221",
    "223211", "221132", "221231", "213212", "223112", "312131", "311222", "321122", "321221",
    "312212", "322112", "322211", "212123", "212321", "232121", "111323", "131123", "131321",
    "112313", "132113", "132311", "211313", "231113", "231311", "112133", "112331", "132131",
    "113123", "113321", "133121", "313121", "211331", "231131", "213113", "213311", "213131",
    "311123", "311321", "331121", "312113", "312311", "332111", "314111", "221411", "431111",
    "111224", "111422", "121124", "121421", "141122", "141221", "112214", "112412", "122114",
    "122411", "142112", "142211", "241211", "221114", "413111", "241112", "134111", "111242",
    "121142", "121241", "114212", "124112", "124211", "411212", "421112", "421211", "212141",
    "214121", "412121", "111143", "111341", "131141", "114113", "114311", "411113", "411311",
    "113141", "114131", "311141", "411131", "211412", "211214", "211232", "2331112",
];
const START_B: usize = 104;
const START_C: usize = 105;
const STOP: usize = 106;

/// Subset C for an even number of digits (half the width), else subset B (printable ASCII).
fn code128(text: &str) -> Option<Vec<bool>> {
    let bytes = text.as_bytes();
    let digits = bytes.iter().all(u8::is_ascii_digit);
    let values: Vec<usize> = if digits && bytes.len().is_multiple_of(2) {
        std::iter::once(START_C)
            .chain(bytes.chunks(2).map(|pair| usize::from((pair[0] - b'0') * 10 + (pair[1] - b'0'))))
            .collect()
    } else {
        if !bytes.iter().all(|b| (32..127).contains(b)) {
            return None;
        }
        std::iter::once(START_B).chain(bytes.iter().map(|&b| usize::from(b - 32))).collect()
    };
    let checksum = values.iter().enumerate().map(|(i, &v)| i.max(1) * v).sum::<usize>() % 103;
    let mut modules = Vec::new();
    for value in values.into_iter().chain([checksum, STOP]) {
        push_widths(&mut modules, CODE128[value].bytes().map(|w| w - b'0'));
    }
    Some(modules)
}

// ---- Code 39 ----------------------------------------------------------------------------------

/// Each character's 9 elements (bar, space, … bar): `w` wide, `n` narrow.
const CODE39: [(char, &str); 44] = [
    ('0', "nnnwwnwnn"), ('1', "wnnwnnnnw"), ('2', "nnwwnnnnw"), ('3', "wnwwnnnnn"),
    ('4', "nnnwwnnnw"), ('5', "wnnwwnnnn"), ('6', "nnwwwnnnn"), ('7', "nnnwnnwnw"),
    ('8', "wnnwnnwnn"), ('9', "nnwwnnwnn"), ('A', "wnnnnwnnw"), ('B', "nnwnnwnnw"),
    ('C', "wnwnnwnnn"), ('D', "nnnnwwnnw"), ('E', "wnnnwwnnn"), ('F', "nnwnwwnnn"),
    ('G', "nnnnnwwnw"), ('H', "wnnnnwwnn"), ('I', "nnwnnwwnn"), ('J', "nnnnwwwnn"),
    ('K', "wnnnnnnww"), ('L', "nnwnnnnww"), ('M', "wnwnnnnwn"), ('N', "nnnnwnnww"),
    ('O', "wnnnwnnwn"), ('P', "nnwnwnnwn"), ('Q', "nnnnnnwww"), ('R', "wnnnnnwwn"),
    ('S', "nnwnnnwwn"), ('T', "nnnnwnwwn"), ('U', "wwnnnnnnw"), ('V', "nwwnnnnnw"),
    ('W', "wwwnnnnnn"), ('X', "nwnnwnnnw"), ('Y', "wwnnwnnnn"), ('Z', "nwwnwnnnn"),
    ('-', "nwnnnnwnw"), ('.', "wwnnnnwnn"), (' ', "nwwnnnwnn"), ('$', "nwnwnwnnn"),
    ('/', "nwnwnnnwn"), ('+', "nwnnnwnwn"), ('%', "nnnwnwnwn"), ('*', "nwnnwnwnn"),
];

fn code39(text: &str) -> Option<Vec<bool>> {
    let pattern = |c: char| CODE39.iter().find(|(k, _)| *k == c).map(|(_, p)| *p);
    // `*` is start/stop only.
    let patterns: Vec<&str> = text.chars().map(|c| if c == '*' { None } else { pattern(c) }).collect::<Option<_>>()?;
    let star = pattern('*')?;
    Some(narrow_wide(std::iter::once(star).chain(patterns).chain([star])))
}

// ---- Codabar ----------------------------------------------------------------------------------

/// Each character's 7 elements (bar, space, … bar): `w` wide, `n` narrow.
const CODABAR: [(char, &str); 20] = [
    ('0', "nnnnnww"), ('1', "nnnnwwn"), ('2', "nnnwnnw"), ('3', "wwnnnnn"), ('4', "nnwnnwn"),
    ('5', "wnnnnwn"), ('6', "nwnnnnw"), ('7', "nwnnwnn"), ('8', "nwwnnnn"), ('9', "wnnwnnn"),
    ('-', "nnnwwnn"), ('$', "nnwwnnn"), (':', "wnnnwnw"), ('/', "wnwnnnw"), ('.', "wnwnwnn"),
    ('+', "nnwnwnw"), ('A', "nnwwnwn"), ('B', "nwnwnnw"), ('C', "nnnwnww"), ('D', "nnnwwwn"),
];

fn codabar(text: &str) -> Option<Vec<bool>> {
    let pattern = |c: char| CODABAR.iter().find(|(k, _)| *k == c).map(|(_, p)| *p);
    // A–D are start/stop only.
    let body: Vec<&str> =
        text.chars().map(|c| if c.is_ascii_alphabetic() { None } else { pattern(c) }).collect::<Option<_>>()?;
    let a = pattern('A')?;
    Some(narrow_wide(std::iter::once(a).chain(body).chain([a])))
}

/// Narrow/wide characters, 3:1, separated by a narrow space.
fn narrow_wide<'a>(chars: impl IntoIterator<Item = &'a str>) -> Vec<bool> {
    let mut modules = Vec::new();
    for (i, pattern) in chars.into_iter().enumerate() {
        if i > 0 {
            modules.push(false);
        }
        push_widths(&mut modules, pattern.bytes().map(|e| if e == b'w' { 3 } else { 1 }));
    }
    modules
}
