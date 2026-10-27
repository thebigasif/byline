//! A dated post as the site's lists and the related-posts choice read it,
//! without its body, so `khalid serve` can keep every post of the last full
//! render and render a saved post's page alone with the same related posts.

use std::path::PathBuf;

use khalid_core::{Locale, Slug};
use khalid_render::{PostCard, PostTopics};

/// One dated post: its card in lists, what its related posts are chosen
/// by, and its author, whose page lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ListedPost {
    /// The content file, relative to the site folder.
    pub(crate) path: PathBuf,
    pub(crate) locale: Locale,
    pub(crate) topics: PostTopics,
    pub(crate) card: PostCard,
    pub(crate) author: Option<Slug>,
}
