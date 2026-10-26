//! The Admin API's posts on a running site: drafts in
//! `drafts/<language>/<slug>.md`, published posts in
//! `content/<language>/posts/`, every write atomic (RULES.md BL-RUN-9), and
//! publishing behind the lint publish gate (ADR "drafts live in drafts/").

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use chrono::{DateTime, NaiveTime, Utc};

use khalid_api::post_store::{CreatePostError, PostStoreError};
use khalid_api::resources::{AdminPost, PostDraft, PostStatus};
use khalid_content::{ContentTree, SiteContent, parse_post, read_content};
use khalid_core::{Config, Locale, Slug};

use crate::atomic;
use crate::post_text::PostText;

/// The folder of drafts at the top of a site.
const DRAFTS: &str = "drafts";

/// The posts of the site in one folder. Cloning shares it.
#[derive(Debug, Clone)]
pub(crate) struct SitePosts(Arc<Site>);

#[derive(Debug)]
struct Site {
    folder: PathBuf,
    config: Config,
    /// Each `Idempotency-Key` seen since start, and the post it made.
    /// Holding the lock also keeps two writes from interleaving.
    seen: Mutex<BTreeMap<String, String>>,
}

/// A post file found by its identity.
struct Found {
    locale: Locale,
    slug: Slug,
    path: PathBuf,
    status: PostStatus,
    text: PostText,
}

impl SitePosts {
    /// The posts of the site in `folder`, with its checked `config`.
    pub(crate) fn new(folder: &Path, config: Config) -> Self {
        Self(Arc::new(Site {
            folder: folder.to_path_buf(),
            config,
            seen: Mutex::new(BTreeMap::new()),
        }))
    }
}

impl Site {
    /// The post with identity `<language>.<slug>`: its draft if there is
    /// one, or else the published post with that slug.
    fn find(&self, id: &str) -> Result<Option<Found>, PostStoreError> {
        let Some((language, slug)) = id.split_once('.') else {
            return Ok(None);
        };
        let locale = self
            .config
            .languages()
            .iter()
            .find(|l| l.as_str() == language);
        let (Some(locale), Ok(slug)) = (locale, Slug::try_new_unicode(slug)) else {
            return Ok(None);
        };
        let draft = self.draft_path(locale, &slug);
        let (path, status) = if draft.is_file() {
            (draft, PostStatus::Draft)
        } else {
            let content = self.content()?;
            let published = content
                .posts
                .iter()
                .find(|doc| &doc.locale == locale && doc.front.slug.as_ref() == Some(&slug));
            let Some(doc) = published else {
                return Ok(None);
            };
            let mut path = self.folder.clone();
            path.push(&doc.path);
            (path, PostStatus::Published)
        };
        let text = fs::read_to_string(&path).map_err(PostStoreError::new)?;
        let text =
            PostText::parse(&text).ok_or_else(|| failure("a post file has no front matter"))?;
        let (locale, slug) = (locale.clone(), slug);
        Ok(Some(Found {
            locale,
            slug,
            path,
            status,
            text,
        }))
    }

    /// The post a found file holds, in Admin API shape. It last changed when
    /// its file did, so an edit by hand also makes an older copy stale.
    fn admin_post(found: &Found) -> Result<AdminPost, PostStoreError> {
        let post = parse_post(&found.path, &found.text.to_text()).map_err(PostStoreError::new)?;
        let changed = fs::metadata(&found.path)
            .and_then(|meta| meta.modified())
            .map_err(PostStoreError::new)?;
        let id = format!("{}.{}", found.locale, found.slug);
        let title = post.front.title.unwrap_or_default();
        let admin = AdminPost::new(
            &id,
            found.slug.clone(),
            &title,
            found.text.markdown(),
            changed.into(),
        )
        .with_author(post.front.author);
        Ok(match (found.status, post.front.date) {
            (PostStatus::Published, Some(date)) => admin.published(
                DateTime::from_naive_utc_and_offset(date.and_time(NaiveTime::MIN), Utc),
            ),
            _ => admin,
        })
    }

    fn get_now(&self, id: &str) -> Result<Option<AdminPost>, PostStoreError> {
        self.find(id)?
            .map(|found| Self::admin_post(&found))
            .transpose()
    }

    fn create_now(
        &self,
        draft: &PostDraft,
        by: &str,
        key: Option<&str>,
    ) -> Result<AdminPost, CreatePostError> {
        let mut seen = self.seen.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(found) = key
            .and_then(|key| seen.get(key))
            .map(|id| self.find(id))
            .transpose()?
            .flatten()
        {
            return Ok(Self::admin_post(&found)?);
        }
        let locale = self.config.default_language();
        let slug = match draft.slug() {
            Some(slug) => slug.clone(),
            None => Slug::from_title(draft.title(), locale, self.config.slugs())
                .map_err(|_| CreatePostError::NoSlug)?,
        };
        let id = format!("{locale}.{slug}");
        if self.find(&id)?.is_some() {
            return Err(CreatePostError::SlugTaken(slug));
        }
        let text = PostText::draft(draft.title(), &slug, draft.author(), draft.markdown());
        let path = self.draft_path(locale, &slug);
        let folder = path
            .parent()
            .ok_or_else(|| failure("a draft has no folder"))?;
        fs::create_dir_all(folder).map_err(PostStoreError::new)?;
        atomic::create(folder, &format!("{slug}.md"), &text.to_text())
            .map_err(PostStoreError::new)?;
        if let Some(key) = key {
            seen.insert(key.to_owned(), id);
        }
        let found = Found {
            locale: locale.clone(),
            slug,
            path,
            status: PostStatus::Draft,
            text,
        };
        Ok(Self::admin_post(&found)?.with_updated_by(by))
    }

    fn draft_path(&self, locale: &Locale, slug: &Slug) -> PathBuf {
        let mut path = self.folder.clone();
        path.push(format!("{DRAFTS}/{locale}/{slug}.md"));
        path
    }

    fn content(&self) -> Result<SiteContent, PostStoreError> {
        ContentTree::walk(&self.folder)
            .and_then(|tree| read_content(&tree))
            .map_err(PostStoreError::new)
    }
}

/// Something about the posts that is not one of their files' own errors.
#[derive(Debug)]
struct Failure(String);

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for Failure {}

fn failure(what: &str) -> PostStoreError {
    PostStoreError::new(Failure(what.to_owned()))
}

#[cfg(test)]
#[path = "site_posts_tests.rs"]
mod tests;
