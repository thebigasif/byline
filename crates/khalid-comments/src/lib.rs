//! Readers' comments on posts (plan Phase 5, task 5.4).
//!
//! This crate owns what a comment is ([`Comment`]) and the fields a reader
//! fills in, each checked once when it is read ([`AuthorName`],
//! [`ReaderEmail`], [`CommentText`]), how a posted form is read into one
//! ([`CommentForm`]) with its honeypot, and how approved comments are
//! shown one level deep ([`threads`]). [`RateLimits`] keeps one network
//! address from sending many, without keeping the address. [`Comments`]
//! sends, moderates and shows comments over a [`CommentStore`], and
//! [`RefusalPages`] tell a reader, in their language, why one was not sent.
//!
//! Comments are visitor data: they live in the site database, never in
//! `content/`, and name their post by its file ([`PostFile`]) so they
//! survive a rebuild. This crate must not know about SQL, templates or
//! content files. `khalid-db` implements the store and the binary renders
//! the comments.

mod api_moderation;
mod comment;
mod comment_form;
mod comment_store;
mod comments;
mod field_problem;
mod fields;
mod form_route;
#[cfg(any(test, feature = "test-util"))]
mod memory_store;
mod rate_limits;
mod refusal_page;
mod thread;
mod webhook_events;

pub use api_moderation::{ApiModeration, Audience, api_shape};
pub use comment::{Comment, CommentAuthor, CommentId, CommentStatus, ParseStatusError, PostFile};
pub use comment_form::{CommentForm, FormComment, Reading};
pub use comment_store::{CommentStore, CommentStoreError, NewComment};
pub use comments::{CommentEvents, Comments, CommentsError};
pub use field_problem::FieldProblem;
pub use fields::{AuthorName, CommentText, ReaderEmail};
pub use form_route::{FormRoute, PostPage, PostPages, form_router};
#[cfg(any(test, feature = "test-util"))]
pub use memory_store::MemoryCommentStore;
pub use rate_limits::{RateLimitError, RateLimits};
pub use refusal_page::{LoadWordsError, Refusal, RefusalPages};
pub use thread::{Thread, threads};
pub use webhook_events::WebhookEvents;
