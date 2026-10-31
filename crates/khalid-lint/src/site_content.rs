//! Site rules about whether each post is complete and current.

use chrono::{Months, NaiveDate};

use crate::{Hit, LintPost, RuleId, Severity, SiteRule, SiteView};

/// A language the site declares with no version of this post, named in the
/// finding. Versions share a `translation` key. Skipped when the site
/// declares fewer than two languages.
#[derive(Debug)]
pub(crate) struct MissingTranslation;

impl SiteRule for MissingTranslation {
    fn id(&self) -> RuleId {
        RuleId::new("missing-translation")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check<'site>(&self, site: &'site dyn SiteView) -> Vec<(&'site LintPost, Hit)> {
        let locales = site.locales();
        if locales.len() < 2 {
            return Vec::new();
        }
        let has_version = |post: &LintPost, locale| {
            site.posts().iter().any(|other| {
                &other.locale == locale
                    && other.translation.is_some()
                    && other.translation == post.translation
            })
        };
        let mut hits = Vec::new();
        for post in site.posts() {
            for locale in locales.iter().filter(|&l| l != &post.locale) {
                if !has_version(post, locale) {
                    hits.push((post, Hit::about(locale.as_str())));
                }
            }
        }
        hits
    }
}

/// A post due for review (F3, decision D4): it has been longer than the
/// site's limit in months since it was last reviewed, published or
/// updated, whichever came last.
#[derive(Debug)]
pub(crate) struct ReviewDue {
    /// How many months a post may go without a review, from
    /// `Thresholds::review_months`.
    pub(crate) months: u32,
    /// The date the linter started, read from the clock.
    pub(crate) today: NaiveDate,
}

impl SiteRule for ReviewDue {
    fn id(&self) -> RuleId {
        RuleId::new("review-due")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check<'site>(&self, site: &'site dyn SiteView) -> Vec<(&'site LintPost, Hit)> {
        let due = |post: &&LintPost| {
            let last = post.reviewed.max(post.updated).max(post.date);
            let next = last.and_then(|day| day.checked_add_months(Months::new(self.months)));
            next.is_some_and(|until| until < self.today)
        };
        site.posts()
            .iter()
            .filter(due)
            .map(|post| (post, Hit::post()))
            .collect()
    }
}

/// A post whose last update, or publish date when it has none, is more than
/// the site's limit in months before today.
#[derive(Debug)]
pub(crate) struct StalePost {
    /// The most months allowed, from `Thresholds::stale_months`.
    pub(crate) months: u32,
    /// The date the linter started, read from the clock.
    pub(crate) today: NaiveDate,
}

impl SiteRule for StalePost {
    fn id(&self) -> RuleId {
        RuleId::new("stale-post")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check<'site>(&self, site: &'site dyn SiteView) -> Vec<(&'site LintPost, Hit)> {
        let stale = |post: &&LintPost| {
            let last = post.updated.or(post.date);
            let fresh_until = last.and_then(|day| day.checked_add_months(Months::new(self.months)));
            fresh_until.is_some_and(|until| until < self.today)
        };
        site.posts()
            .iter()
            .filter(stale)
            .map(|post| (post, Hit::post()))
            .collect()
    }
}
