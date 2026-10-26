//! The five rules that block publishing by default (plan section 6).

use std::cmp::Reverse;
use std::collections::BTreeSet;

use crate::words::is_stopword;
use crate::{HeadingLevel, Hit, LinkKind, LintPost, Rule, RuleId, Severity, SiteView};

/// How many related posts "no internal link" suggests, as the plan asks.
const SUGGESTIONS: usize = 3;

/// The shortest word that counts as a topic word when posts are compared.
/// Shorter words are mostly grammar.
const MIN_TOPIC_CHARS: usize = 4;

/// Whether a header field is missing or holds only spaces.
fn is_blank(field: Option<&str>) -> bool {
    field.is_none_or(|text| text.trim().is_empty())
}

/// The distinct topic words in a post's title and body, in lowercase.
fn topic_words(post: &LintPost) -> BTreeSet<String> {
    let title = post.title.as_deref().unwrap_or_default();
    let text = format!("{title} {}", post.body_text).to_lowercase();
    let language = post.locale.language();
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= MIN_TOPIC_CHARS && !is_stopword(language, w))
        .map(str::to_owned)
        .collect()
}

/// The slugs of the posts in the same language that share the most topic
/// words with `post`, best first, ties in slug order.
fn related_posts(post: &LintPost, site: &dyn SiteView) -> Vec<String> {
    let own = topic_words(post);
    let mut scored: Vec<(usize, &str)> = site
        .posts()
        .iter()
        .filter(|other| other.locale == post.locale && other.slug != post.slug)
        .map(|other| {
            (
                topic_words(other).intersection(&own).count(),
                other.slug.as_str(),
            )
        })
        .filter(|&(shared, _)| shared > 0)
        .collect();
    scored.sort_by_key(|&(shared, slug)| (Reverse(shared), slug));
    scored
        .into_iter()
        .take(SUGGESTIONS)
        .map(|(_, slug)| slug.to_owned())
        .collect()
}

/// A post with no publish date.
#[derive(Debug)]
pub(crate) struct NoDate;

impl Rule for NoDate {
    fn id(&self) -> RuleId {
        RuleId::new("no-date")
    }

    fn default_severity(&self) -> Severity {
        Severity::Block
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        post.date.is_none().then(Hit::post).into_iter().collect()
    }
}

/// A post with no named author.
#[derive(Debug)]
pub(crate) struct NoAuthor;

impl Rule for NoAuthor {
    fn id(&self) -> RuleId {
        RuleId::new("no-author")
    }

    fn default_severity(&self) -> Severity {
        Severity::Block
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        let blank = is_blank(post.author.as_deref());
        blank.then(Hit::post).into_iter().collect()
    }
}

/// A post that links to no other page of the site.
#[derive(Debug)]
pub(crate) struct NoInternalLink;

impl Rule for NoInternalLink {
    fn id(&self) -> RuleId {
        RuleId::new("no-internal-link")
    }

    fn default_severity(&self) -> Severity {
        Severity::Block
    }

    fn check(&self, post: &LintPost, site: &dyn SiteView) -> Vec<Hit> {
        let internal = post
            .links
            .iter()
            .any(|link| link.kind == LinkKind::Internal);
        if internal {
            return Vec::new();
        }
        vec![Hit::post().suggesting(related_posts(post, site))]
    }
}

/// A post with no title, which is the page's main headline.
#[derive(Debug)]
pub(crate) struct NoTitle;

impl Rule for NoTitle {
    fn id(&self) -> RuleId {
        RuleId::new("no-title")
    }

    fn default_severity(&self) -> Severity {
        Severity::Block
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        let blank = is_blank(post.title.as_deref());
        blank.then(Hit::post).into_iter().collect()
    }
}

/// A page with more than one main headline: the title counts as one, and
/// each `#` headline in the body as another.
#[derive(Debug)]
pub(crate) struct ManyHeadlines;

impl Rule for ManyHeadlines {
    fn id(&self) -> RuleId {
        RuleId::new("many-headlines")
    }

    fn default_severity(&self) -> Severity {
        Severity::Block
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        let body = post
            .headings
            .iter()
            .filter(|heading| heading.level == HeadingLevel::H1);
        // Without a title, the first body headline is the one allowed.
        let allowed = usize::from(is_blank(post.title.as_deref()));
        body.skip(allowed)
            .map(|heading| Hit::about(heading.text.clone()))
            .collect()
    }
}
