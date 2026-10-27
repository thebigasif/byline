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
use khalid_api::content_store::{BoxFuture, Listed};
use khalid_api::post_store::{CreatePostError, PostStore, PostStoreError, UpdatePostError};
use khalid_api::query::ListQuery;
use khalid_api::resources::{AdminPost, PostDraft, PostEdit, PostRow, PostStatus};
use khalid_content::{ContentTree, SiteContent, parse_post, read_content};
use khalid_core::{Clock, Config, Locale, Slug};
use khalid_lint::{LintPost, LintSite, Linter, Report, Verdict};

use crate::lint_input::{PostParts, lint_parts, lint_site};
use crate::post_text::{PostText, quoted};
use crate::settings_text::{put, settings_of, write_settings};
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

/// The linter and the site view a post is judged against.
struct Judge {
    linter: Linter,
    site: LintSite,
}

/// A post file found by its identity.
struct Found {
    locale: Locale,
    slug: Slug,
    path: PathBuf,
    status: PostStatus,
    text: PostText,
}

impl Found {
    /// The post file at `path`, read.
    fn read(
        locale: Locale,
        slug: Slug,
        path: PathBuf,
        status: PostStatus,
    ) -> Result<Self, PostStoreError> {
        let text = fs::read_to_string(&path).map_err(PostStoreError::new)?;
        let text =
            PostText::parse(&text).ok_or_else(|| failure("a post file has no front matter"))?;
        Ok(Self {
            locale,
            slug,
            path,
            status,
            text,
        })
    }
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
        Found::read(locale.clone(), slug, path, status).map(Some)
    }

    /// The post a found file holds, in Admin API shape. It last changed when
    /// its file did, so an edit by hand also makes an older copy stale.
    fn admin_post(found: &Found) -> Result<AdminPost, PostStoreError> {
        let post = parse_post(&found.path, &found.text.to_text()).map_err(PostStoreError::new)?;
        let changed = fs::metadata(&found.path)
            .and_then(|meta| meta.modified())
            .map_err(PostStoreError::new)?;
        let id = format!("{}.{}", found.locale, found.slug);
        let settings = settings_of(&post.front);
        let title = post.front.title.unwrap_or_default();
        let admin = AdminPost::new(
            &id,
            found.slug.clone(),
            &title,
            found.text.markdown(),
            changed.into(),
        )
        .with_author(post.front.author)
        .with_settings(settings);
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
        let unpublishes =
            edit.status() == Some(PostStatus::Draft) && found.status == PostStatus::Published;
        let edited = current.clone().edited(edit, self.clock.now(), by);
        let mut text = found.text.clone();
        if edited.title() != current.title() {
            text.set("title", &quoted(edited.title()));
        }
        if edited.markdown() != current.markdown() {
            text.set_body(edited.markdown());
        }
        if edited.author() != current.author() {
            put(
                &mut text,
                "author",
                edited.author().map(ToString::to_string),
            );
        }
        write_settings(
            &mut text,
            current.settings(),
            edited.settings(),
            found.status,
        )?;
        let found = if edited.slug() == current.slug() {
            found
        } else {
            self.moved(found, edited.slug(), &mut text)?
        };
        let (path, status) = if publishes {
            (self.publish(&found, &mut text)?, PostStatus::Published)
        } else if unpublishes {
            (self.unpublish(&found, &text)?, PostStatus::Draft)
        } else {
            atomic::replace(&found.path, &text.to_text()).map_err(PostStoreError::new)?;
            (found.path.clone(), found.status)
        };
        let saved = Found {
            path,
            status,
            text,
            ..found
        };
        Ok(Self::admin_post(&saved)?.with_updated_by(by))
    }

    /// Moves the draft `found` to its file for `slug`, with `text`, which
    /// gains the new slug. Its identity changes with the slug.
    fn moved(
        &self,
        found: Found,
        slug: &Slug,
        text: &mut PostText,
    ) -> Result<Found, UpdatePostError> {
        if found.status != PostStatus::Draft {
            let why = "A live post's address cannot change yet, because links to the old one would break.";
            let setting = "slug";
            return Err(UpdatePostError::Unsupported { setting, why });
        }
        if self.find(&format!("{}.{slug}", found.locale))?.is_some() {
            return Err(UpdatePostError::SlugTaken(slug.clone()));
        }
        text.set("slug", slug.as_str());
        let path = self.draft_path(&found.locale, slug);
        let folder = path
            .parent()
            .ok_or_else(|| failure("a draft has no folder"))?;
        atomic::create(folder, &format!("{slug}.md"), &text.to_text())
            .map_err(PostStoreError::new)?;
        fs::remove_file(&found.path).map_err(PostStoreError::new)?;
        let slug = slug.clone();
        Ok(Found {
            slug,
            path,
            ..found
        })
    }

    /// Dates the draft today, runs the lint publish gate on it as it would
    /// go live, and only when the gate lets it through, writes it under
    /// `content/` and removes the draft. Returns where it now lives.
    fn publish(&self, found: &Found, text: &mut PostText) -> Result<PathBuf, UpdatePostError> {
        let (name, relative) = self.dated(found, text);
        if let Verdict::Refuse { report } = self.gate(found, text, &relative)? {
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

    /// Takes a live post down: writes `text` as its draft in `drafts/`,
    /// then removes it from `content/`. Its date stays in the front matter
    /// until it is published again. Returns where it now lives.
    fn unpublish(&self, found: &Found, text: &PostText) -> Result<PathBuf, PostStoreError> {
        let path = self.draft_path(&found.locale, &found.slug);
        let folder = path
            .parent()
            .ok_or_else(|| failure("a draft has no folder"))?;
        fs::create_dir_all(folder).map_err(PostStoreError::new)?;
        let name = format!("{}.md", found.slug);
        atomic::create(folder, &name, &text.to_text()).map_err(PostStoreError::new)?;
        fs::remove_file(&found.path).map_err(PostStoreError::new)?;
        Ok(path)
    }

    /// Dates a draft's `text` today, as publishing does, and returns the
    /// file name and site path it would go live at.
    fn dated(&self, found: &Found, text: &mut PostText) -> (String, PathBuf) {
        let today = self.clock.now().date_naive();
        text.set("date", &today.format("%Y-%m-%d").to_string());
        let name = format!("{today}-{}.md", found.slug);
        let relative = PathBuf::from(format!("content/{}/posts/{name}", found.locale));
        (name, relative)
    }

    /// The lint publish gate's verdict on `text` as the post at `relative`,
    /// among the site's other posts.
    fn gate(
        &self,
        found: &Found,
        text: &PostText,
        relative: &Path,
    ) -> Result<Verdict, PostStoreError> {
        let judge = self.judge(&self.content()?)?;
        self.gate_with(&judge, found, text, relative)
    }

    /// The linter and the view of `content` every post is judged against,
    /// made once for a whole list.
    fn judge(&self, content: &SiteContent) -> Result<Judge, PostStoreError> {
        let (site, _) = lint_site(content, &self.config);
        let settings =
            lint::lint_settings(&self.folder).map_err(|message| failure(&message.to_string()))?;
        let linter = Linter::new(&settings, self.clock.as_ref()).map_err(PostStoreError::new)?;
        Ok(Judge { linter, site })
    }

    /// [`Site::gate`] with a `judge` already made.
    fn gate_with(
        &self,
        judge: &Judge,
        found: &Found,
        text: &PostText,
        relative: &Path,
    ) -> Result<Verdict, PostStoreError> {
        let candidate = self.candidate(found, text, relative)?;
        Ok(judge.linter.gate(&candidate, &judge.site))
    }

    /// `text` as the linter reads the post at `relative`.
    fn candidate(
        &self,
        found: &Found,
        text: &PostText,
        relative: &Path,
    ) -> Result<LintPost, PostStoreError> {
        let post = parse_post(relative, &text.to_text()).map_err(PostStoreError::new)?;
        let parts = PostParts {
            locale: &found.locale,
            path: relative,
            front: &post.front,
            body: &post.body,
        };
        lint_parts(&parts, &self.config)
            .ok_or_else(|| failure("the post cannot be read for linting"))
    }

    /// `found` as the linter judges it now: a draft as it would go live
    /// today, a published post as it is.
    fn candidate_now(&self, found: &Found) -> Result<LintPost, PostStoreError> {
        let mut text = found.text.clone();
        let relative = match found.status {
            PostStatus::Draft => self.dated(found, &mut text).1,
            _ => found
                .path
                .strip_prefix(&self.folder)
                .map_err(PostStoreError::new)?
                .to_path_buf(),
        };
        self.candidate(found, &text, &relative)
    }

    /// What the publish gate finds in `found` now, writing nothing.
    fn report(&self, judge: &Judge, found: &Found) -> Result<Report, PostStoreError> {
        let candidate = self.candidate_now(found)?;
        let (Verdict::Publish { report } | Verdict::Refuse { report }) =
            judge.linter.gate(&candidate, &judge.site);
        Ok(report)
    }

    /// What the publish gate finds in post `id` now, writing nothing, or
    /// `None` when no post has that identity.
    fn lint_now(&self, id: &str) -> Result<Option<Report>, PostStoreError> {
        let Some(found) = self.find(id)? else {
            return Ok(None);
        };
        let judge = self.judge(&self.content()?)?;
        self.report(&judge, &found).map(Some)
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

impl SitePosts {
    /// What the lint publish gate finds in post `id` now, on a blocking
    /// thread, writing nothing. `khalid mcp`'s `lint_post` calls it; the
    /// Admin API has no lint route yet.
    ///
    /// # Errors
    ///
    /// Returns [`PostStoreError`] when the posts or the lint settings
    /// cannot be read.
    pub(crate) async fn lint(&self, id: &str) -> Result<Option<Report>, PostStoreError> {
        let id = id.to_owned();
        blocking(&self.0, move |site| site.lint_now(&id)).await
    }
}

impl PostStore for SitePosts {
    fn get<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Result<Option<AdminPost>, PostStoreError>> {
        let id = id.to_owned();
        Box::pin(blocking(&self.0, move |site| site.get_now(&id)))
    }

    fn list<'a>(
        &'a self,
        query: &'a ListQuery,
    ) -> BoxFuture<'a, Result<Listed<PostRow>, PostStoreError>> {
        let query = query.clone();
        Box::pin(blocking(&self.0, move |site| site.list_now(&query)))
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

#[path = "site_posts_list.rs"]
mod list;

#[cfg(test)]
#[path = "site_posts_tests.rs"]
mod tests;
