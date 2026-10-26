//! Related posts, chosen once at build time from what posts share.

use std::cmp::Reverse;

use chrono::NaiveDate;

/// The most related posts a post shows, the same five `khalid-content`
/// lets a post pin.
const MOST: usize = 5;

/// The fewest a post shows. With fewer related posts than this, the newest
/// others fill the list, so a reader always has somewhere to go next.
const FEWEST: usize = 3;

/// What a wikilink between two posts, in either direction, counts for.
const LINK: u32 = 3;

/// What sharing a category counts for.
const CATEGORY: u32 = 2;

/// What each shared tag counts for.
const TAG: u32 = 1;

/// What the related-posts choice reads about one post.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostTopics {
    /// The post's slug, which links and pins name it by.
    pub slug: String,
    /// The day it was published, for newest first on ties.
    pub date: NaiveDate,
    /// Its category's slug, if it has one.
    pub category: Option<String>,
    /// Its tags' slugs.
    pub tags: Vec<String>,
    /// The slugs of the posts its body links to with wikilinks.
    pub links: Vec<String>,
}

/// The posts to show under `post`, as indexes into `posts`, best first.
///
/// When `pinned` names any post in `posts`, those are the list, in the
/// order given. Otherwise each other post scores 3 for a wikilink between
/// the two in either direction, 2 for the same category and 1 for each
/// shared tag. The best five that score are kept, newest first on equal
/// scores, then by slug, so the same posts always give the same list. With
/// fewer than three, the newest other posts fill it to three.
pub fn related(post: &PostTopics, posts: &[PostTopics], pinned: &[String]) -> Vec<usize> {
    let others = || {
        posts
            .iter()
            .enumerate()
            .filter(|(_, other)| other.slug != post.slug)
    };
    let chosen: Vec<usize> = pinned
        .iter()
        .filter_map(|slug| others().find(|(_, other)| &other.slug == slug))
        .map(|(index, _)| index)
        .take(MOST)
        .collect();
    if !chosen.is_empty() {
        return chosen;
    }
    let mut scored: Vec<(u32, usize, &PostTopics)> = others()
        .map(|(index, other)| (score(post, other), index, other))
        .collect();
    scored.sort_by_key(|&(score, _, other)| (Reverse(score), Reverse(other.date), &other.slug));
    // Posts that share nothing sort last, newest first, so they are the
    // ones that fill a short list.
    let sharing = scored.iter().filter(|&&(score, _, _)| score > 0).count();
    scored
        .iter()
        .take(sharing.clamp(FEWEST, MOST))
        .map(|&(_, index, _)| index)
        .collect()
}

/// How much `a` and `b` share.
fn score(a: &PostTopics, b: &PostTopics) -> u32 {
    let linked = a.links.contains(&b.slug) || b.links.contains(&a.slug);
    let same_category = a.category.is_some() && a.category == b.category;
    let shared_tags = a.tags.iter().filter(|tag| b.tags.contains(tag)).count();
    let shared_tags = u32::try_from(shared_tags).unwrap_or(u32::MAX);
    u32::from(linked)
        .saturating_mul(LINK)
        .saturating_add(u32::from(same_category).saturating_mul(CATEGORY))
        .saturating_add(shared_tags.saturating_mul(TAG))
}
