//! The pages `khalid serve` answers with: the ones `khalid build` writes,
//! rendered into memory instead of files.

use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use khalid_content::{ContentTree, check_bodies, read_content};
use khalid_core::{Config, UserMessage};
use khalid_db::{Database, SiteId};
use khalid_render::{PageBody, Renderer};
use khalid_routes::Routes;
use khalid_server::{MediaType, Page, PagePath, Render, Rendered};
use tokio::runtime::Handle;

use crate::site_content::{LiveContent, published};
use crate::{build_message, build_pages, content_message};

/// Renders every page of one site from its files. Cloning shares it.
///
/// The settings, the routes and the theme are read once, when `khalid serve`
/// starts. The content is read again on each render, so an edited post shows.
#[derive(Debug, Clone)]
pub(crate) struct SitePages(Arc<Site>);

#[derive(Debug)]
struct Site {
    folder: PathBuf,
    config: Config,
    routes: Routes,
    renderer: Renderer,
    feed: Option<ContentFeed>,
}

/// Where each render also sends the published posts: the content index in
/// the site's database, and the posts the Content API answers with.
#[derive(Debug)]
pub(crate) struct ContentFeed {
    /// The runtime the database belongs to. Renders run off it, on the
    /// main thread at start and on a blocking thread after, so they wait
    /// for the database through it.
    pub(crate) runtime: Handle,
    pub(crate) db: Database,
    pub(crate) site: SiteId,
    pub(crate) live: LiveContent,
}

impl SitePages {
    /// The pages of the site in `folder`, with its checked `config` and
    /// `routes`. With a `feed`, each render also refreshes the index and the
    /// Content API.
    pub(crate) fn new(
        folder: &Path,
        config: Config,
        routes: Routes,
        feed: Option<ContentFeed>,
    ) -> Result<Self, UserMessage> {
        let renderer =
            Renderer::with_theme(folder).map_err(|error| build_message::render(&error))?;
        Ok(Self(Arc::new(Site {
            folder: folder.to_path_buf(),
            config,
            routes,
            renderer,
            feed,
        })))
    }

    /// Every page of the site at its address, and the not-found page of the
    /// default language. Checks every body again, as `khalid --site` does at
    /// start, so a saved edit with a broken wikilink is refused with its file
    /// and line, and the pages already served stay. Reads files and renders,
    /// so it runs off the async runtime (RULES.md BL-RUN-7).
    pub(crate) fn render_all(&self) -> Result<(Vec<Page>, String), UserMessage> {
        let Site {
            folder,
            config,
            routes,
            renderer,
            feed,
        } = &*self.0;
        let content = ContentTree::walk(folder)
            .and_then(|tree| read_content(&tree))
            .and_then(|content| check_bodies(&content, config, routes).map(|()| content))
            .map_err(|error| content_message::of(folder, &error))?;
        if let Some(feed) = feed {
            let fed = published(&feed.db, feed.site, folder, config, &content);
            feed.live.replace(feed.runtime.block_on(fed)?);
        }
        let languages = build_pages::pages(config, routes, &content)?;
        let mut pages = Vec::new();
        let mut not_found = String::new();
        for language in &languages {
            for page in &language.pages {
                let html = renderer
                    .render(&language.site, &page.meta, &page.body)
                    .map_err(|error| build_message::render(&error))?;
                if matches!(page.body, PageBody::NotFound) {
                    if &language.site.locale == config.default_language() {
                        not_found = html;
                    }
                    continue;
                }
                let path = PagePath::try_new(&page.meta.path)
                    .map_err(|_| build_message::address(&page.meta.path))?;
                pages.push(Page::new(path, MediaType::Html, html));
            }
        }
        Ok((pages, not_found))
    }
}

impl Render for SitePages {
    type Error = Unrendered;

    fn render(&self) -> Result<Rendered, Unrendered> {
        let (pages, not_found) = self.render_all().map_err(Unrendered)?;
        Ok(Rendered::new(pages, not_found))
    }
}

/// Why the site could not be rendered after an edit, as a person reads it.
#[derive(Debug)]
pub(crate) struct Unrendered(UserMessage);

impl fmt::Display for Unrendered {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Error for Unrendered {}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use khalid_core::{Config, Environment};
    use khalid_routes::Routes;

    use super::SitePages;

    fn fixture(name: &str) -> PathBuf {
        let mut path = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
        path.push("../../fixtures");
        path.push(name);
        path
    }

    fn pages(site: &Path) -> SitePages {
        let config = Config::load(site, &Environment::empty()).unwrap();
        SitePages::new(site, config, Routes::load(site).unwrap(), None).unwrap()
    }

    #[test]
    fn a_saved_broken_wikilink_stops_the_render_and_names_its_file_and_line() {
        let site = fixture("invalid/content-broken-wikilink");
        let message = pages(&site).render_all().unwrap_err();
        let what = message.what();
        assert!(
            what.ends_with("why-links-matter.md, line 10: [[link-equity]] matches no post, page or glossary term in en."),
            "{what}"
        );
    }

    #[test]
    fn served_pages_link_each_wikilink() {
        let (pages, _) = pages(&fixture("demo-site")).render_all().unwrap();
        let link = "<a href=\"/blog/glossary/topical-authority/\">Topical authority</a>";
        let linking = pages
            .iter()
            .filter(|page| String::from_utf8_lossy(page.body()).contains(link));
        assert_eq!(linking.count(), 2);
    }
}
