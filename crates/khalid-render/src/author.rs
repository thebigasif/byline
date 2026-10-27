//! An author's page, the page of every author, and a plain page such as About.

use chrono::NaiveDate;
use serde::Serialize;

use crate::serde_display::display_option;
use crate::{BodyHtml, Link, Pagination, Person, PostCard, RenderImage};

/// An author's page: who they are, where else they write, and their posts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RenderAuthor {
    /// The author's name, shown as the page's one `h1`.
    pub name: String,
    /// What they do, such as `Technical SEO editor`.
    pub role: Option<String>,
    /// A photo, always lazy.
    pub image: Option<RenderImage>,
    /// The author's own words about themselves, already sanitised.
    pub bio: BodyHtml,
    /// Their profiles elsewhere, marked `rel="me"`.
    pub profiles: Vec<Link>,
    /// Their posts on this page, newest first.
    pub posts: Vec<PostCard>,
    /// Links to the other pages of their posts.
    pub pagination: Option<Pagination>,
    /// The site's other authors, shown at the end of the page.
    pub others: Vec<Person>,
}

/// The page of every author: `/authors/`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RenderAuthors {
    /// The heading, shown as the page's one `h1`, such as `Authors`.
    pub title: String,
    /// Every author, in the order the site lists them.
    pub authors: Vec<AuthorEntry>,
}

/// One author on the authors page: who they are and how many posts they
/// have written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuthorEntry {
    /// Their name, page, role and photo.
    pub person: Person,
    /// How many published posts name them as an author.
    pub posts: usize,
}

/// A page that is not a post, such as About or Contact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RenderPage {
    /// The heading, shown as the page's one `h1`.
    pub title: String,
    /// The day it last changed, if readers should know.
    #[serde(serialize_with = "display_option")]
    pub updated: Option<NaiveDate>,
    /// The page's HTML, already sanitised.
    pub body: BodyHtml,
}
