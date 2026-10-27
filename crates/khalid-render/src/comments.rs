//! Readers' comments under a post, and the form to leave one.

use chrono::NaiveDate;
use serde::Serialize;

use crate::serde_display::display;

/// The comments section of a post: the approved comments, one level of
/// replies, and the plain HTML form that posts a new one (KH-WEB-2).
///
/// Exhaustive on purpose, like every input record (ADR 0009).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RenderComments {
    /// The approved comments, oldest first.
    pub threads: Vec<RenderThread>,
    /// Where the form posts, such as `/blog/comments/`.
    pub action: String,
    /// The post page's own address, which the form sends back.
    pub page: String,
}

/// A comment with the replies under it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RenderThread {
    /// The comment that started the thread.
    pub comment: RenderComment,
    /// Its approved replies, oldest first.
    pub replies: Vec<RenderComment>,
}

/// Who wrote a comment, as the page marks it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommentWriter {
    /// A reader.
    Reader,
    /// The site's staff, marked so readers can tell.
    Staff,
}

/// One approved comment as the page shows it. Its text is plain, and the
/// template escapes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RenderComment {
    /// Its identity, which makes its anchor, `#comment-<id>`.
    pub id: i64,
    /// The name shown with it.
    pub author: String,
    /// A reader or staff.
    pub writer: CommentWriter,
    /// The day it was sent.
    #[serde(serialize_with = "display")]
    pub date: NaiveDate,
    /// Its paragraphs, split where the reader left a blank line.
    pub paragraphs: Vec<String>,
}

impl RenderComment {
    /// The paragraphs of `text`: the parts between blank lines, trimmed,
    /// with the empty ones left out.
    pub fn paragraphs_of(text: &str) -> Vec<String> {
        let mut paragraphs = Vec::new();
        let mut current: Vec<&str> = Vec::new();
        for line in text.lines().map(str::trim).chain([""]) {
            if line.is_empty() {
                if !current.is_empty() {
                    paragraphs.push(current.join("\n"));
                    current.clear();
                }
            } else {
                current.push(line);
            }
        }
        paragraphs
    }
}
