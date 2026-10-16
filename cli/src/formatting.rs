//! Small display helpers every command shares: author names, result counts, dates and a loan's
//! due state. Ported from the desktop app's `state/formatting.rs` (iOS `Formatting.swift`),
//! without Home's greeting. Dates follow docs/native-apps.md §7.1: "27 Oct", the year only when it
//! isn't this year.

use chrono::{DateTime, Datelike, Local, NaiveDate, TimeZone};

use crate::copy::Copy;

/// The trimmed text, or `None` when it's `None` or blank (iOS `String?.nonEmpty`).
pub fn non_empty(text: Option<&str>) -> Option<String> {
    text.map(str::trim).filter(|text| !text.is_empty()).map(str::to_owned)
}

/// "Haig, Matt, author" → "Matt Haig"; "Galligan, Gale, 1964- illustrator" → "Gale Galligan";
/// "Disney Enterprises" → unchanged. Blank → `None`.
pub fn author_name(raw: Option<&str>) -> Option<String> {
    let raw = non_empty(raw)?;
    let parts: Vec<&str> = raw.split(',').map(str::trim).filter(|part| !part.is_empty()).collect();
    let surname = parts.first()?;
    if parts.len() >= 2 && is_given_name(parts[1]) {
        return Some(format!("{} {surname}", parts[1]));
    }
    Some((*surname).to_owned())
}

/// "By Matt Haig", or `None` without an author.
pub fn byline(raw_author: Option<&str>) -> Option<String> {
    author_name(raw_author).map(|name| Copy::Detail::byline(&name))
}

/// "1 result" / "1,234 results".
pub fn result_count(total: Option<i64>) -> Option<String> {
    total.map(Copy::Search::results)
}

/// "27 Oct"; "27 Oct 2027" when it isn't `today`'s year.
pub fn short_date(date: NaiveDate, today: NaiveDate) -> String {
    if date.year() == today.year() { date.format("%-d %b").to_string() } else { date.format("%-d %b %Y").to_string() }
}

/// When the account was fetched, relative to `now`: "Updated just now", "Updated 12 min ago",
/// "Updated today at 9:05 am", "Updated yesterday at 4:20 pm", "Updated Monday at 4:20 pm"
/// (within a week), "Updated 3 Oct at 4:20 pm".
pub fn updated<Tz: TimeZone>(date: DateTime<Tz>, now: DateTime<Tz>) -> String
where
    Tz::Offset: std::fmt::Display,
{
    let seconds = (now.clone() - date.clone()).num_seconds();
    if seconds < 60 {
        return Copy::YourBooks::updatedJustNow.to_owned();
    }
    if seconds < 60 * 60 {
        return Copy::YourBooks::updatedMinutesAgo(seconds / 60);
    }
    let time = date.format("%-I:%M %P").to_string();
    let days = (now.date_naive() - date.date_naive()).num_days();
    match days {
        0 => Copy::YourBooks::updatedToday(&time),
        1 => Copy::YourBooks::updatedYesterday(&time),
        2..7 => Copy::YourBooks::updatedOn(&date.format("%A").to_string(), &time),
        _ => Copy::YourBooks::updatedOn(&short_date(date.date_naive(), now.date_naive()), &time),
    }
}

/// The first letter capitalised, for a sentence that starts with a library's name, which may be
/// "your library".
pub fn capitalised_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Today in the computer's time zone.
pub fn today() -> NaiveDate {
    Local::now().date_naive()
}

fn is_given_name(part: &str) -> bool {
    const ROLES: [&str; 14] = [
        "author",
        "editor",
        "illustrator",
        "translator",
        "narrator",
        "compiler",
        "contributor",
        "performer",
        "director",
        "creator",
        "artist",
        "photographer",
        "composer",
        "writer",
    ];
    part.chars().next().is_some_and(char::is_alphabetic) && !ROLES.contains(&part.to_lowercase().as_str())
}

/// Due dates as the library writes them ("27 Oct 2026").
pub mod due_date {
    use chrono::NaiveDate;

    /// The core's `parse_due_date` ("27 Oct 2026", "27/10/2026", "2026-10-27", …).
    pub fn parse(text: &str) -> Option<NaiveDate> {
        let day = librespydus_core::app::parse_due_date(text.to_owned())?;
        NaiveDate::from_ymd_opt(day.year, u32::from(day.month), u32::from(day.day))
    }

    /// The library's own format, "27 Oct 2026" (demo data).
    pub fn format(date: NaiveDate) -> String {
        date.format("%-d %b %Y").to_string()
    }
}

/// A loan's due state: "Due 27 Oct · 3 days left", urgent (≤ 3 days) or overdue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DueStatus {
    pub due_date: NaiveDate,
    /// Whole days from today; negative when overdue.
    pub days_left: i64,
    today: NaiveDate,
}

impl DueStatus {
    pub fn new(due_date: NaiveDate, today: NaiveDate) -> Self {
        Self { due_date, days_left: (due_date - today).num_days(), today }
    }

    /// From `PatronItem.fields["Due"]`.
    pub fn from_due_text(due_text: Option<&str>, today: NaiveDate) -> Option<Self> {
        let text = non_empty(due_text)?;
        due_date::parse(&text).map(|date| Self::new(date, today))
    }

    pub fn is_overdue(&self) -> bool {
        self.days_left < 0
    }

    /// Danger colour: overdue or due within 3 days.
    pub fn is_urgent(&self) -> bool {
        self.days_left <= 3
    }

    /// "3 days left", "1 day left", "Due today", "2 days overdue".
    pub fn days_text(&self) -> String {
        match self.days_left {
            ..0 => Copy::Loans::daysOverdue(-self.days_left),
            0 => Copy::Loans::dueToday.to_owned(),
            days => Copy::Loans::daysLeft(days),
        }
    }

    /// "Due 27 Oct".
    pub fn due_text(&self) -> String {
        Copy::Loans::dueOn(&short_date(self.due_date, self.today))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn authors_and_counts() {
        assert_eq!(author_name(Some("Haig, Matt, author")).as_deref(), Some("Matt Haig"));
        assert_eq!(author_name(Some("Galligan, Gale, 1964- illustrator")).as_deref(), Some("Gale Galligan"));
        assert_eq!(author_name(Some("Disney Enterprises")).as_deref(), Some("Disney Enterprises"));
        assert_eq!(author_name(Some("Haig, author")).as_deref(), Some("Haig"));
        assert_eq!(author_name(Some("  ")), None);
        assert_eq!(byline(Some("Haig, Matt, author")).as_deref(), Some("By Matt Haig"));
        assert_eq!(result_count(Some(1234)).as_deref(), Some("1,234 results"));
        assert_eq!(result_count(Some(1)).as_deref(), Some("1 result"));
        assert_eq!(capitalised_first("your library asks"), "Your library asks");
    }

    #[test]
    fn dates_drop_the_year_this_year() {
        let today = day(2026, 10, 14);
        assert_eq!(short_date(day(2026, 10, 27), today), "27 Oct");
        assert_eq!(short_date(day(2027, 1, 5), today), "5 Jan 2027");
        assert_eq!(due_date::parse("27 Oct 2026"), Some(day(2026, 10, 27)));
        assert_eq!(due_date::format(day(2026, 11, 3)), "3 Nov 2026");
    }

    #[test]
    fn due_status_lines() {
        let today = day(2026, 10, 14);
        let soon = DueStatus::from_due_text(Some("17 Oct 2026"), today).unwrap();
        assert_eq!(soon.days_text(), "3 days left");
        assert!(soon.is_urgent() && !soon.is_overdue());
        let overdue = DueStatus::new(day(2026, 10, 12), today);
        assert_eq!(overdue.days_text(), "2 days overdue");
        assert!(overdue.is_overdue());
        assert_eq!(DueStatus::new(today, today).days_text(), "Due today");
        assert_eq!(DueStatus::new(day(2026, 10, 15), today).days_text(), "1 day left");
        assert_eq!(DueStatus::new(day(2026, 10, 15), today).due_text(), "Due 15 Oct");
    }

    #[test]
    fn updated_lines() {
        let now = Utc.with_ymd_and_hms(2026, 10, 14, 15, 30, 0).unwrap();
        assert_eq!(updated(now - Duration::seconds(20), now), "Updated just now");
        assert_eq!(updated(now - Duration::minutes(12), now), "Updated 12 min ago");
        assert_eq!(updated(now - Duration::hours(6), now), "Updated today at 9:30 am");
        assert_eq!(updated(now - Duration::days(1), now), "Updated yesterday at 3:30 pm");
        assert_eq!(updated(now - Duration::days(2), now), "Updated Monday at 3:30 pm");
        assert_eq!(updated(now - Duration::days(11), now), "Updated 3 Oct at 3:30 pm");
    }
}
