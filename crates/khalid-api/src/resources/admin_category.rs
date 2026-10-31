//! A category as the Admin API reads and writes it.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use khalid_core::{Locale, Slug};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::display;
use crate::entry_store::Entry;

/// A category, in Admin API shape: one slug, named in each of the site's
/// languages, as one entry of `categories.toml` holds it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct AdminCategory {
    /// The identity: the slug, the same in every language.
    id: String,
    /// The slug posts name the category by in their `category` field.
    #[serde(serialize_with = "display")]
    #[schema(value_type = String, example = "on-page-seo")]
    slug: Slug,
    /// The name readers see, by language code.
    #[schema(example = json!({"en": "On-page SEO", "de": "Onpage-SEO"}))]
    names: BTreeMap<String, String>,
    /// How many posts, drafts included, are in it.
    posts: usize,
    /// When the site's categories last changed. An edit or delete sends it
    /// back.
    updated_at: DateTime<Utc>,
}

impl AdminCategory {
    /// The category `slug`, named `names` by language code, holding
    /// `posts` posts, as its file was at `updated_at`.
    pub fn new(
        slug: Slug,
        names: BTreeMap<String, String>,
        posts: usize,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: slug.as_str().to_owned(),
            slug,
            names,
            posts,
            updated_at,
        }
    }

    /// The slug.
    pub fn slug(&self) -> &Slug {
        &self.slug
    }

    /// How many posts are in it.
    pub fn posts(&self) -> usize {
        self.posts
    }
}

/// A new category, as a create request sends it.
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct NewCategory {
    /// The slug posts will name it by, such as `link-building`.
    #[schema(value_type = String, example = "link-building")]
    slug: Slug,
    /// The name in each language, by language code.
    #[schema(example = json!({"en": "Link building", "de": "Linkaufbau"}))]
    names: BTreeMap<String, String>,
}

impl NewCategory {
    /// The slug.
    pub fn slug(&self) -> &Slug {
        &self.slug
    }

    /// The names, by language code.
    pub fn names(&self) -> &BTreeMap<String, String> {
        &self.names
    }
}

/// A change to a category: the names it sends replace all of its names.
/// Its slug stays, since posts name it.
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CategoryEdit {
    /// The name in each language, by language code.
    #[schema(example = json!({"en": "On-page SEO", "de": "Onpage-SEO"}))]
    names: BTreeMap<String, String>,
    /// The `updated_at` of the copy this edit was made on.
    updated_at: DateTime<Utc>,
}

impl CategoryEdit {
    /// The names, by language code.
    pub fn names(&self) -> &BTreeMap<String, String> {
        &self.names
    }
}

/// The problem with `names`, if any: none, a code that is not a language,
/// or an empty name.
pub fn names_problem(names: &BTreeMap<String, String>) -> Option<&'static str> {
    if names.is_empty() {
        return Some("names must name the category in at least one language");
    }
    if names.keys().any(|code| Locale::try_new(code).is_err()) {
        return Some("each key of names must be a language code, such as en");
    }
    if names.values().any(|name| name.trim().is_empty()) {
        return Some("each name must have a word in it");
    }
    None
}

impl Entry for AdminCategory {
    type New = NewCategory;
    type Edit = CategoryEdit;

    fn id(&self) -> &str {
        &self.id
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    fn seen(edit: &CategoryEdit) -> DateTime<Utc> {
        edit.updated_at
    }

    /// A category spans every language.
    fn language(_: &NewCategory) -> Option<&Locale> {
        None
    }

    fn made(new: NewCategory, _: Locale, at: DateTime<Utc>) -> Self {
        Self::new(new.slug, new.names, 0, at)
    }

    fn edited(self, edit: CategoryEdit, at: DateTime<Utc>) -> Self {
        Self {
            names: edit.names,
            updated_at: at,
            ..self
        }
    }
}
