//! The slugs of a Ghost post's people and tags, and what the post holds
//! that its file cannot yet.

use khalid_core::{AbsoluteUrl, Config};

use crate::Place;
use crate::ghost::post_file::Source;
use crate::ghost::{Author, Held, Post, PostStatus, ShareCard, Visibility};
use crate::slug::post_slug;

/// An author's slug in Khalid: Ghost's own, kept when the site's rules
/// allow, else made from it or from the name.
pub(crate) fn author_slug(author: &Author, site: &Config) -> Option<String> {
    let locale = site.default_language();
    let made = post_slug(&author.slug, &author.name, &author.id, locale, site.slugs()).ok()?;
    Some(made.slug.as_str().to_owned())
}

/// The slugs of the authors of `post` whose ids the export knows, the main
/// one first.
pub(crate) fn author_slugs(post: &Post, from: Source<'_>) -> Vec<String> {
    post.authors
        .iter()
        .filter_map(|id| from.export.authors.iter().find(|author| &author.id == id))
        .filter_map(|author| author_slug(author, from.site))
        .collect()
}

/// `text` as a full address, with Ghost's own address filled in.
pub(crate) fn address(text: Option<&String>, from: Source<'_>) -> Option<AbsoluteUrl> {
    AbsoluteUrl::parse(&text?.replace("__GHOST_URL__", from.origin)).ok()
}

/// What `post`, placed in `place`, holds that its file cannot, in a fixed
/// order; `more` are the slugs of its authors after the main one.
pub(crate) fn held(post: &Post, from: Source<'_>, place: Place, more: Vec<String>) -> Vec<Held> {
    let seo = &post.seo;
    let mut held = Vec::new();
    held.extend(
        seo.meta_title
            .clone()
            .filter(|title| *title != post.title)
            .map(Held::SearchTitle),
    );
    if !more.is_empty() {
        held.push(Held::MoreAuthors(more));
    }
    let tags: Vec<String> = post
        .tags
        .iter()
        .filter_map(|id| from.export.tags.iter().find(|tag| &tag.id == id))
        .filter(|tag| !tag.internal)
        .map(|tag| tag.slug.clone())
        .collect();
    if !tags.is_empty() {
        held.push(Held::Tags(tags));
    }
    held.extend(address(post.feature_image.as_ref(), from).map(Held::FeatureImage));
    let card = |title: &Option<String>, description: &Option<String>, image| ShareCard {
        title: title.clone(),
        description: description.clone(),
        image: address(image, from),
    };
    let open_graph = card(&seo.og_title, &seo.og_description, seo.og_image.as_ref());
    let twitter = card(
        &seo.twitter_title,
        &seo.twitter_description,
        seo.twitter_image.as_ref(),
    );
    if open_graph != ShareCard::default() {
        held.push(Held::OpenGraph(open_graph));
    }
    if twitter != ShareCard::default() {
        held.push(Held::Twitter(twitter));
    }
    if let (PostStatus::Scheduled, Some(at)) = (&post.status, post.published) {
        held.push(Held::Scheduled(at));
    }
    if place == Place::Draft && post.visibility != Visibility::Public {
        held.push(Held::MembersOnly(post.visibility.clone()));
    }
    if place == Place::Page {
        held.extend(address(post.canonical_url.as_ref(), from).map(Held::Canonical));
    }
    held
}
