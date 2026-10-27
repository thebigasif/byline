//! What a page points at: links, images and translations.

use khalid_core::AbsoluteUrl;

/// A link from a page, with any `#fragment` taken off.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub(crate) to: AbsoluteUrl,
    pub(crate) text: String,
    pub(crate) nofollow: bool,
}

impl Link {
    /// Where the link points.
    pub fn to(&self) -> &AbsoluteUrl {
        &self.to
    }

    /// Its anchor text, or the description of the image inside it.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Whether it carries `rel="nofollow"`.
    pub fn nofollow(&self) -> bool {
        self.nofollow
    }
}

/// An image on a page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub(crate) src: AbsoluteUrl,
    pub(crate) alt: Option<String>,
}

impl Image {
    /// Where the image is.
    pub fn src(&self) -> &AbsoluteUrl {
        &self.src
    }

    /// Its `alt` text, empty when it is decorative, `None` when missing.
    pub fn alt(&self) -> Option<&str> {
        self.alt.as_deref()
    }
}

/// A translation a page names with `<link rel="alternate" hreflang>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alternate {
    pub(crate) language: String,
    pub(crate) to: AbsoluteUrl,
}

impl Alternate {
    /// The language code, such as `de` or `x-default`.
    pub fn language(&self) -> &str {
        &self.language
    }

    /// The translation's address.
    pub fn to(&self) -> &AbsoluteUrl {
        &self.to
    }
}
