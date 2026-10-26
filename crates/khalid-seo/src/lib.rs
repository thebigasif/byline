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
mod feed;
mod head;
mod hreflang;
mod html_sitemap;
mod llms_txt;
mod meta_template;
mod meta_templates;
mod news_sitemap;
mod page;
mod robots_meta;
mod robots_txt;
mod seo_error;
mod site;
mod sitemap;
mod sitemap_index;
mod sitemap_set;
mod social;
mod url_path;
mod verification;
mod video;

pub use feed::{Feed, FeedContent, FeedItem, FeedOptions, FeedScope};
pub use head::{HeadTag, HeadTags, head_tags};
pub use hreflang::{Alternate, Hreflang, TranslationKey, link_translations};
pub use html_sitemap::{HtmlSitemap, HtmlSitemapGroup, HtmlSitemapLink};
pub use llms_txt::{LlmsTxt, llms_txt_url};
pub use meta_template::MetaTemplate;
pub use meta_templates::{MetaTemplates, TemplatePair};
pub use news_sitemap::{NewsEntry, NewsSitemap};
pub use page::{ArticleType, PageKind, PublishState, SeoPage, SitemapListing};
pub use robots_meta::{
    Following, ImagePreview, IndexPolicy, Indexing, PreviewLimit, Restriction, RobotsDirectives,
};
pub use robots_txt::{
    BotAccess, CrawlerPolicy, KNOWN_AI_CRAWLERS, RobotsPath, RobotsTxt, UserAgent,
};
pub use seo_error::SeoError;
pub use site::SeoSite;
pub use sitemap::{Sitemap, SitemapEntry};
pub use sitemap_index::{IndexEntry, SitemapIndex, sitemap_index_url};
pub use sitemap_set::{SitemapFile, SitemapSection, SitemapSet};
pub use social::{ShareImage, TwitterHandle};
pub use url_path::UrlPath;
pub use verification::{RootFile, SiteVerification, VerificationCode};
pub use video::{Video, VideoSource};
