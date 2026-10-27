//! The content file of one Ghost post or page.

use std::path::PathBuf;

use khalid_core::Config;
use khalid_routes::{PostAddress, Routes};

use crate::front_matter::FrontMatter;
use crate::ghost::body::post_body;
use crate::ghost::{ImportedPost, Post, PostError};
use crate::slug::{PostSlug, post_slug};
use crate::{Place, PlannedFile};

/// What every post's file is made for: the old site and the new one.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Source<'a> {
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
    let (path, address) = match place {
        Place::Page => {
            front.maybe("description", description);
            let path = format!("content/{locale}/pages/{slug}.md");
            (path, Some(format!("{}{slug}/", site.mount())))
        }
        Place::Post => {
            let date = post.published.ok_or(PostError::NoDate)?.date();
            let address = PostAddress::new("posts", locale, &slug, Some(date));
            let address = from
                .routes
                .address(site, &address)
                .map_err(PostError::Address)?;
            front
                .text("date", &date.to_string())
                .maybe("description", description);
            (
                format!("content/{locale}/posts/{date}-{slug}.md"),
                Some(address),
            )
        }
        Place::Draft => {
            front.maybe("description", description);
            (format!("drafts/{locale}/{slug}.md"), None)
        }
    };
    if post.featured && place != Place::Page {
        front.yes("featured");
    }
    let path = PathBuf::from(path);
    let text = front.file(&body.markdown);
    khalid_content::parse_post(&path, &text).map_err(PostError::Content)?;
    let imported = ImportedPost {
        id: post.id.clone(),
        address,
        path: path.clone(),
        slug,
        slug_origin: origin,
        place,
    };
    Ok((PlannedFile { path, text }, imported))
}
