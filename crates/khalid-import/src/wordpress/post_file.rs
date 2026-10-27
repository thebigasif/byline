//! The content file of one published `WordPress` post.

use std::path::PathBuf;

use khalid_core::{AbsoluteUrl, Config, Slug};

use crate::front_matter::FrontMatter;
use crate::markdown::body_markdown;
use crate::wordpress::post_slug::{PostSlug, post_slug};
use crate::wordpress::{Export, Held, ImportedItem, Item, ItemError, PlannedFile, SeoMeta};

/// The file for `item`, a published post, and what became of it.
pub(crate) fn post_file(
    item: &Item,
    export: &Export,
    site: &Config,
) -> Result<(PlannedFile, ImportedItem), ItemError> {
    let locale = site.default_language();
    let PostSlug { slug, origin } =
        post_slug(&item.name, &item.title, item.id, locale, site.slugs())?;
    let date = item
        .published
        .map(|at| at.date())
        .ok_or(ItemError::NoDate)?;
    let seo = SeoMeta::read(item, &export.site);
    let body = body_markdown(&item.content).map_err(ItemError::Body)?;
    let (category, more) = categories(item, export, seo.primary_category, site);
    let description = seo
        .description
        .as_deref()
        .or(Some(item.excerpt.as_str()).filter(|text| !text.is_empty()))
        .map(decoded);
    let mut front = FrontMatter::default();
    front
        .text("title", &decoded(&item.title))
        .text("slug", slug.as_str())
        .text("date", &date.to_string())
        .maybe("author", author(item, export, site))
        .maybe("category", category)
        .maybe("description", description);
    if item.sticky {
        front.yes("featured");
    }
    front
        .maybe("robots", seo.robots.as_ref())
        .maybe("canonical", seo.canonical.as_ref());
    let path = PathBuf::from(format!("content/{locale}/posts/{date}-{slug}.md"));
    let text = front.file(&body.markdown);
    khalid_content::parse_post(&path, &text).map_err(ItemError::Content)?;
    let mut held = Vec::new();
    held.extend(
        seo.title
            .as_deref()
            .map(|title| Held::SearchTitle(decoded(title))),
    );
    if !item.tags.is_empty() {
        held.push(Held::Tags(item.tags.clone()));
    }
    if !seo.focus_keywords.is_empty() {
        held.push(Held::FocusKeywords(seo.focus_keywords));
    }
    held.extend(feature_image(item, export).map(Held::FeatureImage));
    if !more.is_empty() {
        held.push(Held::MoreCategories(more));
    }
    if !body.shortcodes.is_empty() {
        held.push(Held::Shortcodes(body.shortcodes));
    }
    held.extend(seo.unread.into_iter().map(Held::Unread));
    let imported = ImportedItem {
        id: item.id,
        path: path.clone(),
        slug,
        slug_origin: origin,
        held,
    };
    Ok((PlannedFile { path, text }, imported))
}

/// The author's slug, made from the name shown on posts, or the sign-in
/// name when the export has no such author.
fn author(item: &Item, export: &Export, site: &Config) -> Option<Slug> {
    let shown = export
        .authors
        .iter()
        .find(|author| author.login == item.creator)
        .map_or(item.creator.as_str(), |author| author.display_name.as_str());
    Slug::from_title(&decoded(shown), site.default_language(), site.slugs()).ok()
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
    let name = |slug: &str| {
        let term = export.categories.iter().find(|term| term.slug == slug);
        term.map_or_else(|| slug.to_owned(), |term| decoded(&term.name))
    };
    let main = main.and_then(|slug| {
        let made = post_slug(slug, &name(slug), 0, site.default_language(), site.slugs());
        made.ok().map(|made| made.slug)
    });
    (main, more)
}

/// The address of the image a post's `_thumbnail_id` names.
fn feature_image(item: &Item, export: &Export) -> Option<AbsoluteUrl> {
    let id = item.meta("_thumbnail_id")?.parse().ok()?;
    export.item(id)?.attachment_url.clone()
}

/// `text` with the HTML entities `WordPress` stores in names and titles
/// written as the characters they stand for.
fn decoded(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#039;", "'")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}
