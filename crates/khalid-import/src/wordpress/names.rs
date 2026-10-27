//! The names and slugs of people and categories, the same in every file
//! that mentions them.

use khalid_core::{Config, Slug};

use crate::slug::post_slug;
use crate::wordpress::{Author, Export};

/// `text` with the HTML entities `WordPress` stores in names and titles
/// written as the characters they stand for.
pub(crate) fn decoded(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#039;", "'")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

/// An author's slug, made from the name shown on posts, or from the
/// sign-in name when that name has no letters.
pub(crate) fn author_slug(author: &Author, site: &Config) -> Option<Slug> {
    let made = |text: &str| Slug::from_title(&decoded(text), site.default_language(), site.slugs());
    made(&author.display_name)
        .or_else(|_| made(&author.login))
        .ok()
}

/// The Khalid slug of the category `WordPress` calls `slug`, kept when the
/// site's rules allow, else made from it or from its name.
pub(crate) fn category_slug(slug: &str, export: &Export, site: &Config) -> Option<Slug> {
    let term = export.categories.iter().find(|term| term.slug == slug);
    let name = term.map_or_else(|| slug.to_owned(), |term| decoded(&term.name));
    let made = post_slug(slug, &name, 0, site.default_language(), site.slugs());
    made.ok().map(|made| made.slug)
}
