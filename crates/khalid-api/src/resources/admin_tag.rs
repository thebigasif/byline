//! A tag as the Admin API reads and changes it.

use chrono::{DateTime, Utc};
use khalid_core::{Locale, Slug};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::display;

/// A tag in one language, in Admin API shape. A tag lives in the `tags`
/// of its posts, drafts included, so it exists while one post has it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct AdminTag {
    /// The language and the slug, such as `en.internal-links`.
    id: String,
    /// The slug made from the name, as the tag's page uses it.
    #[serde(serialize_with = "display")]
    #[schema(value_type = String, example = "internal-links")]
    slug: Slug,
    /// The language, such as `en`.
    #[serde(serialize_with = "display")]
    #[schema(value_type = String, example = "en")]
    language: Locale,
    /// The name as posts write it, such as `internal links`.
    name: String,
    /// How many posts, drafts included, have the tag.
    posts: usize,
    /// When the last of those posts changed. A rename or removal sends it
    /// back, so a post tagged since is never changed unseen.
    updated_at: DateTime<Utc>,
}

impl AdminTag {
    /// The tag `name` in `language`, on `posts` posts, the last of which
    /// changed at `updated_at`.
    pub fn new(
        (language, slug): (Locale, Slug),
        name: &str,
        posts: usize,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: format!("{language}.{slug}"),
            slug,
            language,
            name: name.to_owned(),
            posts,
            updated_at,
        }
    }

    /// The identity, such as `en.internal-links`.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The language.
    pub fn language(&self) -> &Locale {
        &self.language
    }

    /// The name as posts write it.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// When the last of its posts changed.
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

/// A new name for a tag, written into every post that has it.
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TagEdit {
    /// The new name, such as `internal linking`. A name another tag in the
    /// language has merges the two.
    name: String,
    /// The `updated_at` of the copy this rename was made on.
    updated_at: DateTime<Utc>,
}

impl TagEdit {
    /// The new name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The `updated_at` of the copy this rename was made on.
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}
