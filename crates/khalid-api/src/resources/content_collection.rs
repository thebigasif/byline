//! A collection: a folder of content in one language, such as `posts`.

use khalid_core::{Locale, Slug};
use serde::Serialize;
use utoipa::ToSchema;

use super::{Listable, display};
use crate::query::{FieldValues, Literal};

/// A collection in one language, in API shape: the folder's name and the
/// title and description its `_index.md` gives, when it has one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct ContentCollection {
    /// The language and the folder, such as `en.posts`.
    id: String,
    /// The folder's name, such as `posts` or `case-studies`.
    #[serde(serialize_with = "display")]
    #[schema(value_type = String, example = "posts")]
    slug: Slug,
    /// The language, such as `en` or `de`.
    #[serde(serialize_with = "display")]
    #[schema(value_type = String, example = "en")]
    language: Locale,
    /// The heading of its list page.
    title: Option<String>,
    /// The summary of its list page.
    description: Option<String>,
}

impl ContentCollection {
    /// The collection in the folder `slug`, with no title or description.
    /// The identity is `language.slug`.
    pub fn new(slug: Slug, language: Locale) -> Self {
        Self {
            id: format!("{}.{}", language.as_str(), slug.as_str()),
            slug,
            language,
            title: None,
            description: None,
        }
    }

    /// The same collection with this title.
    #[must_use]
    pub fn with_title(self, title: &str) -> Self {
        let title = Some(title.to_owned());
        Self { title, ..self }
    }

    /// The same collection with this description.
    #[must_use]
    pub fn with_description(self, description: &str) -> Self {
        let description = Some(description.to_owned());
        Self {
            description,
            ..self
        }
    }
}

impl Listable for ContentCollection {
    const FIELDS: &'static [&'static str] = &["id", "slug", "title", "language"];

    fn id(&self) -> &str {
        &self.id
    }

    fn slug(&self) -> &Slug {
        &self.slug
    }
}

impl FieldValues for ContentCollection {
    fn values(&self, field: &str) -> Vec<Literal> {
        let text = |value: &str| vec![Literal::Text(value.to_owned())];
        match field {
            "id" => text(&self.id),
            "slug" => text(self.slug.as_str()),
            "title" => self.title.as_deref().map(text).unwrap_or_default(),
            "language" => text(self.language.as_str()),
            _ => Vec::new(),
        }
    }
}
