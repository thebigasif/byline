//! Shared building blocks for every Byline crate.
//!
//! This crate owns the types the other crates agree on: the message a person
//! reads ([`UserMessage`]) and validated identifiers such as [`Slug`]. It must
//! not know about HTTP, SQL or files. It depends on no other Byline crate and
//! does no I/O.

mod slug;
mod user_message;

pub use slug::{ParseSlugError, Slug};
pub use user_message::UserMessage;
