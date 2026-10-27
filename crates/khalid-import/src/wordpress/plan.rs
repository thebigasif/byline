//! What an import would write, worked out before anything is written, so
//! a dry run and the real run see the same thing.

use std::path::PathBuf;

use chrono::NaiveDateTime;
use khalid_content::ContentError;
use khalid_core::{AbsoluteUrl, Config, ParseSlugError, RobotsDirectives, Slug};

use crate::wordpress::item_file::{Place, item_file};
use crate::wordpress::site_files::{author_files, categories_file};
use crate::wordpress::{Export, ItemKind, ItemStatus, SlugOrigin, UnreadField};

/// Everything an import of one export would write.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct Plan {
    /// The files to write, each read back by `khalid-content` already.
    pub files: Vec<PlannedFile>,
    /// What became of each item that came in, in export order.
    pub items: Vec<ImportedItem>,
    /// Each item that could not come in, with the reason.
    pub failed: Vec<FailedItem>,
}

/// One file to write.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PlannedFile {
    /// The path from the site folder, such as `content/en/posts/...`.
    pub path: PathBuf,
    /// The whole file.
    pub text: String,
}

/// An item that came in.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ImportedItem {
    /// Its id in the export.
    pub id: u64,
    /// The file it became, from the site folder.
    pub path: PathBuf,
    /// Its slug in Khalid.
    pub slug: Slug,
    /// Whether that is the slug it had.
    pub slug_origin: SlugOrigin,
    /// What it had that Khalid's front matter cannot hold yet, in a fixed
    /// order, so the report can say so.
    pub held: Vec<Held>,
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
}

/// Works out the files an import of `export` into `site` would write.
/// Nothing is written; every file is read back with `khalid-content` first,
/// and an item whose file does not read back is listed in
/// [`Plan::failed`].
///
/// Published posts go under `content/<lang>/posts/`, drafts, posts waiting
/// for review and scheduled posts under `drafts/<lang>/`, and published
/// pages under `content/<lang>/pages/`. Each author gets a file, and the
/// categories go into `categories.toml`.
pub fn plan(export: &Export, site: &Config) -> Plan {
    let mut plan = Plan::default();
    for item in &export.items {
        let place = match (&item.kind, &item.status) {
            (ItemKind::Post, ItemStatus::Publish) => Place::Post,
            (ItemKind::Post, ItemStatus::Draft | ItemStatus::Pending | ItemStatus::Future) => {
                Place::Draft
            }
            (ItemKind::Page, ItemStatus::Publish) => Place::Page,
            _ => continue,
        };
        match item_file(item, export, site, place) {
            Ok((file, imported)) => {
                plan.files.push(file);
                plan.items.push(imported);
            }
            Err(error) => plan.failed.push(FailedItem { id: item.id, error }),
        }
    }
    plan.files.extend(author_files(export, site));
    plan.files.extend(categories_file(export, site));
    plan
}
