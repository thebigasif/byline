//! A list of posts: the front page, a collection, a tag or a category.

use chrono::NaiveDate;
use serde::{Serialize, Serializer};

use crate::serde_display::display;
use crate::{BodyHtml, Link, Person, RenderImage};

/// One page of a list of posts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RenderListing {
    /// The heading, shown as the page's one `h1`.
    pub title: String,
    /// Text above the list, such as the body of a collection's `_index.md`.
    pub intro: Option<BodyHtml>,
    /// The posts on this page, in the order they are shown.
    pub posts: Vec<PostCard>,
    /// Links to the other pages of the list, when there is more than one.
    pub pagination: Option<Pagination>,
}

/// A post as a list shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PostCard {
    /// The post's title.
    pub title: String,
    /// The post's path, such as `/blog/what-a-sitemap-does/`.
    pub href: String,
    /// The day it was published.
    #[serde(serialize_with = "display")]
    pub date: NaiveDate,
    /// One or two sentences that say what the post is about.
    pub excerpt: String,
    /// A small image, always lazy.
    pub image: Option<RenderImage>,
    /// The first author, whose name and photo the card shows.
    pub author: Option<Person>,
    /// The post's primary category.
    pub category: Option<Link>,
}

/// The pages of a list and which one this is. The current page is always one
/// of them, so the links can never point past either end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pagination {
    pages: Vec<String>,
    current: usize,
}

impl Pagination {
    /// The pagination for page `current`, counted from 1, of the pages whose
    /// paths are `pages`. `None` when `current` is not one of them, or when
    /// there is only one page and nothing to link to.
    pub fn new(pages: Vec<String>, current: usize) -> Option<Self> {
        let index = current.checked_sub(1)?;
        (pages.len() > 1 && index < pages.len()).then_some(Self {
            pages,
            current: index,
        })
    }
}

impl Serialize for Pagination {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Shown<'a> {
            current: usize,
            total: usize,
            previous: Option<&'a String>,
            next: Option<&'a String>,
        }
        Shown {
            current: self.current.saturating_add(1),
            total: self.pages.len(),
            previous: self
                .current
                .checked_sub(1)
                .and_then(|index| self.pages.get(index)),
            next: self.pages.get(self.current.saturating_add(1)),
        }
        .serialize(serializer)
    }
}
