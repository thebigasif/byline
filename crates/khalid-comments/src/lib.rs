//! Readers' comments on posts (plan Phase 5, task 5.4).
//!
//! This crate owns what a comment is ([`Comment`]) and the fields a reader
//! fills in, each checked once when it is read ([`AuthorName`],
//! [`ReaderEmail`], [`CommentText`]). Comments are visitor data: they live
//! in the site database, never in `content/`, and name their post by its
//! file ([`PostFile`]) so they survive a rebuild.
//!
//! It must not know about SQL, templates or content files. `khalid-db`
//! stores comments and the binary renders them.

mod comment;
mod field_problem;
mod fields;

pub use comment::{Comment, CommentAuthor, CommentId, CommentStatus, ParseStatusError, PostFile};
pub use field_problem::FieldProblem;
pub use fields::{AuthorName, CommentText, ReaderEmail};
