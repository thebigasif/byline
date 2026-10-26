//! The site's small lists: authors, tags and categories.

use crate::resources::{Author, Term};

/// The site's authors, tags and categories in every language. A site has
/// tens of these, not thousands, so a store hands them over whole and the
/// API filters, orders and pages them itself.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SiteResources {
    authors: Vec<Author>,
    tags: Vec<Term>,
    categories: Vec<Term>,
}

impl SiteResources {
    /// The same lists with these authors, in the order lists show them.
    #[must_use]
    pub fn with_authors(self, authors: Vec<Author>) -> Self {
        Self { authors, ..self }
    }

    /// The same lists with these tags, in the order lists show them.
    #[must_use]
    pub fn with_tags(self, tags: Vec<Term>) -> Self {
        Self { tags, ..self }
    }

    /// The same lists with these categories, in the order lists show them.
    #[must_use]
    pub fn with_categories(self, categories: Vec<Term>) -> Self {
        Self { categories, ..self }
    }

    /// The authors.
    pub fn authors(&self) -> &[Author] {
        &self.authors
    }

    /// The tags.
    pub fn tags(&self) -> &[Term] {
        &self.tags
    }

    /// The categories.
    pub fn categories(&self) -> &[Term] {
        &self.categories
    }
}
