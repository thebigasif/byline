//! The feed model that the RSS, Atom and JSON Feed writers share.

use chrono::{DateTime, Utc};
use khalid_core::{AbsoluteUrl, Locale};

use crate::escape::escape_markup;
use crate::{MetaTemplate, PageKind, PublishState, SeoError, SeoPage, SeoSite, UrlPath};

/// How much of each post a feed carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum FeedContent {
    /// The whole post, the default.
    #[default]
    Full,
    /// The excerpt only, so readers visit the site for the rest.
    Excerpt,
}

/// The site's feed settings.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FeedOptions {
    /// Full posts or excerpts.
    pub content: FeedContent,
    /// Text shown before each item's content. Empty for none.
    pub before_item: String,
    /// Text shown after each item's content. Empty for none.
    pub after_item: String,
    /// A line naming the original address, such as
    /// `{title} first appeared on {site}.`, linked to the post.
    pub source_line: Option<MetaTemplate>,
    /// How many of the newest posts a feed holds.
    pub limit: usize,
}

impl Default for FeedOptions {
    fn default() -> Self {
        Self {
            content: FeedContent::Full,
            before_item: String::new(),
            after_item: String::new(),
            source_line: None,
            limit: 20,
        }
    }
}

/// A feed format, each served next to the page the feed belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum FeedFormat {
    /// RSS 2.0, at `feed.xml`.
    Rss,
    /// Atom 1.0, at `atom.xml`.
    Atom,
    /// JSON Feed 1.1, at `feed.json`.
    Json,
}

impl FeedFormat {
    fn file_name(self) -> &'static str {
        match self {
            Self::Rss => "feed.xml",
            Self::Atom => "atom.xml",
            Self::Json => "feed.json",
        }
    }
}

/// Which posts a feed holds: one language, and one category or all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedScope {
    language: Locale,
    category: Option<(String, UrlPath)>,
}

impl FeedScope {
    /// Every post in `language`.
    pub fn language(language: Locale) -> Self {
        Self {
            language,
            category: None,
        }
    }

    /// The posts in `language` filed under the category `name`, whose
    /// archive lives at `archive`.
    pub fn category(language: Locale, name: &str, archive: UrlPath) -> Self {
        Self {
            language,
            category: Some((name.to_owned(), archive)),
        }
    }
}

/// One post in a feed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FeedItem {
    /// The post's title.
    pub title: String,
    /// The post's address, which is also its permanent id.
    pub url: AbsoluteUrl,
    /// When it was published.
    pub published: DateTime<Utc>,
    /// When it last changed, if it did.
    pub updated: Option<DateTime<Utc>>,
    /// The author's name.
    pub author: Option<String>,
    /// The category name.
    pub category: Option<String>,
    /// The excerpt, as plain text.
    pub summary: String,
    /// The content as HTML: the body or the excerpt, with the site's text
    /// before and after it and the source line.
    pub content_html: String,
    /// The post's share image.
    pub image: Option<AbsoluteUrl>,
}

/// A feed: its title, its pages and its newest posts.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Feed {
    /// The feed's title: the site name, or the category then the site name.
    pub title: String,
    /// One line about the feed: the site's tagline.
    pub description: String,
    /// The page the feed belongs to: the home page or the category archive.
    pub home: AbsoluteUrl,
    /// The language of every item.
    pub language: Locale,
    /// The latest change among the items, or the build time for an empty feed.
    pub updated: DateTime<Utc>,
    /// The posts, newest first.
    pub items: Vec<FeedItem>,
}

fn paragraph(text: &str, out: &mut String) {
    if !text.is_empty() {
        out.push_str("<p>");
        escape_markup(text, out);
        out.push_str("</p>");
    }
}

fn item_content(site: &SeoSite, page: &SeoPage, url: &AbsoluteUrl) -> String {
    let options = &site.feeds;
    let mut out = String::new();
    paragraph(&options.before_item, &mut out);
    match options.content {
        FeedContent::Full => out.push_str(&page.content_html),
        FeedContent::Excerpt => paragraph(&page.excerpt, &mut out),
    }
    paragraph(&options.after_item, &mut out);
    if let Some(line) = &options.source_line {
        out.push_str("<p><a href=\"");
        escape_markup(url.as_str(), &mut out);
        out.push_str("\">");
        escape_markup(&line.render(site, page), &mut out);
        out.push_str("</a></p>");
    }
    out
}

impl Feed {
    /// The feed's own address in `format`, next to its page: the home
    /// page's feed is `/blog/feed.xml`, a category's is in its folder.
    ///
    /// # Errors
    ///
    /// [`SeoError::Url`] when the address cannot be built.
    pub fn url(&self, format: FeedFormat) -> Result<AbsoluteUrl, SeoError> {
        let home = self.home.as_str();
        let slash = if home.ends_with('/') { "" } else { "/" };
        let joined = format!("{home}{slash}{}", format.file_name());
        AbsoluteUrl::parse(&joined).map_err(|source| SeoError::Url {
            path: format.file_name().to_owned(),
            source,
        })
    }

    /// The feed of `scope` at `now`: the newest published posts in its
    /// language and category, up to the site's limit.
    ///
    /// # Errors
    ///
    /// [`SeoError::Url`] when an address cannot be built.
    pub fn for_pages(
        site: &SeoSite,
        pages: &[SeoPage],
        scope: &FeedScope,
        now: DateTime<Utc>,
    ) -> Result<Self, SeoError> {
        let mut chosen: Vec<(&SeoPage, DateTime<Utc>)> = pages
            .iter()
            .filter(|page| {
                page.kind == PageKind::Post
                    && page.state == PublishState::Published
                    && page.language == scope.language
                    && scope
                        .category
                        .as_ref()
                        .is_none_or(|(name, _)| page.category.as_ref() == Some(name))
            })
            .filter_map(|page| page.published.map(|published| (page, published)))
            .collect();
        chosen.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.path.cmp(&b.0.path)));
        chosen.truncate(site.feeds.limit);
        let mut items = Vec::with_capacity(chosen.len());
        for (page, published) in chosen {
            let url = site.url_for(&page.language, &page.path)?;
            items.push(FeedItem {
                title: page.title.clone(),
                content_html: item_content(site, page, &url),
                url,
                published,
                updated: page.modified,
                author: page.author.clone(),
                category: page.category.clone(),
                summary: page.excerpt.clone(),
                image: page.share_image.as_ref().map(|image| image.url.clone()),
            });
        }
        let updated = items
            .iter()
            .map(|item| item.updated.unwrap_or(item.published))
            .max()
            .unwrap_or(now);
        let (title, home) = match &scope.category {
            Some((name, archive)) => (
                format!("{name} {} {}", site.separator, site.name),
                site.url_for(&scope.language, archive)?,
            ),
            None => (
                site.name.clone(),
                site.url_for(&scope.language, &UrlPath::root())?,
            ),
        };
        Ok(Self {
            title,
            description: site.tagline.clone(),
            home,
            language: scope.language.clone(),
            updated,
            items,
        })
    }
}
