//! What fills a page's main area, one variant per page type.

use minijinja::Value;
use minijinja::value::Serde;

use crate::{RenderListing, RenderPost};

/// The main content of one page. The variant picks the template.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PageBody {
    /// The blog's front page in one language.
    Home(RenderListing),
    /// One page of a collection, such as `/blog/` or `/guides/`.
    Collection(RenderListing),
    /// One post.
    Post(Box<RenderPost>),
    /// The page for an address with nothing behind it.
    NotFound,
}

impl PageBody {
    /// The template that renders this page type.
    pub(crate) fn template(&self) -> &'static str {
        match self {
            Self::Home(_) => "home.html",
            Self::Collection(_) => "collection.html",
            Self::Post(_) => "post.html",
            Self::NotFound => "not_found.html",
        }
    }

    /// The page's own data, which templates read as `page`.
    pub(crate) fn value(&self) -> Value {
        match self {
            Self::Home(listing) | Self::Collection(listing) => Value::from(Serde(listing)),
            Self::Post(post) => Value::from(Serde(post)),
            Self::NotFound => Value::UNDEFINED,
        }
    }
}
