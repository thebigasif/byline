//! [`SeoPage`]: what the SEO outputs need to know about one page.

use chrono::{DateTime, Utc};
use khalid_core::{AbsoluteUrl, Locale, RobotsDirectives};

use crate::{
    Alternate, AuthorProfile, CustomSchema, Entity, MetaTemplate, SchemaBlock, SeoComment,
    ShareImage, TranslationKey, UrlPath, Video,
};

/// The kind of page, which picks its title template and index setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum PageKind {
    /// The blog's front page.
    Home,
    /// One post.
    Post,
    /// A standalone page such as "About".
    Page,
    /// The AI info page, which tells a reader who runs the site, what it
    /// covers, how to cite it and how to reach the team (P3).
    AiPage,
    /// An author's archive.
    Author,
    /// A tag archive.
    Tag,
    /// A category archive.
    Category,
    /// A collection index such as `/guides/`.
    Collection,
    /// A date archive.
    Date,
    /// Search results.
    Search,
    /// The glossary's A to Z index.
    GlossaryIndex,
    /// One glossary term.
    GlossaryTerm,
    /// The page shown for an address that does not exist.
    NotFound,
}

/// What kind of article a post is, from its content type.
///
/// It picks the post's schema.org type and whether it goes in the news sitemap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum ArticleType {
    /// A general article.
    Article,
    /// A blog post, the default.
    #[default]
    BlogPosting,
    /// A news story, listed in the news sitemap for two days.
    NewsArticle,
    /// A technical guide or reference.
    TechArticle,
}

/// One step in a page's breadcrumb trail, between the home page and the page.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Crumb {
    /// What the step is called, such as the category name.
    pub name: String,
    /// Its path, such as `/category/on-page-seo/`.
    pub path: UrlPath,
}

impl Crumb {
    /// A step called `name` at `path`.
    pub fn new(name: &str, path: UrlPath) -> Self {
        Self {
            name: name.to_owned(),
            path,
        }
    }
}

/// Whether readers can see the page yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum PublishState {
    /// Live on the site.
    #[default]
    Published,
    /// Not finished, seen only in the admin.
    Draft,
    /// Finished, waiting for its publish time.
    Scheduled,
}

/// Whether the page appears in the XML sitemap when it is indexable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum SitemapListing {
    /// Listed, the default.
    #[default]
    Listed,
    /// Left out, such as a thank-you page after a form.
    Excluded,
}

/// What the SEO outputs need to know about one page.
///
/// Build it with [`SeoPage::new`], then fill the fields the page has.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SeoPage {
    /// The kind of page.
    pub kind: PageKind,
    /// The language the page is written in.
    pub language: Locale,
    /// The path inside the mount folder and language prefix, such as `/why-links-matter/`.
    pub path: UrlPath,
    /// The page's own title, the `{title}` variable.
    pub title: String,
    /// The summary, the `{excerpt}` variable.
    pub excerpt: String,
    /// The category name, the `{category}` variable.
    pub category: Option<String>,
    /// The author's name, the `{author}` variable.
    pub author: Option<String>,
    /// The collection the page belongs to, such as `guides`.
    pub collection: Option<String>,
    /// A title template for this page alone, in place of its kind's template.
    pub seo_title: Option<MetaTemplate>,
    /// A description template for this page alone.
    pub seo_description: Option<MetaTemplate>,
    /// The canonical address when it is not this page, such as the original of a syndicated post.
    pub canonical: Option<AbsoluteUrl>,
    /// Robots directives for this page alone, in place of the site's index policy.
    pub robots: Option<RobotsDirectives>,
    /// For an archive, how many posts it lists. A thin archive is not indexed.
    pub item_count: Option<u32>,
    /// The image link previews show, in place of the site's default.
    pub share_image: Option<ShareImage>,
    /// When the page was first published.
    pub published: Option<DateTime<Utc>>,
    /// When the page last changed in a way a reader would notice.
    pub modified: Option<DateTime<Utc>>,
    /// The group this page's language versions share.
    pub translation: Option<TranslationKey>,
    /// Every language version of the page, itself included, then
    /// `x-default`. [`link_translations`](crate::link_translations) fills it.
    pub alternates: Vec<Alternate>,
    /// Whether readers can see the page yet.
    pub state: PublishState,
    /// Whether the page appears in the XML sitemap.
    pub sitemap: SitemapListing,
    /// For a post, what kind of article it is.
    pub article_type: ArticleType,
    /// The images in the page's content, for the image sitemap.
    pub images: Vec<AbsoluteUrl>,
    /// The videos in the page's content, for the video sitemap.
    pub videos: Vec<Video>,
    /// The rendered body as HTML, for full-content feeds.
    pub content_html: String,
    /// The steps between the home page and this page, such as its category.
    pub breadcrumbs: Vec<Crumb>,
    /// CSS selectors of the parts a voice assistant may read aloud, such as
    /// the summary, for schema.org `speakable`.
    pub speakable: Vec<String>,
    /// The author's full profile. Without one, `author` alone names them.
    pub author_profile: Option<AuthorProfile>,
    /// The people who checked the page, in order, for `reviewedBy`.
    pub reviewers: Vec<AuthorProfile>,
    /// The things the post is about or mentions, for `about` and `mentions`.
    pub entities: Vec<Entity>,
    /// How many words the post has.
    pub word_count: Option<u32>,
    /// The page's blocks that write their own schema, such as an FAQ.
    pub blocks: Vec<SchemaBlock>,
    /// JSON-LD written by hand for this page alone.
    pub custom_schema: CustomSchema,
    /// The approved comments readers left, oldest first, each with its
    /// replies, for the article's `comment` property.
    pub comments: Vec<SeoComment>,
}

impl SeoPage {
    /// A page with a kind, a language, a path and a title, and nothing else set.
    pub fn new(kind: PageKind, language: Locale, path: UrlPath, title: &str) -> Self {
        Self {
            kind,
            language,
            path,
            title: title.to_owned(),
            excerpt: String::new(),
            category: None,
            author: None,
            collection: None,
            seo_title: None,
            seo_description: None,
            canonical: None,
            robots: None,
            item_count: None,
            share_image: None,
            published: None,
            modified: None,
            translation: None,
            alternates: Vec::new(),
            state: PublishState::Published,
            sitemap: SitemapListing::Listed,
            article_type: ArticleType::BlogPosting,
            images: Vec::new(),
            videos: Vec::new(),
            content_html: String::new(),
            breadcrumbs: Vec::new(),
            speakable: Vec::new(),
            author_profile: None,
            reviewers: Vec::new(),
            entities: Vec::new(),
            word_count: None,
            blocks: Vec::new(),
            custom_schema: CustomSchema::default(),
            comments: Vec::new(),
        }
    }
}
