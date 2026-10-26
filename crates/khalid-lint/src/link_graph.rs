//! Which posts link to which, built once per run for the site rules.

use std::collections::{BTreeMap, BTreeSet};

use khalid_core::Locale;

use crate::{LinkKind, LintPost, SiteView};

/// A link target without its `#section` and trailing slash, so two links to
/// the same page compare equal.
pub(crate) fn page_key(target: &str) -> &str {
    let without_section = target.split('#').next().unwrap_or_default();
    without_section.trim_end_matches('/')
}

/// The internal links between posts, by position in `SiteView::posts`.
///
/// A link finds its post by the post's `path` when the post has one, and
/// otherwise by the last part of the link matching a slug in the linking
/// post's language. Links from a post to itself do not count.
#[derive(Debug)]
pub(crate) struct LinkGraph<'site> {
    posts: &'site [LintPost],
    inbound: Vec<BTreeSet<usize>>,
}

impl<'site> LinkGraph<'site> {
    pub(crate) fn new(site: &'site dyn SiteView) -> Self {
        let posts = site.posts();
        let mut by_path: BTreeMap<&str, usize> = BTreeMap::new();
        let mut by_slug: BTreeMap<(&Locale, &str), usize> = BTreeMap::new();
        for (index, post) in posts.iter().enumerate() {
            match &post.path {
                Some(path) => by_path.insert(page_key(path), index),
                None => by_slug.insert((&post.locale, post.slug.as_str()), index),
            };
        }
        let resolve = |from: &LintPost, target: &str| {
            let page = page_key(target);
            by_path.get(page).copied().or_else(|| {
                let last = page.rsplit('/').next().unwrap_or_default();
                by_slug.get(&(&from.locale, last)).copied()
            })
        };
        let mut inbound = vec![BTreeSet::new(); posts.len()];
        for (from, post) in posts.iter().enumerate() {
            let internal = post.links.iter().filter(|l| l.kind == LinkKind::Internal);
            for to in internal.filter_map(|link| resolve(post, &link.target)) {
                if let Some(sources) = inbound.get_mut(to).filter(|_| to != from) {
                    sources.insert(from);
                }
            }
        }
        Self { posts, inbound }
    }

    /// Every post with the posts that link to it, in site order.
    pub(crate) fn posts_with_sources(
        &self,
    ) -> impl Iterator<Item = (&'site LintPost, Vec<&'site LintPost>)> + '_ {
        self.posts.iter().zip(&self.inbound).map(|(post, sources)| {
            let sources = sources.iter().filter_map(|&i| self.posts.get(i)).collect();
            (post, sources)
        })
    }
}
