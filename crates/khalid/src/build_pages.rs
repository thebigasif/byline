//! The pages `khalid build` renders: the site's content turned into the input
//! records of `khalid-render` (ADR 0009), one language at a time.

use std::cmp::Reverse;

use khalid_content::{Document, PostFront, SiteContent, render};
use khalid_core::{AbsoluteUrl, Config, Locale, Slug, UserMessage};
use khalid_render::{
    BodyHtml, Link, Nav, PageBody, PageKind, PageMeta, PostCard, RenderListing, RenderPost,
    RenderSite, Robots,
};

use crate::build_message;

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
}

/// Every page of the site, per language in the order of `site.toml`: each
/// dated post, the home page with the posts newest first, and the 404 page.
pub(crate) fn pages(
    config: &Config,
    content: &SiteContent,
) -> Result<Vec<LanguagePages>, UserMessage> {
    config
        .languages()
        .iter()
        .map(|locale| language_pages(config, content, locale))
        .collect()
}

fn language_pages(
    config: &Config,
    content: &SiteContent,
    locale: &Locale,
) -> Result<LanguagePages, UserMessage> {
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
        let (title, slug) = title_and_slug(config, doc)?;
        let rest = format!("{slug}/");
        let path = path_of(config, locale, &rest);
        let rendered = render(&doc.body, locale, config.slugs())
            .map_err(|error| build_message::block(&doc.path, &error))?;
        let description = doc.front.description.clone().unwrap_or_default();
        cards.push(PostCard {
            title: title.clone(),
            href: path.clone(),
            date,
            excerpt: description.clone(),
            image: None,
        });
        let post = RenderPost {
            title: title.clone(),
            date,
            updated: None,
            authors: authors(config, content, doc),
            category: None,
            tags: Vec::new(),
            image: None,
            body: BodyHtml::from_sanitised(rendered.html),
        };
        let meta = meta(config, title, description, path, PageKind::Article)?;
        pages.push(page(
            locale,
            config,
            &rest,
            meta,
            PageBody::Post(Box::new(post)),
        ));
    }
    let home = RenderListing {
        title: config.name().to_owned(),
        intro: None,
        posts: cards,
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
    pages.push(page(locale, config, "", home_meta, PageBody::Home(home)));
    pages.push(Page {
        file: format!("{}404.html", prefix(config, locale)),
        meta: missing,
        body: PageBody::NotFound,
    });
    Ok(LanguagePages {
        site: render_site(config, locale),
        pages,
    })
}

/// The post's title, or its slug when it has none, and its slug, or one made
/// from the title when it has none.
fn title_and_slug(
    config: &Config,
    doc: &Document<PostFront>,
) -> Result<(String, Slug), UserMessage> {
    let slug = match (&doc.front.slug, &doc.front.title) {
        (Some(slug), _) => slug.clone(),
        (None, Some(title)) => Slug::from_title(title, &doc.locale, config.slugs())
            .map_err(|_| build_message::no_slug(&doc.path))?,
        (None, None) => return Err(build_message::no_slug(&doc.path)),
    };
    let title = doc.front.title.clone().unwrap_or_else(|| slug.to_string());
    Ok((title, slug))
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
    })
}

/// A page whose path ends in `rest`, written to `rest/index.html`.
fn page(locale: &Locale, config: &Config, rest: &str, meta: PageMeta, body: PageBody) -> Page {
    Page {
        file: format!("{}{rest}index.html", prefix(config, locale)),
        meta,
        body,
    }
}

/// The site as pages in `locale` show it. The stylesheet arrives with task 1.12.
fn render_site(config: &Config, locale: &Locale) -> RenderSite {
    RenderSite {
        name: config.name().to_owned(),
        locale: locale.clone(),
        home: path_of(config, locale, ""),
        stylesheet: format!("{}assets/site.css", config.mount()),
        nav: Nav::default(),
        main_site: None,
        share_image: None,
    }
}

/// The path of `rest` in `locale`: the mount, the language prefix for any
/// language but the default, then `rest`, the rule `khalid_seo::SeoSite::url_for`
/// follows, so `why-links-matter/` in German is `/blog/de/why-links-matter/`.
fn path_of(config: &Config, locale: &Locale, rest: &str) -> String {
    format!("{}{}{rest}", config.mount(), prefix(config, locale))
}

/// `de/` for a language other than the default, and nothing for the default.
fn prefix(config: &Config, locale: &Locale) -> String {
    if locale == config.default_language() {
        String::new()
    } else {
        format!("{}/", locale.as_str().to_ascii_lowercase())
    }
}
