//! Rules about a post's web address and the `[[slug]]` links it holds.

use khalid_core::{Slug, SlugMode};

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
        post.slug
            .as_str()
            .split('-')
            .filter(|part| is_year(part))
            .map(Hit::about)
            .collect()
    }
}

/// A letter outside a to z in the web address of a site that keeps ASCII
/// addresses. The finding names the transliterated form, the one
/// `khalid new` would have made (CTO answers, section 2).
#[derive(Debug)]
pub(crate) struct UnicodeSlug;

impl Rule for UnicodeSlug {
    fn id(&self) -> RuleId {
        RuleId::new("unicode-slug")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, site: &dyn SiteView) -> Vec<Hit> {
        let slug = post.slug.as_str();
        if site.slug_mode() != Some(SlugMode::Ascii) || slug.is_ascii() {
            return Vec::new();
        }
        match Slug::from_title(slug, &post.locale, SlugMode::Ascii) {
            Ok(plain) if plain.as_str().is_ascii() => vec![Hit::about(plain.as_str())],
            _ => vec![Hit::post()],
        }
    }
}

/// Whether `part` of a slug is a year from 1900 to 2099.
fn is_year(part: &str) -> bool {
    part.len() == 4
        && part.bytes().all(|b| b.is_ascii_digit())
        && (part.starts_with("19") || part.starts_with("20"))
}

/// Whether the slug rule `rule` flags `part`, one hyphen-separated piece of
/// a slug in `language`: a filler word for `slug-stopwords`, a year for
/// `slug-date`. Every other rule flags no part. `khalid lint --fix` removes
/// the flagged parts, so the repair and the rules agree.
#[must_use]
pub fn flags_slug_part(rule: RuleId, language: &str, part: &str) -> bool {
    match rule.as_str() {
        "slug-stopwords" => is_stopword(language, part),
        "slug-date" => is_year(part),
        _ => false,
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

#[cfg(test)]
mod tests {
    use khalid_core::Locale;

    use super::*;
    use crate::LintSite;

    fn unicode_hits(slug: &str, language: &str, mode: Option<SlugMode>) -> Vec<Hit> {
        let locale = Locale::try_new(language).unwrap();
        let post = LintPost::new(locale, Slug::try_new_unicode(slug).unwrap());
        let site = LintSite {
            slugs: mode,
            ..LintSite::default()
        };
        UnicodeSlug.check(&post, &site)
    }

    fn subjects(hits: &[Hit]) -> Vec<Option<&str>> {
        hits.iter().map(Hit::subject).collect()
    }

    #[test]
    fn a_german_slug_on_an_ascii_site_names_its_plain_form() {
        let hits = unicode_hits("über-größe", "de", Some(SlugMode::Ascii));
        assert_eq!(subjects(&hits), [Some("ueber-groesse")]);
    }

    #[test]
    fn a_unicode_site_or_an_unknown_mode_never_warns() {
        for mode in [Some(SlugMode::Unicode), None] {
            let hits = unicode_hits("café-menus", "en", mode);
            let none: [Option<&str>; 0] = [];
            assert_eq!(subjects(&hits), none, "{mode:?}");
        }
    }

    #[test]
    fn a_slug_with_no_plain_form_warns_without_a_suggestion() {
        let hits = unicode_hits("مرحبا", "ar", Some(SlugMode::Ascii));
        assert_eq!(subjects(&hits), [None]);
    }
}
