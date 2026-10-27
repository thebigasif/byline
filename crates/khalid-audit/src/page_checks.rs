//! The checks each page answers on its own, from its head.

use khalid_core::AbsoluteUrl;

use crate::{Check, Crawl, Issue, Page, Robots, Visit};

/// Runs every page check on every page the crawl read.
pub(crate) fn page_issues(crawl: &Crawl, issues: &mut Vec<Issue>) {
    for visit in crawl.visits() {
        if let Some(page) = visit.page() {
            let mut found = Found {
                url: visit.url(),
                issues: &mut *issues,
            };
            indexing(visit, page, &mut found);
        }
    }
}

/// The issues found on one page so far.
pub(crate) struct Found<'a> {
    pub(crate) url: &'a AbsoluteUrl,
    pub(crate) issues: &'a mut Vec<Issue>,
}

impl Found<'_> {
    /// Notes that `check` failed here, having seen `detail`.
    pub(crate) fn add(&mut self, check: Check, detail: impl Into<String>) {
        self.issues.push(Issue::new(check, self.url, detail));
    }
}

/// What the page's robots tags and its `X-Robots-Tag` header ask together.
pub(crate) fn robots(visit: &Visit, page: &Page) -> Robots {
    visit
        .header("x-robots-tag")
        .map_or(page.robots(), |said| page.robots().with(said))
}

/// Whether search engines may list the page.
fn indexing(visit: &Visit, page: &Page, found: &mut Found<'_>) {
    let robots = robots(visit, page);
    match (robots.noindex(), robots.nofollow()) {
        (true, true) => found.add(Check::NoindexNofollow, ""),
        (true, false) => found.add(Check::Noindex, ""),
        _ => {}
    }
    if let Some(canonical) = page.canonical()
        && canonical != visit.url()
    {
        found.add(Check::NonCanonical, canonical.as_str());
    }
}
