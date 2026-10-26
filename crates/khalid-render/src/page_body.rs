//! What fills a page's main area, one variant per page type.

use minijinja::Value;

/// The main content of one page. The variant picks the template.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PageBody {
    /// The page for an address with nothing behind it.
    NotFound,
}

impl PageBody {
    /// The template that renders this page type.
    pub(crate) fn template(&self) -> &'static str {
        match self {
            Self::NotFound => "not_found.html",
        }
    }

    /// The page's own data, which templates read as `page`.
    pub(crate) fn value(&self) -> Value {
        match self {
            Self::NotFound => Value::UNDEFINED,
        }
    }
}
