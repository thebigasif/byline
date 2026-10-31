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

/// A post whose `updated` day is before its publish date, or later than
/// tomorrow. Tomorrow is allowed because today comes from UTC, and a site
/// ahead of UTC may already be on the next day (F2).
#[derive(Debug)]
pub(crate) struct UpdatedDateWrong {
    /// The date the linter started, read from the clock.
    pub(crate) today: NaiveDate,
}

impl SiteRule for UpdatedDateWrong {
    fn id(&self) -> RuleId {
        RuleId::new("updated-date-wrong")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check<'site>(&self, site: &'site dyn SiteView) -> Vec<(&'site LintPost, Hit)> {
        let latest = self.today.succ_opt().unwrap_or(self.today);
        let wrong = |post: &&LintPost| {
            post.updated.is_some_and(|updated| {
                updated > latest || post.date.is_some_and(|date| updated < date)
            })
        };
        site.posts()
            .iter()
            .filter(wrong)
            .map(|post| (post, Hit::post()))
            .collect()
    }
}
