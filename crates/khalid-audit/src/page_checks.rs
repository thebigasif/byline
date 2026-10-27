//! The checks each page answers on its own, from its head.

use khalid_core::AbsoluteUrl;

use crate::body_checks::body_issues;
use crate::{Check, Crawl, Issue, Page, Robots, Thresholds, Visit};

/// The longest title search results show in full, in characters.
const TITLE_CHARS: usize = 70;
/// The longest description search results show in full, in characters.
const DESCRIPTION_CHARS: usize = 160;

/// Runs every page check on every page the crawl read.
pub(crate) fn page_issues(crawl: &Crawl, thresholds: Thresholds, issues: &mut Vec<Issue>) {
    for visit in crawl.visits() {
        if let Some(page) = visit.page() {
            let mut found = Found {
                url: visit.url(),
                issues: &mut *issues,
            };
            indexing(visit, page, &mut found);
            head(page, robots(visit, page), &mut found);
            body_issues(crawl, (visit, page), thresholds, &mut found);
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

/// The title, the description, the main heading and the language. A page
/// `robots` keeps out of search results shows no snippet, so its
/// description is not checked, as Ahrefs checks indexable pages only.
fn head(page: &Page, robots: Robots, found: &mut Found<'_>) {
    let snippet = !robots.noindex();
    let texts = [
        (page.titles(), Check::TitleMissing, Check::TitleMultiple),
        (
            page.descriptions(),
            Check::DescriptionMissing,
            Check::DescriptionMultiple,
        ),
        (
            page.headings(),
            Check::HeadingMissing,
            Check::HeadingMultiple,
        ),
    ];
    for (texts, missing, multiple) in texts {
        if missing == Check::DescriptionMissing && !snippet {
            continue;
        }
        if texts.iter().all(|text| text.trim().is_empty()) {
            found.add(missing, "");
        } else if texts.len() > 1 {
            found.add(multiple, texts.len().to_string());
        }
    }
    let longest = [
        (page.titles(), TITLE_CHARS, Check::TitleTooLong),
        (
            page.descriptions(),
            DESCRIPTION_CHARS,
            Check::DescriptionTooLong,
        ),
    ];
    for (texts, most, check) in longest {
        let length = texts.first().map_or(0, |text| text.chars().count());
        if length > most && (snippet || check != Check::DescriptionTooLong) {
            found.add(check, format!("{length} characters"));
        }
    }
    if page
        .language()
        .is_none_or(|language| language.trim().is_empty())
    {
        found.add(Check::LanguageMissing, "");
    }
}
