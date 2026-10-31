//! SEO outputs for a Khalid site (plan section 5.2).
//!
//! This crate owns what search engines and feed readers read: the `<head>`
//! tags of every page, hreflang, sitemaps, feeds, `robots.txt`, the redirect
//! table, the JSON-LD schema graphs, and the `IndexNow` key file and
//! requests that tell search engines which pages changed. It takes its own
//! input types, such as [`SeoSite`], and returns ordered output (BL-TYP-5),
//! so the same input always gives the same bytes (ADR 0013).
//!
//! It must not know about templates, files or the database, nor how HTTP is
//! done: [`IndexNow::send`] is the one call that reaches another server, and
//! it does so through the `Outbound` client it is given (BL-RUN-8). The
//! crates that read content and render pages build its inputs and place its
//! output.

mod bing_answer;
mod bing_error;
mod comment;
mod escape;
mod feed;
mod feed_atom;
mod feed_json;
mod feed_rss;
mod head;
mod hreflang;
mod html_sitemap;
mod indexnow;
mod indexnow_send;
mod link_policy;
mod llms_txt;
mod meta_template;
mod meta_templates;
mod news_sitemap;
mod page;
mod redirect_columns;
mod redirect_csv;
mod redirect_explain;
mod redirect_moves;
mod redirect_patterns;
mod redirects;
mod robots_meta;
mod robots_txt;
mod schema;
mod seo_error;
mod site;
mod sitemap;
mod sitemap_index;
mod sitemap_set;
mod social;
mod url_path;
mod verification;
mod video;

pub use bing_answer::{BingSitemap, CrawlDay, CrawlIssue, CrawlIssueKind};
pub use bing_error::{BingError, RequestFailure};
pub use comment::SeoComment;
pub use feed::{Feed, FeedContent, FeedFormat, FeedItem, FeedOptions, FeedScope};
pub use head::{HeadTag, HeadTags, head_tags};
pub use hreflang::{Alternate, Hreflang, TranslationKey, link_translations};
pub use html_sitemap::{HtmlSitemap, HtmlSitemapGroup, HtmlSitemapLink};
pub use indexnow::{IndexNow, KeyFile, Submission};
pub use indexnow_send::{Retries, SendIndexNowError, Sent};
pub use link_policy::{Domain, LinkAttributes, LinkPolicy, LinkRel, LinkTarget};
pub use llms_txt::{LlmsTxt, llms_txt_url};
pub use meta_template::MetaTemplate;
pub use meta_templates::{MetaTemplates, TemplatePair};
pub use news_sitemap::{NewsEntry, NewsSitemap};
pub use page::{ArticleType, Crumb, PageKind, PublishState, SeoPage, SitemapListing};
pub use redirect_columns::{Matching, RedirectColumns};
pub use redirect_csv::{read_redirects_csv, write_redirects_csv};
pub use redirect_explain::{Explanation, Hop, Matched};
pub use redirect_moves::{moved_addresses, nearest_page};
pub use redirect_patterns::{PatternRedirect, PatternRule, RedirectPattern};
pub use redirects::{
    Destination, Move, Redirect, RedirectEntry, RedirectRule, RedirectTable, Resolution,
};
pub use robots_meta::IndexPolicy;
pub use robots_txt::{
    BotAccess, CrawlerPolicy, KNOWN_AI_CRAWLERS, RobotsPath, RobotsTxt, UserAgent,
};
pub use schema::{
    AggregateRating, AuthorProfile, CustomSchema, Entity, EntityRole, FaqItem, HowTo, HowToStep,
    IssueLevel, LocalBusiness, Money, OpeningHours, PostalAddress, Publisher, PublisherKind,
    Rating, Recipe, Review, SchemaBlock, SchemaGraph, SchemaIssue, SchemaNode, SchemaTemplate,
    SchemaType, SchemaValue, SiteSearch, TemplateScope, breadcrumb_list, check_graph, page_graph,
    site_graph,
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
