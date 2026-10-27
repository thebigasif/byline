//! The pages `khalid serve` answers with: the ones `khalid build` writes,
//! rendered into memory instead of files, with the site's files and moved
//! addresses. A headless site has the files and moves and no HTML page.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::{fmt, fs};

use khalid_content::{
    ContentTree, Document, PostFront, SiteContent, check_bodies, parse_post, read_content,
};
use khalid_core::{Clock, Config, SystemClock, UserMessage};
use khalid_render::{PageBody, Renderer};
use khalid_routes::Routes;
use khalid_server::{MediaType, Moved, Page, PagePath, Render, Rendered};

use crate::build_pages::LanguagePages;
use crate::content_feed::ContentFeed;
use crate::listed_post::ListedPost;
use crate::post_files::Assets;
use crate::site_content::Catalog;
use crate::site_files::HEADLESS_NOT_FOUND;
use crate::{
    assets, build_home, build_message, build_pages, build_seo, content_message, post_files,
    site_files,
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
    /// What the last full render read of the site, so a saved post's page
    /// can be rendered alone.
    outline: Mutex<Outline>,
    /// The files beside folder posts as the last full render found them, so
    /// a saved post's page finds them without listing the folder again.
    /// `None` before the first full render.
    assets: Mutex<Option<Assets>>,
}

/// What a saved post's page needs from the rest of the site, in a fraction
/// of the memory the whole content takes.
#[derive(Debug, Default)]
struct Outline {
    /// The posts without their bodies, and the authors whole, since a
    /// post's author box shows their bios.
    content: SiteContent,
    /// Every dated post, which related posts are chosen from.
    listed: Vec<ListedPost>,
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
            assets: Mutex::default(),
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
        let (content, assets) = ContentTree::walk(folder)
            .and_then(|tree| Ok((read_content(&tree)?, Assets::of(&tree))))
            .and_then(|read| check_bodies(&read.0, config, routes).map(|()| read))
            .map_err(|error| content_message::of(folder, &error))?;
        self.render_content(&content, assets)
    }

    /// The first pages `khalid serve` answers with while the rest of the
    /// site renders: the home page of each language, made from the front
    /// matter of `content` without rendering a single body, and the
    /// stylesheet they link. Each is byte for byte the page the full render
    /// makes. A headless site has none.
    pub(crate) fn render_first(&self, content: &SiteContent) -> Result<Vec<Page>, UserMessage> {
        let Site {
            config,
            routes,
            theme,
            ..
        } = &*self.0;
        let Some(theme) = theme else {
            return Ok(Vec::new());
        };
        let href = theme.stylesheet.path().as_str();
        let mut languages = build_home::home_pages(config, routes, content, href)?;
        build_seo::heads(&build_seo::site(config)?, &mut languages)?;
        let mut pages = vec![theme.stylesheet.clone()];
        html_pages(theme, config, &languages, Which::All, &mut pages)?;
        Ok(pages)
    }

    /// Every page made from `content`, already read from the site folder,
    /// with the files `assets` lists beside folder posts, the site's files,
    /// the not-found page and the moved addresses, then hands the feed its
    /// catalog when there is one. At start, `khalid serve` passes what it
    /// checked, so the folder is read once, not twice. `content` must have
    /// passed `check_bodies`. Only a full render hands the feed its
    /// catalog, so the Content API never sees part of the site.
    pub(crate) fn render_content(
        &self,
        content: &SiteContent,
        assets: Assets,
    ) -> Result<Served, UserMessage> {
        let Site {
            folder,
            config,
            feed,
            outline,
            assets: kept,
            ..
        } = &*self.0;
        // The catalog holds no post body, so it is quick to make here, and
        // the Content API's store is built from it in the background.
        let catalog = match feed {
            Some(_) => Some(Catalog::of(folder, config, content)?),
            None => None,
        };
        let (rendered, listed) = self.render_of(&assets, (content, &[]), Which::All)?;
        *outline.lock().unwrap_or_else(PoisonError::into_inner) = Outline {
            content: outline_of(content),
            listed,
        };
        *kept.lock().unwrap_or_else(PoisonError::into_inner) = Some(assets);
        if let (Some(feed), Some(catalog)) = (feed, catalog) {
            feed.send(catalog);
        }
        Ok(rendered)
    }

    /// The `which` pages made from `content`, with the files `assets` lists
    /// beside folder posts, the site's files, the moved addresses and the
    /// not-found page of the default language when `which` is
    /// [`Which::All`], and the dated posts of `content` as lists show them.
    /// Related posts are chosen among those and `others`. Only a full
    /// render serves the files; a saved post's page still points its
    /// images at them. A headless site has no HTML page, as `khalid build`
    /// writes none.
    fn render_of(
        &self,
        assets: &Assets,
        (content, others): (&SiteContent, &[ListedPost]),
        which: Which,
    ) -> Result<(Served, Vec<ListedPost>), UserMessage> {
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
        let mut languages = build_pages::pages(config, routes, (content, others), href)?;
        let beside = post_files::served(folder, config, assets, &mut languages)?;
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
        let listed = languages.into_iter().flat_map(|language| language.listed);
        Ok((served, listed.collect()))
    }

    /// The saved `files`, relative to the site folder, and their other
    /// language versions, read again as posts, with every author their
    /// pages may name, and the dated posts of the last full render, which
    /// their related posts are chosen from. `None` when a saved file is
    /// not a post the last full render read, or one cannot be read now:
    /// the full render then shows it, or says what is wrong.
    ///
    /// Also `None` when a post or an author's bio has a wikilink, which may
    /// name a page outside these, so the page could differ from the full
    /// render's.
    fn saved_posts(&self, files: &[PathBuf]) -> Option<(SiteContent, Vec<ListedPost>)> {
        let outline = self
            .0
            .outline
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let posts = &outline.content.posts;
        let mut saved = SiteContent::default();
        for file in files {
            let mut doc = posts.iter().find(|doc| &doc.path == file)?.clone();
            self.read_again(&mut doc)?;
            saved.posts.push(doc);
        }
        // A page's head links its other language versions, so they are
        // rendered with it.
        let keys: Vec<&String> = saved
            .posts
            .iter()
            .filter_map(|doc| doc.front.translation.as_ref())
            .collect();
        let versions = posts.iter().filter(|doc| {
            let key = doc.front.translation.as_ref();
            key.is_some_and(|key| keys.contains(&key)) && !files.contains(&doc.path)
        });
        let mut versions: Vec<_> = versions.cloned().collect();
        for doc in &mut versions {
            self.read_again(doc)?;
        }
        saved.posts.extend(versions);
        saved.authors.clone_from(&outline.content.authors);
        let bodies = saved.posts.iter().map(|doc| &doc.body);
        let mut bodies = bodies.chain(saved.authors.iter().map(|doc| &doc.body));
        if bodies.any(|body| body.contains(WIKILINK)) {
            return None;
        }
        Some((saved, outline.listed.clone()))
    }

    /// `doc`, a post of the outline, with its front matter and body read
    /// from its file now. `None` when the file cannot be read or parsed.
    fn read_again(&self, doc: &mut Document<PostFront>) -> Option<()> {
        let mut path = self.0.folder.clone();
        path.push(&doc.path);
        let text = fs::read_to_string(&path).ok()?;
        let post = parse_post(&doc.path, &text).ok()?;
        doc.front = post.front;
        doc.body = post.body;
        Some(())
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
        let kept = self
            .0
            .assets
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        let (Some(assets), Some((saved, others))) = (kept, self.saved_posts(files)) else {
            return Ok(None);
        };
        let (served, _) = self
            .render_of(&assets, (&saved, &others), Which::Posts)
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

    use super::{Assets, ContentTree, Render, SitePages, read_content};

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
    fn the_first_pages_are_the_full_renders_home_pages_and_stylesheet() {
        let site = demo_copy();
        let content = read_content(&ContentTree::walk(site.path()).unwrap()).unwrap();
        let source = pages(site.path());
        let first = source.render_first(&content).unwrap();
        let all = source.render_all().unwrap().pages;
        let paths: Vec<&str> = first.iter().map(|page| page.path().as_str()).collect();
        assert_eq!(paths.len(), 3, "{paths:?}");
        assert!(paths.contains(&"/blog/"), "{paths:?}");
        assert!(paths.contains(&"/blog/de/"), "{paths:?}");
        for page in &first {
            assert!(all.contains(page), "{} differs", page.path().as_str());
        }
    }

    #[test]
    fn served_pages_link_each_wikilink() {
        let pages = pages(&fixture("demo-site")).render_all().unwrap().pages;
        let link = "<a href=\"/blog/glossary/topical-authority/\">Topical authority</a>";
        let linking = pages
            .iter()
            .filter(|page| String::from_utf8_lossy(&page.body().unwrap()).contains(link));
        assert_eq!(linking.count(), 2);
    }

    #[test]
    fn the_checked_content_renders_the_same_pages_as_the_folder() {
        let demo = fixture("demo-site");
        let tree = ContentTree::walk(&demo).unwrap();
        let content = read_content(&tree).unwrap();
        let assets = Assets::of(&tree);
        assert_ne!(assets, Assets::default(), "the demo has a folder post");
        drop(tree);
        let source = pages(&demo);
        let given = source.render_content(&content, assets).unwrap();
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
        outline
            .content
            .posts
            .iter()
            .map(|doc| doc.path.clone())
            .collect()
    }

    /// The demo site in a folder of its own, which a test may edit.
    fn demo_copy() -> tempfile::TempDir {
        let site = tempfile::tempdir().unwrap();
        copy_dir(&fixture("demo-site"), site.path());
        site
    }

    /// Whether the file at `path` in `site` holds a wikilink.
    fn links_a_term(site: &Path, path: &Path) -> bool {
        let mut file = site.to_path_buf();
        file.push(path);
        fs::read_to_string(file).unwrap().contains("[[")
    }

    #[test]
    fn each_saved_post_alone_renders_the_page_the_full_render_makes() {
        let site = demo_copy();
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
        let (linking, posts): (Vec<PathBuf>, Vec<PathBuf>) = outlined(&source)
            .into_iter()
            .partition(|path| links_a_term(site.path(), path));
        assert_eq!(posts.len(), 6, "{posts:?}");
        for post in &posts {
            let saved = source.render_saved(std::slice::from_ref(post)).unwrap();
            let saved = saved.unwrap_or_else(|| panic!("{} alone", post.display()));
            assert_eq!(saved.len(), 2, "the post and its translation");
            for page in &saved {
                assert!(all.contains(page), "{} differs", page.path().as_str());
            }
        }
        assert_eq!(linking.len(), 4, "{linking:?}");
        assert_eq!(source.render_saved(&linking).unwrap(), None, "a wikilink");
        let author = vec![PathBuf::from("content/en/authors/asif.md")];
        assert_eq!(source.render_saved(&author).unwrap(), None);
    }

    #[test]
    fn a_saved_edit_that_changes_the_related_posts_renders_the_next_full_render_page() {
        let site = demo_copy();
        let source = pages(site.path());
        source.render_all().unwrap();
        let post = PathBuf::from("content/en/posts/2026-09-28-what-a-sitemap-does.md");
        let mut file = site.path().to_path_buf();
        file.push(&post);
        let text = fs::read_to_string(&file).unwrap();
        let edited = text.replace(
            "title: What a sitemap does,",
            "title: What a sitemap is for,",
        );
        let link = "\nSee [the topic cluster plan](/blog/planning-a-topic-cluster/).\n";
        fs::write(&file, format!("{edited}{link}")).unwrap();

        let saved = source.render_saved(&[post]).unwrap().unwrap();
        let all = source.render_all().unwrap().pages;
        let page = saved
            .iter()
            .find(|page| page.path().as_str() == "/blog/what-a-sitemap-does/")
            .unwrap();
        let body = page.body().unwrap();
        let html = String::from_utf8_lossy(&body);
        assert!(html.contains("What a sitemap is for"), "the edit shows");
        assert!(all.contains(page), "the saved page is the full render's");
    }
}
