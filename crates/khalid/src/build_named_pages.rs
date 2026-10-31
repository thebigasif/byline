//! The pages `khalid build` renders from documents with a name rather than
//! a date: plain pages, glossary terms and authors.

use std::collections::BTreeMap;

use khalid_content::{LeadImage, SiteContent, Wikilinks};
use khalid_core::{AbsoluteUrl, Config, Locale, Slug, UserMessage};
use khalid_render::{Link, PageBody, PageKind, Person, PostCard, RenderAuthor, RenderPage};
use khalid_routes::{PostAddress, Routes};
use khalid_seo::{AuthorProfile, PageKind as SeoKind};

use crate::build_pages::{Page, body_html, meta, named, page, path_of, routed};
use crate::listed_post::ListedPost;
use crate::{build_message, build_seo};

/// The page of every plain page in `locale`, such as About, at `/<slug>/`.
pub(crate) fn plain_pages(
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
pub(crate) fn glossary_pages(
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
pub(crate) fn author_pages(
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
        let meta = meta(
            config,
            name.clone(),
            String::new(),
            path.clone(),
            PageKind::Website,
        )?;
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
        // The author's own page is their `url`, so a reader, and an engine
        // judging the post, can find who stands behind it (P5).
        let address = format!(
            "{}{}",
            config.address().as_str().trim_end_matches('/'),
            path
        );
        profile.url =
            Some(AbsoluteUrl::parse(&address).map_err(|_| build_message::address(&address))?);
        profile.job_title.clone_from(&front.role);
        profile.credentials.clone_from(&front.credentials);
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
