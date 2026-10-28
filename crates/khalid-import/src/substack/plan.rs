//! What a Substack import would write, worked out before anything is
//! written, so a dry run and the real run see the same thing (ADR "an
//! import maps every old address").

use std::path::{Path, PathBuf};

use khalid_content::ContentError;
use khalid_core::{AbsoluteUrl, Config, ParseSlugError, Slug};
use khalid_routes::{AddressError, PostAddress, Routes};

use crate::front_matter::FrontMatter;
use crate::redirects::alias;
use crate::slug::{PostSlug, post_slug};
use crate::substack::body::substack_body;
use crate::substack::redirect_map::{MapEntry, REDIRECTS_CSV, redirect_map, substack_csv};
use crate::substack::{Audience, BodyError, Export, Post, PostKind, UnreadRow};
use crate::write::write_files;
use crate::{Place, PlannedFile, SlugOrigin, WriteError};

/// Everything an import of one Substack export would write.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct Plan {
    /// The files to write, each content file read back by `khalid-content`
    /// already.
    pub files: Vec<PlannedFile>,
    /// What became of each post that came in, in the export's order.
    pub posts: Vec<ImportedPost>,
    /// Each post that could not come in, with the reason.
    pub failed: Vec<FailedPost>,
    /// Each post that stays behind, with the reason.
    pub left_behind: Vec<LeftPost>,
    /// Each row of `posts.csv` that could not be read, with the reason.
    pub unread: Vec<UnreadRow>,
    /// Where the old address of every published post leads.
    /// [`REDIRECTS_CSV`] holds every entry that moves.
    pub redirects: Vec<MapEntry>,
}

/// A post that came in.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ImportedPost {
    /// Substack's number for it.
    pub id: String,
    /// Its old address, `<old site>/p/<slug>`, when it was public.
    pub old: Option<AbsoluteUrl>,
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
    /// Whether only signed-in members may read it, as only subscribers
    /// could on Substack.
    pub members: bool,
    /// Kinds of content left out of its body, such as `subscribe box`.
    pub left: Vec<String>,
}

/// A post that stays behind.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LeftPost {
    /// Substack's number for it.
    pub id: String,
    /// Why it stays behind.
    pub reason: LeftReason,
}

/// Why a post stays behind.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum LeftReason {
    /// A discussion thread, which lives in Substack's comments.
    #[error("a discussion thread, which lives in Substack's comments")]
    Thread,
    /// No body, as for a podcast episode with no notes.
    #[error("the export has no text for it")]
    NoBody,
    /// A kind of post Khalid has no place for.
    #[error("a {0} post, which Khalid has no place for")]
    Kind(String),
}

/// A post that could not come in.
#[derive(Debug)]
#[non_exhaustive]
pub struct FailedPost {
    /// Substack's number for it.
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

/// Works out the files an import of `export`, which lived at `old_site`,
/// into `site` with its `routes` would write. Nothing is written; every
/// content file is read back with `khalid-content` first.
///
/// Published posts and pages go under `content/<lang>/`, the rest under
/// `drafts/<lang>/`, each with the slug of its address, `/p/<slug>`. The
/// subtitle becomes the description; a post for subscribers only gets
/// `access: members`. An old address inside the new blog becomes an alias,
/// and every move is in [`REDIRECTS_CSV`].
pub fn plan(export: &Export, old_site: &AbsoluteUrl, site: &Config, routes: &Routes) -> Plan {
    let mut plan = Plan {
        unread: export.unread.clone(),
        ..Plan::default()
    };
    for post in &export.posts {
        let body = match (&post.kind, post.body.as_deref()) {
            (PostKind::Thread, _) => Err(LeftReason::Thread),
            (PostKind::Other(kind), _) => Err(LeftReason::Kind(kind.clone())),
            (_, None) => Err(LeftReason::NoBody),
            (_, Some(body)) => Ok(body),
        };
        let body = match body {
            Ok(body) => body,
            Err(reason) => {
                let id = post.id.clone();
                plan.left_behind.push(LeftPost { id, reason });
                continue;
            }
        };
        let taken: Vec<&Slug> = plan.posts.iter().map(|post| &post.slug).collect();
        match post_file(post, body, &taken, (old_site, site, routes)) {
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
    plan.redirects = redirect_map(&plan.posts, site);
    plan.files.push(PlannedFile {
        path: PathBuf::from(REDIRECTS_CSV),
        text: substack_csv(&plan.redirects, site),
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

/// Where the posts come from and go to: the old site, and the new site with
/// its routes.
type Source<'a> = (&'a AbsoluteUrl, &'a Config, &'a Routes);

/// The file for `post` with its `body`, whose slug may not be one of
/// `taken`, and what became of it.
fn post_file(
    post: &Post,
    body: &str,
    taken: &[&Slug],
    from: Source<'_>,
) -> Result<(PlannedFile, ImportedPost), PostError> {
    let (old_site, site, routes) = from;
    let locale = site.default_language();
    let mut found = post_slug(&post.slug, &post.title, &post.id, locale, site.slugs())?;
    if taken.contains(&&found.slug) {
        let both = format!("{}-{}", found.slug, post.id);
        found.slug = post_slug(&both, &post.title, &post.id, locale, site.slugs())?.slug;
    }
    let PostSlug { slug, origin } = found;
    let converted = substack_body(body, locale, site.slugs()).map_err(PostError::Body)?;
    let mut front = FrontMatter::default();
    front.text("title", &post.title).text("slug", slug.as_str());
    let origin_url = old_site.as_str().trim_end_matches('/');
    let old = AbsoluteUrl::parse(&format!("{origin_url}/p/{}", post.slug)).ok();
    let date = post.date.map(|at| at.date());
    let (place, path, address) = match (post.published, &post.kind, date) {
        (true, PostKind::Page, _) => {
            let new = PostAddress::new("pages", locale, &slug, None);
            let address = routes.address(site, &new).map_err(PostError::Address)?;
            let path = format!("content/{locale}/pages/{slug}.md");
            (Place::Page, path, Some((address, new)))
        }
        (true, _, Some(date)) => {
            let new = PostAddress::new("posts", locale, &slug, Some(date));
            let address = routes.address(site, &new).map_err(PostError::Address)?;
            front.text("date", &date.to_string());
            let path = format!("content/{locale}/posts/{date}-{slug}.md");
            (Place::Post, path, Some((address, new)))
        }
        _ => (Place::Draft, format!("drafts/{locale}/{slug}.md"), None),
    };
    let aliases: Vec<String> = match (&address, &old) {
        (Some((_, new)), Some(old)) => alias(old, site, routes, new).into_iter().collect(),
        _ => Vec::new(),
    };
    let members = !matches!(post.audience, Audience::Everyone);
    front.maybe("description", post.subtitle.as_ref());
    if members {
        front.text("access", "members");
    }
    front.list("aliases", &aliases);
    let path = PathBuf::from(path);
    let text = front.file(&converted.markdown);
    khalid_content::parse_post(&path, &text).map_err(PostError::Content)?;
    let imported = ImportedPost {
        id: post.id.clone(),
        old: old.filter(|_| address.is_some()),
        address: address.map(|(address, _)| address),
        path: path.clone(),
        slug,
        slug_origin: origin,
        place,
        members,
        left: converted.left,
    };
    Ok((PlannedFile { path, text }, imported))
}
