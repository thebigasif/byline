//! What kind of file a page is, as the `Content-Type` header says it.

/// The kind of file a [`Page`](crate::Page) holds: text in UTF-8, or an
/// image or other file from beside a post.
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
    /// A PNG image.
    Png,
    /// A JPEG image.
    Jpeg,
    /// A WebP image.
    Webp,
    /// An AVIF image.
    Avif,
    /// A GIF image.
    Gif,
    /// An SVG drawing, which is text.
    Svg,
    /// A PDF document.
    Pdf,
    /// Any other file, sent as bytes for the reader to save.
    Binary,
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
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Webp => "image/webp",
            Self::Avif => "image/avif",
            Self::Gif => "image/gif",
            Self::Svg => "image/svg+xml",
            Self::Pdf => "application/pdf",
            Self::Binary => "application/octet-stream",
        }
    }

    /// The kind a file `name` has by its extension, in any case, such as
    /// [`MediaType::Png`] for `chart.PNG`. A name with no extension we
    /// know is [`MediaType::Binary`].
    pub fn for_file(name: &str) -> Self {
        let extension = name
            .rsplit_once('.')
            .map(|(_, extension)| extension.to_ascii_lowercase());
        match extension.as_deref() {
            Some("html" | "htm") => Self::Html,
            Some("xml") => Self::Xml,
            Some("txt" | "md") => Self::PlainText,
            Some("json") => Self::Json,
            Some("css") => Self::Css,
            Some("png") => Self::Png,
            Some("jpg" | "jpeg") => Self::Jpeg,
            Some("webp") => Self::Webp,
            Some("avif") => Self::Avif,
            Some("gif") => Self::Gif,
            Some("svg") => Self::Svg,
            Some("pdf") => Self::Pdf,
            _ => Self::Binary,
        }
    }

    /// Whether Brotli can make this kind smaller. Images, PDF documents and
    /// unknown files are compressed already, or may be, so they are sent
    /// as they are.
    pub(crate) fn compresses(self) -> bool {
        matches!(
            self,
            Self::Html | Self::Xml | Self::PlainText | Self::Json | Self::Css | Self::Svg
        )
    }
}
