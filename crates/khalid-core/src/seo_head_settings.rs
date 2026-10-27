//! The `[seo]` tables that shape every page's head and the files beside it:
//! titles, social, verification, crawlers, feeds and links.
//!
//! Core reads only their shape. `khalid` checks each value against the rules
//! in `khalid-seo`, such as which variables a template may use, before any
//! command runs.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::AbsoluteUrl;

/// Title and description templates, from `[seo.titles]`.
///
/// A template is text with variables in braces: `{title}`, `{site}`,
/// `{tagline}`, `{sep}`, `{excerpt}`, `{category}` and `{author}`. A page
/// type left out keeps its default, `{title} {sep} {site}` with `{excerpt}`,
/// or on the home page `{site} {sep} {tagline}` with `{tagline}`:
///
/// ```toml
/// [seo.titles]
/// separator = "·"
/// tagline = "Plain notes on search"
///
/// [seo.titles.post]
/// title = "{title} {sep} {site}"
/// description = "{excerpt}"
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[non_exhaustive]
pub struct TitleSettings {
    /// What `{sep}` stands for. `None` keeps `|`.
    pub separator: Option<String>,
    /// One line about the site, the `{tagline}` variable. Empty by default.
    pub tagline: String,
    /// The blog's front page.
    pub home: Option<TemplateSettings>,
    /// One post.
    pub post: Option<TemplateSettings>,
    /// A standalone page such as "About".
    pub page: Option<TemplateSettings>,
    /// An author's archive.
    pub author: Option<TemplateSettings>,
    /// A tag archive.
    pub tag: Option<TemplateSettings>,
    /// A category archive.
    pub category: Option<TemplateSettings>,
    /// A collection index such as `/guides/`.
    pub collection: Option<TemplateSettings>,
    /// A date archive.
    pub date: Option<TemplateSettings>,
    /// Search results.
    pub search: Option<TemplateSettings>,
    /// The glossary's A to Z page.
    pub glossary: Option<TemplateSettings>,
    /// One glossary term.
    pub glossary_term: Option<TemplateSettings>,
    /// The page for an address that does not exist.
    pub not_found: Option<TemplateSettings>,
}

/// The title and description templates of one page type. Either may be
/// left out to keep that page type's default.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[non_exhaustive]
pub struct TemplateSettings {
    /// The `<title>` template.
    pub title: Option<String>,
    /// The meta description template.
    pub description: Option<String>,
}

/// Link previews on social networks, from `[seo.social]`:
///
/// ```toml
/// [seo.social]
/// twitter = "@serpdad"
///
/// [seo.social.image]
/// url = "https://www.example.com/media/share.png"
/// alt = "The Field Notes logo on a white card"
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[non_exhaustive]
pub struct SocialSettings {
    /// The site's Twitter or X account, with or without its `@`.
    pub twitter: Option<String>,
    /// The share image of every page that has none of its own.
    pub image: Option<ImageSettings>,
}

/// An image by its public address, size and alternative text.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct ImageSettings {
    /// The image's public address.
    pub url: AbsoluteUrl,
    /// What the image shows, in one short sentence.
    pub alt: String,
    /// The width in pixels. 1200 by default, the size every network takes.
    #[serde(default = "share_width")]
    pub width: u32,
    /// The height in pixels. 630 by default.
    #[serde(default = "share_height")]
    pub height: u32,
}

fn share_width() -> u32 {
    1200
}

fn share_height() -> u32 {
    630
}

/// Proof to search engines that the site is the owner's, from
/// `[seo.verification]`. Each code becomes a meta tag on the home page, and
/// each file is served from the root of the address:
///
/// ```toml
/// [seo.verification]
/// google = "aBc123"
///
/// [seo.verification.files]
/// "google1a2b3c4d.html" = "google-site-verification: google1a2b3c4d.html"
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[non_exhaustive]
pub struct VerificationSettings {
    /// Google Search Console.
    pub google: Option<String>,
    /// Bing Webmaster Tools.
    pub bing: Option<String>,
    /// Yandex Webmaster.
    pub yandex: Option<String>,
    /// Baidu Search Resource Platform.
    pub baidu: Option<String>,
    /// Pinterest.
    pub pinterest: Option<String>,
    /// Verification files by name, each with the text it holds.
    pub files: BTreeMap<String, String>,
}

/// Which crawlers may read the blog, from `[seo.crawlers]`. By default every
/// crawler may read every page:
///
/// ```toml
/// [seo.crawlers]
/// disallow = ["/drafts/"]
/// block = ["GPTBot", "CCBot"]
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[non_exhaustive]
pub struct CrawlerSettings {
    /// Paths inside the mount folder that no crawler may read.
    pub disallow: Vec<String>,
    /// Crawlers named as allowed, such as `Googlebot`.
    pub allow: Vec<String>,
    /// Crawlers that may not read any page of the blog, such as `GPTBot`.
    pub block: Vec<String>,
}

/// What every feed carries, from `[seo.feeds]`:
///
/// ```toml
/// [seo.feeds]
/// content = "excerpt"
/// source_line = "{title} first appeared on {site}."
/// limit = 50
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[non_exhaustive]
pub struct FeedSettings {
    /// Whole posts or excerpts.
    pub content: FeedContentSetting,
    /// Text shown before each item's content. Empty for none.
    pub before_item: String,
    /// Text shown after each item's content. Empty for none.
    pub after_item: String,
    /// A line naming the original address, a template such as
    /// `{title} first appeared on {site}.`
    pub source_line: Option<String>,
    /// How many of the newest posts a feed holds. 20 by default.
    pub limit: u32,
}

impl Default for FeedSettings {
    fn default() -> Self {
        Self {
            content: FeedContentSetting::Full,
            before_item: String::new(),
            after_item: String::new(),
            source_line: None,
            limit: 20,
        }
    }
}

/// How much of each post a feed carries: `"full"` or `"excerpt"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum FeedContentSetting {
    /// The whole post.
    #[default]
    Full,
    /// The excerpt only.
    Excerpt,
}

/// The `rel` and `target` that links get, from `[seo.links]`. By default
/// no link gets anything added:
///
/// ```toml
/// [seo.links]
/// external = ["nofollow"]
/// target = "new-tab"
/// affiliate_domains = ["amazon.com"]
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[non_exhaustive]
pub struct LinkSettings {
    /// What every external link gets.
    pub external: Vec<LinkRelSetting>,
    /// Where external links open.
    pub target: LinkTargetSetting,
    /// Domains whose links are always `sponsored`, with their subdomains.
    pub affiliate_domains: Vec<String>,
    /// Other domains that count as this site, such as the main site's shop.
    pub internal_domains: Vec<String>,
}

/// A `rel` value for every external link: `"nofollow"`, `"sponsored"` or `"ugc"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum LinkRelSetting {
    /// The site does not vouch for the page.
    NoFollow,
    /// The link is paid or an affiliate link.
    Sponsored,
    /// The link was written by a visitor.
    Ugc,
}

/// Where an external link opens: `"same-tab"` or `"new-tab"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum LinkTargetSetting {
    /// In the same tab.
    #[default]
    SameTab,
    /// In a new tab.
    NewTab,
}
