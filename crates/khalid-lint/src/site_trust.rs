//! The rules that judge what a site says about who writes it.

use crate::{Hit, LintPost, Rule, RuleId, Severity, SiteRule, SiteView};

/// **A post whose author has no profile.**
///
/// The byline links to the author's page, which only a profile makes, so
/// without one the link leads nowhere and readers learn nothing about the
/// writer. The finding names the author. Skipped when the site's profiles
/// are unknown.
#[derive(Debug)]
pub(crate) struct AuthorWithoutProfile;

impl Rule for AuthorWithoutProfile {
    fn id(&self) -> RuleId {
        RuleId::new("author-without-profile")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, site: &dyn SiteView) -> Vec<Hit> {
        let (Some(author), Some(profiles)) = (post.author.as_deref(), site.authors()) else {
            return Vec::new();
        };
        let author = author.trim();
        let known = profiles
            .iter()
            .any(|profile| profile.locale == post.locale && profile.slug.as_str() == author);
        if author.is_empty() || known {
            return Vec::new();
        }
        vec![Hit::about(author)]
    }
}

/// The slugs a site's About or contact page may carry. A reader who wants
/// to know who writes here, and an engine judging the site's trust, looks
/// for one of these (P5).
const ABOUT_SLUGS: &[&str] = &["about", "contact", "about-us", "contact-us"];

/// **P5: a site with no About or contact page.**
///
/// A reader who cannot find who stands behind a site trusts it less, and
/// an answer engine citing it has no author to credit. The rule fires
/// once for the site when none of its pages is an About or contact page.
#[derive(Debug)]
pub(crate) struct NoAboutPage;

impl SiteRule for NoAboutPage {
    fn id(&self) -> RuleId {
        RuleId::new("no-about-page")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check<'site>(&self, site: &'site dyn SiteView) -> Vec<(&'site LintPost, Hit)> {
        let has_about = site
            .pages()
            .iter()
            .any(|page| ABOUT_SLUGS.iter().any(|slug| page.slug.as_str() == *slug));
        if has_about {
            return Vec::new();
        }
        // A site rule reports about a post; this one is about the site, so
        // it reports against the first post, which `site_health` shows.
        site.posts()
            .first()
            .map(|post| (post, Hit::post()))
            .into_iter()
            .collect()
    }
}
