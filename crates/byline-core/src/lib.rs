//! Shared building blocks for every Byline crate.
//!
//! This crate owns the types the other crates agree on, starting with the
//! message a person reads ([`UserMessage`]). It must not know about HTTP, SQL
//! or files. It depends on no other Byline crate and does no I/O.

mod user_message;

pub use user_message::UserMessage;
