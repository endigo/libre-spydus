//! Human output (docs/cli.md §3): aligned columns wrapped to the terminal's width, and colour
//! only when it's wanted (headings bold, overdue in the error colour, ready to collect in the
//! success colour). Everything stays readable with colour off.

use anstyle::{AnsiColor, Color, Style};

/// How a piece of text is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tone {
    #[default]
    Plain,
    Bold,
    Error,
    Success,
    Dim,
}

impl Tone {
    fn style(self) -> Style {
        match self {
            Tone::Plain => Style::new(),
            Tone::Bold => Style::new().bold(),
            Tone::Error => Style::new().fg_color(Some(Color::Ansi(AnsiColor::Red))),
            Tone::Success => Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green))),
            Tone::Dim => Style::new().dimmed(),
        }
    }
}

/// `text` in `tone`, with ANSI codes only when `color`.
pub fn styled(text: &str, tone: Tone, color: bool) -> String {
    if !color || tone == Tone::Plain || text.is_empty() {
        return text.to_owned();
    }
    let style = tone.style();
    format!("{style}{text}{style:#}")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    #[default]
    Left,
    Right,
}

/// One column of a table.
#[derive(Debug, Clone)]
pub struct Column {
    pub heading: String,
    pub align: Align,
    /// Never wider than this, whatever the text.
    pub max: Option<usize>,
    /// Shrinks (wrapping its words) when the table is wider than the terminal.
    pub wrap: bool,
}

impl Column {
    pub fn new(heading: &str) -> Self {
        Self { heading: heading.to_owned(), align: Align::Left, max: None, wrap: false }
    }

    pub fn right(mut self) -> Self {
        self.align = Align::Right;
        self
    }

    pub fn max(mut self, max: usize) -> Self {
        self.max = Some(max);
        self
    }

    pub fn wrap(mut self) -> Self {
        self.wrap = true;
        self
    }
}

#[derive(Debug, Clone, Default)]
pub struct Cell {
    pub text: String,
    pub tone: Tone,
}

impl Cell {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), tone: Tone::Plain }
    }

    pub fn toned(text: impl Into<String>, tone: Tone) -> Self {
        Self { text: text.into(), tone }
    }
}

/// Columns can't shrink below this when wrapping.
const MIN_WRAP_WIDTH: usize = 12;
const GAP: usize = 2;

/// `rows` under `columns`, aligned and wrapped to `width`; headings bold when `color`. Cells
/// wrap at word boundaries; a word longer than its column is split.
pub fn render_table(columns: &[Column], rows: &[Vec<Cell>], width: usize, color: bool) -> String {
    let widths = column_widths(columns, rows, width);
    let mut out = String::new();
    if columns.iter().any(|column| !column.heading.is_empty()) {
        let heading: Vec<Cell> = columns.iter().map(|column| Cell::toned(column.heading.clone(), Tone::Bold)).collect();
        push_row(&mut out, columns, &widths, &heading, color);
    }
    for row in rows {
        push_row(&mut out, columns, &widths, row, color);
    }
    out
}

fn column_widths(columns: &[Column], rows: &[Vec<Cell>], width: usize) -> Vec<usize> {
    let mut widths: Vec<usize> = columns
        .iter()
        .enumerate()
        .map(|(index, column)| {
            let content = rows
                .iter()
                .filter_map(|row| row.get(index))
                .map(|cell| cell.text.lines().map(text_width).max().unwrap_or(0))
                .max()
                .unwrap_or(0);
            let natural = content.max(text_width(&column.heading));
            column.max.map_or(natural, |max| natural.min(max))
        })
        .collect();
    let total = |widths: &[usize]| widths.iter().sum::<usize>() + GAP * columns.len().saturating_sub(1);
    while total(&widths) > width {
        // Shrink the widest wrapping column that still has room.
        let Some(index) = columns
            .iter()
            .enumerate()
            .filter(|(index, column)| column.wrap && widths[*index] > MIN_WRAP_WIDTH)
            .max_by_key(|(index, _)| widths[*index])
            .map(|(index, _)| index)
        else {
            break;
        };
        widths[index] -= 1;
    }
    widths
}

fn push_row(out: &mut String, columns: &[Column], widths: &[usize], row: &[Cell], color: bool) {
    let wrapped: Vec<Vec<String>> = columns
        .iter()
        .enumerate()
        .map(|(index, _)| {
            let text = row.get(index).map_or("", |cell| cell.text.as_str());
            wrap_lines(text, widths[index])
        })
        .collect();
    let height = wrapped.iter().map(Vec::len).max().unwrap_or(0).max(1);
    for line_index in 0..height {
        let mut line = String::new();
        for (index, column) in columns.iter().enumerate() {
            let text = wrapped[index].get(line_index).map_or("", String::as_str);
            let tone = row.get(index).map_or(Tone::Plain, |cell| cell.tone);
            let padding = widths[index].saturating_sub(text_width(text));
            let is_last = index + 1 == columns.len();
            match column.align {
                Align::Right => {
                    line.push_str(&" ".repeat(padding));
                    line.push_str(&styled(text, tone, color));
                }
                Align::Left => {
                    line.push_str(&styled(text, tone, color));
                    if !is_last {
                        line.push_str(&" ".repeat(padding));
                    }
                }
            }
            if !is_last {
                line.push_str(&" ".repeat(GAP));
            }
        }
        out.push_str(line.trim_end());
        out.push('\n');
    }
}

/// `text` broken into lines no wider than `width`, at spaces, keeping its own line breaks.
pub fn wrap_lines(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();
    for paragraph in text.lines() {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            let mut word = word.to_owned();
            // A word that can't fit on an empty line is split.
            while text_width(&word) > width {
                if !line.is_empty() {
                    lines.push(std::mem::take(&mut line));
                }
                let at = word.char_indices().nth(width).map_or(word.len(), |(at, _)| at);
                lines.push(word[..at].to_owned());
                word = word[at..].to_owned();
            }
            if line.is_empty() {
                line = word;
            } else if text_width(&line) + 1 + text_width(&word) <= width {
                line.push(' ');
                line.push_str(&word);
            } else {
                lines.push(std::mem::replace(&mut line, word));
            }
        }
        lines.push(line);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// A paragraph wrapped to `width`, each line prefixed with `indent`.
pub fn wrap_paragraph(text: &str, width: usize, indent: &str) -> String {
    let inner = width.saturating_sub(text_width(indent)).max(MIN_WRAP_WIDTH);
    wrap_lines(text, inner).into_iter().map(|line| format!("{indent}{line}")).collect::<Vec<_>>().join("\n")
}

/// Columns on a terminal: one per character (close enough for titles and names).
pub fn text_width(text: &str) -> usize {
    text.chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_align_and_wrap() {
        let columns = vec![Column::new("").right(), Column::new("Title").wrap(), Column::new("Year")];
        let rows = vec![
            vec![Cell::new("#1"), Cell::new("A fairly long title that must wrap somewhere"), Cell::new("2020")],
            vec![Cell::new("#10"), Cell::new("Short"), Cell::new("")],
        ];
        let text = render_table(&columns, &rows, 41, false);
        let lines: Vec<&str> = text.lines().collect();
        assert!(lines[0].starts_with("     Title") && lines[0].ends_with("  Year"));
        assert!(lines[1].starts_with(" #1  A fairly long title that must  ") && lines[1].ends_with("  2020"));
        assert_eq!(lines[2], "     wrap somewhere");
        assert_eq!(lines[3], "#10  Short");
        assert_eq!(text_width(lines[0]), text_width(lines[1]));
        assert!(lines.iter().all(|line| text_width(line) <= 41));
    }

    #[test]
    fn words_wrap_and_long_words_split() {
        assert_eq!(wrap_lines("one two three", 7), ["one two", "three"]);
        assert_eq!(wrap_lines("abcdefghij", 4), ["abcd", "efgh", "ij"]);
        assert_eq!(wrap_lines("a\nb", 10), ["a", "b"]);
        assert_eq!(wrap_lines("", 10), [""]);
        assert_eq!(wrap_paragraph("one two three four", 14, "  "), "  one two\n  three four");
    }

    #[test]
    fn colour_only_when_asked() {
        assert_eq!(styled("x", Tone::Error, false), "x");
        assert!(styled("x", Tone::Error, true).contains("\u{1b}["));
        assert_eq!(styled("x", Tone::Plain, true), "x");
    }
}
