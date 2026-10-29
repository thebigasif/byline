//! The site rules that judge what a site says about itself.

use crate::{Hit, LintPost, RuleId, Severity, SiteRule, SiteView};

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
