//! The pages `khalid serve` answers with: the ones `khalid build` writes,
//! rendered into memory instead of files.

use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use khalid_content::{ContentTree, Document, SiteContent, parse_post, read_content};
use khalid_core::{Config, UserMessage};
use khalid_db::{Database, SiteId};
use khalid_render::{PageBody, Renderer};
use khalid_server::{MediaType, Page, PagePath, Render, Rendered};
use tokio::runtime::Handle;

use crate::site_content::{LiveContent, published};
use crate::{assets, build_message, build_pages, content_message};

/// Renders every page of one site from its files. Cloning shares it.
///
/// The settings and the theme, with the stylesheet built from it, are read
/// once, when `khalid serve` starts. The content is read again on each
/// render, so an edited post shows.
#[derive(Debug, Clone)]
pub(crate) struct SitePages(Arc<Site>);

#[derive(Debug)]
struct Site {
    folder: PathBuf,
    config: Config,
    renderer: Renderer,
    stylesheet: Page,
    feed: Option<ContentFeed>,
    /// The posts and authors the last full render read, without bodies, so
    /// a saved post's page can be rendered alone.
    outline: Mutex<SiteContent>,
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
        let css = assets::site_stylesheet(folder)?;
        let stylesheet = assets::stylesheet_page(&config, &css)?;
        Ok(Self(Arc::new(Site {
            folder: folder.to_path_buf(),
            config,
            renderer,
            stylesheet,
            feed,
            outline: Mutex::default(),
        })))
    }

    /// Every page of the site at its address, the stylesheet they link, and
    /// the not-found page of the default language. Reads files and renders,
    /// so it runs off the async runtime (RULES.md BL-RUN-7).
    pub(crate) fn render_all(&self) -> Result<(Vec<Page>, String), UserMessage> {
        let folder = &self.0.folder;
        let content = ContentTree::walk(folder)
            .and_then(|tree| read_content(&tree))
            .map_err(|error| content_message::of(folder, &error))?;
        self.render_content(&content)
    }

    /// Every page made from `content`, already read from the site folder,
    /// and the not-found page, refreshing the feed first when there is one.
    /// At start, `khalid serve` passes the content it checked, so the folder
    /// is read once, not twice.
    pub(crate) fn render_content(
        &self,
        content: &SiteContent,
    ) -> Result<(Vec<Page>, String), UserMessage> {
        let Site {
            folder,
            config,
            feed,
            outline,
            ..
        } = &*self.0;
        if let Some(feed) = feed {
            let fed = published(&feed.db, feed.site, folder, config, content);
            feed.live.replace(feed.runtime.block_on(fed)?);
        }
        let rendered = self.render_of(content, Which::All)?;
        *outline.lock().unwrap_or_else(PoisonError::into_inner) = outline_of(content);
        Ok(rendered)
    }

    /// The `which` pages made from `content`, and the not-found page of the
    /// default language when `which` is [`Which::All`].
    fn render_of(
        &self,
        content: &SiteContent,
        which: Which,
    ) -> Result<(Vec<Page>, String), UserMessage> {
        let Site {
            config,
            renderer,
            stylesheet,
            ..
        } = &*self.0;
        let languages = build_pages::pages(config, content, stylesheet.path().as_str())?;
        let mut pages = match which {
            Which::All => vec![stylesheet.clone()],
            Which::Posts => Vec::new(),
        };
        let mut not_found = String::new();
        for language in &languages {
            for page in &language.pages {
                if which == Which::Posts && !matches!(page.body, PageBody::Post(_)) {
                    continue;
                }
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

    /// The saved `files`, relative to the site folder, read again as posts,
    /// with every author their pages may name. `None` when one of them is
    /// not a post the last full render read, or cannot be read now: the
    /// full render then shows it, or says what is wrong.
    fn saved_posts(&self, files: &[PathBuf]) -> Option<SiteContent> {
        let mut saved = SiteContent::default();
        {
            let outline = self
                .0
                .outline
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            for file in files {
                saved
                    .posts
                    .push(outline.posts.iter().find(|doc| &doc.path == file)?.clone());
            }
            saved.authors.clone_from(&outline.authors);
        }
        for doc in &mut saved.posts {
            let mut path = self.0.folder.clone();
            path.push(&doc.path);
            let text = fs::read_to_string(&path).ok()?;
            let post = parse_post(&doc.path, &text).ok()?;
            doc.front = post.front;
            doc.body = post.body;
        }
        Some(saved)
    }
}

/// Which pages a render makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Which {
    /// Every page, the stylesheet and the not-found page.
    All,
    /// Only the page of each dated post.
    Posts,
}

/// The posts and authors of `content` without their bodies: what a saved
/// post's page needs from the rest of the site, in a fraction of the
/// memory the whole content takes.
fn outline_of(content: &SiteContent) -> SiteContent {
    let mut outline = SiteContent::default();
    outline.posts = content.posts.iter().map(without_body).collect();
    outline.authors = content.authors.iter().map(without_body).collect();
    outline
}

fn without_body<F: Clone>(doc: &Document<F>) -> Document<F> {
    let mut doc = doc.clone();
    doc.body = String::new();
    doc
}

impl Render for SitePages {
    type Error = Unrendered;

    fn render(&self) -> Result<Rendered, Unrendered> {
        let (pages, not_found) = self.render_all().map_err(Unrendered)?;
        Ok(Rendered::new(pages, not_found))
    }

    /// A saved post's page alone: its own file read again, and the authors
    /// as the last full render read them. Nothing else on a post's page
    /// comes from other files, so it is the page the full render will make.
    fn render_saved(&self, files: &[PathBuf]) -> Result<Option<Vec<Page>>, Unrendered> {
        let Some(saved) = self.saved_posts(files) else {
            return Ok(None);
        };
        let (pages, _) = self.render_of(&saved, Which::Posts).map_err(Unrendered)?;
        Ok(Some(pages))
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
    use khalid_core::Environment;

    use super::*;

    #[test]
    fn the_checked_content_renders_the_same_pages_as_the_folder() {
        let mut demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        demo.push("../../fixtures/demo-site");
        let config = Config::load(&demo, &Environment::default()).unwrap();
        let content = read_content(&ContentTree::walk(&demo).unwrap()).unwrap();
        let source = SitePages::new(&demo, config, None).unwrap();
        let (given, given_missing) = source.render_content(&content).unwrap();
        let (read, read_missing) = source.render_all().unwrap();
        assert!(given.len() > 10, "{} pages", given.len());
        assert_eq!(given, read);
        assert_eq!(given_missing, read_missing);
    }

    #[test]
    fn a_saved_post_alone_renders_the_page_the_full_render_makes() {
        let mut demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        demo.push("../../fixtures/demo-site");
        let config = Config::load(&demo, &Environment::default()).unwrap();
        let source = SitePages::new(&demo, config, None).unwrap();
        let unread = vec![PathBuf::from(
            "content/en/posts/2026-09-28-what-a-sitemap-does.md",
        )];
        assert_eq!(
            source.render_saved(&unread).unwrap(),
            None,
            "no full render yet"
        );

        let (all, _) = source.render_all().unwrap();
        let posts: Vec<PathBuf> = source
            .0
            .outline
            .lock()
            .unwrap()
            .posts
            .iter()
            .map(|doc| doc.path.clone())
            .collect();
        assert!(posts.len() >= 5, "{posts:?}");
        let saved = source.render_saved(&posts).unwrap().unwrap();
        assert_eq!(saved.len(), posts.len(), "one page per dated post");
        for page in &saved {
            assert!(all.contains(page), "{} differs", page.path().as_str());
        }
        let author = vec![PathBuf::from("content/en/authors/asif.md")];
        assert_eq!(source.render_saved(&author).unwrap(), None);
    }
}
