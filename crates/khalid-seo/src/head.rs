//! The SEO tags in a page's `<head>`, in a fixed order.

use khalid_core::AbsoluteUrl;

use crate::escape::escape_markup;
use crate::social::push_social_tags;
use crate::{SeoError, SeoPage, SeoSite};

/// One tag in a page's `<head>`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum HeadTag {
    /// `<title>`.
    Title(String),
    /// `<meta name="..." content="...">`.
    Meta {
        /// The `name` attribute.
        name: &'static str,
        /// The `content` attribute.
        content: String,
    },
    /// `<meta property="..." content="...">`, for Open Graph.
    Property {
        /// The `property` attribute, such as `og:title`.
        property: &'static str,
        /// The `content` attribute.
        content: String,
    },
    /// `<link rel="..." href="...">`.
    Link {
        /// The `rel` attribute.
        rel: &'static str,
        /// The `href` attribute.
        href: AbsoluteUrl,
    },
}

impl HeadTag {
    fn write_html(&self, out: &mut String) {
        match self {
            Self::Title(text) => {
                out.push_str("<title>");
                escape_markup(text, out);
                out.push_str("</title>");
            }
            Self::Meta { name, content } => {
                out.push_str("<meta name=\"");
                escape_markup(name, out);
                out.push_str("\" content=\"");
                escape_markup(content, out);
                out.push_str("\">");
            }
            Self::Property { property, content } => {
                out.push_str("<meta property=\"");
                escape_markup(property, out);
                out.push_str("\" content=\"");
                escape_markup(content, out);
                out.push_str("\">");
            }
            Self::Link { rel, href } => {
                out.push_str("<link rel=\"");
                escape_markup(rel, out);
                out.push_str("\" href=\"");
                escape_markup(href.as_str(), out);
                out.push_str("\">");
            }
        }
    }
}

/// The SEO tags of one page, in the order they are written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadTags(Vec<HeadTag>);

impl HeadTags {
    /// The tags, in order.
    pub fn as_slice(&self) -> &[HeadTag] {
        &self.0
    }

    /// The tags as HTML, one per line, with every value escaped.
    pub fn to_html(&self) -> String {
        let mut out = String::new();
        for tag in &self.0 {
            tag.write_html(&mut out);
            out.push('\n');
        }
        out
    }
}

/// The SEO tags for `page` on `site`: title, description, robots, canonical,
/// then Open Graph and the Twitter card.
///
/// A page's own title, description, robots directives and canonical address
/// win over the site's templates and index policy.
///
/// # Errors
///
/// [`SeoError::Url`] when the page's address cannot be built.
pub fn head_tags(site: &SeoSite, page: &SeoPage) -> Result<HeadTags, SeoError> {
    let pair = site.templates.for_kind(page.kind);
    let title = page.seo_title.as_ref().unwrap_or(&pair.title);
    let description = page.seo_description.as_ref().unwrap_or(&pair.description);
    let canonical = match &page.canonical {
        Some(url) => url.clone(),
        None => site.url_for(&page.language, &page.path)?,
    };
    let mut tags = vec![HeadTag::Title(title.render(site, page))];
    let description = description.render(site, page);
    if !description.is_empty() {
        tags.push(HeadTag::Meta {
            name: "description",
            content: description.clone(),
        });
    }
    tags.push(HeadTag::Meta {
        name: "robots",
        content: site.indexing.directives_for(page).to_string(),
    });
    tags.push(HeadTag::Link {
        rel: "canonical",
        href: canonical.clone(),
    });
    push_social_tags(site, page, &canonical, &description, &mut tags);
    Ok(HeadTags(tags))
}
