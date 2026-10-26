//! The site's small lists, such as its authors.

use crate::resources::Author;

/// The site's authors in every language. A site has tens of these, not
/// thousands, so a store hands them over whole and the API filters, orders
/// and pages them itself.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SiteResources {
    authors: Vec<Author>,
}

impl SiteResources {
    /// The same lists with these authors, in the order lists show them.
    #[must_use]
    pub fn with_authors(self, authors: Vec<Author>) -> Self {
        Self { authors }
    }

    /// The authors.
    pub fn authors(&self) -> &[Author] {
        &self.authors
    }
}
