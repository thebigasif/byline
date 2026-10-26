//! Open Graph and Twitter card tags, and the share image they point to.

use chrono::{DateTime, SecondsFormat, Utc};
use khalid_core::{AbsoluteUrl, Locale};

use crate::{HeadTag, PageKind, SeoError, SeoPage, SeoSite};

/// The longest Twitter handle, without its `@`.
const TWITTER_HANDLE_MAX: usize = 15;

/// The image a link preview shows: on social networks, in chat apps and in Discover.
///
/// 1200 by 630 pixels suits every network. The alternative text is read
/// aloud by screen readers on the networks that support it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ShareImage {
    /// The image's public address.
    pub url: AbsoluteUrl,
    /// The width in pixels.
    pub width: u32,
    /// The height in pixels.
    pub height: u32,
    /// What the image shows, in one short sentence.
    pub alt: String,
}

impl ShareImage {
    /// An image with its address, size and alternative text.
    pub fn new(url: AbsoluteUrl, width: u32, height: u32, alt: &str) -> Self {
        Self {
            url,
            width,
            height,
            alt: alt.to_owned(),
        }
    }
}

/// A Twitter or X account name, such as `@serpdad`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TwitterHandle(String);

impl TwitterHandle {
    /// Checks a handle, with or without its leading `@`, and stores it with one.
    ///
    /// # Errors
    ///
    /// [`SeoError::TwitterHandle`] when the name is empty, longer than 15
    /// letters, or holds anything but ASCII letters, digits and `_`.
    pub fn try_new(text: &str) -> Result<Self, SeoError> {
        let name = text.strip_prefix('@').unwrap_or(text);
        let valid = !name.is_empty()
            && name.len() <= TWITTER_HANDLE_MAX
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
        if valid {
            Ok(Self(format!("@{name}")))
        } else {
            Err(SeoError::TwitterHandle {
                handle: text.to_owned(),
            })
        }
    }

    /// The handle with its `@`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The Open Graph form of a language tag: `pt-BR` becomes `pt_BR`.
fn og_locale(language: &Locale) -> String {
    language.as_str().replace('-', "_")
}

fn rfc3339(time: &DateTime<Utc>) -> String {
    time.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn property(out: &mut Vec<HeadTag>, property: &'static str, content: String) {
    out.push(HeadTag::Property { property, content });
}

/// Appends the Open Graph and Twitter card tags for `page` to `out`.
///
/// The title is the page's own title, or the site name on a page without
/// one. The image is the page's own, else the site's default, else none, in
/// which case the Twitter card is the small `summary`.
pub(crate) fn push_social_tags(
    site: &SeoSite,
    page: &SeoPage,
    canonical: &AbsoluteUrl,
    description: &str,
    out: &mut Vec<HeadTag>,
) {
    let is_article = page.kind == PageKind::Post;
    let title = if page.title.is_empty() {
        site.name.clone()
    } else {
        page.title.clone()
    };
    property(out, "og:site_name", site.name.clone());
    let og_type = if is_article { "article" } else { "website" };
    property(out, "og:type", og_type.to_owned());
    property(out, "og:title", title);
    if !description.is_empty() {
        property(out, "og:description", description.to_owned());
    }
    property(out, "og:url", canonical.as_str().to_owned());
    property(out, "og:locale", og_locale(&page.language));
    let image = page.share_image.as_ref().or(site.share_image.as_ref());
    if let Some(image) = image {
        property(out, "og:image", image.url.as_str().to_owned());
        property(out, "og:image:width", image.width.to_string());
        property(out, "og:image:height", image.height.to_string());
        if !image.alt.is_empty() {
            property(out, "og:image:alt", image.alt.clone());
        }
    }
    if is_article {
        if let Some(published) = &page.published {
            property(out, "article:published_time", rfc3339(published));
        }
        if let Some(modified) = &page.modified {
            property(out, "article:modified_time", rfc3339(modified));
        }
        if let Some(category) = &page.category {
            property(out, "article:section", category.clone());
        }
    }
    let card = if image.is_some() {
        "summary_large_image"
    } else {
        "summary"
    };
    out.push(HeadTag::Meta {
        name: "twitter:card",
        content: card.to_owned(),
    });
    if let Some(handle) = &site.twitter_site {
        out.push(HeadTag::Meta {
            name: "twitter:site",
            content: handle.as_str().to_owned(),
        });
    }
}
