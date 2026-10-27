//! What a Medium import would write, worked out before anything is
//! written, so a dry run and the real run see the same thing (ADR "an
//! import maps every old address").

use std::path::{Path, PathBuf};

use khalid_content::ContentError;
use khalid_core::{AbsoluteUrl, Config, ParseSlugError, Slug};
use khalid_routes::{AddressError, PostAddress, Routes};

use crate::front_matter::FrontMatter;
use crate::medium::body::medium_body;
use crate::medium::redirect_map::{MapEntry, REDIRECTS_CSV, medium_csv, redirect_map};
use crate::medium::site_files::{author_files, author_slug};
use crate::medium::{BodyError, Export, Post, PostStatus, UnreadFile};
use crate::redirects::path_of;
use crate::slug::{PostSlug, post_slug};
use crate::write::write_files;
use crate::{Place, PlannedFile, SlugOrigin, WriteError};

/// Everything an import of one Medium export would write.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct Plan {
    /// The files to write, each content file read back by `khalid-content`
    /// already.
    pub files: Vec<PlannedFile>,
    /// What became of each post that came in, in file name order.
    pub posts: Vec<ImportedPost>,
    /// Each post that could not come in, with the reason.
    pub failed: Vec<FailedPost>,
    /// Each file of the export that is not a post, with the reason.
    pub unread: Vec<UnreadFile>,
    /// Where every old address of a published post leads, in file name
    /// order. [`REDIRECTS_CSV`] holds every entry that moves.
    pub redirects: Vec<MapEntry>,
    /// How many author files the plan writes.
    pub authors: usize,
}

/// A post that came in.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ImportedPost {
    /// Medium's id for it.
    pub id: String,
    /// Its address from the site root, such as `/blog/rye/`, once it is
    /// published; none for a draft.
    pub address: Option<String>,
    /// The file it became, from the site folder.
    pub path: PathBuf,
    /// Its slug in Khalid.
    pub slug: Slug,
    /// Whether that is the slug it had.
    pub slug_origin: SlugOrigin,
    /// Where its file went.
    pub place: Place,
    /// The kinds of content in its body with no place in Khalid, such as
    /// `frame` for a GitHub gist.
    pub left: Vec<String>,
}

/// A post that could not come in.
#[derive(Debug)]
#[non_exhaustive]
pub struct FailedPost {
    /// Medium's id for it.
    pub id: String,
    /// Why it could not come in.
    pub error: PostError,
}

/// Why a post could not come in.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PostError {
    /// No slug could be made.
    #[error("no slug could be made for it")]
    Slug(#[from] ParseSlugError),
    /// Its body could not be made into markdown.
    #[error("its body could not be read")]
    Body(#[source] BodyError),
    /// The file the import would write does not read back as content.
    #[error("the file made from it does not read back as content")]
    Content(#[source] ContentError),
    /// Its address could not be made from the site's routes.
    #[error("its address could not be made from the site's routes")]
    Address(#[source] AddressError),
}

/// Works out the files an import of `export` into `site` with its
/// `routes` would write. Nothing is written; every content file is read
/// back with `khalid-content` first, and a post whose file does not read
/// back is listed in [`Plan::failed`].
///
/// Published posts go under `content/<lang>/posts/` and drafts under
/// `drafts/<lang>/`. The subtitle becomes the description. A post keeps
/// the readable part of its Medium address as its slug, such as `rye`
/// from `.../rye-3f1a8c4e2b9d`; when an earlier post took that slug, it
/// keeps the whole last part, id and all, as Medium does. A draft whose
/// slug, made from its title, is taken gets Medium's id after it.
///
/// Each author of an imported post gets a file under
/// `content/<lang>/authors/`. When a post lived on the new site's own
/// host, inside the blog, its old path goes into its `aliases`, so Khalid
/// answers it; every move is also in [`REDIRECTS_CSV`].
pub fn plan(export: &Export, site: &Config, routes: &Routes) -> Plan {
    let mut plan = Plan {
        unread: export.unread.clone(),
        ..Plan::default()
    };
    for post in &export.posts {
        let taken: Vec<&Slug> = plan.posts.iter().map(|post| &post.slug).collect();
        match post_file(post, &taken, site, routes) {
            Ok((file, imported)) => {
                plan.files.push(file);
                plan.posts.push(imported);
            }
            Err(error) => plan.failed.push(FailedPost {
                id: post.id.clone(),
                error,
            }),
        }
    }
    let authors = author_files(export, &plan.posts, site);
    plan.authors = authors.len();
    plan.files.extend(authors);
    plan.redirects = redirect_map(export, &plan.posts, site);
    plan.files.push(PlannedFile {
        path: PathBuf::from(REDIRECTS_CSV),
        text: medium_csv(&plan.redirects, site),
    });
    plan
}

impl Plan {
    /// Writes every planned file into the site folder at `site`, each one
    /// atomically: to a temporary file in the same folder, synced, then
    /// renamed. Returns the files written, from the site folder.
    ///
    /// # Errors
    ///
    /// Returns [`WriteError::Exists`] when any planned file exists already,
    /// before anything is written; [`WriteError::Path`] when a planned path
    /// leads outside the site folder, also before anything is written; and
    /// [`WriteError::Write`] when a file cannot be written.
    pub fn write(&self, site: &Path) -> Result<Vec<PathBuf>, WriteError> {
        write_files(&self.files, site)
    }
}

/// The file for `post`, whose slug may not be one of `taken`, and what
/// became of it.
fn post_file(
    post: &Post,
    taken: &[&Slug],
    site: &Config,
    routes: &Routes,
) -> Result<(PlannedFile, ImportedPost), PostError> {
    let locale = site.default_language();
    let old = old_slug(post);
    let slug_of = |name: &str| post_slug(name, &post.title, &post.id, locale, site.slugs());
    let mut found = slug_of(old.readable)?;
    if taken.contains(&&found.slug) {
        found = slug_of(old.whole)?;
    }
    if taken.contains(&&found.slug) {
        // Drafts have no address, so two with one title meet here.
        found.slug = slug_of(&format!("{}-{}", found.slug, post.id))?.slug;
    }
    let PostSlug { slug, origin } = found;
    let body = medium_body(&post.body, locale, site.slugs()).map_err(PostError::Body)?;
    let mut front = FrontMatter::default();
    front.text("title", &post.title).text("slug", slug.as_str());
    let date = post.published.map(|at| at.date());
    let mut aliases = Vec::new();
    let (place, path, address) = match (post.status, date) {
        (PostStatus::Published, Some(date)) => {
            let new = PostAddress::new("posts", locale, &slug, Some(date));
            let address = routes.address(site, &new).map_err(PostError::Address)?;
            front.text("date", &date.to_string());
            aliases.extend(alias(post, site, routes, &new));
            let path = format!("content/{locale}/posts/{date}-{slug}.md");
            (Place::Post, path, Some(address))
        }
        _ => (Place::Draft, format!("drafts/{locale}/{slug}.md"), None),
    };
    let author = post
        .author
        .as_ref()
        .and_then(|author| author_slug(author, site));
    front
        .maybe("author", author)
        .maybe("description", post.subtitle.as_ref())
        .list("aliases", &aliases);
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
        left: body.left,
    };
    Ok((PlannedFile { path, text }, imported))
}

/// The post's old path as an alias of `new`, when it lived on the new
/// site's host and the path lies inside the blog.
fn alias(post: &Post, site: &Config, routes: &Routes, new: &PostAddress<'_>) -> Option<String> {
    let old = post.canonical.as_ref()?;
    if old.host() != site.address().host() {
        return None;
    }
    let redirect = routes.redirect(site, path_of(old), new).ok()?;
    Some(redirect.from().to_owned())
}

/// The last part of a post's Medium address, whole and without its id.
struct OldSlug<'a> {
    /// Such as `rye-3f1a8c4e2b9d`.
    whole: &'a str,
    /// Such as `rye`; empty for a draft or a `/p/<id>` address.
    readable: &'a str,
}

/// The slug a post had on Medium, from the last part of its address.
fn old_slug(post: &Post) -> OldSlug<'_> {
    let last = post
        .canonical
        .as_ref()
        .map(AbsoluteUrl::as_str)
        .map(|url| url.split(['?', '#']).next().unwrap_or(url))
        .and_then(|url| url.trim_end_matches('/').rsplit('/').next())
        .unwrap_or("");
    let readable = last
        .strip_suffix(post.id.as_str())
        .and_then(|rest| rest.strip_suffix('-'))
        .unwrap_or(if last == post.id { "" } else { last });
    OldSlug {
        whole: last,
        readable,
    }
}
