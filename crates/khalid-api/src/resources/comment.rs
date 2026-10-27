//! A reader's comment as the Admin API and the `comment.added` webhook send it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Where a comment stands in moderation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum CommentStatus {
    /// Waiting for approval. Every reader's comment starts here.
    Pending,
    /// Shown under its post.
    Approved,
    /// Marked as spam.
    Spam,
    /// Removed, kept until the trash is emptied.
    Trash,
}

/// Who wrote a comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum CommentWriter {
    /// A reader, through the form under a post.
    Reader,
    /// The site's staff, through the Admin API.
    Staff,
}

/// A comment in Admin API shape.
///
/// Exhaustive on purpose: the comments crate fills it in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct AdminComment {
    /// Its identity.
    pub id: i64,
    /// The post it is on, by the post's file in the site folder.
    #[schema(example = "content/en/posts/why-links-matter.md")]
    pub post: String,
    /// The comment it answers. Threads are one level deep.
    pub parent: Option<i64>,
    /// The name shown with it.
    pub author_name: String,
    /// A reader or staff.
    pub author_kind: CommentWriter,
    /// The reader's email, when they gave one. Never shown on the site,
    /// and never sent in a webhook.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_email: Option<String>,
    /// What it says, as plain text with a line break between lines.
    pub text: String,
    /// Where it stands in moderation.
    pub status: CommentStatus,
    /// When it was sent.
    pub created_at: DateTime<Utc>,
}

/// A staff reply to a comment.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CommentReply {
    /// The name shown with the reply, such as the writer's own.
    #[schema(example = "Asif")]
    pub author_name: String,
    /// What the reply says, as plain text.
    #[schema(example = "Thanks, Lena. The follow-up post is out next week.")]
    pub text: String,
}
