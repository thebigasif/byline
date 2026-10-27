//! A post as the Admin API sees it: drafts included, markdown always.

use chrono::{DateTime, Utc};
use khalid_core::Slug;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{PostSettings, display, display_option};

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
    /// The author's slug, matching a file in the site's `authors/` folder.
    #[serde(serialize_with = "display_option")]
    #[schema(value_type = Option<String>, example = "asif")]
    author: Option<Slug>,
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
    /// The settings beside the title and body, as the admin's settings
    /// panel shows them.
    settings: PostSettings,
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
            author: None,
            status: PostStatus::Draft,
            markdown: markdown.to_owned(),
            published_at: None,
            updated_at,
            updated_by: None,
            settings: PostSettings::default(),
        }
    }

    /// The settings beside the title and body.
    pub fn settings(&self) -> &PostSettings {
        &self.settings
    }

    /// The same post with `settings`.
    #[must_use]
    pub fn with_settings(self, settings: PostSettings) -> Self {
        Self { settings, ..self }
    }

    /// The identity.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The slug.
    pub fn slug(&self) -> &Slug {
        &self.slug
    }

    /// The title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The author's slug, if the post names one.
    pub fn author(&self) -> Option<&Slug> {
        self.author.as_ref()
    }

    /// The markdown body.
    pub fn markdown(&self) -> &str {
        &self.markdown
    }

    /// Where it stands.
    pub fn status(&self) -> PostStatus {
        self.status
    }

    /// When it went or goes live, if it is not a draft.
    pub fn published_at(&self) -> Option<DateTime<Utc>> {
        self.published_at
    }

    /// When it last changed.
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    /// Who made the last change, if known.
    pub fn updated_by(&self) -> Option<&str> {
        self.updated_by.as_deref()
    }

    /// The same post, by the author with slug `author`.
    #[must_use]
    pub fn with_author(self, author: Option<Slug>) -> Self {
        Self { author, ..self }
    }

    /// The same post, last changed by `by`.
    #[must_use]
    pub fn with_updated_by(self, by: &str) -> Self {
        let updated_by = Some(by.to_owned());
        Self { updated_by, ..self }
    }

    /// The same post, live since `at`. A post already live keeps the
    /// time it went live.
    #[must_use]
    pub fn published(self, at: DateTime<Utc>) -> Self {
        Self {
            status: PostStatus::Published,
            published_at: Some(
                self.published_at
                    .filter(|_| self.status == PostStatus::Published)
                    .unwrap_or(at),
            ),
            ..self
        }
    }

    /// The post after `edit`, made by `by` at `at`. The store checks
    /// `edit.updated_at()` against this post's first, and runs the publish
    /// gate before it applies [`PostEdit::publishes`].
    #[must_use]
    pub fn edited(self, edit: PostEdit, at: DateTime<Utc>, by: &str) -> Self {
        let publishes = edit.publishes();
        let post = Self {
            title: edit.title.unwrap_or(self.title),
            markdown: edit.markdown.unwrap_or(self.markdown),
            updated_at: at,
            updated_by: Some(by.to_owned()),
            ..self
        };
        if publishes { post.published(at) } else { post }
    }
}

/// An edit to a post: the fields to change, and the `updated_at` of the
/// copy it was made on. Fields left out keep their value. A `status` of
/// `published` publishes a draft, once the linter lets it through.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PostEdit {
    /// A new title.
    title: Option<String>,
    /// A new markdown body.
    markdown: Option<String>,
    /// `published` to publish the post. Publishing runs the linter, and a
    /// post with a blocking finding stays a draft.
    #[schema(value_type = Option<String>, example = "published")]
    status: Option<PostStatus>,
    /// The `updated_at` of the copy the edit was made on.
    updated_at: DateTime<Utc>,
}

impl PostEdit {
    /// The `updated_at` of the copy the edit was made on.
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    /// Whether the edit asks to publish the post.
    pub fn publishes(&self) -> bool {
        self.status == Some(PostStatus::Published)
    }
}

/// A new post as a client sends it, before any checks.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct NewPost {
    /// The title. Required.
    pub(crate) title: String,
    /// The slug; made from the title when left out.
    pub(crate) slug: Option<String>,
    /// The author's slug, such as `asif`. The linter refuses to publish a
    /// post without one.
    pub(crate) author: Option<String>,
    /// The markdown body, unless `source=html`.
    pub(crate) markdown: Option<String>,
    /// The HTML body, with `source=html` only.
    pub(crate) html: Option<String>,
}

/// A new post, checked: a title, a valid slug or none, an author or none,
/// and a body. The store makes the slug from the title when there is none,
/// because only the site's settings say which language and slug mode to
/// use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostDraft {
    title: String,
    slug: Option<Slug>,
    author: Option<Slug>,
    markdown: String,
}

impl PostDraft {
    /// A draft with this title, slug and markdown body, and no author.
    pub fn new(title: &str, slug: Option<Slug>, markdown: &str) -> Self {
        Self {
            title: title.to_owned(),
            slug,
            author: None,
            markdown: markdown.to_owned(),
        }
    }

    /// The same draft, by the author with slug `author`.
    #[must_use]
    pub fn with_author(self, author: Option<Slug>) -> Self {
        Self { author, ..self }
    }

    /// The title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The slug asked for, if any.
    pub fn slug(&self) -> Option<&Slug> {
        self.slug.as_ref()
    }

    /// The author's slug, if given.
    pub fn author(&self) -> Option<&Slug> {
        self.author.as_ref()
    }

    /// The markdown body.
    pub fn markdown(&self) -> &str {
        &self.markdown
    }
}
