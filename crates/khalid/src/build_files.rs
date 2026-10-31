//! The files `khalid build` writes beside the pages, from `khalid-seo`: the
//! sitemap index and its sitemaps, the news sitemap, `robots.txt`, `llms.txt`, the RSS,
//! Atom and JSON feeds of each language, and the `IndexNow` key file
//! (KH-SEO-5, KH-SEO-6, KH-SEO-7, KH-SEO-11).

use chrono::{DateTime, Utc};
use khalid_core::{IndexNowSettings, Locale, UserMessage};
use khalid_seo::{
    ArticleType, Feed, FeedScope, IndexNow, LlmsTxt, NewsSitemap, RobotsTxt, SeoError, SeoPage,
    SeoSite, SitemapSet, UrlPath,
};

use crate::build_message;

/// One file and the place it goes, relative to the output folder.
#[derive(Debug)]
pub(crate) struct SiteFile {
    pub(crate) file: String,
    pub(crate) text: String,
}

/// Every file of `site` beside its `pages`, whose language versions are
/// already linked. Each lives in the mount folder, where its address
/// points. `now` dates a feed that has no posts yet. With `indexnow` set,
/// the key file is one of them, at the address requests name in
/// `keyLocation`.
pub(crate) fn files(
    site: &SeoSite,
    pages: &[SeoPage],
    indexnow: Option<&IndexNowSettings>,
    now: DateTime<Utc>,
) -> Result<Vec<SiteFile>, UserMessage> {
    collect(site, pages, indexnow, now).map_err(|error| build_message::seo(&error))
}

/// The folder inside the output that holds the files for the root of the
/// host, for a blog in a folder (ADR "root files go in their own folder").
pub(crate) const ROOT_FOLDER: &str = "_root";

/// The files a blog in a folder needs at the root of its host, where search
/// engines look for them: `robots.txt`, whose first line says where it goes,
/// and each verification file. None for a blog at the root, whose own
/// files are already there.
pub(crate) fn root_files(site: &SeoSite) -> Result<Vec<SiteFile>, UserMessage> {
    if site.mount == UrlPath::root() {
        return Ok(Vec::new());
    }
    let robots = RobotsTxt::for_site(site).map_err(|error| build_message::seo(&error))?;
    let host = site.address.as_str().trim_end_matches('/');
    let mount = site.mount.as_str();
    let said = format!(
        "# For {host}/robots.txt. Every rule here is about {mount}, so use it as the whole file or add it to the one there.\n{robots}"
    );
    let mut files = vec![file("robots.txt", said)];
    for root in &site.root_files {
        let name = root.path().as_str().trim_start_matches('/');
        files.push(file(name, root.body().to_owned()));
    }
    Ok(files)
}

/// Where the news sitemap lives in the mount folder, beside the others. It
/// is written only for a site with a news collection, and lists its posts
/// of the last two days, so Google News reads it often.
const NEWS_SITEMAP: &str = "/sitemaps/news.xml";

fn collect(
    site: &SeoSite,
    pages: &[SeoPage],
    indexnow: Option<&IndexNowSettings>,
    now: DateTime<Utc>,
) -> Result<Vec<SiteFile>, SeoError> {
    let sitemaps = SitemapSet::from_pages(site, pages)?;
    let mut index = sitemaps.index().clone();
    let mut files = Vec::new();
    for sitemap in sitemaps.files() {
        let path = sitemap.path.as_str();
        files.push(file(
            path.strip_prefix('/').unwrap_or(path),
            sitemap.sitemap.to_xml(),
        ));
    }
    if pages
        .iter()
        .any(|page| page.article_type == ArticleType::NewsArticle)
    {
        let news = NewsSitemap::from_pages(site, pages, now)?;
        let path = UrlPath::try_new(NEWS_SITEMAP)?;
        let lastmod = news.entries().iter().map(|entry| entry.published).max();
        index.push(site.url_for(&site.default_language, &path)?, lastmod);
        files.push(file(NEWS_SITEMAP.trim_start_matches('/'), news.to_xml()));
    }
    files.insert(0, file("sitemap.xml", index.to_xml()));
    files.push(file("robots.txt", RobotsTxt::for_site(site)?.to_string()));
    files.push(file(
        "llms.txt",
        LlmsTxt::for_site(site, pages)?.to_string(),
    ));
    for language in &site.languages {
        let feed = Feed::for_pages(site, pages, &FeedScope::language(language.clone()), now)?;
        let folder = prefix(site, language);
        files.push(file(&format!("{folder}feed.xml"), feed.to_rss()?));
        files.push(file(&format!("{folder}atom.xml"), feed.to_atom()?));
        files.push(file(&format!("{folder}feed.json"), feed.to_json()?));
    }
    if let Some(settings) = indexnow {
        let key = IndexNow::new(site, settings)?.key_file();
        let name = key.path().as_str().trim_start_matches('/');
        files.push(file(name, key.body().to_owned()));
    }
    Ok(files)
}

fn file(name: &str, text: String) -> SiteFile {
    SiteFile {
        file: name.to_owned(),
        text,
    }
}

/// `de/` for a language other than the default, as `SeoSite::url_for`
/// places it, and nothing for the default. Each feed lives next to its home
/// page, where [`Feed::url`] in each format points.
fn prefix(site: &SeoSite, language: &Locale) -> String {
    if language == &site.default_language {
        String::new()
    } else {
        format!("{}/", language.as_str().to_ascii_lowercase())
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use khalid_core::{AbsoluteUrl, Locale};
    use khalid_seo::{Feed, FeedFormat, FeedScope, SeoSite};

    use super::{files, prefix};

    #[test]
    fn each_feed_file_sits_where_the_feed_says_it_lives() {
        let mut site = SeoSite::new(
            "Field Notes on SEO",
            AbsoluteUrl::parse("https://www.example.com").unwrap(),
            Locale::try_new("en").unwrap(),
        );
        site.languages.push(Locale::try_new("de").unwrap());
        let now = Utc.with_ymd_and_hms(2026, 10, 26, 12, 0, 0).unwrap();
        let written = files(&site, &[], None, now).unwrap();
        for language in &site.languages {
            let feed =
                Feed::for_pages(&site, &[], &FeedScope::language(language.clone()), now).unwrap();
            for (format, name) in [
                (FeedFormat::Rss, "feed.xml"),
                (FeedFormat::Atom, "atom.xml"),
                (FeedFormat::Json, "feed.json"),
            ] {
                let file = format!("{}{name}", prefix(&site, language));
                let url = format!("https://www.example.com/{file}");
                assert_eq!(feed.url(format).unwrap().as_str(), url);
                assert!(written.iter().any(|written| written.file == file), "{file}");
            }
        }
    }
}
