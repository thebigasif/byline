//! The book block: `Book`.

use khalid_core::{AbsoluteUrl, Locale};

use crate::schema::node_id;
use crate::{SchemaNode, SchemaValue, SeoError};

/// A book block: a book the post reviews, recommends or announces.
///
/// Google shows book results only for its partners' feeds, so no rule
/// checks it; the name and author are required, as schema.org readers
/// need them to know which book it is.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Book {
    /// The title.
    pub name: String,
    /// Who wrote it.
    pub author: String,
    /// The language it is written in, if known.
    pub language: Option<Locale>,
    /// Its ISBN, such as `978-0-00-000000-2`.
    pub isbn: Option<String>,
    /// Where to read or buy it.
    pub url: Option<AbsoluteUrl>,
}

impl Book {
    /// The book `name` by `author`.
    pub fn new(name: &str, author: &str) -> Self {
        Self {
            name: name.to_owned(),
            author: author.to_owned(),
            language: None,
            isbn: None,
            url: None,
        }
    }
}

/// The `Book` at `<page>#book-<number>`.
pub(crate) fn book_node(
    book: &Book,
    page: &AbsoluteUrl,
    number: usize,
) -> Result<SchemaNode, SeoError> {
    let author = SchemaNode::new("Person").with("name", SchemaValue::text(&book.author));
    let mut node = SchemaNode::new("Book")
        .with_id(node_id(page, &format!("book-{number}"))?)
        .with("name", SchemaValue::text(&book.name))
        .with("author", SchemaValue::Node(Box::new(author)));
    if let Some(language) = &book.language {
        node.set("inLanguage", SchemaValue::text(language.as_str()));
    }
    if let Some(isbn) = &book.isbn {
        node.set("isbn", SchemaValue::text(isbn));
    }
    if let Some(url) = &book.url {
        node.set("url", SchemaValue::Url(url.clone()));
    }
    Ok(node)
}
