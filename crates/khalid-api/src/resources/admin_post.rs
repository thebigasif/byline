//! A post as the Admin API sees it: drafts included, markdown always.

use chrono::{DateTime, Utc};
use khalid_core::Slug;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::display;

/// Where a post stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum PostStatus {
    /// Not public yet.
    Draft,
    /// Live.
    Published,
    /// Goes live at its publish time.
    Scheduled,
}

/// A post in Admin API shape: any status, with its markdown source and
/// the `updated_at` an edit must send back (RULES.md BL-API-7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct AdminPost {
    /// Stable across edits.
    id: String,
    /// The last part of its address.
    #[serde(serialize_with = "display")]
    #[schema(value_type = String, example = "choosing-anchor-text")]
    slug: Slug,
    /// The title.
    title: String,
    /// Draft, published or scheduled.
    status: PostStatus,
    /// The markdown source, which the content file holds.
    markdown: String,
    /// When it went or goes live; empty for a draft.
    published_at: Option<DateTime<Utc>>,
    /// When it last changed. An edit sends this back unchanged.
    updated_at: DateTime<Utc>,
    /// Who made the last change: a staff name or a key's identity.
    updated_by: Option<String>,
}

impl AdminPost {
    /// A draft with `markdown` as its body, last changed at `updated_at`.
    pub fn new(
        id: &str,
        slug: Slug,
        title: &str,
        markdown: &str,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: id.to_owned(),
            slug,
            title: title.to_owned(),
            status: PostStatus::Draft,
            markdown: markdown.to_owned(),
            published_at: None,
            updated_at,
            updated_by: None,
        }
    }

    /// The identity.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Where it stands.
    pub fn status(&self) -> PostStatus {
        self.status
    }

    /// When it last changed.
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    /// Who made the last change, if known.
    pub fn updated_by(&self) -> Option<&str> {
        self.updated_by.as_deref()
    }

    /// The same post, last changed by `by`.
    #[must_use]
    pub fn with_updated_by(self, by: &str) -> Self {
        let updated_by = Some(by.to_owned());
        Self { updated_by, ..self }
    }

    /// The post after `edit`, made by `by` at `at`. The store checks
    /// `edit.updated_at()` against this post's first.
    #[must_use]
    pub fn edited(self, edit: PostEdit, at: DateTime<Utc>, by: &str) -> Self {
        Self {
            title: edit.title.unwrap_or(self.title),
            markdown: edit.markdown.unwrap_or(self.markdown),
            updated_at: at,
            updated_by: Some(by.to_owned()),
            ..self
        }
    }
}

/// An edit to a post: the fields to change, and the `updated_at` of the
/// copy it was made on. Fields left out keep their value.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PostEdit {
    /// A new title.
    title: Option<String>,
    /// A new markdown body.
    markdown: Option<String>,
    /// The `updated_at` of the copy the edit was made on.
    updated_at: DateTime<Utc>,
}

impl PostEdit {
    /// The `updated_at` of the copy the edit was made on.
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}
