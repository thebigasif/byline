//! What an import of a markdown folder would write, worked out before
//! anything is written, so a dry run and the real run see the same thing
//! (ADR "an import maps every old address").

use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use khalid_content::ContentError;
use khalid_core::{AbsoluteUrl, Config, ParseSlugError, Slug};
use khalid_routes::{AddressError, PostAddress, Routes};

use crate::folder::body::folder_body;
use crate::folder::read::date_of;
use crate::folder::{BodyError, Export, SourceFile, UnreadFile};
use crate::front_matter::FrontMatter;
use crate::slug::{PostSlug, post_slug};
use crate::write::write_files;
use crate::{Place, PlannedFile, SlugOrigin, WriteError};

/// Everything an import of one markdown folder would write.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct Plan {
    /// The files to write, each read back by `khalid-content` already.
    pub files: Vec<PlannedFile>,
    /// What became of each file that came in, in path order.
    pub posts: Vec<ImportedPost>,
    /// Each file that could not come in, with the reason.
    pub failed: Vec<FailedPost>,
    /// Each markdown file that could not be read, with the reason.
    pub unread: Vec<UnreadFile>,
}

/// A file that came in.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ImportedPost {
    /// Its path inside the folder.
    pub source: String,
    /// The addresses it had on the old site: its own and its aliases.
    pub old: Vec<AbsoluteUrl>,
    /// Its address from the site root; none for a draft.
    pub address: Option<String>,
    /// The file it became, from the site folder.
    pub path: PathBuf,
    /// Its slug in Khalid.
    pub slug: Slug,
    /// Whether that is the slug it had.
    pub slug_origin: SlugOrigin,
    /// Where its file went.
    pub place: Place,
    /// What was left out, such as template tags and `tags`.
    pub left: Vec<String>,
}

/// A file that could not come in.
#[derive(Debug)]
#[non_exhaustive]
pub struct FailedPost {
    /// Its path inside the folder.
    pub source: String,
    /// Why it could not come in.
    pub error: PostError,
}

/// Why a file could not come in.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PostError {
    /// No slug could be made.
    #[error("no slug could be made for it")]
    Slug(#[from] ParseSlugError),
    /// Its body could not be made into Khalid markdown.
    #[error("its body could not be read")]
    Body(#[source] BodyError),
    /// The file the import would write does not read back as content.
    #[error("the file made from it does not read back as content")]
    Content(#[source] ContentError),
    /// Its address could not be made from the site's routes.
    #[error("its address could not be made from the site's routes")]
    Address(#[source] AddressError),
}

/// Where the files come from and go to: the old site, and the new site
/// with its routes.
type Source<'a> = (&'a AbsoluteUrl, &'a Config, &'a Routes);

/// Works out the files an import of `export`, which was the site at
/// `old_site`, into `site` with its `routes` would write.
///
/// A dated file becomes a post, a file with no date a page, and a draft,
/// or a file under `_drafts/`, a draft. The slug is the front matter's,
/// else the file's name without Jekyll's date, else, for `index.md`, its
/// folder's name. The old address is the front matter's `permalink` or
/// `url`, else the one the generator gives: `/<y>/<m>/<d>/<slug>.html`
/// under `_posts/`, else the file's path without `content/`, `src/pages/`
/// or `src/content/`.
pub fn plan(export: &Export, old_site: &AbsoluteUrl, site: &Config, routes: &Routes) -> Plan {
    let mut plan = Plan {
        unread: export.unread.clone(),
        ..Plan::default()
    };
    for file in &export.files {
        let taken: Vec<&Slug> = plan.posts.iter().map(|post| &post.slug).collect();
        match post_file(file, &taken, (old_site, site, routes)) {
            Ok((planned, imported)) => {
                plan.files.push(planned);
                plan.posts.push(imported);
            }
            Err(error) => plan.failed.push(FailedPost {
                source: file.path.clone(),
                error,
            }),
        }
    }
    plan
}

impl Plan {
    /// Writes every planned file into the site folder at `site`, each one
    /// atomically, and returns the files written, from the site folder.
    ///
    /// # Errors
    ///
    /// Returns [`WriteError::Exists`] or [`WriteError::Path`] before
    /// anything is written, and [`WriteError::Write`] when a file fails.
    pub fn write(&self, site: &Path) -> Result<Vec<PathBuf>, WriteError> {
        write_files(&self.files, site)
    }
}

/// The file's name without its extension or Jekyll's date, its folder's
/// name for an `index.md`, and the date its name starts with.
fn name_of(path: &str) -> (&str, Option<NaiveDate>) {
    let mut parts = path.rsplit('/');
    let file = parts.next().unwrap_or(path);
    let stem = file.rsplit_once('.').map_or(file, |(stem, _)| stem);
    let stem = match stem {
        "index" | "_index" => parts.next().unwrap_or(stem),
        stem => stem,
    };
    match (date_of(stem), stem.get(11..)) {
        (Some(date), Some(rest)) if stem.get(10..11) == Some("-") => (rest, Some(date)),
        _ => (stem, None),
    }
}

/// The file's address on the old site, from the site root.
fn old_path(file: &SourceFile, slug: &str, date: Option<NaiveDate>) -> String {
    if let Some(permalink) = file
        .front
        .permalink
        .as_deref()
        .filter(|p| p.starts_with('/'))
    {
        return permalink.to_owned();
    }
    if let (true, Some(date)) = (file.path.contains("_posts/"), date) {
        return format!("/{}/{slug}.html", date.format("%Y/%m/%d"));
    }
    let path = ["content/", "src/pages/", "src/content/"]
        .iter()
        .find_map(|prefix| file.path.strip_prefix(prefix))
        .unwrap_or(&file.path);
    let path = path.rsplit_once('.').map_or(path, |(path, _)| path);
    let path = path.trim_end_matches("_index").trim_end_matches("index");
    let path = path.trim_end_matches('/');
    let folder = path.rsplit_once('/').map_or("", |(folder, _)| folder);
    match folder {
        "" => format!("/{slug}/"),
        folder => format!("/{folder}/{slug}/"),
    }
}

/// The slug for `file`, whose name and title are `named`: its own or its
/// name's, unless one of `taken` has it, when its whole path makes one.
fn slug_for(
    file: &SourceFile,
    named: (&str, &str),
    taken: &[&Slug],
    site: &Config,
) -> Result<PostSlug, ParseSlugError> {
    let ((name, title), locale) = (named, site.default_language());
    let own = file.front.slug.as_deref().unwrap_or(name);
    let found = post_slug(own, title, name, locale, site.slugs())?;
    if !taken.contains(&&found.slug) {
        return Ok(found);
    }
    let path = file.path.replace(['/', '.', '_'], "-");
    let slug = post_slug(&path, title, name, locale, site.slugs())?.slug;
    let origin = SlugOrigin::Changed {
        was: own.to_owned(),
    };
    Ok(PostSlug { slug, origin })
}

/// The planned file for `file`, whose slug may not be one of `taken`.
fn post_file(
    file: &SourceFile,
    taken: &[&Slug],
    from: Source<'_>,
) -> Result<(PlannedFile, ImportedPost), PostError> {
    let (old_site, site, routes) = from;
    let locale = site.default_language();
    let (name, named_date) = name_of(&file.path);
    let front = &file.front;
    let title = front.title.as_deref().unwrap_or(name);
    let PostSlug { slug, origin } = slug_for(file, (name, title), taken, site)?;
    let converted = folder_body(&file.body, locale, site.slugs()).map_err(PostError::Body)?;
    let date = front.date.or(named_date);
    let mut left = converted.left;
    let draft = front.draft || file.path.contains("_drafts/");
    let mut out = FrontMatter::default();
    out.text("title", title).text("slug", slug.as_str());
    let (place, path, new) = match (draft, date) {
        (true, _) => (Place::Draft, format!("drafts/{locale}/{slug}.md"), None),
        (false, Some(date)) => {
            out.text("date", &date.to_string());
            let new = PostAddress::new("posts", locale, &slug, Some(date));
            (
                Place::Post,
                format!("content/{locale}/posts/{date}-{slug}.md"),
                Some(new),
            )
        }
        (false, None) => {
            let new = PostAddress::new("pages", locale, &slug, None);
            (
                Place::Page,
                format!("content/{locale}/pages/{slug}.md"),
                Some(new),
            )
        }
    };
    let origin_url = old_site.as_str().trim_end_matches('/');
    let old_paths = [old_path(file, name, date)]
        .into_iter()
        .chain(front.aliases.iter().cloned());
    let old: Vec<AbsoluteUrl> = old_paths
        .filter_map(|path| AbsoluteUrl::parse(&format!("{origin_url}{path}")).ok())
        .collect();
    let address = match &new {
        Some(new) => Some(routes.address(site, new).map_err(PostError::Address)?),
        None => None,
    };
    out.maybe("description", front.description.as_ref());
    for (field, values) in [("tags", &front.tags), ("categories", &front.categories)] {
        if !values.is_empty() {
            left.push(field.to_owned());
        }
    }
    let path = PathBuf::from(path);
    let text = out.file(&converted.markdown);
    khalid_content::parse_post(&path, &text).map_err(PostError::Content)?;
    let imported = ImportedPost {
        source: file.path.clone(),
        old: if address.is_some() { old } else { Vec::new() },
        address,
        path: path.clone(),
        slug,
        slug_origin: origin,
        place,
        left,
    };
    Ok((PlannedFile { path, text }, imported))
}
