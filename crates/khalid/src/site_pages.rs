//! The pages `khalid serve` answers with: the ones `khalid build` writes,
//! rendered into memory instead of files.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{fmt, fs};

use khalid_content::{ContentTree, read_content};
use khalid_core::{Config, UserMessage};
use khalid_db::{Database, SiteId};
use khalid_render::{PageBody, Renderer};
use khalid_server::{MediaType, Page, PagePath, Render, Rendered};
use tokio::runtime::Handle;

use crate::build_pages::LanguagePages;
use crate::site_content::{LiveContent, published};
use crate::{build_message, build_pages, content_message, post_files, post_images};

/// Renders every page of one site from its files. Cloning shares it.
///
/// The settings and the theme are read once, when `khalid serve` starts.
/// The content is read again on each render, so an edited post shows.
#[derive(Debug, Clone)]
pub(crate) struct SitePages(Arc<Site>);

#[derive(Debug)]
struct Site {
    folder: PathBuf,
    config: Config,
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
    /// The pages of the site in `folder`, with its checked `config`. With
    /// a `feed`, each render also refreshes the index and the Content API.
    pub(crate) fn new(
        folder: &Path,
        config: Config,
        feed: Option<ContentFeed>,
    ) -> Result<Self, UserMessage> {
        let renderer =
            Renderer::with_theme(folder).map_err(|error| build_message::render(&error))?;
        Ok(Self(Arc::new(Site {
            folder: folder.to_path_buf(),
            config,
            renderer,
            feed,
        })))
    }

    /// Every page of the site at its address, and the not-found page of the
    /// default language. Reads files and renders, so it runs off the async
    /// runtime (RULES.md BL-RUN-7).
    pub(crate) fn render_all(&self) -> Result<(Vec<Page>, String), UserMessage> {
        let Site {
            folder,
            config,
            renderer,
            feed,
        } = &*self.0;
        let tree =
            ContentTree::walk(folder).map_err(|error| content_message::of(folder, &error))?;
        let content = read_content(&tree).map_err(|error| content_message::of(folder, &error))?;
        if let Some(feed) = feed {
            let fed = published(&feed.db, feed.site, folder, config, &content);
            feed.live.replace(feed.runtime.block_on(fed)?);
        }
        let mut languages = build_pages::pages(config, &content)?;
        let mut pages = files(folder, config, &tree, &mut languages)?;
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

/// The files beside folder posts in `tree`, each at its page's address,
/// and the sizes of their images, which never change under their names.
fn files(
    folder: &Path,
    config: &Config,
    tree: &ContentTree,
    languages: &mut [LanguagePages],
) -> Result<Vec<Page>, UserMessage> {
    let address = |text: &str| PagePath::try_new(text).map_err(|_| build_message::address(text));
    let beside = post_files::attach(tree, languages);
    let mut pages = Vec::with_capacity(beside.len());
    for file in &beside {
        let kind = MediaType::for_file(&file.address);
        let bytes = post_files::read(folder, file)?;
        pages.push(Page::new(address(&file.address)?, kind, bytes));
    }
    for size in post_images::attach(folder, config, &beside, languages)? {
        let kind = MediaType::for_file(&size.address);
        let bytes =
            fs::read(&size.path).map_err(|error| post_files::unreadable(&size.path, &error))?;
        pages.push(Page::immutable(address(&size.address)?, kind, bytes));
    }
    Ok(pages)
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
