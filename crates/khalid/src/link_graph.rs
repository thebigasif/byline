//! What a post's page shows from the site's link graph: under it, the posts
//! whose bodies link to it, and on each link in its body to another post,
//! that post's summary for the hover preview. Both read the links and cards
//! the related-posts choice already keeps, so a saved post rendered alone
//! shows the same as the full render.

use std::fmt::Write as _;

use khalid_core::Reader;
use khalid_render::{BodyHtml, Link, PageBody, PostTopics};

use crate::build_pages::Page;
use crate::listed_post::ListedPost;
use crate::post_images;

/// How every link in a body starts, up to its address.
const LINK: &str = "<a href=\"";

/// Fills the post `page`, whose links and slug are `topic`, from the posts
/// `among` in its language, as the features on in `reader` ask.
pub(crate) fn attach(page: &mut Page, topic: &PostTopics, among: &[&ListedPost], reader: &Reader) {
    let PageBody::Post(post) = &mut page.body else {
        return;
    };
    if reader.linked_mentions {
        post.mentions = mentions(topic, among);
    }
    if reader.link_previews {
        let previewed = previews(post.body.as_str(), &page.meta.path, among);
        post.body = BodyHtml::from_sanitised(previewed);
    }
}

/// Each post of `among` other than the one `topic` describes that links to
/// it, newest first, then by title.
fn mentions(topic: &PostTopics, among: &[&ListedPost]) -> Vec<Link> {
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
    linking.into_iter().map(link).collect()
}

/// `html`, the body of the post at `own`, with each link to another post of
/// `among` carrying that post's summary in `data-preview`.
fn previews(html: &str, own: &str, among: &[&ListedPost]) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(at) = rest.find(LINK) {
        let (before, link) = rest.split_at(at);
        out.push_str(before);
        let after = link.strip_prefix(LINK).unwrap_or(link);
        let Some((href, tail)) = after.split_once('"') else {
            rest = link;
            break;
        };
        let page = href.split('#').next().unwrap_or_default();
        let target = among
            .iter()
            .find(|other| other.card.href == page && page != own);
        let _ = write!(out, "{LINK}{href}\"");
        if let Some(target) = target.filter(|t| !t.card.excerpt.is_empty()) {
            let summary = post_images::attribute(&target.card.excerpt);
            let _ = write!(out, " data-preview=\"{summary}\"");
        }
        rest = tail;
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_body_without_links_is_left_as_it_is() {
        let html = "<p>No links, and a broken <a href=\"/x";
        assert_eq!(previews(html, "/blog/a/", &[]), html);
    }
}
