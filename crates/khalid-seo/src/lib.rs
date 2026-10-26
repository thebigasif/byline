//! SEO outputs for a Khalid site (plan section 5.2).
//!
//! This crate owns what search engines and feed readers read: the `<head>`
//! tags of every page, hreflang, sitemaps, feeds, `robots.txt`, the redirect
//! table and the JSON-LD schema graphs. It takes its own input types, such as
//! [`SeoSite`], and returns ordered output (BL-TYP-5), so the same input
//! always gives the same bytes (ADR 0009).
//!
//! It must not know about templates, files, HTTP or the database. The crates
//! that read content and render pages build its inputs and place its output.

mod escape;
mod head;
mod meta_template;
mod meta_templates;
mod page;
mod robots_meta;
mod seo_error;
mod site;
mod url_path;

pub use head::{HeadTag, HeadTags, head_tags};
pub use meta_template::MetaTemplate;
pub use meta_templates::{MetaTemplates, TemplatePair};
pub use page::{PageKind, SeoPage};
pub use robots_meta::{
    Following, ImagePreview, IndexPolicy, Indexing, PreviewLimit, Restriction, RobotsDirectives,
};
pub use seo_error::SeoError;
pub use site::SeoSite;
pub use url_path::UrlPath;
