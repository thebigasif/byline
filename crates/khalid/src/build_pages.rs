//! The pages `khalid build` renders: the site's content turned into the input
//! records of `khalid-render` (ADR 0009), one language at a time.

use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::mem;
use std::path::Path;

use chrono::NaiveDate;
use khalid_content::{
    Document, LeadImage, PostFront, Rendered, SiteContent, Wikilinks, render_linked,
};
use khalid_core::{AbsoluteUrl, Config, Locale, Slug, UserMessage};
use khalid_render::{
    BodyHtml, Link, Nav, PageBody, PageKind, PageMeta, Person, PostCard, PostTopics, ReadingTime,
    RenderAuthor, RenderHome, RenderListing, RenderPage, RenderPost, RenderSite, Robots, related,
};
use khalid_routes::{PostAddress, Routes};
use khalid_seo::{AuthorProfile, MetaTemplate, PageKind as SeoKind, SeoPage};

use crate::listed_post::{ListedPost, ListedTag};
use crate::post_lock::Unlocked;
pub(crate) use crate::site_page::Page;
use crate::{build_home, build_message, build_reader, build_schema, build_seo};

/// The pages of one language, and the site as they show it.
#[derive(Debug)]
pub(crate) struct LanguagePages {
    pub(crate) site: RenderSite,
    pub(crate) pages: Vec<Page>,
    /// The full pages of the password posts, which only `khalid serve`
    /// sends, to a reader who gave the password.
    pub(crate) unlocked: Vec<Unlocked>,
    /// The dated posts the pages were made from, newest first.
    pub(crate) listed: Vec<ListedPost>,
}

/// Every page of the site, per language in the order of `site.toml`: each
/// dated post and each glossary term at the address `routes` gives it, each
/// plain page, each author with their posts, the home page with the posts
/// newest first, and the 404 page. Every page links the stylesheet at
/// `stylesheet`.
///
/// Each post's related posts are chosen from the dated posts of `content`
/// and from `others`, the posts of an earlier render that `content` does
/// not hold, so `khalid serve` can render a saved post's page alone. The
/// lists only show the posts of `content`.
pub(crate) fn pages(
    config: &Config,
    routes: &Routes,
    (content, others): (&SiteContent, &[ListedPost]),
    stylesheet: &str,
) -> Result<Vec<LanguagePages>, UserMessage> {
    config
        .languages()
        .iter()
        .map(|locale| language_pages(config, routes, (content, others), locale, stylesheet))
        .collect()
}

fn language_pages(
    config: &Config,
    routes: &Routes,
    (content, others): (&SiteContent, &[ListedPost]),
    locale: &Locale,
    stylesheet: &str,
) -> Result<LanguagePages, UserMessage> {
    let links = Wikilinks::for_language(content, config, routes, locale);
    let people = build_reader::people(config, &links, content, locale)?;
    let posts = Posts {
        content,
        others,
        locale,
        people: &people,
    };
    let (mut pages, listed) = post_pages(config, routes, &links, &posts)?;
    pages.extend(plain_pages(config, &links, content, locale)?);
    pages.extend(glossary_pages(config, routes, &links, content, locale)?);
    pages.extend(author_pages(
        config, &links, content, locale, &listed, &people,
    )?);
    let cards = listed.iter().map(|post| post.card.clone()).collect();
    let home = home_page(config, locale, cards)?;
    let mut missing = home.meta.clone();
    missing.robots = Robots::NoIndex;
    pages.push(home);
    let mut missing_seo = build_seo::page(SeoKind::NotFound, locale, "404.html", config.name())?;
    missing_seo.seo_title = MetaTemplate::parse("{site}").ok();
    pages.push(Page {
        file: format!("{}404.html", prefix(config, locale)),
        meta: missing,
        body: PageBody::NotFound,
        seo: missing_seo,
        source: None,
        template: None,
        head_code: None,
        lead: None,
    });
    Ok(LanguagePages {
        site: render_site(config, locale, stylesheet),
        pages,
        listed,
        unlocked: Vec::new(),
    })
}

/// The home page of `locale`, listing `cards`, newest first.
pub(crate) fn home_page(
    config: &Config,
    locale: &Locale,
    cards: Vec<PostCard>,
) -> Result<Page, UserMessage> {
    let listing = RenderListing {
        title: config.name().to_owned(),
        intro: None,
        posts: cards,
        pagination: None,
    };
    let meta = meta(
        config,
        config.name().to_owned(),
        String::new(),
        path_of(config, locale, ""),
        PageKind::Website,
    )?;
    let mut seo = build_seo::page(SeoKind::Home, locale, "", config.name())?;
    seo.translation = build_seo::translation(Some("home"))?;
    let body = PageBody::Home(RenderHome {
        listing,
        sections: Vec::new(),
        topics: Vec::new(),
    });
    Ok(page(locale, config, "", meta, body, seo))
}

/// The dated posts of one language that pages are made from or choose
/// their related posts from, and the people they name.
struct Posts<'a> {
    content: &'a SiteContent,
    /// Posts of an earlier render, in any language, some of which `content`
    /// may hold again.
    others: &'a [ListedPost],
    locale: &'a Locale,
    people: &'a BTreeMap<Slug, Person>,
}

/// A dated post, read and rendered, before its page is built.
struct Draft<'a> {
    doc: &'a Document<PostFront>,
    date: NaiveDate,
    title: String,
    slug: Slug,
    /// The end of its address, after the mount and the language prefix.
    rest: String,
    rendered: Rendered,
}

/// The page of every dated post of `posts` in its language, and the posts
/// as lists show them, newest first. Every post is listed before any page
/// is made, so each page can show related ones, chosen among these and the
/// other posts of `posts`.
fn post_pages(
    config: &Config,
    routes: &Routes,
    links: &Wikilinks,
    posts: &Posts<'_>,
) -> Result<(Vec<Page>, Vec<ListedPost>), UserMessage> {
    let Posts {
        content,
        locale,
        people,
        ..
    } = *posts;
    let mut drafts = Vec::new();
    for doc in content.posts.iter().filter(|doc| &doc.locale == locale) {
        let Some(date) = doc.front.date else {
            continue;
        };
        let front = &doc.front;
        let (title, slug) = named(config, doc, front.slug.as_ref(), front.title.as_deref())?;
        let entry = PostAddress::new(&doc.collection, locale, &slug, Some(date));
        let rest = routed(config, routes, locale, &entry, &doc.path)?;
        let mut rendered = render_linked(&doc.body, &doc.locale, config.slugs(), links)
            .map_err(|error| build_message::block(&doc.path, &error))?;
        rendered.html = build_seo::with_link_policy(config, mem::take(&mut rendered.html));
        drafts.push(Draft {
            doc,
            date,
            title,
            slug,
            rest,
            rendered,
        });
    }
    drafts.sort_by_key(|draft| Reverse(draft.date));
    let others: Vec<&ListedPost> = posts
        .others
        .iter()
        .filter(|other| &other.locale == locale)
        .filter(|other| !drafts.iter().any(|draft| draft.doc.path == other.path))
        .collect();
    let drafted = drafts
        .iter()
        .map(|draft| (path_of(config, locale, &draft.rest), draft.slug.to_string()));
    let known = others
        .iter()
        .map(|other| (other.card.href.clone(), other.topics.slug.clone()));
    let slugs: BTreeMap<String, String> = drafted.chain(known).collect();
    let listed: Vec<ListedPost> = drafts
        .iter()
        .map(|draft| listed(config, posts, draft, &slugs))
        .collect();
    let among: Vec<&ListedPost> = listed.iter().chain(others).collect();
    let topics: Vec<PostTopics> = among.iter().map(|post| post.topics.clone()).collect();
    let mut pages = Vec::with_capacity(drafts.len() + 2);
    for (draft, topic) in drafts.into_iter().zip(&topics) {
        let pinned: Vec<String> = draft
            .doc
            .front
            .related
            .iter()
            .map(Slug::to_string)
            .collect();
        let related = related(topic, &topics, &pinned)
            .into_iter()
            .filter_map(|index| among.get(index).map(|post| post.card.clone()))
            .collect();
        pages.push(post_page(config, locale, draft, related, people)?);
    }
    Ok((pages, listed))
}

/// A post as lists and the related-posts choice read it, its links found
/// by their paths in `slugs`.
fn listed(
    config: &Config,
    posts: &Posts<'_>,
    draft: &Draft<'_>,
    slugs: &BTreeMap<String, String>,
) -> ListedPost {
    let (locale, front) = (posts.locale, &draft.doc.front);
    let named = (draft.title.clone(), draft.rest.as_str(), draft.date);
    let card = build_home::card(config, locale, posts.people, named, front);
    let tags = ListedTag::all(&front.tags, locale, config.slugs());
    let mut topics = build_reader::topics(&draft.slug, draft.date, front, &draft.rendered, slugs);
    topics.tags = tags.iter().map(|tag| tag.slug.to_string()).collect();
    ListedPost {
        path: draft.doc.path.clone(),
        locale: locale.clone(),
        collection: draft.doc.collection.clone(),
        featured: front.featured,
        tags,
        topics,
        card,
        author: front.author.clone(),
    }
}

/// The page of one post, with the posts chosen to show under it.
fn post_page(
    config: &Config,
    locale: &Locale,
    draft: Draft<'_>,
    related: Vec<PostCard>,
    people: &BTreeMap<Slug, Person>,
) -> Result<Page, UserMessage> {
    let Draft {
        doc,
        date,
        title,
        rest,
        rendered,
        ..
    } = draft;
    let front = &doc.front;
    let path = path_of(config, locale, &rest);
    let description = front.description.clone().unwrap_or_default();
    let mut seo = build_seo::post(locale, (&rest, &title), front, date)?;
    seo.collection = Some(doc.collection.clone());
    seo.content_html = build_seo::full_addresses(config, &rendered.html);
    let (reviewers, profiles) = build_reader::reviewers(front, people, config)?;
    seo.reviewers = profiles;
    let html = BodyHtml::from_sanitised(rendered.html);
    let post = RenderPost {
        title: title.clone(),
        date,
        updated: None,
        reading_time: ReadingTime::of(&html, locale),
        authors: front
            .author
            .iter()
            .map(|slug| build_reader::person(people, slug, config, locale))
            .collect(),
        reviewers,
        category: None,
        tags: Vec::new(),
        image: None,
        contents: build_reader::contents(&rendered.headings),
        originals: BTreeMap::new(),
        related,
        comments: None,
        body: html,
    };
    seo.author = post.authors.first().map(|author| author.name.clone());
    let by = (path.as_str(), seo.author.clone().unwrap_or_default());
    let body = (&rendered.structured[..], &rendered.links[..]);
    build_schema::fill(&mut seo, config, (by.0, &by.1), body)?;
    seo.custom_schema = build_schema::custom(&doc.path, front.schema.as_deref())?;
    let meta = meta(config, title, description, path, PageKind::Article)?;
    let body = PageBody::Post(Box::new(post));
    let mut page = page(locale, config, &rest, meta, body, seo).made_from(&doc.path);
    page.template.clone_from(&front.template);
    page.head_code = front.head_code.clone().filter(|code| !code.is_empty());
    page.lead.clone_from(&front.image);
    Ok(page)
}

/// The page of every plain page in `locale`, such as About, at `/<slug>/`.
fn plain_pages(
    config: &Config,
    links: &Wikilinks,
    content: &SiteContent,
    locale: &Locale,
) -> Result<Vec<Page>, UserMessage> {
    let mut pages = Vec::new();
    for doc in content.pages.iter().filter(|doc| &doc.locale == locale) {
        let front = &doc.front;
        let (title, slug) = named(config, doc, front.slug.as_ref(), front.title.as_deref())?;
        let rest = format!("{slug}/");
        let description = front.description.clone().unwrap_or_default();
        let path = path_of(config, locale, &rest);
        let mut seo = build_seo::page(SeoKind::Page, locale, &rest, &title)?;
        seo.excerpt.clone_from(&description);
        seo.translation = build_seo::translation(front.translation.as_deref())?;
        let meta = meta(config, title.clone(), description, path, PageKind::Website)?;
        let body = RenderPage {
            title,
            updated: None,
            body: body_html(config, links, doc)?,
        };
        let body = PageBody::Page(body);
        pages.push(page(locale, config, &rest, meta, body, seo).made_from(&doc.path));
    }
    Ok(pages)
}

/// The page of every glossary term in `locale`, at the address `routes`
/// gives its collection, `/glossary/<slug>/` unless `routes.toml` names
/// another, so the wikilinks that reach it find it.
fn glossary_pages(
    config: &Config,
    routes: &Routes,
    links: &Wikilinks,
    content: &SiteContent,
    locale: &Locale,
) -> Result<Vec<Page>, UserMessage> {
    let mut pages = Vec::new();
    for doc in content.glossary.iter().filter(|doc| &doc.locale == locale) {
        let front = &doc.front;
        let (term, slug) = named(config, doc, front.slug.as_ref(), front.term.as_deref())?;
        let entry = PostAddress::new(&doc.collection, locale, &slug, None);
        let rest = routed(config, routes, locale, &entry, &doc.path)?;
        let path = path_of(config, locale, &rest);
        let mut seo = build_seo::page(SeoKind::GlossaryTerm, locale, &rest, &term)?;
        seo.translation = build_seo::translation(front.translation.as_deref())?;
        let meta = meta(config, term.clone(), String::new(), path, PageKind::Website)?;
        let body = RenderPage {
            title: term,
            updated: None,
            body: body_html(config, links, doc)?,
        };
        let body = PageBody::Page(body);
        pages.push(page(locale, config, &rest, meta, body, seo).made_from(&doc.path));
    }
    Ok(pages)
}

/// The page of every author in `locale`, at `/authors/<slug>/`, with the
/// cards of their posts.
fn author_pages(
    config: &Config,
    links: &Wikilinks,
    content: &SiteContent,
    locale: &Locale,
    listed: &[ListedPost],
    people: &BTreeMap<Slug, Person>,
) -> Result<Vec<Page>, UserMessage> {
    let mut pages = Vec::new();
    for doc in content.authors.iter().filter(|doc| &doc.locale == locale) {
        let front = &doc.front;
        let (name, slug) = named(config, doc, front.slug.as_ref(), front.name.as_deref())?;
        let rest = format!("authors/{slug}/");
        let path = path_of(config, locale, &rest);
        let meta = meta(config, name.clone(), String::new(), path, PageKind::Website)?;
        let photo = front
            .photo
            .as_deref()
            .and_then(|src| LeadImage::try_new(src, &name));
        let profiles = front
            .same_as
            .iter()
            .map(|url| Link {
                name: url.host().to_owned(),
                href: url.to_string(),
            })
            .collect();
        let posts: Vec<PostCard> = listed
            .iter()
            .filter(|post| post.author.as_ref() == Some(&slug))
            .map(|post| post.card.clone())
            .collect();
        let mut seo = build_seo::page(SeoKind::Author, locale, &rest, &name)?;
        seo.author = Some(name.clone());
        seo.item_count = u32::try_from(posts.len()).ok();
        // An author is one person in every language, so their pages pair by slug.
        seo.translation = build_seo::translation(Some(&format!("author {slug}")))?;
        let mut profile = AuthorProfile::new(&name);
        profile.job_title.clone_from(&front.role);
        profile.same_as.clone_from(&front.same_as);
        seo.author_profile = Some(profile);
        let author = RenderAuthor {
            name,
            role: front.role.clone(),
            image: None,
            bio: body_html(config, links, doc)?,
            profiles,
            posts,
            pagination: None,
            others: people
                .iter()
                .filter(|(other, _)| **other != slug)
                .map(|(_, person)| person.clone())
                .collect(),
        };
        let body = PageBody::Author(Box::new(author));
        let mut page = page(locale, config, &rest, meta, body, seo).made_from(&doc.path);
        page.lead = photo;
        pages.push(page);
    }
    Ok(pages)
}

/// A document's title, or its slug when it has none, and its slug, or one
/// made from the title when it has none.
pub(crate) fn named<F>(
    config: &Config,
    doc: &Document<F>,
    slug: Option<&Slug>,
    title: Option<&str>,
) -> Result<(String, Slug), UserMessage> {
    let slug = match (slug, title) {
        (Some(slug), _) => slug.clone(),
        (None, Some(title)) => Slug::from_title(title, &doc.locale, config.slugs())
            .map_err(|_| build_message::no_slug(&doc.path))?,
        (None, None) => return Err(build_message::no_slug(&doc.path)),
    };
    let title = title.map_or_else(|| slug.to_string(), str::to_owned);
    Ok((title, slug))
}

/// The end of the address `routes` gives the document at `file`, after the
/// mount and the language prefix, such as `glossary/seo/`.
pub(crate) fn routed(
    config: &Config,
    routes: &Routes,
    locale: &Locale,
    entry: &PostAddress<'_>,
    file: &Path,
) -> Result<String, UserMessage> {
    let address = routes
        .address(config, entry)
        .map_err(|error| build_message::route(file, &error))?;
    let rest = address.strip_prefix(config.mount()).unwrap_or(&address);
    let language = format!("{locale}/");
    let rest = if locale == config.default_language() {
        rest
    } else {
        rest.strip_prefix(&language).unwrap_or(rest)
    };
    Ok(rest.to_owned())
}

/// A document's body as HTML, with raw HTML escaped, maths drawn as `MathML`
/// and each wikilink a link to the page `links` gives it, by
/// `khalid_content::render_linked`, and each link given the site's link
/// policy. `main` has checked every wikilink reaches a page, so none falls
/// back to plain text.
pub(crate) fn body_html<F>(
    config: &Config,
    links: &Wikilinks,
    doc: &Document<F>,
) -> Result<BodyHtml, UserMessage> {
    render_linked(&doc.body, &doc.locale, config.slugs(), links)
        .map(|rendered| {
            BodyHtml::from_sanitised(build_seo::with_link_policy(config, rendered.html))
        })
        .map_err(|error| build_message::block(&doc.path, &error))
}

/// The head of a page at `path`, indexable, with no alternates yet.
pub(crate) fn meta(
    config: &Config,
    title: String,
    description: String,
    path: String,
    kind: PageKind,
) -> Result<PageMeta, UserMessage> {
    let address = format!("{}{path}", config.address().as_str().trim_end_matches('/'));
    let canonical = AbsoluteUrl::parse(&address).map_err(|_| build_message::address(&address))?;
    Ok(PageMeta {
        title,
        description,
        canonical,
        path,
        kind,
        robots: Robots::Index,
        image: None,
        alternates: Vec::new(),
        x_default: None,
        seo_head: None,
    })
}

/// A page whose path ends in `rest`, written to `rest/index.html`.
fn page(
    locale: &Locale,
    config: &Config,
    rest: &str,
    meta: PageMeta,
    body: PageBody,
    seo: SeoPage,
) -> Page {
    Page {
        file: format!("{}{rest}index.html", prefix(config, locale)),
        meta,
        body,
        seo,
        source: None,
        template: None,
        head_code: None,
        lead: None,
    }
}

/// The site as pages in `locale` show it, linking the stylesheet at `stylesheet`.
pub(crate) fn render_site(config: &Config, locale: &Locale, stylesheet: &str) -> RenderSite {
    RenderSite {
        name: config.name().to_owned(),
        locale: locale.clone(),
        home: path_of(config, locale, ""),
        stylesheet: stylesheet.to_owned(),
        nav: Nav::default(),
        main_site: None,
        share_image: None,
        features: build_reader::features(config.reader()),
        search: None,
        script: None,
    }
}

/// The path of `rest` in `locale`: the mount, the language prefix for any
/// language but the default, then `rest`, the rule `khalid_seo::SeoSite::url_for`
/// follows, so `why-links-matter/` in German is `/blog/de/why-links-matter/`.
pub(crate) fn path_of(config: &Config, locale: &Locale, rest: &str) -> String {
    format!("{}{}{rest}", config.mount(), prefix(config, locale))
}

/// `de/` for a language other than the default, and nothing for the default.
pub(crate) fn prefix(config: &Config, locale: &Locale) -> String {
    if locale == config.default_language() {
        String::new()
    } else {
        format!("{}/", locale.as_str().to_ascii_lowercase())
    }
}

#[cfg(test)]
#[path = "build_pages_tests.rs"]
mod tests;
