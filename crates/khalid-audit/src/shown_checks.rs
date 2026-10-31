//! The AI search checks on what a page shows (phase GEO, A1, built on P1
//! and F1): answers in view rather than folded away, the article's latest
//! date on the page, and whether that date is recent.

use chrono::{Months, NaiveDate};
use serde_json::Value;

use crate::page_checks::Found;
use crate::{Check, Page, Thresholds};

/// The schema.org types that are an article, as Google reads them.
const ARTICLES: [&str; 4] = ["Article", "BlogPosting", "NewsArticle", "TechArticle"];

/// Runs the checks on what one page shows.
pub(crate) fn shown_issues(page: &Page, thresholds: Thresholds, found: &mut Found<'_>) {
    for question in page.folded() {
        found.add(Check::HiddenAnswer, question.as_str());
    }
    let Some(last) = latest_date(page) else {
        return;
    };
    let day = last.format("%Y-%m-%d").to_string();
    if !page.dates().iter().any(|shown| shown.starts_with(&day)) {
        found.add(Check::DateNotShown, day.as_str());
    }
    let window = thresholds
        .today
        .and_then(|today| today.checked_sub_months(Months::new(thresholds.stale_months)));
    if window.is_some_and(|since| last < since) {
        found.add(Check::StalePage, day);
    }
}

/// The day the page's article was last changed: its `dateModified`, or its
/// `datePublished` when it has none. `None` when no JSON-LD block holds an
/// article with a readable date.
fn latest_date(page: &Page) -> Option<NaiveDate> {
    page.structured_data()
        .iter()
        .filter_map(|block| serde_json::from_str::<Value>(block).ok())
        .find_map(|value| article_date(&value))
}

/// The latest date of the first article in `value`, searched depth first,
/// so an article inside `@graph` counts.
fn article_date(value: &Value) -> Option<NaiveDate> {
    match value {
        Value::Array(items) => items.iter().find_map(article_date),
        Value::Object(node) => {
            let kind = node.get("@type");
            let is_article = match kind {
                Some(Value::String(kind)) => ARTICLES.contains(&kind.as_str()),
                Some(Value::Array(kinds)) => kinds
                    .iter()
                    .any(|kind| kind.as_str().is_some_and(|kind| ARTICLES.contains(&kind))),
                _ => false,
            };
            let date = ["dateModified", "datePublished"]
                .iter()
                .find_map(|key| node.get(*key).and_then(Value::as_str).and_then(day_of));
            match date {
                Some(date) if is_article => Some(date),
                _ => node.values().find_map(article_date),
            }
        }
        _ => None,
    }
}

/// The day an ISO 8601 date or date and time falls on, such as
/// `2026-09-14T08:00:00Z`.
fn day_of(text: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(text.get(..10)?, "%Y-%m-%d").ok()
}
