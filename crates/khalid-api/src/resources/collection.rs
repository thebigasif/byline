//! The collections whose entries share the post shape.

/// A collection of dated, written entries. Posts and pages, and the three
/// fixed collections from plan section 5.13, share one API shape: [`Post`].
///
/// [`Post`]: super::Post
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Collection {
    /// Blog posts.
    Posts,
    /// Standalone pages.
    Pages,
    /// Glossary terms.
    Glossary,
    /// Case studies.
    CaseStudies,
    /// Changelog entries.
    Changelog,
}
