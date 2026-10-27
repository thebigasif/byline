//! The audit: every check over one crawl.

use crate::{Crawl, Issue, Thresholds, page_checks};

/// Runs every check over `crawl`, measuring against `thresholds`, and
/// returns the issues, sorted by check, then address.
pub fn audit(crawl: &Crawl, thresholds: Thresholds) -> Vec<Issue> {
    let mut issues = Vec::new();
    page_checks::page_issues(crawl, thresholds, &mut issues);
    issues.sort();
    issues
}
