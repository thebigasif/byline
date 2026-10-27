//! The content file of one `WordPress` post or page.

use std::path::PathBuf;

use khalid_core::{AbsoluteUrl, Config, Slug};

use crate::front_matter::FrontMatter;
use crate::markdown::body_markdown;
use crate::wordpress::names::{author_slug, category_slug, decoded};
use crate::wordpress::post_slug::{PostSlug, post_slug};
use crate::wordpress::{
    Export, Held, ImportedItem, Item, ItemError, ItemStatus, PlannedFile, SeoMeta,
};

/// Where an item's file goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Place {
    /// A published post, under `content/<lang>/posts/`.
    Post,
    /// A post not yet published, under `drafts/<lang>/` (ADR 0030).
    Draft,
    /// A page, under `content/<lang>/pages/`.
    Page,
}

/// The file for `item` in `place`, and what became of it. Every file is
/// read back as a post first; a page writes only fields a page also has.
pub(crate) fn item_file(
    item: &Item,
    export: &Export,
    site: &Config,
    place: Place,
) -> Result<(PlannedFile, ImportedItem), ItemError> {
    let locale = site.default_language();
    let PostSlug { slug, origin } =
        post_slug(&item.name, &item.title, item.id, locale, site.slugs())?;
    let seo = SeoMeta::read(item, &export.site);
    let body = body_markdown(&item.content).map_err(ItemError::Body)?;
    let description = seo
        .description
        .as_deref()
        .or(Some(item.excerpt.as_str()).filter(|text| !text.is_empty()))
        .map(decoded);
    let mut front = FrontMatter::default();
    front
        .text("title", &decoded(&item.title))
        .text("slug", slug.as_str());
    let mut held = Vec::new();
    held.extend(
        seo.title
            .as_deref()
            .map(|title| Held::SearchTitle(decoded(title))),
    );
    let path = match place {
        Place::Page => {
            front.maybe("description", description);
            held.extend(seo.robots.clone().map(Held::Robots));
            held.extend(seo.canonical.clone().map(Held::Canonical));
            format!("content/{locale}/pages/{slug}.md")
        }
        Place::Post | Place::Draft => {
            let date = item.published.map(|at| at.date());
            if place == Place::Post {
                front.text("date", &date.ok_or(ItemError::NoDate)?.to_string());
            }
            let (category, more) = categories(item, export, seo.primary_category, site);
            front
                .maybe("author", author(item, export, site))
                .maybe("category", category)
                .maybe("description", description);
            if item.sticky {
                front.yes("featured");
            }
            front
                .maybe("robots", seo.robots.as_ref())
                .maybe("canonical", seo.canonical.as_ref());
            post_held(item, export, more, &mut held);
            match (place, date) {
                (Place::Post, Some(date)) => format!("content/{locale}/posts/{date}-{slug}.md"),
                _ => format!("drafts/{locale}/{slug}.md"),
            }
        }
    };
    if !seo.focus_keywords.is_empty() {
        held.push(Held::FocusKeywords(seo.focus_keywords));
    }
    if !body.shortcodes.is_empty() {
        held.push(Held::Shortcodes(body.shortcodes));
    }
    held.extend(seo.unread.into_iter().map(Held::Unread));
    if let (ItemStatus::Future, Some(at)) = (&item.status, item.published) {
        held.push(Held::Scheduled(at));
    }
    let path = PathBuf::from(path);
    let text = front.file(&body.markdown);
    khalid_content::parse_post(&path, &text).map_err(ItemError::Content)?;
    let imported = ImportedItem {
        id: item.id,
        path: path.clone(),
        slug,
        slug_origin: origin,
        held,
    };
    Ok((PlannedFile { path, text }, imported))
}

/// What a post had that its front matter cannot hold: tags, its feature
/// image and categories beyond the first.
fn post_held(item: &Item, export: &Export, more: Vec<String>, held: &mut Vec<Held>) {
    if !item.tags.is_empty() {
        held.push(Held::Tags(item.tags.clone()));
    }
    held.extend(feature_image(item, export).map(Held::FeatureImage));
    if !more.is_empty() {
        held.push(Held::MoreCategories(more));
    }
}

/// The author's slug, the same as their own file's.
fn author(item: &Item, export: &Export, site: &Config) -> Option<Slug> {
    match export
        .authors
        .iter()
        .find(|author| author.login == item.creator)
    {
        Some(author) => author_slug(author, site),
        None => Slug::from_title(&item.creator, site.default_language(), site.slugs()).ok(),
    }
}

/// The category front matter holds, the primary one when a plugin named it,
/// and the slugs of the rest.
fn categories(
    item: &Item,
    export: &Export,
    primary: Option<u64>,
    site: &Config,
) -> (Option<Slug>, Vec<String>) {
    let primary = export
        .categories
        .iter()
        .find(|term| term.id.is_some() && term.id == primary)
        .map(|term| term.slug.as_str())
        .filter(|slug| item.categories.iter().any(|own| own == slug));
    let main = primary.or_else(|| item.categories.first().map(String::as_str));
    let more = item
        .categories
        .iter()
        .filter(|slug| Some(slug.as_str()) != main)
        .cloned()
        .collect();
    (
        main.and_then(|slug| category_slug(slug, export, site)),
        more,
    )
}

/// The address of the image a post's `_thumbnail_id` names.
fn feature_image(item: &Item, export: &Export) -> Option<AbsoluteUrl> {
    let id = item.meta("_thumbnail_id")?.parse().ok()?;
    export.item(id)?.attachment_url.clone()
}
