//! Rules about a post's web address and the `[[slug]]` links it holds.

use crate::words::is_stopword;
use crate::{Hit, LintPost, Rule, RuleId, Severity, SiteView};

/// A web address ending longer than the site's limit.
#[derive(Debug)]
pub(crate) struct LongSlug {
    /// The most characters, from `Thresholds::slug_max_chars`.
    pub(crate) max: usize,
}

impl Rule for LongSlug {
    fn id(&self) -> RuleId {
        RuleId::new("long-slug")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        let long = post.slug.as_str().chars().count() > self.max;
        long.then(Hit::post).into_iter().collect()
    }
}

/// Filler words such as "the" in the web address, named in one finding.
#[derive(Debug)]
pub(crate) struct SlugStopwords;

impl Rule for SlugStopwords {
    fn id(&self) -> RuleId {
        RuleId::new("slug-stopwords")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        let language = post.locale.language();
        let mut found: Vec<&str> = Vec::new();
        for word in post.slug.as_str().split('-') {
            if is_stopword(language, word) && !found.contains(&word) {
                found.push(word);
            }
        }
        if found.is_empty() {
            Vec::new()
        } else {
            vec![Hit::about(found.join(", "))]
        }
    }
}

/// A year, such as 2026, as one part of the web address.
#[derive(Debug)]
pub(crate) struct SlugDate;

impl Rule for SlugDate {
    fn id(&self) -> RuleId {
        RuleId::new("slug-date")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        let is_year = |part: &&str| {
            part.len() == 4
                && part.bytes().all(|b| b.is_ascii_digit())
                && (part.starts_with("19") || part.starts_with("20"))
        };
        post.slug
            .as_str()
            .split('-')
            .filter(is_year)
            .map(Hit::about)
            .collect()
    }
}

/// A `[[slug]]` that matches no post, page or glossary term in the post's
/// language.
#[derive(Debug)]
pub(crate) struct BrokenWikilink;

impl Rule for BrokenWikilink {
    fn id(&self) -> RuleId {
        RuleId::new("broken-wikilink")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, site: &dyn SiteView) -> Vec<Hit> {
        let here = &post.locale;
        let exists = |target: &str| {
            let posts = site.posts().iter().map(|p| (&p.locale, &p.slug));
            let pages = site.pages().iter().map(|p| (&p.locale, &p.slug));
            let terms = site.glossary().iter().map(|t| (&t.locale, &t.slug));
            posts
                .chain(pages)
                .chain(terms)
                .any(|(locale, slug)| locale == here && slug.as_str() == target)
        };
        post.wikilinks
            .iter()
            .filter(|target| !exists(target.trim()))
            .map(|target| Hit::about(target.trim()))
            .collect()
    }
}
