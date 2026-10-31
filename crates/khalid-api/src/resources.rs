//! The resources as the API sends them. Other crates fill them from
//! content files and the database; this crate only shapes them.

mod admin_author;
mod admin_category;
mod admin_key;
mod admin_page;
mod admin_post;
mod admin_settings;
mod admin_snippet;
mod admin_tag;
mod author;
mod collection;
mod comment;
mod content_collection;
mod media;
mod menu;
mod post;
mod post_head;
mod post_row;
mod post_settings;
mod real_update;
mod seo_fields;
mod settings;
mod settings_edit;
mod term;

pub use admin_author::{AdminAuthor, AuthorEdit, AuthorFile, NewAuthor};
pub use admin_category::{AdminCategory, CategoryEdit, NewCategory, names_problem};
pub use admin_key::{AdminKey, NewKey, NewKeyKind};
pub use admin_page::{AdminPage, NewPage, PageEdit};
pub use admin_post::{AdminPost, NewPost, PostDraft, PostEdit, PostStatus};
pub use admin_settings::{AdminSettings, SettingsChange};
pub use admin_snippet::{AdminSnippet, NewSnippet, SnippetEdit};
pub use admin_tag::{AdminTag, TagEdit};
pub use author::Author;
pub use collection::Collection;
pub use comment::{AdminComment, CommentReply, CommentStatus, CommentWriter};
pub use content_collection::ContentCollection;
pub use media::{
    AdminMedia, CropBoxBody, FocalPointBody, MediaEditBody, MediaEdits, MediaItems, MediaList,
    PictureEditBody,
};
pub use menu::{Menu, MenuLink};
pub use post::{Bodies, Post};
pub use post_head::{HeadElement, HeadParts, PostHead};
pub use post_row::{Gate, IndexStatus, LintCounts, PostRow};
pub use post_settings::{
    Alias, FeatureImage, NonBlank, PostSettings, ReviewState, ShareCard, ShareCards,
};
pub use real_update::{MAX_UPDATE_NOTE, NotRealUpdate, RealUpdate};
pub use seo_fields::{CustomJsonLd, SearchTitle};
pub use settings::Settings;
pub use settings_edit::SettingsEdit;
pub use term::Term;

use std::fmt::Display;

use khalid_core::Slug;
use serde::Serializer;

use crate::query::FieldValues;

/// A resource a list route sends: it has an identity and a slug to be read
/// by, and fields a `filter` and `order` may name.
pub(crate) trait Listable: FieldValues + Clone {
    /// The fields a `filter` and `order` may name.
    const FIELDS: &'static [&'static str];

    /// The identity, as in `/authors/{id}/`.
    fn id(&self) -> &str;

    /// The slug, as in `/authors/slug/{slug}/`.
    fn slug(&self) -> &Slug;
}

/// Writes a value through its `Display`, for the core newtypes such as
/// `Slug`, which read themselves from text but leave writing to the edge.
pub(crate) fn display<T: Display, S: Serializer>(
    value: &T,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_str(value)
}

/// Writes an optional value through its `Display`, or `null`.
#[expect(
    clippy::ref_option,
    reason = "serde's serialize_with hands over a reference to the field"
)]
fn display_option<T: Display, S: Serializer>(
    value: &Option<T>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match value {
        Some(value) => serializer.collect_str(value),
        None => serializer.serialize_none(),
    }
}
