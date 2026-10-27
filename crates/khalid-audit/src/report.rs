//! One audit's result: the health score, the counts and the CSV export.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use crate::{Category, Check, Crawl, Issue, Severity, Thresholds, audit};

/// What one audit found: how many pages it read, which issues it found on
/// them, and the health score out of 100.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pages: usize,
    healthy: usize,
    issues: Vec<Issue>,
}

impl Report {
    /// Audits `crawl` with `thresholds`.
    ///
    /// The pages are the addresses the crawl read as HTML pages and the
    /// linked addresses that were broken. A healthy page has no error.
    pub fn of(crawl: &Crawl, thresholds: Thresholds) -> Self {
        let issues = audit(crawl, thresholds);
        let broken = [Check::Page4xx, Check::Page5xx, Check::PageUnreached];
        let pages: BTreeSet<&str> = crawl
            .visits()
            .iter()
            .filter(|visit| visit.page().is_some())
            .map(|visit| visit.url().as_str())
            .chain(
                issues
                    .iter()
                    .filter(|issue| broken.contains(&issue.check()))
                    .map(|issue| issue.url().as_str()),
            )
            .collect();
        let faulty: BTreeSet<&str> = issues
            .iter()
            .filter(|issue| issue.check().severity() == Severity::Error)
            .map(|issue| issue.url().as_str())
            .filter(|url| pages.contains(url))
            .collect();
        Self {
            pages: pages.len(),
            healthy: pages.len().saturating_sub(faulty.len()),
            issues,
        }
    }

    /// The health score out of 100: the share of pages without an error,
    /// rounded down, as Ahrefs counts it. A crawl with no page scores 0.
    pub fn score(&self) -> u8 {
        let share = self
            .healthy
            .saturating_mul(100)
            .checked_div(self.pages)
            .unwrap_or(0);
        u8::try_from(share).unwrap_or(100)
    }

    /// How many pages the audit read.
    pub fn pages(&self) -> usize {
        self.pages
    }

    /// Every issue, sorted by check, then address.
    pub fn issues(&self) -> &[Issue] {
        &self.issues
    }

    /// How many issues have `severity`.
    pub fn count(&self, severity: Severity) -> usize {
        self.issues
            .iter()
            .filter(|issue| issue.check().severity() == severity)
            .count()
    }

    /// The checks that found something, by group, each with how many
    /// issues it found, in the order the groups and checks are declared.
    pub fn by_category(&self) -> BTreeMap<Category, BTreeMap<Check, usize>> {
        let mut groups: BTreeMap<Category, BTreeMap<Check, usize>> = BTreeMap::new();
        for issue in &self.issues {
            let check = issue.check();
            let count = groups
                .entry(check.category())
                .or_default()
                .entry(check)
                .or_default();
            *count = count.saturating_add(1);
        }
        groups
    }

    /// Every issue as CSV with a header row: severity, group, check,
    /// address, detail and fix, quoted where a spreadsheet needs it.
    pub fn csv(&self) -> String {
        let mut csv = String::from("severity,category,check,url,detail,fix\n");
        for issue in &self.issues {
            let check = issue.check();
            let fields = [
                check.severity().as_str(),
                check.category().title(),
                check.title(),
                issue.url().as_str(),
                issue.detail(),
                check.fix(),
            ];
            let row: Vec<String> = fields.iter().map(|field| quoted(field)).collect();
            let _ = writeln!(csv, "{}", row.join(","));
        }
        csv
    }
}

/// `field` as one CSV field: in quotes, with quotes doubled, when it holds
/// a comma, a quote or a line break. A field a spreadsheet would read as a
/// formula gets a leading `'`, so opening the file runs nothing.
fn quoted(field: &str) -> String {
    let field = if field.starts_with(['=', '+', '-', '@']) {
        format!("'{field}")
    } else {
        field.to_owned()
    };
    if field.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field
    }
}

#[cfg(test)]
mod tests {
    use super::quoted;

    #[test]
    fn a_field_never_starts_a_formula_and_quotes_are_doubled() {
        assert_eq!(quoted("plain"), "plain");
        assert_eq!(quoted("=HYPERLINK(\"x\")"), "\"'=HYPERLINK(\"\"x\"\")\"");
        assert_eq!(quoted("+1"), "'+1");
        assert_eq!(quoted("a\nb"), "\"a\nb\"");
    }
}
