//! What a Substack export holds, as Khalid reads it.

use chrono::NaiveDateTime;

use crate::substack::RowError;

/// The posts of one Substack export, in the order `posts.csv` lists them.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct Export {
    /// Every post, draft and page that could be read.
    pub posts: Vec<Post>,
    /// Every row of `posts.csv` that could not be read, with why.
    pub unread: Vec<UnreadRow>,
}

/// One post, draft or page.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Post {
    /// Substack's number for it, such as `140062712`.
    pub id: String,
    /// The slug of its address, `/p/<slug>`, such as `why-rye`.
    pub slug: String,
    /// The headline.
    pub title: String,
    /// The line under the headline.
    pub subtitle: Option<String>,
    /// When it was published or last dated, in UTC.
    pub date: Option<NaiveDateTime>,
    /// Whether it was published.
    pub published: bool,
    /// What kind of post it is.
    pub kind: PostKind,
    /// Who could read it.
    pub audience: Audience,
    /// The body, as the HTML Substack wrote; none when the export has no
    /// file for it, as for a podcast episode with no notes.
    pub body: Option<String>,
}

/// What kind of post Substack says it is.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PostKind {
    /// A newsletter post, the usual kind.
    Newsletter,
    /// A podcast episode, whose audio stays on Substack.
    Podcast,
    /// A discussion thread, which lives in Substack's comments.
    Thread,
    /// A page, such as the about page.
    Page,
    /// Any other kind, as written.
    Other(String),
}

/// Who could read a post on Substack.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Audience {
    /// Everyone.
    Everyone,
    /// Subscribers only, free or paid.
    Subscribers,
    /// Paying subscribers only, founding members among them.
    Paid,
    /// Any other audience, as written.
    Other(String),
}

/// A row of `posts.csv` that could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct UnreadRow {
    /// The line it starts on.
    pub line: usize,
    /// Why it could not be read.
    pub error: RowError,
}

impl PostKind {
    /// The kind Substack names `name`.
    pub(crate) fn named(name: &str) -> Self {
        match name {
            "newsletter" => Self::Newsletter,
            "podcast" => Self::Podcast,
            "thread" => Self::Thread,
            "page" => Self::Page,
            other => Self::Other(other.to_owned()),
        }
    }
}

impl Audience {
    /// The audience Substack names `name`; an empty one is everyone.
    pub(crate) fn named(name: &str) -> Self {
        match name {
            "" | "everyone" => Self::Everyone,
            "only_free" => Self::Subscribers,
            "only_paid" | "founding" => Self::Paid,
            other => Self::Other(other.to_owned()),
        }
    }
}
