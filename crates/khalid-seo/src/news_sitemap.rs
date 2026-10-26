//! The news sitemap: news articles from the last two days, for Google News.

use chrono::{DateTime, TimeDelta, Utc};
use khalid_core::{AbsoluteUrl, Locale};

use crate::escape::escape_markup;
use crate::sitemap::is_listed;
use crate::social::rfc3339;
use crate::{ArticleType, PageKind, SeoError, SeoPage, SeoSite};

/// Google News reads articles published in the last two days.
const NEWS_WINDOW: TimeDelta = TimeDelta::days(2);

/// One article in the news sitemap.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct NewsEntry {
    /// The article's address.
    pub loc: AbsoluteUrl,
    /// The language the article is written in.
    pub language: Locale,
    /// When it was published.
    pub published: DateTime<Utc>,
    /// Its headline.
    pub title: String,
}

/// A news sitemap: a `urlset` of news articles with `news:news` details.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewsSitemap {
    publication: String,
    entries: Vec<NewsEntry>,
}

/// The language code Google News wants: the language alone, except
/// Chinese, which keeps its region or script (`zh-cn`, `zh-tw`).
fn news_language(language: &Locale) -> String {
    if language.language() == "zh" {
        language.as_str().to_ascii_lowercase()
    } else {
        language.language().to_owned()
    }
}

impl NewsSitemap {
    /// The news sitemap of `pages` at `now`: every listed post whose article
    /// type is news and which was published in the two days before `now`.
    /// The publication name is the site name.
    ///
    /// # Errors
    ///
    /// [`SeoError::Url`] when an article's address cannot be built.
    pub fn from_pages(
        site: &SeoSite,
        pages: &[SeoPage],
        now: DateTime<Utc>,
    ) -> Result<Self, SeoError> {
        let since = now - NEWS_WINDOW;
        let mut entries = Vec::new();
        for page in pages {
            let Some(published) = page.published else {
                continue;
            };
            let fresh = since <= published && published <= now;
            if page.kind == PageKind::Post
                && page.article_type == ArticleType::NewsArticle
                && fresh
                && is_listed(site, page)
            {
                entries.push(NewsEntry {
                    loc: site.url_for(&page.language, &page.path)?,
                    language: page.language.clone(),
                    published,
                    title: page.title.clone(),
                });
            }
        }
        Ok(Self {
            publication: site.name.clone(),
            entries,
        })
    }

    /// The articles, in the order the pages were given.
    pub fn entries(&self) -> &[NewsEntry] {
        &self.entries
    }

    /// The news sitemap as a UTF-8 XML document.
    pub fn to_xml(&self) -> String {
        let mut out = String::from(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\" \
             xmlns:news=\"http://www.google.com/schemas/sitemap-news/0.9\">\n",
        );
        for entry in &self.entries {
            out.push_str("  <url>\n    <loc>");
            escape_markup(entry.loc.as_str(), &mut out);
            out.push_str("</loc>\n    <news:news>\n      <news:publication>\n        <news:name>");
            escape_markup(&self.publication, &mut out);
            out.push_str("</news:name>\n        <news:language>");
            out.push_str(&news_language(&entry.language));
            out.push_str(
                "</news:language>\n      </news:publication>\n      <news:publication_date>",
            );
            out.push_str(&rfc3339(&entry.published));
            out.push_str("</news:publication_date>\n      <news:title>");
            escape_markup(&entry.title, &mut out);
            out.push_str("</news:title>\n    </news:news>\n  </url>\n");
        }
        out.push_str("</urlset>\n");
        out
    }
}
