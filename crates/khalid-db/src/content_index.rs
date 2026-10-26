//! What a rebuild writes: the content index, as read from the files.

use chrono::NaiveDate;
use khalid_core::{Locale, Slug};

/// Everything a rebuild writes into the derived tables. The caller reads the
/// content files and fills this in; this crate never reads `content/`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ContentIndex {
    /// Every post in every collection and language, in any order.
    pub posts: Vec<IndexedPost>,
}

/// One post as the index stores it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct IndexedPost {
    /// The file, relative to the site folder, with `/` between folders, such
    /// as `content/en/posts/hello.md`. Visitor rows name posts by this path.
    pub path: String,
    /// The language folder the file sits in.
    pub locale: Locale,
    /// The collection folder, such as `posts`.
    pub collection: String,
    /// The last part of the post's address.
    pub slug: Slug,
    /// The headline, when the front matter has one.
    pub title: Option<String>,
    /// The summary, when the front matter has one.
    pub description: Option<String>,
    /// The day the post is dated, when the front matter has one.
    pub date: Option<NaiveDate>,
    /// The author's slug, when the front matter names one.
    pub author: Option<Slug>,
    /// The category's slug, when the front matter names one.
    pub category: Option<Slug>,
    /// The key shared by the versions of this post in other languages.
    pub translation: Option<String>,
    /// The SHA-256 of the file and its assets.
    pub content_hash: [u8; 32],
}

impl IndexedPost {
    /// A post with the fields every post has; set the optional ones directly.
    pub fn new(
        path: &str,
        locale: Locale,
        collection: &str,
        slug: Slug,
        content_hash: [u8; 32],
    ) -> Self {
        Self {
            path: path.to_owned(),
            locale,
            collection: collection.to_owned(),
            slug,
            title: None,
            description: None,
            date: None,
            author: None,
            category: None,
            translation: None,
            content_hash,
        }
    }
}
