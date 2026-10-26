//! What kind of file a page is, as the `Content-Type` header says it.

/// The kind of file a [`Page`](crate::Page) holds. Every kind is text in
/// UTF-8.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum MediaType {
    /// A web page.
    Html,
    /// A sitemap, or any other XML file.
    Xml,
    /// A plain text file, such as `robots.txt` or `llms.txt`.
    PlainText,
    /// A JSON file, such as a JSON feed.
    Json,
    /// A stylesheet.
    Css,
}

impl MediaType {
    /// The value of the `Content-Type` header for this kind.
    pub fn as_header(self) -> &'static str {
        match self {
            Self::Html => "text/html; charset=utf-8",
            Self::Xml => "application/xml; charset=utf-8",
            Self::PlainText => "text/plain; charset=utf-8",
            Self::Json => "application/json",
            Self::Css => "text/css; charset=utf-8",
        }
    }
}
