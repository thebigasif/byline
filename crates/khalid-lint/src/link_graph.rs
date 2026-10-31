//! Which posts link to which, built once per run for the site rules.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use khalid_core::Locale;

use crate::{LinkKind, LintPost, SiteView};

/// A link target without its `#section` and trailing slash, so two links to
/// the same page compare equal.
pub(crate) fn page_key(target: &str) -> &str {
    let without_section = target.split('#').next().unwrap_or_default();
    without_section.trim_end_matches('/')
}

/// The last part of a page key, which is a slug for most addresses.
fn last_part(page: &str) -> &str {
    page.rsplit('/').next().unwrap_or_default()
}

/// Whether `from` has an internal link that opens `to`, by `to`'s path when
/// it has one, or else by its slug in the same language.
pub(crate) fn links_to(from: &LintPost, to: &LintPost) -> bool {
    let mut internal = from.links.iter().filter(|l| l.kind == LinkKind::Internal);
    internal.any(|link| opens(&from.locale, &link.target, to))
}

/// Whether an internal link to `target` in a post written in `locale` opens
/// `to`, by `to`'s path when it has one, or else by its slug in the same
/// language.
pub(crate) fn opens(locale: &Locale, target: &str, to: &LintPost) -> bool {
    let page = page_key(target);
    match &to.path {
        Some(path) => page == page_key(path),
        None => *locale == to.locale && last_part(page) == to.slug.as_str(),
    }
}

/// The internal links between posts, by position in `SiteView::posts`.
///
/// A link finds its post by the post's `path` when the post has one, and
/// otherwise by the last part of the link matching a slug in the linking
/// post's language. Links from a post to itself do not count.
#[derive(Debug)]
pub(crate) struct LinkGraph<'site> {
    posts: &'site [LintPost],
    by_path: BTreeMap<&'site str, usize>,
    by_slug: BTreeMap<(&'site Locale, &'site str), usize>,
    inbound: Vec<BTreeSet<usize>>,
    outbound: Vec<BTreeSet<usize>>,
}

impl<'site> LinkGraph<'site> {
    pub(crate) fn new(site: &'site dyn SiteView) -> Self {
        let posts = site.posts();
        let mut graph = Self {
            posts,
            by_path: BTreeMap::new(),
            by_slug: BTreeMap::new(),
            inbound: vec![BTreeSet::new(); posts.len()],
            outbound: vec![BTreeSet::new(); posts.len()],
        };
        for (index, post) in posts.iter().enumerate() {
            match &post.path {
                Some(path) => graph.by_path.insert(page_key(path), index),
                None => graph
                    .by_slug
                    .insert((&post.locale, post.slug.as_str()), index),
            };
        }
        for (from, post) in posts.iter().enumerate() {
            let internal = post.links.iter().filter(|l| l.kind == LinkKind::Internal);
            let targets: BTreeSet<usize> = internal
                .filter_map(|link| graph.resolve(&link.target, Some(&post.locale)))
                .filter(|&to| to != from)
                .collect();
            for &to in &targets {
                if let Some(sources) = graph.inbound.get_mut(to) {
                    sources.insert(from);
                }
            }
            if let Some(slot) = graph.outbound.get_mut(from) {
                *slot = targets;
            }
        }
        graph
    }

    /// The post a link to `target` opens. Without a `locale`, a slug may
    /// match a post in any language.
    fn resolve(&self, target: &str, locale: Option<&Locale>) -> Option<usize> {
        let page = page_key(target);
        let slug = last_part(page);
        self.by_path.get(page).copied().or_else(|| match locale {
            Some(locale) => self.by_slug.get(&(locale, slug)).copied(),
            None => self
                .by_slug
                .iter()
                .find(|((_, candidate), _)| *candidate == slug)
                .map(|(_, &index)| index),
        })
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

    /// Every post with the fewest clicks it takes to reach it from the home
    /// page, whose links are `home_links`; `None` when no path reaches it.
    pub(crate) fn posts_with_depth(
        &self,
        home_links: &[String],
    ) -> impl Iterator<Item = (&'site LintPost, Option<usize>)> + '_ {
        let mut depth: Vec<Option<usize>> = vec![None; self.posts.len()];
        let mut queue = VecDeque::new();
        for start in home_links
            .iter()
            .filter_map(|link| self.resolve(link, None))
        {
            if let Some(slot @ None) = depth.get_mut(start) {
                *slot = Some(1);
                queue.push_back((start, 1));
            }
        }
        while let Some((from, clicks)) = queue.pop_front() {
            for &to in self.outbound.get(from).into_iter().flatten() {
                if let Some(slot @ None) = depth.get_mut(to) {
                    *slot = Some(clicks + 1);
                    queue.push_back((to, clicks + 1));
                }
            }
        }
        self.posts.iter().zip(depth)
    }
}
