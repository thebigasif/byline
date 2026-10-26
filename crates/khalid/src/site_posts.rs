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
use khalid_api::content_store::BoxFuture;
use khalid_api::post_store::{CreatePostError, PostStore, PostStoreError, UpdatePostError};
use khalid_api::resources::{AdminPost, PostDraft, PostEdit, PostStatus};
use khalid_content::{ContentTree, SiteContent, parse_post, read_content};
use khalid_core::{Clock, Config, Locale, Slug};
use khalid_lint::{Linter, Verdict};

use crate::lint_input::{PostParts, lint_parts, lint_site};
use crate::post_text::{PostText, quoted};
use crate::{atomic, lint};

/// The folder of drafts at the top of a site.
const DRAFTS: &str = "drafts";

/// The posts of the site in one folder. Cloning shares it.
#[derive(Debug, Clone)]
pub(crate) struct SitePosts(Arc<Site>);

#[derive(Debug)]
struct Site {
    folder: PathBuf,
    config: Config,
    clock: Box<dyn Clock>,
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
    /// The posts of the site in `folder`, with its checked `config`,
    /// stamping and dating with `clock`. Every step runs on a blocking
    /// thread, since it reads or writes files (RULES.md BL-RUN-7).
    pub(crate) fn new(folder: &Path, config: Config, clock: impl Clock + 'static) -> Self {
        Self(Arc::new(Site {
            folder: folder.to_path_buf(),
            config,
            clock: Box::new(clock),
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

    fn update_now(&self, id: &str, edit: PostEdit, by: &str) -> Result<AdminPost, UpdatePostError> {
        let _writing = self.seen.lock().unwrap_or_else(PoisonError::into_inner);
        let found = self.find(id)?.ok_or(UpdatePostError::NotFound)?;
        let current = Self::admin_post(&found)?;
        if current.updated_at() != edit.updated_at() {
            let updated_at = current.updated_at();
            return Err(UpdatePostError::Stale {
                updated_at,
                updated_by: None,
            });
        }
        let publishes = edit.publishes() && found.status == PostStatus::Draft;
        let edited = current.clone().edited(edit, self.clock.now(), by);
        let mut text = found.text.clone();
        if edited.title() != current.title() {
            text.set("title", &quoted(edited.title()));
        }
        if edited.markdown() != current.markdown() {
            text.set_body(edited.markdown());
        }
        let path = if publishes {
            self.publish(&found, &mut text)?
        } else {
            atomic::replace(&found.path, &text.to_text()).map_err(PostStoreError::new)?;
            found.path.clone()
        };
        let status = if publishes {
            PostStatus::Published
        } else {
            found.status
        };
        let saved = Found {
            path,
            status,
            text,
            ..found
        };
        Ok(Self::admin_post(&saved)?.with_updated_by(by))
    }

    /// Dates the draft today, runs the lint publish gate on it as it would
    /// go live, and only when the gate lets it through, writes it under
    /// `content/` and removes the draft. Returns where it now lives.
    fn publish(&self, found: &Found, text: &mut PostText) -> Result<PathBuf, UpdatePostError> {
        let today = self.clock.now().date_naive();
        text.set("date", &today.format("%Y-%m-%d").to_string());
        let name = format!("{today}-{}.md", found.slug);
        let relative = PathBuf::from(format!("content/{}/posts/{name}", found.locale));
        let post = parse_post(&relative, &text.to_text()).map_err(PostStoreError::new)?;
        let content = self.content()?;
        let (site, _) = lint_site(&content, &self.config);
        let parts = PostParts {
            locale: &found.locale,
            path: &relative,
            front: &post.front,
            body: &post.body,
        };
        let candidate = lint_parts(&parts, &self.config)
            .ok_or_else(|| failure("the post cannot be read for linting"))?;
        let settings =
            lint::lint_settings(&self.folder).map_err(|message| failure(&message.to_string()))?;
        let linter = Linter::new(&settings, self.clock.as_ref()).map_err(PostStoreError::new)?;
        if let Verdict::Refuse { report } = linter.gate(&candidate, &site) {
            return Err(UpdatePostError::Refused(report));
        }
        let mut folder = self.folder.clone();
        folder.push(format!("content/{}/posts", found.locale));
        fs::create_dir_all(&folder).map_err(PostStoreError::new)?;
        atomic::create(&folder, &name, &text.to_text()).map_err(PostStoreError::new)?;
        fs::remove_file(&found.path).map_err(PostStoreError::new)?;
        folder.push(name);
        Ok(folder)
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

/// Runs `work` on a blocking thread.
async fn blocking<T, E>(
    site: &Arc<Site>,
    work: impl FnOnce(&Site) -> Result<T, E> + Send + 'static,
) -> Result<T, E>
where
    T: Send + 'static,
    E: From<PostStoreError> + Send + 'static,
{
    let site = Arc::clone(site);
    tokio::task::spawn_blocking(move || work(&site))
        .await
        .map_err(|error| E::from(PostStoreError::new(error)))?
}

impl PostStore for SitePosts {
    fn get<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Result<Option<AdminPost>, PostStoreError>> {
        let id = id.to_owned();
        Box::pin(blocking(&self.0, move |site| site.get_now(&id)))
    }

    fn update<'a>(
        &'a self,
        id: &'a str,
        edit: PostEdit,
        by: &'a str,
    ) -> BoxFuture<'a, Result<AdminPost, UpdatePostError>> {
        let (id, by) = (id.to_owned(), by.to_owned());
        Box::pin(blocking(&self.0, move |site| {
            site.update_now(&id, edit, &by)
        }))
    }

    fn create<'a>(
        &'a self,
        draft: PostDraft,
        by: &'a str,
        key: Option<&'a str>,
    ) -> BoxFuture<'a, Result<AdminPost, CreatePostError>> {
        let (by, key) = (by.to_owned(), key.map(str::to_owned));
        Box::pin(blocking(&self.0, move |site| {
            site.create_now(&draft, &by, key.as_deref())
        }))
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
