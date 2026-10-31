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
use khalid_core::{Clock, Config, Locale, Slug, SystemClock, UserMessage};
use khalid_render::{
    PageBody, PageMeta, RenderSearch, RenderSite, Renderer, SearchMode, SearchResult, Tokens,
};
use khalid_routes::Routes;
use khalid_server::{Moved, Page, Render, Rendered};

use crate::build_pages::LanguagePages;
use crate::comment_pages::{self, ShownComments};
use crate::content_feed::ContentFeed;
use crate::listed_post::ListedPost;
use crate::post_files::Assets;
use crate::post_photos::Shown;
use crate::site_content::Catalog;
use crate::site_files::HEADLESS_NOT_FOUND;
use crate::site_page::HeadNonce;
use crate::unlock::Gate;
use crate::{
    assets, build_home, build_lists, build_message, build_pages, build_seo, build_sitemap,
    categories, content_message, post_files, post_lock, search_page, serve_message, share_images,
    site_files, site_outline,
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
    /// The search page of each language as the last full render made it,
    /// which a search's results are rendered into.
    searches: Mutex<Vec<SearchShell>>,
    /// The comments post pages show, or `None` when the site takes none.
    comments: Option<ShownComments>,
    /// The password posts of the last full render, for the password form.
    gate: Gate,
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
    /// The lead images and author photos the full render drew, which cards
    /// and bylines on a saved post's page show.
    shown: Shown,
}

/// The theme's renderer, and the stylesheet every page links.
#[derive(Debug)]
struct Theme {
    renderer: Renderer,
    stylesheet: Page,
    /// The tokens the stylesheet was built from, for the share cards.
    tokens: Tokens,
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
            let tokens = Tokens::load(folder).map_err(|error| build_message::tokens(&error))?;
            let stylesheet = assets::stylesheet_page(&config, &tokens)?;
            Some(Theme {
                renderer,
                stylesheet,
                tokens,
            })
        } else {
            None
        };
        let comments =
            (theme.is_some() && comment_pages::taken(config.mode())).then(ShownComments::default);
        Ok(Self(Arc::new(Site {
            folder: folder.to_path_buf(),
            config,
            routes,
            theme,
            feed,
            outline: Mutex::default(),
            assets: Mutex::default(),
            searches: Mutex::default(),
            gate: Gate::default(),
            comments,
        })))
    }

    /// The text `key` of the strings file for `locale`, as the theme's
    /// templates read it. `None` on a headless site.
    pub(crate) fn text(&self, locale: &Locale, key: &str) -> Option<String> {
        let theme = self.0.theme.as_ref()?;
        theme.renderer.text(locale, key).map(str::to_owned)
    }

    /// The page of a search for `query` that found `results`, rendered into
    /// the search page at `path` as the last full render made it. `None`
    /// before the first full render, or when `path` has no search page.
    pub(crate) fn render_search(
        &self,
        path: &str,
        query: Option<String>,
        results: Vec<SearchResult>,
    ) -> Result<Option<String>, UserMessage> {
        let Some(theme) = &self.0.theme else {
            return Ok(None);
        };
        let shell = self
            .0
            .searches
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .find(|shell| shell.meta.path == path)
            .cloned();
        let Some(SearchShell { site, meta }) = shell else {
            return Ok(None);
        };
        let body = PageBody::Search(RenderSearch {
            title: meta.title.clone(),
            mode: SearchMode::Live,
            action: meta.path.clone(),
            query,
            results,
        });
        let html = theme.renderer.render(&site, &meta, &body);
        html.map(Some)
            .map_err(|error| build_message::render(&error))
    }

    /// The comments the post pages show, or `None` when the site takes none.
    pub(crate) fn comments(&self) -> Option<ShownComments> {
        self.0.comments.clone()
    }

    /// The address of the stylesheet every page links; `None` when headless.
    pub(crate) fn stylesheet(&self) -> Option<String> {
        (self.0.theme.as_ref()).map(|theme| theme.stylesheet.path().as_str().to_owned())
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
            folder,
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
        search_page::link(config, &mut languages);
        let seo_site = build_seo::site(config)?;
        let cards = share_images::attach(folder, config, &theme.tokens, &seo_site, &mut languages)?;
        // The home pages answer first, so their cards are drawn now; the
        // rest warm in the background once the pages are live (G14).
        share_images::draw_now(folder, cards)?;
        build_seo::heads(&seo_site, &mut languages)?;
        let mut pages = vec![theme.stylesheet.clone()];
        html_pages(theme, config, &languages, Which::All, &mut pages)?;
        Ok(pages)
    }

    /// The password posts the password form checks, as the last full
    /// render found them.
    pub(crate) fn gate(&self) -> Gate {
        self.0.gate.clone()
    }

    /// The page the draft of post `slug` in `locale` would have if it went
    /// live today, rendered among the site's posts as they are now, for
    /// its preview link. `None` when there is no such draft, the site has
    /// no theme, or it has no post to shape the draft's entry on. Reads
    /// files and renders, so it runs off the async runtime (BL-RUN-7).
    pub(crate) fn preview(
        &self,
        locale: &Locale,
        slug: &Slug,
    ) -> Result<Option<String>, UserMessage> {
        let Site { folder, config, .. } = &*self.0;
        let mut file = folder.clone();
        file.push(format!("drafts/{locale}/{slug}.md"));
        let (Some(_), Ok(text)) = (&self.0.theme, fs::read_to_string(&file)) else {
            return Ok(None);
        };
        let unreadable = |error| content_message::of(folder, &error);
        let tree = ContentTree::walk(folder).map_err(unreadable)?;
        let mut content = read_content(&tree).map_err(unreadable)?;
        let today = SystemClock.now().date_naive();
        let path = PathBuf::from(format!("content/{locale}/posts/{today}-{slug}.md"));
        let post = parse_post(&path, &text).map_err(unreadable)?;
        // A Document is made only by khalid-content, so the draft's entry
        // is another post's with every field that names a file replaced.
        let Some(mut doc) = content.posts.first().cloned() else {
            return Ok(None);
        };
        doc.locale = locale.clone();
        "posts".clone_into(&mut doc.collection);
        doc.path = path;
        doc.front = post.front;
        doc.front.date.get_or_insert(today);
        let front_lines = text
            .lines()
            .count()
            .saturating_sub(post.body.lines().count());
        doc.body_line = front_lines.saturating_add(1);
        doc.body = post.body;
        content
            .posts
            .retain(|other| &other.locale != locale || other.front.slug.as_ref() != Some(slug));
        content.posts.push(doc);
        let assets = Assets::of(&tree);
        let mut shown = Shown::default();
        let which = Which::Posts;
        let (served, _) = self.render_of(&assets, (&content, &[]), which, &mut shown)?;
        let address = build_pages::path_of(config, locale, &format!("{slug}/"));
        let page = served
            .pages
            .iter()
            .rev()
            .find(|page| page.path().as_str() == address);
        let Some(page) = page else {
            return Ok(None);
        };
        let body = page.body().map_err(|_| serve_message::internal())?;
        Ok(Some(String::from_utf8_lossy(&body).into_owned()))
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
        let mut shown = Shown::default();
        let (rendered, listed) = self.render_of(&assets, (content, &[]), Which::All, &mut shown)?;
        *outline.lock().unwrap_or_else(PoisonError::into_inner) = Outline {
            content: site_outline::outline_of(content),
            listed,
            shown,
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
    /// writes none. Cards and bylines show the images in `shown`, which
    /// gains the ones these pages draw.
    fn render_of(
        &self,
        assets: &Assets,
        (content, others): (&SiteContent, &[ListedPost]),
        which: Which,
        shown: &mut Shown,
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
        let today = SystemClock.now().date_naive();
        let mut languages = build_pages::pages(config, routes, (content, others), href, today)?;
        let renderer = theme.as_ref().map(|theme| &theme.renderer);
        post_lock::lock(config, content, renderer, &mut languages);
        if let Some(theme) = theme {
            search_page::add(config, &theme.renderer, &mut languages)?;
            // A saved post's page alone needs no list, and its content
            // holds too few posts to make one.
            if which == Which::All {
                let categories = categories::of(folder)?;
                let sources = (content, categories.as_slice());
                build_lists::add((config, routes), &theme.renderer, sources, &mut languages)?;
                build_sitemap::add(config, &theme.renderer, &mut languages)?;
            }
        }
        comment_pages::fill(config, self.0.comments.as_ref(), &mut languages);
        let beside = post_files::served(folder, config, assets, &mut languages, shown)?;
        let seo_site = build_seo::site(config)?;
        let default_tokens = Tokens::default();
        let tokens = theme.as_ref().map_or(&default_tokens, |t| &t.tokens);
        let cards = share_images::attach(folder, config, tokens, &seo_site, &mut languages)?;
        let seo_pages = build_seo::heads(&seo_site, &mut languages)?;
        build_sitemap::fill(config, &seo_site, &seo_pages, &mut languages)?;
        post_lock::share_heads(&mut languages);
        if which == Which::All {
            self.0.gate.replace(&languages);
        }
        let mut served = Served {
            pages: Vec::new(),
            not_found: HEADLESS_NOT_FOUND.to_owned(),
            moves: Vec::new(),
        };
        if which == Which::All {
            served.pages = site_files::files(config, &seo_site, &seo_pages, SystemClock.now())?;
            served.pages.extend(beside);
            served.moves = site_files::moves(config, routes, content)?;
        }
        if let Some(theme) = theme {
            if which == Which::All {
                served.pages.push(theme.stylesheet.clone());
                let shells = search_shells(&languages);
                *self
                    .0
                    .searches
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner) = shells;
            }
            served.not_found = html_pages(theme, config, &languages, which, &mut served.pages)?;
        }
        // The cards draw behind the answers: their addresses are already
        // baked into the heads, so the pages are complete without the
        // files, and the cache warms while the server answers (G14, ADR
        // "khalid serve answers before the full render").
        if !cards.is_empty() {
            let site = folder.clone();
            std::thread::spawn(move || share_images::warm(&site, cards));
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

/// A language's search page without results: the site as its pages show it
/// and the page's head, with its title and its SEO tags.
#[derive(Debug, Clone)]
struct SearchShell {
    site: RenderSite,
    meta: PageMeta,
}

/// The search page of each of `languages`.
fn search_shells(languages: &[LanguagePages]) -> Vec<SearchShell> {
    let mut shells = Vec::new();
    for language in languages {
        let searches = language
            .pages
            .iter()
            .filter(|page| matches!(page.body, PageBody::Search(_)));
        shells.extend(searches.map(|page| SearchShell {
            site: language.site.clone(),
            meta: page.meta.clone(),
        }));
    }
    shells
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
            let html = page.rendered(&theme.renderer, &language.site, HeadNonce::PerRequest)?;
            if matches!(page.body, PageBody::NotFound) {
                if &language.site.locale == config.default_language() {
                    not_found = html;
                }
                continue;
            }
            pages.push(page.served_at(&page.meta.path, html)?);
        }
        for unlocked in &language.unlocked {
            let page = &unlocked.page;
            let html = page.rendered(&theme.renderer, &language.site, HeadNonce::PerRequest)?;
            pages.push(page.served_at(&post_lock::hidden(&page.meta.path), html)?);
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
        // A post that is or was locked changes the password posts the
        // form checks, which only the full render keeps.
        let locked = |doc: &Document<PostFront>| {
            post_lock::Lock::of(&doc.front).is_some() || self.0.gate.guards(&doc.path)
        };
        if saved.posts.iter().any(locked) {
            return Ok(None);
        }
        let outline = self.0.outline.lock();
        let mut shown = outline
            .unwrap_or_else(PoisonError::into_inner)
            .shown
            .clone();
        let (served, _) = self
            .render_of(&assets, (&saved, &others), Which::Posts, &mut shown)
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
#[path = "site_pages_tests.rs"]
mod tests;
