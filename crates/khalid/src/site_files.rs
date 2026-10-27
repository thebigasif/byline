//! What `khalid serve` answers beside the pages: the sitemaps, `robots.txt`,
//! `llms.txt` and feeds `khalid build` writes, at the same addresses, and
//! the post aliases its `_redirects` lists, answered with 301 (KH-SEO-5 to
//! KH-SEO-8, KH-INS-5).

use chrono::{DateTime, Utc};
use khalid_content::SiteContent;
use khalid_core::{Config, UserMessage};
use khalid_seo::{SeoPage, SeoSite};
use khalid_server::{MediaType, Moved, Page, PagePath};

use crate::{build_files, build_message, build_redirects};

/// Every file of `site` beside its `pages`, at the address its build file
/// has under the mount folder. `now` dates a feed that has no posts yet.
pub(crate) fn files(
    config: &Config,
    site: &SeoSite,
    pages: &[SeoPage],
    now: DateTime<Utc>,
) -> Result<Vec<Page>, UserMessage> {
    build_files::files(site, pages, now)?
        .into_iter()
        .map(|file| {
            let path = page_path(&format!("{}{}", config.mount(), file.file))?;
            Ok(Page::new(path, media_type(&file.file), file.text))
        })
        .collect()
}

/// Each alias of a dated post, moved to the post's address.
pub(crate) fn moves(config: &Config, content: &SiteContent) -> Result<Vec<Moved>, UserMessage> {
    build_redirects::hops(config, content)?
        .into_iter()
        .map(|hop| Ok(Moved::new(page_path(&hop.from)?, page_path(&hop.to)?)))
        .collect()
}

/// The media type of a file `build_files` makes, by its extension.
fn media_type(file: &str) -> MediaType {
    match file.rsplit_once('.').map(|(_, extension)| extension) {
        Some("json") => MediaType::Json,
        Some("txt") => MediaType::PlainText,
        _ => MediaType::Xml,
    }
}

fn page_path(text: &str) -> Result<PagePath, UserMessage> {
    PagePath::try_new(text).map_err(|_| build_message::address(text))
}

#[cfg(test)]
mod tests {
    use khalid_server::MediaType;

    use super::media_type;

    #[test]
    fn each_file_kind_has_its_media_type() {
        assert_eq!(media_type("sitemap-posts-1.xml"), MediaType::Xml);
        assert_eq!(media_type("de/feed.json"), MediaType::Json);
        assert_eq!(media_type("robots.txt"), MediaType::PlainText);
        assert_eq!(media_type("de/atom.xml"), MediaType::Xml);
    }
}
