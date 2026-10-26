//! [`AuthorProfile`] and [`Entity`]: who wrote a post and what it is about.

use khalid_core::AbsoluteUrl;

use crate::ShareImage;

/// An author's profile, from their author file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct AuthorProfile {
    /// The author's name.
    pub name: String,
    /// Their author page on this site.
    pub url: Option<AbsoluteUrl>,
    /// What they do, such as `Founder, SerpDad`.
    pub job_title: Option<String>,
    /// A sentence or two about them.
    pub bio: String,
    /// Their profiles elsewhere.
    pub same_as: Vec<AbsoluteUrl>,
    /// The topics they know well.
    pub knows_about: Vec<String>,
    /// Their photo.
    pub image: Option<ShareImage>,
}

impl AuthorProfile {
    /// A profile with a name and nothing else set.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            url: None,
            job_title: None,
            bio: String::new(),
            same_as: Vec::new(),
            knows_about: Vec::new(),
            image: None,
        }
    }
}

/// How a post relates to an entity it links to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EntityRole {
    /// The post is about it: schema.org `about`.
    About,
    /// The post mentions it: schema.org `mentions`.
    Mention,
}

/// A thing a post links to, such as a concept, a product or a company.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Entity {
    /// Its name.
    pub name: String,
    /// How the post relates to it.
    pub role: EntityRole,
    /// Pages that identify it, such as its Wikipedia or Wikidata page.
    pub same_as: Vec<AbsoluteUrl>,
}

impl Entity {
    /// An entity called `name`.
    pub fn new(name: &str, role: EntityRole, same_as: Vec<AbsoluteUrl>) -> Self {
        Self {
            name: name.to_owned(),
            role,
            same_as,
        }
    }
}
