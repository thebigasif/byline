//! A reader's approved comment, as a post's schema lists it.

use chrono::{DateTime, Utc};

/// One approved comment on a post, with its replies, for the `comment`
/// property of the post's article node (KH-SEO-3).
///
/// Exhaustive on purpose, like every input record (ADR 0013).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeoComment {
    /// Its identity, which makes its anchor on the page, `#comment-<id>`.
    pub id: i64,
    /// The name shown with it.
    pub author: String,
    /// What it says, as plain text.
    pub text: String,
    /// When it was sent.
    pub published: DateTime<Utc>,
    /// The replies under it, oldest first. Threads are one level deep, so
    /// a reply has none.
    pub replies: Vec<SeoComment>,
}
