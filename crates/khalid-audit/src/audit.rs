//! The audit: every check over one crawl.

use crate::{Crawl, Issue, page_checks};

/// Runs every check over `crawl` and returns the issues, sorted by check,
/// then address.
pub fn audit(crawl: &Crawl) -> Vec<Issue> {
    let mut issues = Vec::new();
    page_checks::page_issues(crawl, &mut issues);
    issues.sort();
    issues
}
