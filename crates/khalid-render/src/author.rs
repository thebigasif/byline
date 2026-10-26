//! An author's page and a plain page such as About.

use chrono::NaiveDate;
use serde::Serialize;

use crate::serde_display::display_option;
use crate::{BodyHtml, Link, Pagination, PostCard, RenderImage};

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
