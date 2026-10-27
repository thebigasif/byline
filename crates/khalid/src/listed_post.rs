//! A dated post as the site's lists and the related-posts choice read it,
//! without its body, so `khalid serve` can keep every post of the last full
//! render and render a saved post's page alone with the same related posts.

use std::path::PathBuf;

use khalid_core::{Locale, Slug, SlugMode};
use khalid_render::{PostCard, PostTopics};

/// One dated post: its card in lists, what its related posts are chosen
/// by, and its author, whose page lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ListedPost {
    /// The content file, relative to the site folder.
    pub(crate) path: PathBuf,
    pub(crate) locale: Locale,
    /// The collection folder it sits in, such as `posts`, whose page lists it.
    pub(crate) collection: String,
    /// Whether its front matter says `featured: true`, which channels read.
    pub(crate) featured: bool,
    /// Its tags, in the order written. `topics` holds their slugs too.
    pub(crate) tags: Vec<ListedTag>,
    pub(crate) topics: PostTopics,
    pub(crate) card: PostCard,
    pub(crate) author: Option<Slug>,
}

/// A tag as a post names it, and the slug its page is found by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ListedTag {
    /// The slug made from the name, such as `internal-links`.
    pub(crate) slug: Slug,
    /// The name as written, such as `internal links`, which titles its page.
    pub(crate) name: String,
}

impl ListedTag {
    /// The tags `names` of a post in `locale`, each slug made from its name
    /// as a title's is, in `mode`. A name no slug can be made from is left
    /// out, and the linter names it.
    pub(crate) fn all(names: &[String], locale: &Locale, mode: SlugMode) -> Vec<Self> {
        let tag = |name: &String| {
            let slug = Slug::from_title(name, locale, mode).ok()?;
            Some(Self {
                slug,
                name: name.clone(),
            })
        };
        names.iter().filter_map(tag).collect()
    }
}
