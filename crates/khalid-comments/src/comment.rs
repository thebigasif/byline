//! One comment as it is stored, with its identity, post, author and status.

use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};

use crate::{AuthorName, CommentText, ReaderEmail};

/// The database's identity of a comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommentId(i64);

impl CommentId {
    /// The comment whose row id is `id`.
    pub fn new(id: i64) -> Self {
        Self(id)
    }

    /// The row id.
    pub fn get(self) -> i64 {
        self.0
    }
}

impl fmt::Display for CommentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// The post a comment is on, named by its file inside the site folder,
/// such as `content/en/posts/why-links-matter.md`.
///
/// It comes from Khalid's own page index or the database, never from a
/// reader, so it is not checked again here.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PostFile(String);

impl PostFile {
    /// The post in `file`, a path relative to the site folder.
    pub fn new(file: impl Into<String>) -> Self {
        Self(file.into())
    }

    /// The path, with `/` between folders.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Where a comment stands in moderation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CommentStatus {
    /// Waiting for someone to approve it. Every reader's comment starts here.
    Pending,
    /// Shown under the post.
    Approved,
    /// Marked as spam by a moderator.
    Spam,
    /// Removed by a moderator, kept until the trash is emptied.
    Trash,
}

impl CommentStatus {
    /// Every status, in the order a comment moves through them.
    pub const ALL: [Self; 4] = [Self::Pending, Self::Approved, Self::Spam, Self::Trash];

    /// The name the database and the API use, such as `pending`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Spam => "spam",
            Self::Trash => "trash",
        }
    }
}

impl fmt::Display for CommentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A name that is not a comment status.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{name:?} is not a comment status")]
pub struct ParseStatusError {
    name: String,
}

impl FromStr for CommentStatus {
    type Err = ParseStatusError;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|status| status.as_str() == name)
            .ok_or_else(|| ParseStatusError {
                name: name.to_owned(),
            })
    }
}

/// Who wrote a comment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommentAuthor {
    /// A reader, through the form under a post. The email is never shown.
    Reader {
        /// The name shown with the comment.
        name: AuthorName,
        /// Where the site may write back, when the reader gave one.
        email: Option<ReaderEmail>,
    },
    /// Someone on the site's staff, replying through the Admin API.
    Staff {
        /// The name shown with the reply.
        name: AuthorName,
    },
}

impl CommentAuthor {
    /// The name shown with the comment.
    pub fn name(&self) -> &AuthorName {
        match self {
            Self::Reader { name, .. } | Self::Staff { name } => name,
        }
    }
}

/// One stored comment.
///
/// Exhaustive on purpose, like every record (ADR 0009): the store builds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    /// Its identity.
    pub id: CommentId,
    /// The post it is on.
    pub post: PostFile,
    /// The comment it answers. Always a comment with no parent itself,
    /// since threads are one level deep.
    pub parent: Option<CommentId>,
    /// Who wrote it.
    pub author: CommentAuthor,
    /// What they wrote, as plain text.
    pub text: CommentText,
    /// Where it stands in moderation.
    pub status: CommentStatus,
    /// When it was sent.
    pub created_at: DateTime<Utc>,
}
