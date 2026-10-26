//! A tag or a category.

use khalid_core::{Locale, Slug};
use serde::Serialize;
use utoipa::ToSchema;

use super::{Listable, display};
use crate::query::{FieldValues, Literal};

/// A tag or a category in one language, in API shape. Both are a name for
/// a group of posts, so they share one shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct Term {
    /// The language and the slug, such as `en.on-page-seo`.
    id: String,
    /// The slug posts name it by.
    #[serde(serialize_with = "display")]
    #[schema(value_type = String, example = "on-page-seo")]
    slug: Slug,
    /// The name readers see in this language.
    name: String,
    /// The language, such as `en` or `de`.
    #[serde(serialize_with = "display")]
    #[schema(value_type = String, example = "en")]
    language: Locale,
}

impl Term {
    /// A term named `name` in `language`. The identity is `language.slug`.
    pub fn new(slug: Slug, name: &str, language: Locale) -> Self {
        Self {
            id: format!("{}.{}", language.as_str(), slug.as_str()),
            slug,
            name: name.to_owned(),
            language,
        }
    }
}

impl Listable for Term {
    const FIELDS: &'static [&'static str] = &["id", "slug", "name", "language"];

    fn id(&self) -> &str {
        &self.id
    }

    fn slug(&self) -> &Slug {
        &self.slug
    }
}

impl FieldValues for Term {
    fn values(&self, field: &str) -> Vec<Literal> {
        let text = |value: &str| vec![Literal::Text(value.to_owned())];
        match field {
            "id" => text(&self.id),
            "slug" => text(self.slug.as_str()),
            "name" => text(&self.name),
            "language" => text(self.language.as_str()),
            _ => Vec::new(),
        }
    }
}
