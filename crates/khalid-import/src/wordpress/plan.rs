//! What an import would write, worked out before anything is written, so
//! a dry run and the real run see the same thing.

use std::path::{Path, PathBuf};

use chrono::NaiveDateTime;
use khalid_content::ContentError;
use khalid_core::{AbsoluteUrl, Config, ParseSlugError, RobotsDirectives, Slug};

use crate::wordpress::item_file::{Source, item_file};
use crate::wordpress::redirect_map::{redirect_map, wordpress_csv};
use crate::wordpress::site_files::{author_files, categories_file};
use crate::wordpress::{
    Export, Item, ItemKind, ItemStatus, MapEntry, Redirection, SkippedRule, UnreadField,
};
use crate::write::write_files;
use crate::{PlannedFile, SlugOrigin, WriteError};
use khalid_routes::{AddressError, Routes};

/// Everything an import of one export would write.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct Plan {
    /// The files to write, each read back by `khalid-content` already.
    pub files: Vec<PlannedFile>,
    /// What became of each item that came in, in export order.
    pub items: Vec<ImportedItem>,
    /// Where every old address leads, posts and pages first in export order.
    pub redirects: Vec<MapEntry>,
    /// Each item that could not come in, with the reason.
    pub failed: Vec<FailedItem>,
    /// Each item that stays behind, with the reason, in export order.
    pub left_behind: Vec<LeftItem>,
    /// The Redirection plugin's rules that stay behind, with the reason.
    pub skipped_rules: Vec<SkippedRule>,
    /// How many author files the plan writes.
    pub authors: usize,
    /// How many categories `categories.toml` lists.
    pub categories: usize,
}

/// Where an item's file goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Place {
    /// A published post, under `content/<lang>/posts/`.
    Post,
    /// A post not yet published, under `drafts/<lang>/` (ADR 0030).
    Draft,
    /// A page, under `content/<lang>/pages/`.
    Page,
}

/// An item that stays behind.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LeftItem {
    /// Its id in the export.
    pub id: u64,
    /// Why it stays behind.
    pub reason: LeftReason,
}

/// Why an item stays behind.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum LeftReason {
    /// An uploaded file. Its page is in the redirect map, and the file stays
    /// at its address until the media is fetched.
    Attachment,
    /// Private, seen only by signed-in editors.
    Private,
    /// In the bin.
    Trash,
    /// A page that was never published; drafts are kept for posts only.
    UnpublishedPage,
    /// A kind Khalid has no place for, such as a menu item, by its name.
    Kind(String),
    /// A post with a status Khalid has no place for, by its name.
    Status(String),
}

/// An item that came in.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ImportedItem {
    /// Its id in the export.
    pub id: u64,
    /// Its address from the site root, such as `/blog/rye/`, once it is
    /// published; none for a draft.
    pub address: Option<String>,
    /// The file it became, from the site folder.
    pub path: PathBuf,
    /// Its slug in Khalid.
    pub slug: Slug,
    /// Whether that is the slug it had.
    pub slug_origin: SlugOrigin,
    /// What it had that Khalid's front matter cannot hold yet, in a fixed
    /// order, so the report can say so.
    pub held: Vec<Held>,
    /// Where its file went.
    pub place: Place,
}

/// Something an item had that the file does not hold.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Held {
    /// The title for search results, when it differs from the headline.
    SearchTitle(String),
    /// The tags' slugs.
    Tags(Vec<String>),
    /// The focus keywords, the main one first.
    FocusKeywords(Vec<String>),
    /// The image shown with the post in lists and share cards.
    FeatureImage(AbsoluteUrl),
    /// Categories beyond the one front matter holds, by slug.
    MoreCategories(Vec<String>),
    /// Shortcodes left as text in the body, by name.
    Shortcodes(Vec<String>),
    /// An SEO field that could not be carried as it was.
    Unread(UnreadField),
    /// The time a scheduled post was due to go live; it waits in drafts.
    Scheduled(NaiveDateTime),
    /// A page's robots directives, which page front matter cannot hold.
    Robots(RobotsDirectives),
    /// A page's canonical address, which page front matter cannot hold.
    Canonical(AbsoluteUrl),
}

/// An item that could not come in.
#[derive(Debug)]
#[non_exhaustive]
pub struct FailedItem {
    /// Its id in the export.
    pub id: u64,
    /// Why it could not come in.
    pub error: ItemError,
}

/// Why an item could not come in.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ItemError {
    /// A published post has no date, so it has no file name.
    #[error("the post was published but has no date")]
    NoDate,
    /// No slug could be made.
    #[error("no slug could be made for it")]
    Slug(#[from] ParseSlugError),
    /// The body's HTML could not be read.
    #[error("its body could not be read as HTML")]
    Body(#[source] std::io::Error),
    /// The file the import would write does not read back as content.
    #[error("the file made from it does not read back as content")]
    Content(#[source] ContentError),
    /// Its address could not be made from the site's routes.
    #[error("its address could not be made from the site's routes")]
    Address(#[source] AddressError),
}

/// The redirect map's file, from the site folder, for the host or the
/// redirect manager.
pub const REDIRECTS_CSV: &str = "import/wordpress-redirects.csv";

/// Works out the files an import of `export` into `site`, with its
/// `routes` and the Redirection plugin's rules when there are any, would
/// write. Nothing is written; every content file is read back with
/// `khalid-content` first, and an item whose file does not read back is
/// listed in [`Plan::failed`].
///
/// Published posts go under `content/<lang>/posts/`, drafts, posts waiting
/// for review and scheduled posts under `drafts/<lang>/`, and published
/// pages under `content/<lang>/pages/`. Each author gets a file, and the
/// categories go into `categories.toml`. A published post whose old
/// address lies inside the blog, on the same host, keeps it as an alias;
/// [`Plan::redirects`] says where every old address leads, and
/// [`REDIRECTS_CSV`] holds every entry that redirects (ADR "an import maps
/// every old address").
pub fn plan(
    export: &Export,
    redirection: Option<&Redirection>,
    site: &Config,
    routes: &Routes,
) -> Plan {
    let from = Source {
        export,
        site,
        routes,
    };
    let mut plan = Plan::default();
    for item in &export.items {
        let place = match place_of(item) {
            Ok(place) => place,
            Err(reason) => {
                plan.left_behind.push(LeftItem {
                    id: item.id,
                    reason,
                });
                continue;
            }
        };
        match item_file(item, from, place) {
            Ok((file, imported)) => {
                plan.files.push(file);
                plan.items.push(imported);
            }
            Err(error) => plan.failed.push(FailedItem { id: item.id, error }),
        }
    }
    let authors = author_files(export, site);
    plan.authors = authors.len();
    plan.files.extend(authors);
    if let Some((file, count)) = categories_file(export, site) {
        plan.files.push(file);
        plan.categories = count;
    }
    plan.redirects = redirect_map(export, redirection, &plan.items, site, routes);
    plan.skipped_rules = redirection.map(|r| r.skipped.clone()).unwrap_or_default();
    plan.files.push(PlannedFile {
        path: PathBuf::from(REDIRECTS_CSV),
        text: wordpress_csv(&plan.redirects),
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

/// Where an item's file goes, or why it stays behind.
fn place_of(item: &Item) -> Result<Place, LeftReason> {
    match (&item.kind, &item.status) {
        (ItemKind::Post, ItemStatus::Publish) => Ok(Place::Post),
        (ItemKind::Post, ItemStatus::Draft | ItemStatus::Pending | ItemStatus::Future) => {
            Ok(Place::Draft)
        }
        (ItemKind::Page, ItemStatus::Publish) => Ok(Place::Page),
        (ItemKind::Attachment, _) => Err(LeftReason::Attachment),
        (ItemKind::Other(kind), _) => Err(LeftReason::Kind(kind.clone())),
        (_, ItemStatus::Private) => Err(LeftReason::Private),
        (_, ItemStatus::Trash) => Err(LeftReason::Trash),
        (ItemKind::Page, _) => Err(LeftReason::UnpublishedPage),
        (_, ItemStatus::Other(status)) => Err(LeftReason::Status(status.clone())),
        (_, status) => Err(LeftReason::Status(format!("{status:?}").to_lowercase())),
    }
}
