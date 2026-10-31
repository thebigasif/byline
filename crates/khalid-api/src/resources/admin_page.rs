//! A page as the Admin API reads and writes it.

use chrono::{DateTime, Utc};
use khalid_core::{Locale, Slug};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::display;
use crate::entry_store::Entry;

/// A page in one language, such as About, in Admin API shape. It is live
/// as soon as it is saved: pages have no drafts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct AdminPage {
    /// The language and the slug, such as `en.about`.
    id: String,
    /// The last part of the page's address.
    #[serde(serialize_with = "display")]
    #[schema(value_type = String, example = "about")]
    slug: Slug,
    /// The language, such as `en`.
    #[serde(serialize_with = "display")]
    #[schema(value_type = String, example = "en")]
    language: Locale,
    /// The title readers see.
    title: String,
    /// The text search engines show under the title, if set.
    description: Option<String>,
    /// The body, in markdown.
    markdown: String,
    /// When the page last changed. An edit or delete sends it back.
    updated_at: DateTime<Utc>,
}

impl AdminPage {
    /// A page as its file holds it, last changed at `updated_at`.
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
            description: None,
            markdown: markdown.to_owned(),
            updated_at,
        }
    }

    /// The same page with this description.
    #[must_use]
    pub fn with_description(self, description: Option<&str>) -> Self {
        let description = description.map(str::to_owned);
        Self {
            description,
            ..self
        }
    }
}

/// A new page, as a create request sends it.
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct NewPage {
    /// The language, such as `de`; the site's default when left out.
    #[schema(value_type = Option<String>, example = "en")]
    language: Option<Locale>,
    /// The last part of the address, such as `about`.
    #[schema(value_type = String, example = "about")]
    slug: Slug,
    /// The title readers see.
    title: String,
    /// The text search engines show under the title.
    description: Option<String>,
    /// The body, in markdown.
    markdown: String,
}

impl NewPage {
    /// The slug.
    pub fn slug(&self) -> &Slug {
        &self.slug
    }

    /// The title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The description, if any.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// The body.
    pub fn markdown(&self) -> &str {
        &self.markdown
    }
}

/// A change to a page: only the fields sent change.
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PageEdit {
    /// A new title.
    title: Option<String>,
    /// A new description; an empty one removes it.
    description: Option<String>,
    /// A new body, in markdown.
    markdown: Option<String>,
    /// The `updated_at` of the copy this edit was made on.
    updated_at: DateTime<Utc>,
}

impl PageEdit {
    /// The new title, if sent.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// The new description, if sent: `Some("")` removes it.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// The new body, if sent.
    pub fn markdown(&self) -> Option<&str> {
        self.markdown.as_deref()
    }
}

impl Entry for AdminPage {
    type New = NewPage;
    type Edit = PageEdit;

    fn id(&self) -> &str {
        &self.id
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    fn seen(edit: &PageEdit) -> DateTime<Utc> {
        edit.updated_at
    }

    fn language(new: &NewPage) -> Option<&Locale> {
        new.language.as_ref()
    }

    fn made(new: NewPage, language: Locale, at: DateTime<Utc>) -> Self {
        Self::new((language, new.slug), &new.title, &new.markdown, at)
            .with_description(new.description.as_deref())
    }

    fn edited(self, edit: PageEdit, at: DateTime<Utc>) -> Self {
        let description = match edit.description {
            Some(text) if text.is_empty() => None,
            Some(text) => Some(text),
            None => self.description,
        };
        Self {
            title: edit.title.unwrap_or(self.title),
            markdown: edit.markdown.unwrap_or(self.markdown),
            description,
            updated_at: at,
            ..self
        }
    }
}
