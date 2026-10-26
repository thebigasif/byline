//! What fills a page's main area, one variant per page type.

/// The main content of one page. The variant picks the template.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PageBody {
    /// The page for an address with nothing behind it.
    NotFound,
}
