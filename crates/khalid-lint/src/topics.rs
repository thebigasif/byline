//! Rules that compare a post with related content: near copies, glossary
//! terms and topic groups.

use std::collections::BTreeSet;
use std::hash::{DefaultHasher, Hash, Hasher};

use crate::link_graph::{links_to, page_key};
use crate::{ClusterRole, Hit, LinkKind, LintPost, Rule, RuleId, Severity, SiteView};

/// How many words make one shingle, the unit two texts are compared in.
const SHINGLE_WORDS: usize = 5;

/// The fewest shingles a text needs before it is compared at all, so two
/// one-line drafts never count as copies of each other.
const MIN_SHINGLES: usize = 10;

/// The runs of five words in `text`, ignoring case and punctuation, hashed.
/// Hashes are only compared with others from the same run, so the hasher's
/// exact output never matters.
fn shingles(text: &str) -> BTreeSet<u64> {
    let lower = text.to_lowercase();
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    let hash = |run: &[&str]| {
        let mut hasher = DefaultHasher::new();
        run.hash(&mut hasher);
        hasher.finish()
    };
    words.windows(SHINGLE_WORDS).map(hash).collect()
}

/// Another post in the same language that shares more than the site's limit
/// of its shingles with this one, named in the finding.
#[derive(Debug)]
pub(crate) struct NearDuplicate {
    /// The largest share in percent, from `Thresholds::near_duplicate_percent`.
    pub(crate) max_percent: usize,
}

impl Rule for NearDuplicate {
    fn id(&self) -> RuleId {
        RuleId::new("near-duplicate")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, site: &dyn SiteView) -> Vec<Hit> {
        let own = shingles(&post.body_text);
        if own.len() < MIN_SHINGLES {
            return Vec::new();
        }
        let others = site.posts().iter();
        let others = others.filter(|o| o.locale == post.locale && o.slug != post.slug);
        let copies = others.filter(|other| {
            let theirs = shingles(&other.body_text);
            let shared = own.intersection(&theirs).count();
            let union = own.len() + theirs.len() - shared;
            theirs.len() >= MIN_SHINGLES && shared * 100 > self.max_percent * union
        });
        copies
            .map(|other| Hit::about(other.slug.as_str()))
            .collect()
    }
}

/// Whether `term` appears in `text` as whole words. Both are lowercase.
fn mentions(text: &str, term: &str) -> bool {
    text.match_indices(term).any(|(at, _)| {
        let before = text.get(..at).and_then(|head| head.chars().next_back());
        let after = text
            .get(at + term.len()..)
            .and_then(|tail| tail.chars().next());
        !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
    })
}

/// A glossary term in the post's language that the body uses but never
/// links to, named in the finding.
#[derive(Debug)]
pub(crate) struct GlossaryTermNotLinked;

impl Rule for GlossaryTermNotLinked {
    fn id(&self) -> RuleId {
        RuleId::new("glossary-term-not-linked")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, site: &dyn SiteView) -> Vec<Hit> {
        let body = post.body_text.to_lowercase();
        let linked = |slug: &str| {
            post.wikilinks.iter().any(|target| target.trim() == slug)
                || post.links.iter().any(|link| {
                    link.kind == LinkKind::Internal
                        && page_key(&link.target).rsplit('/').next() == Some(slug)
                })
        };
        site.glossary()
            .iter()
            .filter(|term| term.locale == post.locale && !linked(term.slug.as_str()))
            .filter(|term| mentions(&body, &term.term.to_lowercase()))
            .map(|term| Hit::about(term.term.clone()))
            .collect()
    }
}

/// A member of a topic group that does not link to the group's main post in
/// its language, named in the finding. Skipped when the group has none.
#[derive(Debug)]
pub(crate) struct ClusterWithoutPillar;

impl Rule for ClusterWithoutPillar {
    fn id(&self) -> RuleId {
        RuleId::new("cluster-without-pillar")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, site: &dyn SiteView) -> Vec<Hit> {
        let Some(cluster) = post
            .cluster
            .as_ref()
            .filter(|c| c.role == ClusterRole::Member)
        else {
            return Vec::new();
        };
        let is_pillar = |other: &&LintPost| {
            other.locale == post.locale
                && other.slug != post.slug
                && other.cluster.as_ref().is_some_and(|theirs| {
                    theirs.role == ClusterRole::Pillar && theirs.name == cluster.name
                })
        };
        let pillar = site.posts().iter().find(is_pillar);
        pillar
            .filter(|pillar| !links_to(post, pillar))
            .map(|pillar| Hit::about(pillar.slug.as_str()))
            .into_iter()
            .collect()
    }
}
