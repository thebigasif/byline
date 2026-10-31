//! An author as the Admin API reads and writes them.

use chrono::{DateTime, Utc};
use khalid_core::{AbsoluteUrl, Locale, Slug};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::author::addresses;
use super::display;
use crate::entry_store::Entry;

/// An author in one language, in Admin API shape: everything their file
/// holds, which bylines, author pages and the author's schema show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct AdminAuthor {
    /// The language and the slug, such as `en.asif`.
    id: String,
    /// The slug posts name the author by in their `author` field.
    #[serde(serialize_with = "display")]
    #[schema(value_type = String, example = "asif")]
    slug: Slug,
    /// The language, such as `en`.
    #[serde(serialize_with = "display")]
    #[schema(value_type = String, example = "en")]
    language: Locale,
    /// The name readers see.
    name: String,
    /// What the author does, such as `Founder of SerpDad`.
    role: Option<String>,
    /// Why to trust the author, such as ten years in the field.
    credentials: Option<String>,
    /// The author's profiles elsewhere, in order.
    #[serde(serialize_with = "addresses")]
    #[schema(value_type = Vec<String>, example = json!(["https://serpdad.com/"]))]
    same_as: Vec<AbsoluteUrl>,
    /// The photo: a file beside the author's file, a media path or a full
    /// address.
    photo: Option<String>,
    /// The bio, in markdown.
    bio: String,
    /// When the author's file last changed. An edit or delete sends it back.
    updated_at: DateTime<Utc>,
}

/// What an author's file says besides their slug and language.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuthorFile {
    /// The name readers see.
    pub name: String,
    /// What the author does.
    pub role: Option<String>,
    /// Why to trust the author.
    pub credentials: Option<String>,
    /// Profiles elsewhere.
    pub same_as: Vec<AbsoluteUrl>,
    /// The photo.
    pub photo: Option<String>,
    /// The bio, in markdown.
    pub bio: String,
}

impl AdminAuthor {
    /// An author as their file holds them, last changed at `updated_at`.
    pub fn new(
        (language, slug): (Locale, Slug),
        file: AuthorFile,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: format!("{language}.{slug}"),
            slug,
            language,
            name: file.name,
            role: file.role,
            credentials: file.credentials,
            same_as: file.same_as,
            photo: file.photo,
            bio: file.bio,
            updated_at,
        }
    }
}

/// A new author, as a create request sends them.
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct NewAuthor {
    /// The language, such as `de`; the site's default when left out.
    #[schema(value_type = Option<String>, example = "en")]
    language: Option<Locale>,
    /// The slug posts will name the author by, such as `asif`.
    #[schema(value_type = String, example = "asif")]
    slug: Slug,
    /// The name readers see.
    name: String,
    /// What the author does.
    role: Option<String>,
    /// Why to trust the author.
    credentials: Option<String>,
    /// Profiles elsewhere, each a full address.
    #[serde(default)]
    #[schema(value_type = Vec<String>, example = json!(["https://serpdad.com/"]))]
    same_as: Vec<AbsoluteUrl>,
    /// The photo.
    photo: Option<String>,
    /// The bio, in markdown.
    #[serde(default)]
    bio: String,
}

impl NewAuthor {
    /// The slug.
    pub fn slug(&self) -> &Slug {
        &self.slug
    }

    /// The name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Everything else the file will hold.
    pub fn file(&self) -> AuthorFile {
        AuthorFile {
            name: self.name.clone(),
            role: self.role.clone(),
            credentials: self.credentials.clone(),
            same_as: self.same_as.clone(),
            photo: self.photo.clone(),
            bio: self.bio.clone(),
        }
    }
}

/// A change to an author: only the fields sent change. An empty role,
/// credentials or photo removes it, and `same_as` replaces the whole list.
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AuthorEdit {
    /// A new name.
    name: Option<String>,
    /// A new role; an empty one removes it.
    role: Option<String>,
    /// New credentials; empty ones remove them.
    credentials: Option<String>,
    /// A new list of profiles.
    #[schema(value_type = Option<Vec<String>>)]
    same_as: Option<Vec<AbsoluteUrl>>,
    /// A new photo; an empty one removes it.
    photo: Option<String>,
    /// A new bio, in markdown.
    bio: Option<String>,
    /// The `updated_at` of the copy this edit was made on.
    updated_at: DateTime<Utc>,
}

impl AuthorEdit {
    /// The new name, if sent.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// `file` with this edit applied.
    #[must_use]
    pub fn applied(&self, file: AuthorFile) -> AuthorFile {
        let or_gone = |sent: &Option<String>, kept: Option<String>| match sent.as_deref() {
            Some("") => None,
            Some(text) => Some(text.to_owned()),
            None => kept,
        };
        AuthorFile {
            name: self.name.clone().unwrap_or(file.name),
            role: or_gone(&self.role, file.role),
            credentials: or_gone(&self.credentials, file.credentials),
            same_as: self.same_as.clone().unwrap_or(file.same_as),
            photo: or_gone(&self.photo, file.photo),
            bio: self.bio.clone().unwrap_or(file.bio),
        }
    }
}

impl Entry for AdminAuthor {
    type New = NewAuthor;
    type Edit = AuthorEdit;

    fn id(&self) -> &str {
        &self.id
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    fn seen(edit: &AuthorEdit) -> DateTime<Utc> {
        edit.updated_at
    }

    fn language(new: &NewAuthor) -> Option<&Locale> {
        new.language.as_ref()
    }

    fn made(new: NewAuthor, language: Locale, at: DateTime<Utc>) -> Self {
        let file = new.file();
        Self::new((language, new.slug), file, at)
    }

    fn edited(self, edit: AuthorEdit, at: DateTime<Utc>) -> Self {
        let file = AuthorFile {
            name: self.name,
            role: self.role,
            credentials: self.credentials,
            same_as: self.same_as,
            photo: self.photo,
            bio: self.bio,
        };
        Self::new((self.language, self.slug), edit.applied(file), at)
    }
}
