//! The resources as the API sends them. Other crates fill them from
//! content files and the database; this crate only shapes them.

mod post;

pub use post::{Bodies, Post};

use std::fmt::Display;

use serde::Serializer;

/// Writes a value through its `Display`, for the core newtypes such as
/// `Slug`, which read themselves from text but leave writing to the edge.
fn display<T: Display, S: Serializer>(value: &T, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_str(value)
}
