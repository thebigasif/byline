//! What a Ghost import would write, worked out before anything is
//! written, so a dry run and the real run see the same thing (ADR "an
//! import maps every old address").

use std::path::PathBuf;

use khalid_content::ContentError;
use khalid_core::{AbsoluteUrl, Config, ParseSlugError, Slug};
use khalid_routes::{AddressError, Routes};

use crate::ghost::post_file::{Source, post_file};
use crate::ghost::{BodyError, Export, Post, PostKind, PostStatus, Visibility};
use crate::{Place, PlannedFile, SlugOrigin};

/// Everything an import of one Ghost export would write.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct Plan {
    /// The files to write, each content file read back by `khalid-content`
    /// already.
    pub files: Vec<PlannedFile>,
    /// What became of each post and page that came in, in export order.
    pub posts: Vec<ImportedPost>,
    /// Each post that could not come in, with the reason.
    pub failed: Vec<FailedPost>,
    /// Each post that stays behind, with the reason, in export order.
    pub left_behind: Vec<LeftPost>,
}

/// A post or page that came in.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ImportedPost {
    /// Its id in the export.
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
}

/// A post that stays behind.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LeftPost {
    /// Its id in the export.
    pub id: String,
    /// Why it stays behind.
    pub reason: LeftReason,
}

/// Why a post stays behind.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum LeftReason {
    /// A newsletter that was only ever emailed, with no page on the site.
    Newsletter,
    /// A page that was never published; drafts are kept for posts only.
    UnpublishedPage,
    /// A page for members only. Khalid has no members' pages yet, and
    /// publishing it would show it to everyone.
    MembersPage(Visibility),
    /// A status Khalid has no place for, by its name.
    Status(String),
}

/// A post that could not come in.
#[derive(Debug)]
#[non_exhaustive]
pub struct FailedPost {
    /// Its id in the export.
    pub id: String,
    /// Why it could not come in.
    pub error: PostError,
}

/// Why a post could not come in.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PostError {
    /// A published post has no date, so it has no file name.
    #[error("the post was published but has no date")]
    NoDate,
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

/// Works out the files an import of `export`, from the Ghost site at
/// `old_site`, into `site` with its `routes` would write. Nothing is
/// written; every content file is read back with `khalid-content` first,
/// and a post whose file does not read back is listed in
/// [`Plan::failed`].
///
/// Published public posts go under `content/<lang>/posts/`. Drafts,
/// scheduled posts and posts for members only go under `drafts/<lang>/`,
/// so nothing a reader paid for is published. Published public pages go
/// under `content/<lang>/pages/`. The export does not say where the old
/// site lived, so `old_site` fills in Ghost's `__GHOST_URL__`.
pub fn plan(export: &Export, old_site: &AbsoluteUrl, site: &Config, routes: &Routes) -> Plan {
    let from = Source {
        origin: old_site.as_str().trim_end_matches('/'),
        site,
        routes,
    };
    let mut plan = Plan::default();
    for post in &export.posts {
        let id = post.id.clone();
        match place_of(post).map(|place| post_file(post, from, place)) {
            Err(reason) => plan.left_behind.push(LeftPost { id, reason }),
            Ok(Err(error)) => plan.failed.push(FailedPost { id, error }),
            Ok(Ok((file, imported))) => {
                plan.files.push(file);
                plan.posts.push(imported);
            }
        }
    }
    plan
}

/// Where a post's file goes, or why it stays behind.
fn place_of(post: &Post) -> Result<Place, LeftReason> {
    let public = post.visibility == Visibility::Public;
    match (&post.kind, &post.status) {
        (PostKind::Post, PostStatus::Published) if public => Ok(Place::Post),
        (PostKind::Post, PostStatus::Published | PostStatus::Draft | PostStatus::Scheduled) => {
            Ok(Place::Draft)
        }
        (PostKind::Page, PostStatus::Published) if public => Ok(Place::Page),
        (PostKind::Page, PostStatus::Published) => {
            Err(LeftReason::MembersPage(post.visibility.clone()))
        }
        (PostKind::Page, PostStatus::Draft | PostStatus::Scheduled) => {
            Err(LeftReason::UnpublishedPage)
        }
        (_, PostStatus::Sent) => Err(LeftReason::Newsletter),
        (_, PostStatus::Other(status)) => Err(LeftReason::Status(status.clone())),
    }
}
