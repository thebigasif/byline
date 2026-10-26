//! The files `khalid build` writes beside the pages, from `khalid-seo`: the
//! sitemap index and its sitemaps, `robots.txt` and `llms.txt` (KH-SEO-5,
//! KH-SEO-7).

use khalid_core::UserMessage;
use khalid_seo::{LlmsTxt, RobotsTxt, SeoError, SeoPage, SeoSite, SitemapSet};

use crate::build_message;

/// One file and the place it goes, relative to the output folder.
#[derive(Debug)]
pub(crate) struct SiteFile {
    pub(crate) file: String,
    pub(crate) text: String,
}

/// Every file of `site` beside its `pages`, whose language versions are
/// already linked. Each lives in the mount folder, where its address points.
pub(crate) fn files(site: &SeoSite, pages: &[SeoPage]) -> Result<Vec<SiteFile>, UserMessage> {
    collect(site, pages).map_err(|error| build_message::seo(&error))
}

fn collect(site: &SeoSite, pages: &[SeoPage]) -> Result<Vec<SiteFile>, SeoError> {
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
    Ok(files)
}

fn file(name: &str, text: String) -> SiteFile {
    SiteFile {
        file: name.to_owned(),
        text,
    }
}
