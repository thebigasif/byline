//! What fills a page's main area, one variant per page type.

use minijinja::Value;
use minijinja::value::Serde;

use crate::{
    RenderAuthor, RenderAuthors, RenderGlossary, RenderHome, RenderListing, RenderPage, RenderPost,
    RenderSearch,
};

/// The main content of one page. The variant picks the template.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PageBody {
    /// The blog's front page in one language.
    Home(RenderHome),
    /// One page of a collection, such as `/blog/` or `/guides/`.
    Collection(RenderListing),
    /// One page of the posts with a tag.
    Tag(RenderListing),
    /// One page of the posts in a category.
    Category(RenderListing),
    /// Every published post across collections, newest first, at `/archive/`.
    Archive(RenderListing),
    /// One post.
    Post(Box<RenderPost>),
    /// A page that is not a post, such as About.
    Page(RenderPage),
    /// Every author, at `/authors/`.
    Authors(RenderAuthors),
    /// An author and their posts.
    Author(Box<RenderAuthor>),
    /// The glossary from A to Z.
    Glossary(RenderGlossary),
    /// The search form and its results.
    Search(RenderSearch),
    /// The page for an address with nothing behind it.
    NotFound,
}

impl PageBody {
    /// The template that renders this page type.
    pub(crate) fn template(&self) -> &'static str {
        match self {
            Self::Home(_) => "home.html",
            Self::Collection(_) => "collection.html",
            Self::Tag(_) => "tag.html",
            Self::Category(_) => "category.html",
            Self::Archive(_) => "archive.html",
            Self::Post(_) => "post.html",
            Self::Page(_) => "page.html",
            Self::Author(_) => "author.html",
            Self::Authors(_) => "authors.html",
            Self::Glossary(_) => "glossary.html",
            Self::Search(_) => "search.html",
            Self::NotFound => "not_found.html",
        }
    }

    /// The page's own data, which templates read as `page`.
    pub(crate) fn value(&self) -> Value {
        match self {
            Self::Home(home) => Value::from(Serde(home)),
            Self::Collection(listing)
            | Self::Tag(listing)
            | Self::Category(listing)
            | Self::Archive(listing) => Value::from(Serde(listing)),
            Self::Post(post) => Value::from(Serde(post)),
            Self::Page(page) => Value::from(Serde(page)),
            Self::Author(author) => Value::from(Serde(author)),
            Self::Authors(authors) => Value::from(Serde(authors)),
            Self::Glossary(glossary) => Value::from(Serde(glossary)),
            Self::Search(search) => Value::from(Serde(search)),
            Self::NotFound => Value::UNDEFINED,
        }
    }
}
