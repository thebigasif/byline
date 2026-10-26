//! [`Publisher`]: who publishes the site, a company or a person.

use chrono::NaiveDate;
use khalid_core::AbsoluteUrl;

use crate::ShareImage;

/// Whether the site is a company's or a person's. The admin asks once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum PublisherKind {
    /// A company or another organisation: schema.org `Organization`.
    #[default]
    Organization,
    /// One person: schema.org `Person`.
    Person,
}

/// Who publishes the site, from `site.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Publisher {
    /// A company or a person.
    pub kind: PublisherKind,
    /// The company's or the person's name.
    pub name: String,
    /// The logo, or for a person their photo.
    pub logo: Option<ShareImage>,
    /// Profiles elsewhere that are the same publisher, such as a company page.
    pub same_as: Vec<AbsoluteUrl>,
    /// A public contact address.
    pub email: Option<String>,
    /// A public phone number.
    pub telephone: Option<String>,
    /// When the company was founded.
    pub founding_date: Option<NaiveDate>,
}

impl Publisher {
    /// A publisher with a kind and a name and nothing else set.
    pub fn new(kind: PublisherKind, name: &str) -> Self {
        Self {
            kind,
            name: name.to_owned(),
            logo: None,
            same_as: Vec::new(),
            email: None,
            telephone: None,
            founding_date: None,
        }
    }
}
