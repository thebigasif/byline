//! The list pages of each language (audit brief G12): each collection's
//! page where `routes.toml` puts it, the archive of every dated post and
//! the page of every author, as `[reader]` in `site.toml` switches the last
//! two on. A long list is split into pages of [`PER_PAGE`] posts, the first
//! at the list's own address and the rest at `page/2/` and on.

use khalid_content::{SiteContent, Wikilinks};
use khalid_core::{Config, Indexing, Locale, RobotsDirectives, UserMessage};
use khalid_render::{
    AuthorEntry, BodyHtml, PageBody, PageKind, Pagination, Person, PostCard, RenderAuthors,
    RenderListing, Renderer,
};
use khalid_routes::Routes;
use khalid_seo::PageKind as SeoKind;

use crate::build_pages::{self, LanguagePages, Page};
use crate::listed_post::ListedPost;
use crate::{build_seo, serve_message};

/// How many posts one page of a list shows: enough to scan in one go, few
/// enough that the page stays light on a phone.
const PER_PAGE: usize = 20;

/// One list of posts before it is split into pages.
struct List {
    /// The end of its first page's address, after the mount and the
    /// language prefix, such as `archive/`.
    rest: String,
    title: String,
    /// The summary for search results, empty when it has none.
    description: String,
    /// Text above the first page's posts, such as a collection's `_index.md`.
    intro: Option<BodyHtml>,
    /// The key its versions in other languages share. Page 2 and on add
    /// their number.
    translation: String,
    kind: SeoKind,
    /// Its own robots directives, in place of the site's index policy.
    robots: Option<RobotsDirectives>,
    /// The page type its pages render as.
    body: fn(RenderListing) -> PageBody,
    /// Its posts, newest first.
    cards: Vec<PostCard>,
}

/// Adds the list pages to each of `languages`, titled in its language by
/// `renderer`'s strings: each collection's page where `routes` puts it,
/// with its `_index.md` from `content`, then the archive and the authors
/// page, unless `[reader]` turns them off.
///
/// # Errors
///
/// Returns what, why and fix when a page's address cannot be made from the
/// site's settings, or an `_index.md` body cannot be rendered.
pub(crate) fn add(
    (config, routes): (&Config, &Routes),
    renderer: &Renderer,
    content: &SiteContent,
    languages: &mut [LanguagePages],
) -> Result<(), UserMessage> {
    let reader = config.reader();
    for language in languages.iter_mut() {
        let locale = language.site.locale.clone();
        let mut lists = collections((config, routes), content, &locale, &language.listed)?;
        if reader.archive {
            lists.push(List {
                rest: "archive/".to_owned(),
                title: text(renderer, &locale, "archive_title")?,
                description: String::new(),
                intro: None,
                translation: "archive".to_owned(),
                kind: SeoKind::Collection,
                // The home page already lists every post, newest first.
                robots: reader.home_sections.is_empty().then(no_index),
                body: PageBody::Archive,
                cards: language
                    .listed
                    .iter()
                    .map(|post| post.card.clone())
                    .collect(),
            });
        }
        let mut pages = Vec::new();
        for list in lists {
            pages.extend(paged(config, renderer, &locale, &list)?);
        }
        if reader.authors_page {
            let title = text(renderer, &locale, "authors_title")?;
            pages.push(authors(config, &locale, title, &language.pages)?);
        }
        language.pages.extend(pages);
    }
    Ok(())
}

/// The pages of `list`, [`PER_PAGE`] posts each, linked to each other. A
/// list with no posts has one page, which says so.
fn paged(
    config: &Config,
    renderer: &Renderer,
    locale: &Locale,
    list: &List,
) -> Result<Vec<Page>, UserMessage> {
    let total = u32::try_from(list.cards.len()).ok();
    let mut chunks: Vec<Vec<PostCard>> = list.cards.chunks(PER_PAGE).map(<[_]>::to_vec).collect();
    if chunks.is_empty() {
        chunks.push(Vec::new());
    }
    let rests: Vec<String> = (1..=chunks.len())
        .map(|number| match number {
            1 => list.rest.clone(),
            _ => format!("{}page/{number}/", list.rest),
        })
        .collect();
    let paths: Vec<String> = rests
        .iter()
        .map(|rest| build_pages::path_of(config, locale, rest))
        .collect();
    let mut pages = Vec::with_capacity(chunks.len());
    for (number, ((posts, rest), path)) in (1..).zip(chunks.into_iter().zip(&rests).zip(&paths)) {
        let (title, translation) = if number == 1 {
            (list.title.clone(), list.translation.clone())
        } else {
            let title = text(renderer, locale, "paged_title")?
                .replace("{title}", &list.title)
                .replace("{current}", &number.to_string());
            (title, format!("{} {number}", list.translation))
        };
        let description = list.description.clone();
        let meta = build_pages::meta(
            config,
            title.clone(),
            description.clone(),
            path.clone(),
            PageKind::Website,
        )?;
        let mut seo = build_seo::page(list.kind, locale, rest, &title)?;
        seo.excerpt = description;
        seo.item_count = total;
        seo.robots.clone_from(&list.robots);
        seo.translation = build_seo::translation(Some(&translation))?;
        let body = (list.body)(RenderListing {
            title,
            intro: list.intro.clone().filter(|_| number == 1),
            posts,
            pagination: Pagination::new(paths.clone(), number),
        });
        pages.push(Page {
            file: format!("{}{rest}index.html", build_pages::prefix(config, locale)),
            meta,
            body,
            seo,
            source: None,
        });
    }
    Ok(pages)
}

/// The page of every author of a language, `/authors/`, made from the
/// author pages among `pages`, in their order, each with how many posts it
/// lists.
fn authors(
    config: &Config,
    locale: &Locale,
    title: String,
    pages: &[Page],
) -> Result<Page, UserMessage> {
    const REST: &str = "authors/";
    let entries = pages
        .iter()
        .filter_map(|page| match &page.body {
            PageBody::Author(author) => Some(AuthorEntry {
                person: Person {
                    name: author.name.clone(),
                    href: page.meta.path.clone(),
                    role: author.role.clone(),
                    image: author.image.clone(),
                    bio: None,
                    profiles: Vec::new(),
                },
                posts: author.posts.len(),
            }),
            _ => None,
        })
        .collect();
    let path = build_pages::path_of(config, locale, REST);
    let meta = build_pages::meta(
        config,
        title.clone(),
        String::new(),
        path,
        PageKind::Website,
    )?;
    let mut seo = build_seo::page(SeoKind::Page, locale, REST, &title)?;
    seo.translation = build_seo::translation(Some("authors"))?;
    Ok(Page {
        file: format!("{}{REST}index.html", build_pages::prefix(config, locale)),
        meta,
        body: PageBody::Authors(RenderAuthors {
            title,
            authors: entries,
        }),
        seo,
        source: None,
    })
}

/// The address pattern of a collection `routes.toml` does not name, the
/// one `khalid_routes::Routes::address` falls back to.
const UNNAMED: &str = "/{collection}/{slug}/";

/// The list of each collection of `listed` that has a page of its own, in
/// the order of its newest post, with the title, description and body of
/// its `_index.md` in `locale`. Collections whose pages would sit at the
/// same address share one list.
fn collections(
    (config, routes): (&Config, &Routes),
    content: &SiteContent,
    locale: &Locale,
    listed: &[ListedPost],
) -> Result<Vec<List>, UserMessage> {
    let mut lists: Vec<List> = Vec::new();
    let mut links = None;
    for post in listed {
        let Some(rest) = collection_rest(routes, &post.collection) else {
            continue;
        };
        if let Some(list) = lists.iter_mut().find(|list| list.rest == rest) {
            list.cards.push(post.card.clone());
            continue;
        }
        let index = content
            .indexes
            .iter()
            .find(|doc| &doc.locale == locale && doc.collection == post.collection);
        let front = index.map(|doc| &doc.front);
        let intro = match index.filter(|doc| !doc.body.trim().is_empty()) {
            Some(doc) => {
                let links = links.get_or_insert_with(|| {
                    Wikilinks::for_language(content, config, routes, locale)
                });
                Some(build_pages::body_html(config, links, doc)?)
            }
            None => None,
        };
        lists.push(List {
            rest,
            title: front
                .and_then(|front| front.title.clone())
                .unwrap_or_else(|| post.collection.clone()),
            description: front
                .and_then(|front| front.description.clone())
                .unwrap_or_default(),
            intro,
            translation: format!("collection {}", post.collection),
            kind: SeoKind::Collection,
            robots: None,
            body: PageBody::Collection,
            cards: vec![post.card.clone()],
        });
    }
    Ok(lists)
}

/// Where the page of `collection` sits, after the mount and the language
/// prefix: the folders of its address pattern before the first placeholder
/// other than `{collection}`, such as `guides/` for `/guides/{slug}/`.
/// `None` when that is the home page, which lists every post already.
fn collection_rest(routes: &Routes, collection: &str) -> Option<String> {
    let pattern = routes
        .collection(collection)
        .map_or(UNNAMED, |found| found.permalink().as_str());
    let filled = pattern.replace("{collection}", collection);
    let fixed = filled.split('{').next().unwrap_or_default();
    let folders = fixed.rsplit_once('/').map_or("", |(folders, _)| folders);
    let rest = folders.trim_start_matches('/');
    (!rest.is_empty()).then(|| format!("{rest}/"))
}

/// The text `key` of the strings file for `locale`.
fn text(renderer: &Renderer, locale: &Locale, key: &str) -> Result<String, UserMessage> {
    renderer
        .text(locale, key)
        .map(str::to_owned)
        .ok_or_else(serve_message::internal)
}

/// Directives that keep a page out of search results.
fn no_index() -> RobotsDirectives {
    let mut directives = RobotsDirectives::default();
    directives.indexing = Indexing::NoIndex;
    directives
}
