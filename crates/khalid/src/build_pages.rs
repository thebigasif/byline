//! The pages `khalid build` renders: the site's content turned into the input
//! records of `khalid-render` (ADR 0009), one language at a time.

use std::cmp::Reverse;
use std::path::{Path, PathBuf};

use khalid_content::{Document, PostFront, SiteContent, Wikilinks, render_linked};
use khalid_core::{AbsoluteUrl, Config, Locale, Slug, UserMessage};
use khalid_render::{
    BodyHtml, Link, Nav, PageBody, PageKind, PageMeta, PostCard, RenderAuthor, RenderListing,
    RenderPage, RenderPost, RenderSite, Robots,
};
use khalid_routes::{PostAddress, Routes};
use khalid_seo::{AuthorProfile, MetaTemplate, PageKind as SeoKind, SeoPage};

use crate::{build_message, build_seo};

/// The pages of one language, and the site as they show it.
#[derive(Debug)]
pub(crate) struct LanguagePages {
    pub(crate) site: RenderSite,
    pub(crate) pages: Vec<Page>,
}

/// One page and the file it goes to, relative to the output folder.
#[derive(Debug)]
pub(crate) struct Page {
    pub(crate) file: String,
    pub(crate) meta: PageMeta,
    pub(crate) body: PageBody,
    /// The page as the SEO outputs read it, before its language versions
    /// are linked by `build_seo::heads`.
    pub(crate) seo: SeoPage,
    /// The content file it was made from, relative to the site folder, so
    /// `post_files` can find the files beside it. `None` for lists.
    pub(crate) source: Option<PathBuf>,
}

impl Page {
    /// The page, made from the content file at `path`.
    fn made_from(self, path: &Path) -> Self {
        Self {
            source: Some(path.to_path_buf()),
            ..self
        }
    }
}

/// Every page of the site, per language in the order of `site.toml`: each
/// dated post and each glossary term at the address `routes` gives it, each
/// plain page, each author with their posts, the home page with the posts
/// newest first, and the 404 page. Every page links the stylesheet at
/// `stylesheet`.
pub(crate) fn pages(
    config: &Config,
    routes: &Routes,
    content: &SiteContent,
    stylesheet: &str,
) -> Result<Vec<LanguagePages>, UserMessage> {
    config
        .languages()
        .iter()
        .map(|locale| language_pages(config, routes, content, locale, stylesheet))
        .collect()
}

fn language_pages(
    config: &Config,
    routes: &Routes,
    content: &SiteContent,
    locale: &Locale,
    stylesheet: &str,
) -> Result<LanguagePages, UserMessage> {
    let links = Wikilinks::for_language(content, config, routes, locale);
    let (mut pages, cards) = post_pages(config, routes, &links, content, locale)?;
    pages.extend(plain_pages(config, &links, content, locale)?);
    pages.extend(glossary_pages(config, routes, &links, content, locale)?);
    pages.extend(author_pages(config, &links, content, locale, &cards)?);
    let home = RenderListing {
        title: config.name().to_owned(),
        intro: None,
        posts: cards.into_iter().map(|(_, card)| card).collect(),
        pagination: None,
    };
    let home_meta = meta(
        config,
        config.name().to_owned(),
        String::new(),
        path_of(config, locale, ""),
        PageKind::Website,
    )?;
    let mut missing = home_meta.clone();
    missing.robots = Robots::NoIndex;
    let mut home_seo = build_seo::page(SeoKind::Home, locale, "", config.name())?;
    home_seo.translation = build_seo::translation(Some("home"))?;
    pages.push(page(
        locale,
        config,
        "",
        home_meta,
        PageBody::Home(home),
        home_seo,
    ));
    let mut missing_seo = build_seo::page(SeoKind::NotFound, locale, "404.html", config.name())?;
    missing_seo.seo_title = MetaTemplate::parse("{site}").ok();
    pages.push(Page {
        file: format!("{}404.html", prefix(config, locale)),
        meta: missing,
        body: PageBody::NotFound,
        seo: missing_seo,
        source: None,
    });
    Ok(LanguagePages {
        site: render_site(config, locale, stylesheet),
        pages,
    })
}

/// A post's card with the slug of its author, so author pages can list it.
type Card<'a> = (Option<&'a Slug>, PostCard);

/// The page of every dated post in `locale`, and their cards, newest first.
fn post_pages<'a>(
    config: &Config,
    routes: &Routes,
    links: &Wikilinks,
    content: &'a SiteContent,
    locale: &Locale,
) -> Result<(Vec<Page>, Vec<Card<'a>>), UserMessage> {
    let mut dated: Vec<_> = content
        .posts
        .iter()
        .filter(|doc| &doc.locale == locale)
        .filter_map(|doc| doc.front.date.map(|date| (doc, date)))
        .collect();
    dated.sort_by_key(|&(_, date)| Reverse(date));
    let mut pages = Vec::with_capacity(dated.len() + 2);
    let mut cards = Vec::with_capacity(dated.len());
    for (doc, date) in dated {
        let front = &doc.front;
        let (title, slug) = named(config, doc, front.slug.as_ref(), front.title.as_deref())?;
        let entry = PostAddress::new(&doc.collection, locale, &slug, Some(date));
        let rest = routed(config, routes, locale, &entry, &doc.path)?;
        let path = path_of(config, locale, &rest);
        let html = body_html(config, links, doc)?;
        let description = front.description.clone().unwrap_or_default();
        let card = PostCard {
            title: title.clone(),
            href: path.clone(),
            date,
            excerpt: description.clone(),
            image: None,
        };
        cards.push((front.author.as_ref(), card));
        let mut seo = build_seo::page(SeoKind::Post, locale, &rest, &title)?;
        seo.excerpt.clone_from(&description);
        seo.published = Some(build_seo::midnight(date));
        seo.translation = build_seo::translation(front.translation.as_deref())?;
        seo.content_html = full_addresses(config, html.as_str());
        let post = RenderPost {
            title: title.clone(),
            date,
            updated: None,
            authors: authors(config, content, doc),
            category: None,
            tags: Vec::new(),
            image: None,
            body: html,
        };
        seo.author = post.authors.first().map(|author| author.name.clone());
        let meta = meta(config, title, description, path, PageKind::Article)?;
        let body = PageBody::Post(Box::new(post));
        pages.push(page(locale, config, &rest, meta, body, seo).made_from(&doc.path));
    }
    Ok((pages, cards))
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
    cards: &[Card<'_>],
) -> Result<Vec<Page>, UserMessage> {
    let mut pages = Vec::new();
    for doc in content.authors.iter().filter(|doc| &doc.locale == locale) {
        let front = &doc.front;
        let (name, slug) = named(config, doc, front.slug.as_ref(), front.name.as_deref())?;
        let rest = format!("authors/{slug}/");
        let path = path_of(config, locale, &rest);
        let meta = meta(config, name.clone(), String::new(), path, PageKind::Website)?;
        let profiles = front
            .same_as
            .iter()
            .map(|url| Link {
                name: url.host().to_owned(),
                href: url.to_string(),
            })
            .collect();
        let posts: Vec<PostCard> = cards
            .iter()
            .filter(|(author, _)| *author == Some(&slug))
            .map(|(_, card)| card.clone())
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
        };
        let body = PageBody::Author(Box::new(author));
        pages.push(page(locale, config, &rest, meta, body, seo).made_from(&doc.path));
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
fn routed(
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
/// `khalid_content::render_linked`. `main` has checked every wikilink
/// reaches a page, so none falls back to plain text.
fn body_html<F>(
    config: &Config,
    links: &Wikilinks,
    doc: &Document<F>,
) -> Result<BodyHtml, UserMessage> {
    render_linked(&doc.body, &doc.locale, config.slugs(), links)
        .map(|rendered| BodyHtml::from_sanitised(rendered.html))
        .map_err(|error| build_message::block(&doc.path, &error))
}

/// `html` with each `href` and `src` that starts at the site root, such as
/// `/blog/about/`, made a full address on the site's host, for feed readers,
/// which have no page to read such a path against. A body escapes `"` in its
/// text, so `="/` can only end an attribute name.
fn full_addresses(config: &Config, html: &str) -> String {
    let origin = config.address().as_str().trim_end_matches('/');
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some((before, after)) = rest.split_once("=\"/") {
        out.push_str(before);
        out.push_str("=\"");
        let named = before.ends_with(" href") || before.ends_with(" src");
        if named && !after.starts_with('/') {
            out.push_str(origin);
        }
        out.push('/');
        rest = after;
    }
    out.push_str(rest);
    out
}

/// The post's author, named as their file in the same language names them,
/// linked to their author page.
fn authors(config: &Config, content: &SiteContent, doc: &Document<PostFront>) -> Vec<Link> {
    let Some(author) = &doc.front.author else {
        return Vec::new();
    };
    let name = content
        .authors
        .iter()
        .filter(|person| person.locale == doc.locale)
        .find(|person| person.front.slug.as_ref() == Some(author))
        .and_then(|person| person.front.name.clone())
        .unwrap_or_else(|| author.to_string());
    vec![Link {
        name,
        href: path_of(config, &doc.locale, &format!("authors/{author}/")),
    }]
}

/// The head of a page at `path`, indexable, with no alternates yet.
fn meta(
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
    }
}

/// The site as pages in `locale` show it, linking the stylesheet at `stylesheet`.
fn render_site(config: &Config, locale: &Locale, stylesheet: &str) -> RenderSite {
    RenderSite {
        name: config.name().to_owned(),
        locale: locale.clone(),
        home: path_of(config, locale, ""),
        stylesheet: stylesheet.to_owned(),
        nav: Nav::default(),
        main_site: None,
        share_image: None,
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
mod tests {
    use std::path::{Path, PathBuf};

    use chrono::NaiveDate;
    use khalid_content::{ContentTree, read_content};
    use khalid_core::{Config, Environment};
    use khalid_routes::Routes;
    use khalid_seo::{Feed, FeedScope};

    use crate::build_seo;

    fn demo() -> PathBuf {
        let mut path = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
        path.push("../../fixtures/demo-site");
        path
    }

    #[test]
    fn the_english_feed_carries_each_wikilink_as_a_full_address() {
        let site = demo();
        let config = Config::load(&site, &Environment::empty()).unwrap();
        let routes = Routes::load(&site).unwrap();
        let content = read_content(&ContentTree::walk(&site).unwrap()).unwrap();
        let stylesheet = "/blog/assets/site.css";
        let mut languages = super::pages(&config, &routes, &content, stylesheet).unwrap();
        let seo_site = build_seo::site(&config).unwrap();
        let seo_pages = build_seo::heads(&seo_site, &mut languages).unwrap();
        let english = FeedScope::language(config.default_language().clone());
        let now = build_seo::midnight(NaiveDate::default());
        let rss = Feed::for_pages(&seo_site, &seo_pages, &english, now)
            .unwrap()
            .to_rss()
            .unwrap();
        let link = "https://www.example.com/blog/glossary/topical-authority/";
        assert_eq!(rss.matches(link).count(), 2, "{rss}");
        assert!(rss.contains("Topical authority&lt;/a&gt;"), "{rss}");
        assert!(!rss.contains("topical-authority."), "{rss}");
    }

    #[test]
    fn only_addresses_from_the_site_root_become_full() {
        let config = Config::load(&demo(), &Environment::empty()).unwrap();
        let html = "<a href=\"/blog/a/\">a</a><img src=\"/blog/b.png\" alt=\"\">\
                    <a href=\"//cdn.example.org/c\">c</a><a href=\"https://x.org/\">x</a>\
                    <a title=\"/t\" href=\"#d\">d</a><code>href=&quot;/e&quot;</code>";
        assert_eq!(
            super::full_addresses(&config, html),
            "<a href=\"https://www.example.com/blog/a/\">a</a>\
             <img src=\"https://www.example.com/blog/b.png\" alt=\"\">\
             <a href=\"//cdn.example.org/c\">c</a><a href=\"https://x.org/\">x</a>\
             <a title=\"/t\" href=\"#d\">d</a><code>href=&quot;/e&quot;</code>"
        );
    }
}
