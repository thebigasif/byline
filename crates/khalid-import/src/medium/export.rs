//! What a Medium export holds, as Khalid reads it.

use chrono::NaiveDateTime;
use khalid_core::AbsoluteUrl;

use crate::medium::PostFileError;

/// The posts of one Medium export, in the order of their file names.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct Export {
    /// Every post and draft that could be read.
    pub posts: Vec<Post>,
    /// Every file in `posts/` that could not be read as a post, with why.
    pub unread: Vec<UnreadFile>,
}

/// One post or draft.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Post {
    /// Medium's id for it: the hex code that ends its file name and its
    /// address, such as `1a2b3c4d5e6f`.
    pub id: String,
    /// Its file name in `posts/`, such as `2024-03-12_Rye-1a2b3c4d5e6f.html`.
    pub file: String,
    /// The headline.
    pub title: String,
    /// The line under the headline, which Medium shows in lists and cards.
    pub subtitle: Option<String>,
    /// The body, as the HTML Medium wrote.
    pub body: String,
    /// When it was published, in UTC; none for a draft.
    pub published: Option<NaiveDateTime>,
    /// Its address on Medium or on the publication's own domain.
    pub canonical: Option<AbsoluteUrl>,
    /// Who wrote it, as the footer names them.
    pub author: Option<Author>,
    /// Whether it was published.
    pub status: PostStatus,
}

/// Whether a post was published.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PostStatus {
    /// Published, with a date.
    Published,
    /// Not published: its file has no publication time.
    Draft,
}

/// The writer of a post.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Author {
    /// Their name as Medium shows it.
    pub name: String,
    /// Their profile, such as `https://medium.com/@mara`.
    pub profile: Option<AbsoluteUrl>,
}

impl Author {
    /// Their Medium username without the `@`, such as `mara`, taken from
    /// a profile address of the form `https://medium.com/@mara`.
    pub fn username(&self) -> Option<&str> {
        let profile = self.profile.as_ref()?;
        let last = profile.as_str().trim_end_matches('/').rsplit('/').next()?;
        last.strip_prefix('@').filter(|name| !name.is_empty())
    }
}

/// A file in `posts/` that is not a Medium post.
#[derive(Debug)]
#[non_exhaustive]
pub struct UnreadFile {
    /// Its file name in `posts/`.
    pub file: String,
    /// Why it could not be read.
    pub error: PostFileError,
}
