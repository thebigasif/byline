//! The pages `khalid serve` answers with: the ones `khalid build` writes,
//! rendered into memory instead of files, with the site's files and moved
//! addresses. A headless site has the files and moves and no HTML page.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::{fmt, fs};

use khalid_content::{ContentTree, Document, SiteContent, check_bodies, parse_post, read_content};
use khalid_core::{Clock, Config, SystemClock, UserMessage};
use khalid_render::{PageBody, Renderer};
use khalid_routes::Routes;
use khalid_server::{MediaType, Moved, Page, PagePath, Render, Rendered};

use crate::build_pages::LanguagePages;
use crate::content_feed::ContentFeed;
use crate::site_content::Catalog;
use crate::site_files::HEADLESS_NOT_FOUND;
use crate::{
    assets, build_message, build_pages, build_seo, content_message, post_files, post_images,
    search_page, site_files,
};

/// Renders every page of one site from its files. Cloning shares it.
///
/// The settings, the routes and the theme, with the stylesheet built from
/// it, are read once, when `khalid serve` starts. The content is read again
/// on each render, so an edited post shows.
#[derive(Debug, Clone)]
pub(crate) struct SitePages(Arc<Site>);

#[derive(Debug)]
struct Site {
    folder: PathBuf,
    config: Config,
    routes: Routes,
    /// `None` on a headless site, whose theme is never read.
    theme: Option<Theme>,
    feed: Option<ContentFeed>,
    /// The posts the last full render read, without bodies, and its
    /// authors, so a saved post's page can be rendered alone.
    outline: Mutex<SiteContent>,
    /// The content files the last full render listed, so a saved post's
    /// page finds the files beside it without listing the folder again.
    /// `None` before the first full render.
    tree: Mutex<Option<ContentTree>>,
}

/// The theme's renderer, and the stylesheet every page links.
#[derive(Debug)]
struct Theme {
    renderer: Renderer,
    stylesheet: Page,
}

/// One render of the site: what [`Rendered`] holds, kept apart so that
/// `khalid serve` can answer before compressing.
#[derive(Debug)]
pub(crate) struct Served {
    pub(crate) pages: Vec<Page>,
    pub(crate) not_found: String,
    pub(crate) moves: Vec<Moved>,
}

impl SitePages {
    /// The pages of the site in `folder`, with its checked `config` and
    /// `routes`. With a `feed`, each render also hands the Content API what
    /// it read.
    pub(crate) fn new(
        folder: &Path,
        config: Config,
        routes: Routes,
        feed: Option<ContentFeed>,
    ) -> Result<Self, UserMessage> {
        let theme = if config.mode().has_pages() {
            let renderer =
                Renderer::with_theme(folder).map_err(|error| build_message::render(&error))?;
            let css = assets::site_stylesheet(folder)?;
            let stylesheet = assets::stylesheet_page(&config, &css)?;
            Some(Theme {
                renderer,
                stylesheet,
            })
        } else {
            None
        };
        Ok(Self(Arc::new(Site {
            folder: folder.to_path_buf(),
            config,
            routes,
            theme,
            feed,
            outline: Mutex::default(),
            tree: Mutex::default(),
        })))
    }

    /// Every page of the site at its address, the stylesheet they link, the
    /// site's files, the not-found page of the default language and the
    /// moved addresses, read from the folder. Checks every body again,
    /// as `khalid --site` does at start, so a saved edit with a broken
    /// wikilink is refused with its file and line, and the pages already
    /// served stay. Reads files and renders,
    /// so it runs off the async runtime (RULES.md BL-RUN-7).
    pub(crate) fn render_all(&self) -> Result<Served, UserMessage> {
        let Site {
            folder,
            config,
            routes,
            ..
        } = &*self.0;
        let tree = self.tree()?;
        let content = read_content(&tree)
            .and_then(|content| check_bodies(&content, config, routes).map(|()| content))
            .map_err(|error| content_message::of(folder, &error))?;
        self.render_from(&tree, &content)
    }

    /// Every page made from `content`, already read from the site folder,
    /// with the site's files, the not-found page and the moved addresses,
    /// then hands the feed its catalog when there is one. At start, `khalid
    /// serve` passes the content it checked, so the posts are read once, not
    /// twice. The folder is only listed again, for the files beside folder
    /// posts. `content` must have passed `check_bodies`.
    pub(crate) fn render_content(&self, content: &SiteContent) -> Result<Served, UserMessage> {
        self.render_from(&self.tree()?, content)
    }

    /// The list of the site's content files, as found on disk now.
    fn tree(&self) -> Result<ContentTree, UserMessage> {
        let folder = &self.0.folder;
        ContentTree::walk(folder).map_err(|error| content_message::of(folder, &error))
    }

    /// Every page made from `content`, with the files `tree` holds beside
    /// folder posts, the site's files, the not-found page and the moved
    /// addresses. Only a full render hands the feed its catalog, so the
    /// Content API never sees part of the site.
    fn render_from(
        &self,
        tree: &ContentTree,
        content: &SiteContent,
    ) -> Result<Served, UserMessage> {
        let Site {
            folder,
            config,
            feed,
            outline,
            tree: listed,
            ..
        } = &*self.0;
        // The catalog holds no post body, so it is quick to make here, and
        // the Content API's store is built from it in the background.
        let catalog = match feed {
            Some(_) => Some(Catalog::of(folder, config, content)?),
            None => None,
        };
        let rendered = self.render_of(tree, content, Which::All)?;
        *outline.lock().unwrap_or_else(PoisonError::into_inner) = outline_of(content);
        *listed.lock().unwrap_or_else(PoisonError::into_inner) = Some(tree.clone());
        if let (Some(feed), Some(catalog)) = (feed, catalog) {
            feed.send(catalog);
        }
        Ok(rendered)
    }

    /// The `which` pages made from `content`, with the files `tree` holds
    /// beside folder posts, the site's files, the moved addresses and the
    /// not-found page of the default language when `which` is
    /// [`Which::All`]. Only a full render serves those files; a saved
    /// post's page still points its images at them. A headless site has no
    /// HTML page, as `khalid build` writes none.
    fn render_of(
        &self,
        tree: &ContentTree,
        content: &SiteContent,
        which: Which,
    ) -> Result<Served, UserMessage> {
        let Site {
            folder,
            config,
            routes,
            theme,
            ..
        } = &*self.0;
        let href = theme
            .as_ref()
            .map_or("", |theme| theme.stylesheet.path().as_str());
        let mut languages = build_pages::pages(config, routes, content, href)?;
        if let Some(theme) = theme {
            search_page::add(config, &theme.renderer, &mut languages)?;
        }
        let beside = files(folder, config, tree, &mut languages)?;
        let seo_site = build_seo::site(config)?;
        let seo_pages = build_seo::heads(&seo_site, &mut languages)?;
        let mut served = Served {
            pages: Vec::new(),
            not_found: HEADLESS_NOT_FOUND.to_owned(),
            moves: Vec::new(),
        };
        if which == Which::All {
            served.pages = site_files::files(config, &seo_site, &seo_pages, SystemClock.now())?;
            served.pages.extend(beside);
            served.moves = site_files::moves(config, content)?;
        }
        if let Some(theme) = theme {
            if which == Which::All {
                served.pages.push(theme.stylesheet.clone());
            }
            served.not_found = html_pages(theme, config, &languages, which, &mut served.pages)?;
        }
        Ok(served)
    }

    /// The saved `files`, relative to the site folder, read again as posts,
    /// with every author their pages may name. `None` when one of them is
    /// not a post the last full render read, or cannot be read now: the
    /// full render then shows it, or says what is wrong.
    ///
    /// Also `None` when a post's page would read more than these files, so
    /// it could differ from the full render's: when the pages show related
    /// posts, chosen from every post's links, or when a saved post or an
    /// author's bio has a wikilink, which may name a page outside them.
    fn saved_posts(&self, files: &[PathBuf]) -> Option<SiteContent> {
        if self.0.config.reader().related_posts {
            return None;
        }
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
        let bodies = saved.posts.iter().map(|doc| &doc.body);
        let mut bodies = bodies.chain(saved.authors.iter().map(|doc| &doc.body));
        if bodies.any(|body| body.contains(WIKILINK)) {
            return None;
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

/// How a wikilink starts in markdown, such as `[[link-equity]]`.
const WIKILINK: &str = "[[";

/// The posts of `content` without their bodies, and its authors whole,
/// since a post's author box shows their bios: what a saved post's page
/// needs from the rest of the site, in a fraction of the memory the whole
/// content takes.
fn outline_of(content: &SiteContent) -> SiteContent {
    let mut outline = SiteContent::default();
    outline.posts = content.posts.iter().map(without_body).collect();
    outline.authors.clone_from(&content.authors);
    outline
}

fn without_body<F: Clone>(doc: &Document<F>) -> Document<F> {
    let mut doc = doc.clone();
    doc.body = String::new();
    doc
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

/// Renders the `which` pages of `languages` with `theme` into `pages`, and
/// returns the not-found page of the default language, empty unless
/// `which` is [`Which::All`].
fn html_pages(
    theme: &Theme,
    config: &Config,
    languages: &[LanguagePages],
    which: Which,
    pages: &mut Vec<Page>,
) -> Result<String, UserMessage> {
    let mut not_found = String::new();
    for language in languages {
        for page in &language.pages {
            if which == Which::Posts && !matches!(page.body, PageBody::Post(_)) {
                continue;
            }
            let html = theme
                .renderer
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
    Ok(not_found)
}

impl Render for SitePages {
    type Error = Unrendered;

    fn render(&self) -> Result<Rendered, Unrendered> {
        let served = self.render_all().map_err(Unrendered)?;
        Ok(Rendered::new(served.pages, served.not_found).with_moves(served.moves))
    }

    /// A saved post's page alone: its own file read again, with the authors
    /// and the files beside it as the last full render read them. Only
    /// when nothing else can reach the page (see `saved_posts`), so it is
    /// the page the full render will make.
    fn render_saved(&self, files: &[PathBuf]) -> Result<Option<Vec<Page>>, Unrendered> {
        let listed = self
            .0
            .tree
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        let (Some(tree), Some(saved)) = (listed, self.saved_posts(files)) else {
            return Ok(None);
        };
        let served = self
            .render_of(&tree, &saved, Which::Posts)
            .map_err(Unrendered)?;
        Ok(Some(served.pages))
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
    use std::fs;
    use std::path::{Path, PathBuf};

    use khalid_core::{Config, Environment};
    use khalid_routes::Routes;

    use super::{ContentTree, Render, SitePages, read_content};

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
        let pages = pages(&fixture("demo-site")).render_all().unwrap().pages;
        let link = "<a href=\"/blog/glossary/topical-authority/\">Topical authority</a>";
        let linking = pages
            .iter()
            .filter(|page| String::from_utf8_lossy(page.body()).contains(link));
        assert_eq!(linking.count(), 2);
    }

    #[test]
    fn the_checked_content_renders_the_same_pages_as_the_folder() {
        let demo = fixture("demo-site");
        let content = read_content(&ContentTree::walk(&demo).unwrap()).unwrap();
        let source = pages(&demo);
        let given = source.render_content(&content).unwrap();
        let read = source.render_all().unwrap();
        assert!(given.pages.len() > 10, "{} pages", given.pages.len());
        assert_eq!(given.pages, read.pages);
        assert_eq!(given.not_found, read.not_found);
        assert_eq!(given.moves, read.moves);
    }

    fn copy_dir(from: &Path, into: &Path) {
        fs::create_dir_all(into).unwrap();
        for entry in fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            let mut target = into.to_path_buf();
            target.push(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_dir(&entry.path(), &target);
            } else {
                fs::copy(entry.path(), &target).unwrap();
            }
        }
    }

    /// The paths of the posts the last full render of `source` read.
    fn outlined(source: &SitePages) -> Vec<PathBuf> {
        let outline = source.0.outline.lock().unwrap();
        outline.posts.iter().map(|doc| doc.path.clone()).collect()
    }

    #[test]
    fn a_saved_post_alone_renders_the_page_the_full_render_makes() {
        let site = tempfile::tempdir().unwrap();
        copy_dir(&fixture("demo-site"), site.path());
        let mut settings = site.path().to_path_buf();
        settings.push("site.toml");
        let text = fs::read_to_string(&settings).unwrap();
        fs::write(
            &settings,
            format!("{text}\n[reader]\nrelated_posts = false\n"),
        )
        .unwrap();
        let source = pages(site.path());
        let unread = vec![PathBuf::from(
            "content/en/posts/2026-09-28-what-a-sitemap-does.md",
        )];
        assert_eq!(
            source.render_saved(&unread).unwrap(),
            None,
            "no full render yet"
        );

        let all = source.render_all().unwrap().pages;
        let (linking, posts): (Vec<PathBuf>, Vec<PathBuf>) =
            outlined(&source).into_iter().partition(|path| {
                let mut file = site.path().to_path_buf();
                file.push(path);
                fs::read_to_string(file).unwrap().contains("[[")
            });
        assert!(posts.len() >= 3, "{posts:?}");
        let saved = source.render_saved(&posts).unwrap().unwrap();
        assert_eq!(saved.len(), posts.len(), "one page per dated post");
        for page in &saved {
            assert!(all.contains(page), "{} differs", page.path().as_str());
        }
        assert_eq!(linking.len(), 4, "{linking:?}");
        assert_eq!(source.render_saved(&linking).unwrap(), None, "a wikilink");
        let author = vec![PathBuf::from("content/en/authors/asif.md")];
        assert_eq!(source.render_saved(&author).unwrap(), None);
    }

    #[test]
    fn a_saved_post_waits_for_the_full_render_when_posts_show_related_ones() {
        let source = pages(&fixture("demo-site"));
        source.render_all().unwrap();
        let posts = outlined(&source);
        assert_eq!(posts.len(), 10, "{posts:?}");
        assert_eq!(source.render_saved(&posts).unwrap(), None);
    }
}
