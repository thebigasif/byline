//! The resources as the API sends them. Other crates fill them from
//! content files and the database; this crate only shapes them.

mod admin_post;
mod author;
mod collection;
mod content_collection;
mod menu;
mod post;
mod settings;
mod term;

pub use admin_post::{AdminPost, NewPost, PostDraft, PostEdit, PostStatus};
pub use author::Author;
pub use collection::Collection;
pub use content_collection::ContentCollection;
pub use menu::{Menu, MenuLink};
pub use post::{Bodies, Post};
pub use settings::Settings;
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
