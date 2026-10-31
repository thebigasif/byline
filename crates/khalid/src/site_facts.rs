//! The site's fact registry, from `facts.toml` in the site folder (P7).
//!
//! Each table is one fact posts repeat, such as a price, named by its key:
//!
//! ```toml
//! [pro-plan-price]
//! value = "29 USD a month"
//! old = ["19 USD a month"]
//! checked = 2026-09-01
//! ```
//!
//! The linter warns when a post still gives an old value, and when a post
//! gives a fact nobody has checked for six months.

use std::collections::BTreeMap;
use std::path::Path;

use chrono::NaiveDate;
use khalid_core::{SitePath, Slug, UserMessage};
use khalid_lint::LintFact;
use serde::Deserialize;
use toml::value::Datetime;

/// The file's name in the site folder.
const FILE: &str = "facts.toml";

/// Every facts message shares this reason.
const WHY: &str = "facts.toml holds the values every post must agree on, so Khalid reads it first.";

/// One fact as written, under its key.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    value: String,
    #[serde(default)]
    old: Vec<String>,
    checked: Datetime,
}

/// Every fact in `site`'s `facts.toml`, sorted by key. A site without the
/// file has none.
///
/// # Errors
///
/// Returns what, why and fix when the file cannot be read, is not TOML, or
/// holds a fact with a key that is not lowercase words and hyphens, an
/// empty value, or a `checked` that is not a plain date.
pub(crate) fn load(site: &Path) -> Result<Vec<LintFact>, UserMessage> {
    let path = SitePath::within(site, FILE).map_err(|_| unreadable())?;
    let text = match std::fs::read_to_string(path.as_path()) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(_) => return Err(unreadable()),
    };
    parse(&text)
}

/// The facts in `text`, the contents of a `facts.toml`, sorted by key.
fn parse(text: &str) -> Result<Vec<LintFact>, UserMessage> {
    let entries: BTreeMap<String, Entry> = toml::from_str(text).map_err(|error| {
        let line = error
            .span()
            .and_then(|span| text.get(..span.start))
            .map_or(1, |before| before.matches('\n').count().saturating_add(1));
        wrong(line, error.message())
    })?;
    entries
        .into_iter()
        .map(|(key, entry)| fact(&key, entry))
        .collect()
}

/// The fact `key` as the linter reads it, once its parts are checked.
fn fact(key: &str, entry: Entry) -> Result<LintFact, UserMessage> {
    if Slug::try_new(key).is_err() {
        return Err(bad_key(key));
    }
    let value = entry.value.trim();
    if value.is_empty() {
        return Err(no_value(key));
    }
    let checked = plain_date(&entry.checked).ok_or_else(|| not_a_date(key))?;
    let old = entry.old.into_iter().map(|old| old.trim().to_owned());
    let old = old.filter(|old| !old.is_empty()).collect();
    Ok(LintFact::new(key, value, checked).with_old(old))
}

/// The day `datetime` names, when it is a date with no time of day.
fn plain_date(datetime: &Datetime) -> Option<NaiveDate> {
    let date = datetime.date.filter(|_| datetime.time.is_none())?;
    let year = i32::from(date.year);
    NaiveDate::from_ymd_opt(year, u32::from(date.month), u32::from(date.day))
}

fn unreadable() -> UserMessage {
    UserMessage::new(
        "Khalid could not read facts.toml in the site folder.",
        WHY,
        "Check that facts.toml is a file you can open, or remove it.",
    )
}

fn wrong(line: usize, detail: &str) -> UserMessage {
    UserMessage::new(
        format!(
            "facts.toml has a mistake on line {line}: {}.",
            detail.trim_end_matches('.')
        ),
        WHY,
        "Write each fact as a table with a value and a checked date, such as [pro-plan-price].",
    )
}

fn bad_key(key: &str) -> UserMessage {
    UserMessage::new(
        format!("facts.toml names a fact `{key}`, which is not a usable name."),
        WHY,
        "Rename it with lowercase letters, digits and hyphens, such as [pro-plan-price].",
    )
}

fn no_value(key: &str) -> UserMessage {
    UserMessage::new(
        format!("The fact `{key}` in facts.toml has an empty value."),
        WHY,
        format!(
            "Write the value posts should give in [{key}], such as value = \"29 USD a month\"."
        ),
    )
}

fn not_a_date(key: &str) -> UserMessage {
    UserMessage::new(
        format!("The fact `{key}` in facts.toml has a checked date that is not a plain date."),
        WHY,
        format!("Write the day you last checked it in [{key}], such as checked = 2026-09-01."),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    #[test]
    fn facts_are_read_sorted_by_key_with_old_values_trimmed() {
        let text = "[pro-plan-price]\nvalue = \" 29 USD a month \"\n\
            old = [\"19 USD a month \", \"\"]\nchecked = 2026-09-01\n\n\
            [founded]\nvalue = \"2019\"\nchecked = 2026-06-01\n";
        let want = vec![
            LintFact::new("founded", "2019", day(2026, 6, 1)),
            LintFact::new("pro-plan-price", "29 USD a month", day(2026, 9, 1))
                .with_old(vec!["19 USD a month".to_owned()]),
        ];
        assert_eq!(parse(text).unwrap(), want);
    }

    #[test]
    fn an_empty_file_has_no_facts() {
        assert_eq!(parse("").unwrap(), Vec::new());
    }

    #[test]
    fn a_fact_without_a_checked_date_is_refused_with_its_line() {
        let message = parse("\n[price]\nvalue = \"29 USD\"\n").unwrap_err();
        assert!(message.what().contains("line 2"), "{message}");
    }

    #[test]
    fn a_key_that_is_not_a_usable_name_is_refused() {
        let message = parse("[\"Pro Price\"]\nvalue = \"29\"\nchecked = 2026-09-01\n").unwrap_err();
        assert!(message.what().contains("`Pro Price`"), "{message}");
    }

    #[test]
    fn an_empty_value_is_refused() {
        let message = parse("[price]\nvalue = \"  \"\nchecked = 2026-09-01\n").unwrap_err();
        assert!(message.what().contains("empty value"), "{message}");
    }

    #[test]
    fn a_checked_time_or_text_is_refused() {
        let with_time = "[price]\nvalue = \"29\"\nchecked = 2026-09-01T10:00:00\n";
        assert!(parse(with_time).unwrap_err().what().contains("plain date"));
        let as_text = "[price]\nvalue = \"29\"\nchecked = \"2026-09-01\"\n";
        assert!(parse(as_text).is_err(), "a quoted date is text, not a date");
    }

    #[test]
    fn an_unknown_key_in_a_fact_is_refused() {
        let text = "[price]\nvalue = \"29\"\nchecked = 2026-09-01\nsource = \"x\"\n";
        assert!(parse(text).unwrap_err().what().contains("line 4"));
    }
}
