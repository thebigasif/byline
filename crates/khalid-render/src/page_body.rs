//! What fills a page's main area, one variant per page type.

use minijinja::Value;
use minijinja::value::Serde;

use crate::RenderPost;

/// The main content of one page. The variant picks the template.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PageBody {
    /// One post.
    Post(Box<RenderPost>),
    /// The page for an address with nothing behind it.
    NotFound,
}

impl PageBody {
    /// The template that renders this page type.
    pub(crate) fn template(&self) -> &'static str {
        match self {
            Self::Post(_) => "post.html",
            Self::NotFound => "not_found.html",
        }
    }

    /// The page's own data, which templates read as `page`.
    pub(crate) fn value(&self) -> Value {
        match self {
            Self::Post(post) => Value::from(Serde(post)),
            Self::NotFound => Value::UNDEFINED,
        }
    }
}
