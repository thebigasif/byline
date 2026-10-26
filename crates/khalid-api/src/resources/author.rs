//! An author.

use khalid_core::{AbsoluteUrl, Locale, Slug};
use serde::Serialize;
use utoipa::ToSchema;

use super::{Listable, display, display_option};
use crate::query::{FieldValues, Literal};

/// An author in one language, in API shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct Author {
    /// The language and the slug, such as `en.asif`.
    id: String,
    /// The slug posts name the author by.
    #[serde(serialize_with = "display")]
    #[schema(value_type = String, example = "asif")]
    slug: Slug,
    /// The name readers see.
    name: String,
    /// The language, such as `en` or `de`.
    #[serde(serialize_with = "display")]
    #[schema(value_type = String, example = "en")]
    language: Locale,
    /// What the author does, such as `Founder of SerpDad`.
    role: Option<String>,
    /// The introduction from the author's file, in markdown.
    bio: Option<String>,
    /// The author's page on the blog, when it has one.
    #[serde(serialize_with = "display_option")]
    #[schema(value_type = Option<String>, example = "https://example.com/blog/authors/asif/")]
    url: Option<AbsoluteUrl>,
    /// The author's profiles elsewhere, in the order the file gives them.
    #[schema(value_type = Vec<String>)]
    #[serde(serialize_with = "addresses")]
    same_as: Vec<AbsoluteUrl>,
}

impl Author {
    /// An author with only a name. The identity is `language.slug`.
    pub fn new(slug: Slug, name: &str, language: Locale) -> Self {
        Self {
            id: format!("{}.{}", language.as_str(), slug.as_str()),
            slug,
            name: name.to_owned(),
            language,
            role: None,
            bio: None,
            url: None,
            same_as: Vec::new(),
        }
    }

    /// The same author with this role.
    #[must_use]
    pub fn with_role(self, role: &str) -> Self {
        let role = Some(role.to_owned());
        Self { role, ..self }
    }

    /// The same author with this introduction.
    #[must_use]
    pub fn with_bio(self, bio: &str) -> Self {
        let bio = Some(bio.to_owned());
        Self { bio, ..self }
    }

    /// The same author with a page at `url`.
    #[must_use]
    pub fn with_url(self, url: AbsoluteUrl) -> Self {
        let url = Some(url);
        Self { url, ..self }
    }

    /// The same author with these profiles elsewhere.
    #[must_use]
    pub fn with_same_as(self, same_as: Vec<AbsoluteUrl>) -> Self {
        Self { same_as, ..self }
    }
}

/// Writes each address through its `Display`.
fn addresses<S: serde::Serializer>(urls: &[AbsoluteUrl], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(urls.iter().map(AbsoluteUrl::as_str))
}

impl Listable for Author {
    const FIELDS: &'static [&'static str] = &["id", "slug", "name", "language"];

    fn id(&self) -> &str {
        &self.id
    }

    fn slug(&self) -> &Slug {
        &self.slug
    }
}

impl FieldValues for Author {
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
