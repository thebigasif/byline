//! The files that are Khalid's source of truth (RULES.md BL-FS-1).
//!
//! This crate owns reading content: front matter, markdown and the content
//! tree of language folders, collections and posts. It must not know about
//! templates, HTTP or the database; it hands typed content to the crates that
//! render, serve and index it, and never writes derived data under
//! `content/` (BL-FS-2).

mod content_error;
mod content_hash;
mod front_matter;
mod post;
mod tree;

pub use content_error::ContentError;
pub use content_hash::ContentHash;
pub use post::{Post, PostFront, parse_post};
pub use tree::{Collection, ContentTree, Entry, Language};
