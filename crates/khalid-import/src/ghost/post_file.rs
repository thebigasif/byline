//! The content file of one Ghost post or page.

use std::path::PathBuf;

use chrono::NaiveDate;
use khalid_core::{AbsoluteUrl, Config, Slug};
use khalid_routes::{PostAddress, Routes};

use crate::front_matter::FrontMatter;
use crate::ghost::body::post_body;
use crate::ghost::held::{address, author_slugs, held};
use crate::ghost::{Dropped, Export, ImportedPost, Post, PostError};
use crate::slug::{PostSlug, post_slug};
use crate::{Place, PlannedFile};

/// What every post's file is made for: the export, the old site and the
/// new one.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Source<'a> {
    pub(crate) export: &'a Export,
    pub(crate) old_site: &'a AbsoluteUrl,
    /// The old site's address, without a closing slash.
    pub(crate) origin: &'a str,
    pub(crate) site: &'a Config,
    pub(crate) routes: &'a Routes,
}

/// The file for `post` in `place`, and what became of it. Every file is
/// read back as a post first; a page writes only fields a page also has.
pub(crate) fn post_file(
    post: &Post,
    from: Source<'_>,
    place: Place,
) -> Result<(PlannedFile, ImportedPost), PostError> {
    let site = from.site;
    let locale = site.default_language();
    let PostSlug { slug, origin } =
        post_slug(&post.slug, &post.title, &post.id, locale, site.slugs())?;
    let body = post_body(&post.body, from.origin, locale, site.slugs()).map_err(PostError::Body)?;
    let mut front = FrontMatter::default();
    front.text("title", &post.title).text("slug", slug.as_str());
    let description = post.seo.meta_description.as_ref().or(post.excerpt.as_ref());
    let mut authors = author_slugs(post, from).into_iter();
    let (path, address) = match place {
        Place::Page => {
            front.maybe("description", description);
            let path = format!("content/{locale}/pages/{slug}.md");
            (path, Some(format!("{}{slug}/", site.mount())))
        }
        Place::Post | Place::Draft => {
            let date = post.published.map(|at| at.date());
            let address = match (place, date) {
                (Place::Post, Some(date)) => {
                    let new = PostAddress::new("posts", locale, &slug, Some(date));
                    let address = from.routes.address(site, &new);
                    front.text("date", &date.to_string());
                    Some(address.map_err(PostError::Address)?)
                }
                (Place::Post, None) => return Err(PostError::NoDate),
                _ => None,
            };
            front
                .maybe("author", authors.next())
                .maybe("description", description);
            if let Some(alias) = date
                .filter(|_| address.is_some())
                .and_then(|day| alias(post, from, &slug, day))
            {
                front.list("aliases", &[alias]);
            }
            if post.featured {
                front.yes("featured");
            }
            front.maybe(
                "canonical",
                self::address(post.canonical_url.as_ref(), from),
            );
            let path = match (address.is_some(), date) {
                (true, Some(date)) => format!("content/{locale}/posts/{date}-{slug}.md"),
                _ => format!("drafts/{locale}/{slug}.md"),
            };
            (path, address)
        }
    };
    let path = PathBuf::from(path);
    let text = front.file(&body.markdown);
    khalid_content::parse_post(&path, &text).map_err(PostError::Content)?;
    let mut dropped = Vec::new();
    if post.code_injection.head.is_some() || post.code_injection.foot.is_some() {
        dropped.push(Dropped::CodeInjection);
    }
    if !body.left.is_empty() {
        dropped.push(Dropped::Cards(body.left));
    }
    let imported = ImportedPost {
        id: post.id.clone(),
        address,
        path: path.clone(),
        // A page has no author field, and Ghost themes do not show one.
        held: held(
            post,
            from,
            place,
            authors.filter(|_| place != Place::Page).collect(),
        ),
        dropped,
        slug,
        slug_origin: origin,
        place,
    };
    Ok((PlannedFile { path, text }, imported))
}

/// The post's old path as an alias, when the old site and the blog share
/// a host, the path lies inside the blog and the address changed. Ghost's
/// default address for a post is `/<slug>/`.
fn alias(post: &Post, from: Source<'_>, slug: &Slug, date: NaiveDate) -> Option<String> {
    let Source { site, routes, .. } = from;
    if from.old_site.host() != site.address().host() || post.slug.is_empty() {
        return None;
    }
    let new = PostAddress::new("posts", site.default_language(), slug, Some(date));
    let old = format!("/{}/", post.slug);
    let redirect = routes.redirect(site, &old, &new).ok()?;
    Some(redirect.from().to_owned())
}
