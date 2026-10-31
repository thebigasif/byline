//! The audit: every check over one crawl.

use crate::{Crawl, Issue, Thresholds, head_link_checks, link_checks, page_checks, sitemap_checks};

/// Runs every check over `crawl`, measuring against `thresholds`, and
/// returns the issues, sorted by check, then address.
pub fn audit(crawl: &Crawl, thresholds: Thresholds) -> Vec<Issue> {
    let mut issues = Vec::new();
    page_checks::page_issues(crawl, thresholds, &mut issues);
    link_checks::link_issues(crawl, &mut issues);
    head_link_checks::head_link_issues(crawl, &mut issues);
    sitemap_checks::sitemap_issues(crawl, &mut issues);
    issues.sort();
    issues
}
