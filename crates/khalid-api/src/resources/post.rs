//! A post.

use chrono::{DateTime, SecondsFormat, Utc};
use khalid_core::{AbsoluteUrl, Locale, Slug};
use serde::Serialize;
use utoipa::ToSchema;

use super::{HeadParts, Listable, PostHead, display};
use crate::query::{FieldValues, Format, Literal};

/// A published post in API shape. Bodies are present only when asked for
/// with `formats`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct Post {
    /// Stable across edits: the content file's identity.
    id: String,
    /// The last part of its address.
    #[serde(serialize_with = "display")]
    #[schema(value_type = String, example = "why-internal-links-matter")]
    slug: Slug,
    /// The title.
    title: String,
    /// The full public address.
    #[serde(serialize_with = "display")]
    #[schema(value_type = String, example = "https://example.com/blog/why-internal-links-matter/")]
    url: AbsoluteUrl,
    /// The language, such as `en` or `de`.
    #[serde(serialize_with = "display")]
    #[schema(value_type = String, example = "en")]
    language: Locale,
    /// A short summary, if the writer gave one.
    excerpt: Option<String>,
    /// Whether the post is marked as featured.
    featured: bool,
    /// Tag slugs, in the order the writer gave them.
    tags: Vec<String>,
    /// Author slugs, in the order the writer gave them.
    authors: Vec<String>,
    /// When it went live.
    published_at: DateTime<Utc>,
    /// When it last changed.
    updated_at: DateTime<Utc>,
    /// What its page carries in `<head>` for search engines and share
    /// cards, so a front end of its own keeps the meta tags and schema.
    /// Absent until the site's pages have been made once.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<HeadParts>)]
    head: Option<PostHead>,
    /// The body, in the formats asked for.
    #[serde(flatten)]
    bodies: Bodies,
}

/// A post's body in each format. A format left out of `formats` is absent.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, ToSchema)]
pub struct Bodies {
    /// Rendered HTML.
    #[serde(skip_serializing_if = "Option::is_none")]
    html: Option<String>,
    /// The markdown source.
    #[serde(skip_serializing_if = "Option::is_none")]
    markdown: Option<String>,
    /// Text with the markup removed.
    #[serde(skip_serializing_if = "Option::is_none")]
    plaintext: Option<String>,
}

impl Bodies {
    /// All three formats.
    pub fn new(html: &str, markdown: &str, plaintext: &str) -> Self {
        Self {
            html: Some(html.to_owned()),
            markdown: Some(markdown.to_owned()),
            plaintext: Some(plaintext.to_owned()),
        }
    }
}

impl Post {
    /// The fields a `filter` and `order` may name on posts. `tag` and
    /// `author` are Ghost's names for one tag or author slug.
    pub const FILTER_FIELDS: [&'static str; 11] = [
        "id",
        "slug",
        "title",
        "language",
        "featured",
        "tag",
        "tags",
        "author",
        "authors",
        "published_at",
        "updated_at",
    ];

    /// A post with no excerpt, tags, authors or body, not featured, last
    /// changed when it was published.
    pub fn new(
        id: &str,
        slug: Slug,
        title: &str,
        url: AbsoluteUrl,
        language: Locale,
        published_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: id.to_owned(),
            slug,
            title: title.to_owned(),
            url,
            language,
            excerpt: None,
            featured: false,
            tags: Vec::new(),
            authors: Vec::new(),
            published_at,
            updated_at: published_at,
            head: None,
            bodies: Bodies::default(),
        }
    }

    /// The same post with this excerpt.
    #[must_use]
    pub fn with_excerpt(self, excerpt: &str) -> Self {
        let excerpt = Some(excerpt.to_owned());
        Self { excerpt, ..self }
    }

    /// The same post with these tag slugs.
    #[must_use]
    pub fn with_tags(self, tags: Vec<String>) -> Self {
        Self { tags, ..self }
    }

    /// The same post with these author slugs.
    #[must_use]
    pub fn with_authors(self, authors: Vec<String>) -> Self {
        Self { authors, ..self }
    }

    /// The same post, marked as featured.
    #[must_use]
    pub fn featured(self) -> Self {
        Self {
            featured: true,
            ..self
        }
    }

    /// The same post, last changed at `updated_at`.
    #[must_use]
    pub fn with_updated_at(self, updated_at: DateTime<Utc>) -> Self {
        Self { updated_at, ..self }
    }

    /// The same post with the head of its page, or none.
    #[must_use]
    pub fn with_head(self, head: Option<PostHead>) -> Self {
        Self { head, ..self }
    }

    /// The same post with these bodies.
    #[must_use]
    pub fn with_bodies(self, bodies: Bodies) -> Self {
        Self { bodies, ..self }
    }

    /// The same post with only the body formats in `formats`.
    #[must_use]
    pub fn with_formats(self, formats: &[Format]) -> Self {
        let keep = |format, body: Option<String>| body.filter(|_| formats.contains(&format));
        let bodies = Bodies {
            html: keep(Format::Html, self.bodies.html),
            markdown: keep(Format::Markdown, self.bodies.markdown),
            plaintext: keep(Format::Plaintext, self.bodies.plaintext),
        };
        Self { bodies, ..self }
    }

    /// The identity.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The slug.
    pub fn slug(&self) -> &Slug {
        &self.slug
    }

    /// The full public address.
    pub fn url(&self) -> &AbsoluteUrl {
        &self.url
    }

    /// The language.
    pub fn language(&self) -> &Locale {
        &self.language
    }
}

impl Listable for Post {
    const FIELDS: &'static [&'static str] = &Self::FILTER_FIELDS;

    fn id(&self) -> &str {
        &self.id
    }

    fn slug(&self) -> &Slug {
        &self.slug
    }
}

impl FieldValues for Post {
    fn values(&self, field: &str) -> Vec<Literal> {
        let text = |value: &str| vec![Literal::Text(value.to_owned())];
        let time = |at: &DateTime<Utc>| text(&at.to_rfc3339_opts(SecondsFormat::Secs, true));
        let all = |values: &[String]| values.iter().map(|v| Literal::Text(v.clone())).collect();
        match field {
            "id" => text(&self.id),
            "slug" => text(self.slug.as_str()),
            "title" => text(&self.title),
            "language" => text(self.language.as_str()),
            "featured" => vec![Literal::Bool(self.featured)],
            "tag" | "tags" => all(&self.tags),
            "author" | "authors" => all(&self.authors),
            "published_at" => time(&self.published_at),
            "updated_at" => time(&self.updated_at),
            _ => Vec::new(),
        }
    }
}
