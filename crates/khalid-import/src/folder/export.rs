//! What a folder of markdown files holds, as Khalid reads it.

use chrono::NaiveDate;

use crate::folder::FileError;

/// The markdown files of one folder, in path order.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct Export {
    /// Every markdown file that could be read.
    pub files: Vec<SourceFile>,
    /// Every markdown file that could not be read, with why.
    pub unread: Vec<UnreadFile>,
}

/// One markdown file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SourceFile {
    /// Its path inside the folder, such as `_posts/2024-03-12-rye.md`.
    pub path: String,
    /// What its front matter says.
    pub front: Front,
    /// The markdown after the front matter.
    pub body: String,
}

/// The front matter fields Khalid reads, whichever generator wrote them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Front {
    /// `title`.
    pub title: Option<String>,
    /// `date`, `pubDate` (Astro) or `publishDate`, the day only.
    pub date: Option<NaiveDate>,
    /// `slug`.
    pub slug: Option<String>,
    /// `draft: true`, or `published: false` (Jekyll).
    pub draft: bool,
    /// `description`, `summary` (Hugo) or `excerpt` (Jekyll).
    pub description: Option<String>,
    /// `tags`, a list or words split by spaces.
    pub tags: Vec<String>,
    /// `categories`, a list or words split by spaces.
    pub categories: Vec<String>,
    /// `aliases` (Hugo): old paths that led to the post.
    pub aliases: Vec<String>,
    /// `permalink` (Jekyll, Eleventy) or `url` (Hugo): the post's address,
    /// when it was set by hand.
    pub permalink: Option<String>,
    /// `author`.
    pub author: Option<String>,
}

/// A markdown file that could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct UnreadFile {
    /// Its path inside the folder.
    pub path: String,
    /// Why it could not be read.
    pub error: FileError,
}
