//! A person a post names: an author or a reviewer.

use serde::Serialize;

use crate::{BodyHtml, Link, RenderImage};

/// An author as a post shows them: in the byline, and in the author box at
/// the end.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Person {
    /// The name a reader sees.
    pub name: String,
    /// Their author page, such as `/blog/authors/lena-vogt/`.
    pub href: String,
    /// What they do, such as `Technical SEO editor`.
    pub role: Option<String>,
    /// A small photo, always lazy.
    pub image: Option<RenderImage>,
    /// A short bio for the author box, already sanitised.
    pub bio: Option<BodyHtml>,
    /// Their profiles elsewhere, marked `rel="me"` on their own page.
    pub profiles: Vec<Link>,
}
