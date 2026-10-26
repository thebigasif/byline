//! The files `khalid build` writes beside the pages, from `khalid-seo`: the
//! sitemap index and its sitemaps, `robots.txt`, `llms.txt`, and the RSS,
//! Atom and JSON feeds of each language (KH-SEO-5, KH-SEO-6, KH-SEO-7).

use chrono::{DateTime, Utc};
use khalid_core::{Locale, UserMessage};
use khalid_seo::{Feed, FeedScope, LlmsTxt, RobotsTxt, SeoError, SeoPage, SeoSite, SitemapSet};

use crate::build_message;

/// One file and the place it goes, relative to the output folder.
#[derive(Debug)]
pub(crate) struct SiteFile {
    pub(crate) file: String,
    pub(crate) text: String,
}

/// Every file of `site` beside its `pages`, whose language versions are
/// already linked. Each lives in the mount folder, where its address
/// points. `now` dates a feed that has no posts yet.
pub(crate) fn files(
    site: &SeoSite,
    pages: &[SeoPage],
    now: DateTime<Utc>,
) -> Result<Vec<SiteFile>, UserMessage> {
    collect(site, pages, now).map_err(|error| build_message::seo(&error))
}

fn collect(
    site: &SeoSite,
    pages: &[SeoPage],
    now: DateTime<Utc>,
) -> Result<Vec<SiteFile>, SeoError> {
    let sitemaps = SitemapSet::from_pages(site, pages)?;
    let mut files = vec![file("sitemap.xml", sitemaps.index().to_xml())];
    for sitemap in sitemaps.files() {
        let path = sitemap.path.as_str();
        files.push(file(
            path.strip_prefix('/').unwrap_or(path),
            sitemap.sitemap.to_xml(),
        ));
    }
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
        let written = files(&site, &[], now).unwrap();
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
