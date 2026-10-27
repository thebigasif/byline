//! The list pages of each language (audit brief G12): each collection's
//! page where `routes.toml` puts it, each category, tag and channel under
//! the prefixes and paths it gives them, the year, month and day pages its
//! date patterns make room for, the archive of every dated post and
//! the page of every author, as `[reader]` in `site.toml` switches the last
//! two on. A long list is split into pages of [`PER_PAGE`] posts, the first
//! at the list's own address and the rest at `page/2/` and on.

use chrono::Datelike;
use khalid_content::{SiteContent, Wikilinks};
use khalid_core::{Config, Indexing, Locale, RobotsDirectives, UserMessage};
use khalid_db::IndexedTerm;
use khalid_render::{
    AuthorEntry, BodyHtml, PageBody, PageKind, Pagination, Person, PostCard, RenderAuthors,
    RenderListing, Renderer,
};
use khalid_routes::{ChannelFilter, Routes};
use khalid_seo::{PageKind as SeoKind, TranslationKey};

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
/// with its `_index.md` from `content`, each category named as
/// `categories` names it, each tag and each channel, then the archive and
/// the authors page, unless `[reader]` turns them off.
///
/// # Errors
///
/// Returns what, why and fix when a page's address cannot be made from the
/// site's settings, or an `_index.md` body cannot be rendered.
pub(crate) fn add(
    (config, routes): (&Config, &Routes),
    renderer: &Renderer,
    (content, categories): (&SiteContent, &[IndexedTerm]),
    languages: &mut [LanguagePages],
) -> Result<(), UserMessage> {
    let reader = config.reader();
    for language in languages.iter_mut() {
        let locale = language.site.locale.clone();
        let listed = &language.listed;
        let mut lists = collections((config, routes), content, &locale, listed)?;
        lists.extend(terms(routes, categories, &locale, listed));
        lists.extend(dates(routes, renderer, &locale, listed)?);
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
        seo.translation = list_key(&translation)?;
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
    seo.translation = list_key("authors")?;
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
/// prefix, such as `guides/` for `/guides/{slug}/`. `None` when that is
/// the home page, which lists every post already.
fn collection_rest(routes: &Routes, collection: &str) -> Option<String> {
    let (folders, _) = shape(routes, collection);
    (!folders.is_empty()).then_some(folders)
}

/// What `collection`'s address pattern makes room for: its fixed folders
/// before the first placeholder other than `{collection}`, such as `news/`
/// or nothing, and how many of `{year}/`, `{month}/` and `{day}/` follow
/// them in that order, from 0 to 3.
fn shape(routes: &Routes, collection: &str) -> (String, usize) {
    let pattern = routes
        .collection(collection)
        .map_or(UNNAMED, |found| found.permalink().as_str());
    let filled = pattern.replace("{collection}", collection);
    let fixed = filled.split('{').next().unwrap_or_default();
    let folders = fixed.rsplit_once('/').map_or("", |(folders, _)| folders);
    let after = filled.strip_prefix(folders).unwrap_or_default();
    let depth = (after.trim_start_matches('/').split('/'))
        .zip(["{year}", "{month}", "{day}"])
        .take_while(|(segment, wanted)| segment == wanted)
        .count();
    let folders = folders.trim_start_matches('/');
    let folders = if folders.is_empty() {
        String::new()
    } else {
        format!("{folders}/")
    };
    (folders, depth)
}

/// The year, month and day lists of `listed` in `locale`, for each
/// collection whose address pattern has room for them (see [`shape`]),
/// such as `news/2026/` and `news/2026/09/`, titled with the words of
/// `renderer`'s strings, such as "September 2026".
fn dates(
    routes: &Routes,
    renderer: &Renderer,
    locale: &Locale,
    listed: &[ListedPost],
) -> Result<Vec<List>, UserMessage> {
    let mut lists: Vec<List> = Vec::new();
    for post in listed {
        let (base, depth) = shape(routes, &post.collection);
        if depth == 0 {
            continue;
        }
        let date = post.card.date;
        let (year, month, day) = (date.year(), date.month(), date.day());
        let month_name = text(renderer, locale, &format!("month_{month}"))?;
        let named = |key: &str| -> Result<String, UserMessage> {
            Ok(text(renderer, locale, key)?
                .replace("{day}", &day.to_string())
                .replace("{month}", &month_name)
                .replace("{year}", &year.to_string()))
        };
        let levels = [
            (format!("{base}{year:04}/"), year.to_string()),
            (format!("{base}{year:04}/{month:02}/"), named("month_year")?),
            (
                format!("{base}{year:04}/{month:02}/{day:02}/"),
                named("date_format")?,
            ),
        ];
        for (rest, title) in levels.into_iter().take(depth) {
            let key = format!("date {rest}");
            add_card(&mut lists, rest, &post.card, || {
                term_list(title, key, SeoKind::Date, PageBody::Date)
            });
        }
    }
    Ok(lists)
}

/// The list of each category, tag and channel of `listed`, in `locale`:
/// categories and tags under the prefixes `routes` gives them, named as
/// `categories` names them, and every channel `routes.toml` declares, even
/// one no post matches yet.
fn terms(
    routes: &Routes,
    categories: &[IndexedTerm],
    locale: &Locale,
    listed: &[ListedPost],
) -> Vec<List> {
    let mut lists: Vec<List> = Vec::new();
    for post in listed {
        if let Some(slug) = &post.topics.category {
            let rest = format!("{}/{slug}/", routes.category_prefix());
            add_card(&mut lists, rest, &post.card, || {
                let name = categories
                    .iter()
                    .find(|term| &term.locale == locale && term.slug.as_str() == slug)
                    .map_or_else(|| slug.clone(), |term| term.name.clone());
                let key = format!("category {slug}");
                term_list(name, key, SeoKind::Category, PageBody::Category)
            });
        }
        for tag in &post.tags {
            let slug = &tag.slug;
            let rest = format!("{}/{slug}/", routes.tag_prefix());
            add_card(&mut lists, rest, &post.card, || {
                let key = format!("tag {slug}");
                term_list(tag.name.clone(), key, SeoKind::Tag, PageBody::Tag)
            });
        }
    }
    for channel in routes.channels() {
        let mut list = term_list(
            readable(channel.name()),
            format!("channel {}", channel.name()),
            SeoKind::Collection,
            PageBody::Channel,
        );
        channel
            .path()
            .trim_start_matches('/')
            .clone_into(&mut list.rest);
        let filter = channel.filter();
        list.cards = listed
            .iter()
            .filter(|post| matches(filter, post))
            .map(|post| post.card.clone())
            .collect();
        lists.push(list);
    }
    lists
}

/// Whether `post` matches every field `filter` sets.
fn matches(filter: &ChannelFilter, post: &ListedPost) -> bool {
    let topics = &post.topics;
    let author = filter.author.as_ref();
    let tag = filter.tag.as_ref();
    let category = filter.category.as_ref();
    author.is_none_or(|author| post.author.as_ref() == Some(author))
        && tag.is_none_or(|tag| topics.tags.iter().any(|own| own == tag.as_str()))
        && category.is_none_or(|category| topics.category.as_deref() == Some(category.as_str()))
        && (filter.collection.as_ref()).is_none_or(|collection| &post.collection == collection)
        && (filter.featured).is_none_or(|featured| post.featured == featured)
}

/// Adds `card` to the list at `rest`, made by `new` when there is none yet.
fn add_card(lists: &mut Vec<List>, rest: String, card: &PostCard, new: impl FnOnce() -> List) {
    if let Some(list) = lists.iter_mut().find(|list| list.rest == rest) {
        list.cards.push(card.clone());
        return;
    }
    let mut list = new();
    list.rest = rest;
    list.cards.push(card.clone());
    lists.push(list);
}

/// A list titled `title` with no posts yet, no intro and no description.
fn term_list(
    title: String,
    translation: String,
    kind: SeoKind,
    body: fn(RenderListing) -> PageBody,
) -> List {
    List {
        rest: String::new(),
        title,
        description: String::new(),
        intro: None,
        translation,
        kind,
        robots: None,
        body,
        cards: Vec::new(),
    }
}

/// A channel's key as a heading: `from-the-founder` reads "From the founder".
fn readable(name: &str) -> String {
    let spaced = name.replace(['-', '_'], " ");
    let mut chars = spaced.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

/// The translation key a list page's versions share, such as
/// `list:archive`. The prefix keeps it apart from the keys people write in
/// front matter, where `archive` or `sitemap` may name a post.
///
/// # Errors
///
/// Returns what, why and fix for an empty `name`, which no caller passes.
pub(crate) fn list_key(name: &str) -> Result<Option<TranslationKey>, UserMessage> {
    build_seo::translation(Some(&format!("list:{name}")))
}

/// The text `key` of the strings file for `locale`.
pub(crate) fn text(renderer: &Renderer, locale: &Locale, key: &str) -> Result<String, UserMessage> {
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use chrono::NaiveDate;
    use khalid_core::Slug;
    use khalid_render::PostTopics;

    use super::*;
    use crate::listed_post::ListedTag;

    /// A post with `tags`, each named as its slug.
    fn tagged(slug: &str, tags: &[&str]) -> ListedPost {
        let date = NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();
        ListedPost {
            path: format!("content/en/posts/{slug}.md").into(),
            locale: Locale::try_new("en").unwrap(),
            collection: "posts".to_owned(),
            featured: false,
            tags: (tags.iter())
                .map(|tag| ListedTag {
                    slug: Slug::try_new(tag).unwrap(),
                    name: (*tag).to_owned(),
                })
                .collect(),
            topics: PostTopics {
                slug: slug.to_owned(),
                date,
                category: None,
                tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
                links: Vec::new(),
            },
            card: PostCard {
                title: slug.to_owned(),
                href: format!("/blog/{slug}/"),
                date,
                excerpt: String::new(),
                image: None,
                author: None,
                category: None,
            },
            author: None,
        }
    }

    #[test]
    fn date_folders_count_only_right_after_the_fixed_folders_and_in_order() {
        let text = "[collections.news]\npermalink = \"/news/{year}/{month}/{slug}/\"\n\n\
                    [collections.diary]\npermalink = \"/{year}/{month}/{day}/{slug}/\"\n\n\
                    [collections.odd]\npermalink = \"/odd/{month}/{year}/{slug}/\"\n";
        let routes = Routes::from_toml(Path::new("routes.toml"), text).unwrap();
        let shapes: Vec<(String, usize)> = ["news", "diary", "odd", "posts", "notes"]
            .iter()
            .map(|collection| shape(&routes, collection))
            .collect();
        let want = [("news/", 2), ("", 3), ("odd/", 0), ("", 0), ("notes/", 0)]
            .map(|(folders, depth)| (folders.to_owned(), depth));
        assert_eq!(shapes, want);
    }

    #[test]
    fn each_tag_lists_its_posts_under_the_tag_prefix_and_a_channel_can_pick_one() {
        let text = "[taxonomies]\ntag = \"topic\"\n\n\
                    [channels.need-for-speed]\npath = \"/fast/\"\nfilter = { tag = \"speed\" }\n";
        let routes = Routes::from_toml(Path::new("routes.toml"), text).unwrap();
        let posts = [tagged("a", &["links"]), tagged("b", &["links", "speed"])];
        let english = Locale::try_new("en").unwrap();
        let shown: Vec<String> = terms(&routes, &[], &english, &posts)
            .iter()
            .map(|list| format!("{} {} {}", list.rest, list.title, list.cards.len()))
            .collect();
        assert_eq!(
            shown,
            [
                "topic/links/ links 2",
                "topic/speed/ speed 1",
                "fast/ Need for speed 1"
            ]
        );
    }
}
