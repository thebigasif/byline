//! A snippet as the Admin API reads and writes it.

use chrono::{DateTime, Utc};
use khalid_core::{Locale, Slug};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::display;
use crate::entry_store::Entry;

/// A snippet in one language, such as an affiliate disclosure, in Admin API
/// shape. A post places it by its slug, so a change shows in every post
/// that uses it as soon as it is saved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct AdminSnippet {
    /// The language and the slug, such as `en.affiliate-disclosure`.
    id: String,
    /// The slug a post names the snippet by.
    #[serde(serialize_with = "display")]
    #[schema(value_type = String, example = "affiliate-disclosure")]
    slug: Slug,
    /// The language, such as `en`.
    #[serde(serialize_with = "display")]
    #[schema(value_type = String, example = "en")]
    language: Locale,
    /// The name shown in the editor.
    title: String,
    /// The blocks, in markdown.
    markdown: String,
    /// When the snippet last changed. An edit or delete sends it back.
    updated_at: DateTime<Utc>,
}

impl AdminSnippet {
    /// A snippet as its file holds it, last changed at `updated_at`.
    pub fn new(
        (language, slug): (Locale, Slug),
        title: &str,
        markdown: &str,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: format!("{language}.{slug}"),
            slug,
            language,
            title: title.to_owned(),
            markdown: markdown.to_owned(),
            updated_at,
        }
    }
}

/// A new snippet, as a create request sends it.
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct NewSnippet {
    /// The language, such as `de`; the site's default when left out.
    #[schema(value_type = Option<String>, example = "en")]
    language: Option<Locale>,
    /// The slug a post will name it by, such as `affiliate-disclosure`.
    #[schema(value_type = String, example = "affiliate-disclosure")]
    slug: Slug,
    /// The name shown in the editor.
    title: String,
    /// The blocks, in markdown.
    markdown: String,
}

impl NewSnippet {
    /// The slug.
    pub fn slug(&self) -> &Slug {
        &self.slug
    }

    /// The title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The blocks.
    pub fn markdown(&self) -> &str {
        &self.markdown
    }
}

/// A change to a snippet: only the fields sent change.
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SnippetEdit {
    /// A new title.
    title: Option<String>,
    /// New blocks, in markdown.
    markdown: Option<String>,
    /// The `updated_at` of the copy this edit was made on.
    updated_at: DateTime<Utc>,
}

impl SnippetEdit {
    /// The new title, if sent.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// The new blocks, if sent.
    pub fn markdown(&self) -> Option<&str> {
        self.markdown.as_deref()
    }
}

impl Entry for AdminSnippet {
    type New = NewSnippet;
    type Edit = SnippetEdit;

    fn id(&self) -> &str {
        &self.id
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    fn seen(edit: &SnippetEdit) -> DateTime<Utc> {
        edit.updated_at
    }

    fn language(new: &NewSnippet) -> Option<&Locale> {
        new.language.as_ref()
    }

    fn made(new: NewSnippet, language: Locale, at: DateTime<Utc>) -> Self {
        Self::new((language, new.slug), &new.title, &new.markdown, at)
    }

    fn edited(self, edit: SnippetEdit, at: DateTime<Utc>) -> Self {
        Self {
            title: edit.title.unwrap_or(self.title),
            markdown: edit.markdown.unwrap_or(self.markdown),
            updated_at: at,
            ..self
        }
    }
}
