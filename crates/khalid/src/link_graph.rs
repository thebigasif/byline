//! Linked mentions from the site's link graph: under a post, the posts
//! whose bodies link to it. They read the links the related-posts choice
//! already keeps, so a saved post rendered alone lists the same posts as
//! the full render.

use khalid_core::Reader;
use khalid_render::{Link, PageBody, PostTopics};

use crate::build_pages::Page;
use crate::listed_post::ListedPost;

/// Lists on the post `page`, whose links and slug are `topic`, each post of
/// `among` that links to it, newest first, then by title, when `reader`
/// has linked mentions on.
pub(crate) fn attach(page: &mut Page, topic: &PostTopics, among: &[&ListedPost], reader: &Reader) {
    let PageBody::Post(post) = &mut page.body else {
        return;
    };
    if !reader.linked_mentions {
        return;
    }
    let mut linking: Vec<&ListedPost> = among
        .iter()
        .copied()
        .filter(|other| other.topics.slug != topic.slug)
        .filter(|other| other.topics.links.contains(&topic.slug))
        .collect();
    linking.sort_by(|a, b| {
        (b.card.date.cmp(&a.card.date)).then_with(|| a.card.title.cmp(&b.card.title))
    });
    let link = |other: &ListedPost| Link {
        name: other.card.title.clone(),
        href: other.card.href.clone(),
    };
    post.mentions = linking.into_iter().map(link).collect();
}
