//! The one door a post goes live through. The Admin API's edit calls it,
//! and so does every other way to publish, such as the scheduler in
//! `khalid serve`, so a post that goes live is always announced (ADR 0003,
//! CTO answers 2026-10-27 section 4). The door also keeps each post's
//! schedule beside the post.

use std::fmt;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use khalid_core::{Clock, SystemClock};
use tokio::sync::Mutex;

use crate::content_store::Listed;
use crate::post_store::{DeletePostError, PostStore, PostStoreError, UpdatePostError};
use crate::preview_link::PreviewSigner;
use crate::query::ListQuery;
use crate::resources::{AdminPost, PostEdit, PostRow, PostStatus};
use crate::schedule_store::{Due, Schedule, ScheduleStore};

/// Tells the site's receivers what happened to a post: the webhooks that
/// asked for the event and, where it is switched on, `IndexNow`.
///
/// Each method must return at once and send on a task of its own, so a
/// slow receiver never holds up the edit that caused it.
pub trait Announce: Send + Sync {
    /// `post` went live just now. Called once per post going live.
    fn published(&self, post: &AdminPost);

    /// `post` was given a new publish time. Nobody is told by default.
    fn scheduled(&self, _post: &AdminPost) {}

    /// `post` was live and was just taken down. Nobody is told by default.
    fn unpublished(&self, _post: &AdminPost) {}
}

/// Tells nobody, for a site with nowhere to send, and for tests.
#[derive(Debug, Clone, Copy, Default)]
pub struct Silent;

impl Announce for Silent {
    fn published(&self, _post: &AdminPost) {}
}

/// Why an edit's schedule cannot be kept. Nothing is saved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ScheduleError {
    /// `status: scheduled` came without `published_at`.
    #[error("status scheduled needs published_at, the time the post goes live")]
    NoTime,
    /// `published_at` came without `status: scheduled`.
    #[error("published_at goes with status scheduled")]
    NotScheduled,
    /// A time has already passed.
    #[error("the time has already passed")]
    Past,
    /// `unpublish_at` is not after `published_at`.
    #[error("unpublish_at must come after published_at")]
    DownBeforeUp,
    /// The post is live already.
    #[error("the post is live already, so it cannot be scheduled to go live")]
    Live,
    /// The door was made without a [`ScheduleStore`].
    #[error("this site keeps no schedules")]
    NoSchedules,
}

/// Who the scheduler's edits are made by, as `updated_by` shows.
pub const SCHEDULER: &str = "scheduler";

/// What [`PublishDoor::run_due`] did about one post whose time had come.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Ran {
    /// The post went live.
    Published(String),
    /// The post came down.
    Unpublished(String),
    /// The linter refused to publish the post. It stays a draft, and its
    /// publish time is dropped.
    Refused(String),
    /// The post changed while it was read, so it is tried next run.
    Waiting(String),
    /// Nothing was left to do: the post was live or down already, or is
    /// gone. Its time is cleared.
    Settled(String),
}

/// The posts, their schedules, and who to tell when one goes live.
pub struct PublishDoor {
    posts: Arc<dyn PostStore>,
    announce: Arc<dyn Announce>,
    schedules: Option<Arc<dyn ScheduleStore>>,
    previews: Option<PreviewSigner>,
    clock: Arc<dyn Clock>,
    /// Held through every edit, so two at once cannot both see the same
    /// draft go live and announce it twice.
    editing: Mutex<()>,
}

impl fmt::Debug for PublishDoor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PublishDoor").finish_non_exhaustive()
    }
}

impl PublishDoor {
    /// The door to `posts`, telling `announce` about each post that goes
    /// live. It keeps no schedules, and refuses an edit that sets one.
    pub fn new(posts: Arc<dyn PostStore>, announce: Arc<dyn Announce>) -> Self {
        Self {
            posts,
            announce,
            schedules: None,
            clock: Arc::new(SystemClock),
            previews: None,
            editing: Mutex::new(()),
        }
    }

    /// The same door, keeping schedules in `schedules` and judging which
    /// times have passed by `clock`.
    #[must_use]
    pub fn with_schedules(
        self,
        schedules: Arc<dyn ScheduleStore>,
        clock: impl Clock + 'static,
    ) -> Self {
        Self {
            schedules: Some(schedules),
            clock: Arc::new(clock),
            ..self
        }
    }

    /// The same door, making preview links with `signer`.
    #[must_use]
    pub fn with_previews(self, signer: PreviewSigner) -> Self {
        let previews = Some(signer);
        Self { previews, ..self }
    }

    /// What makes and checks preview links, if this site has one.
    pub fn previews(&self) -> Option<&PreviewSigner> {
        self.previews.as_ref()
    }

    /// The posts behind the door, for new drafts, which nobody is told
    /// about.
    pub fn posts(&self) -> &dyn PostStore {
        self.posts.as_ref()
    }

    /// One page of the admin's posts table, as [`PostStore::list`] reads
    /// it. A row's status is the one its file holds.
    ///
    /// # Errors
    ///
    /// Returns [`PostStoreError`] when the posts or the lint settings
    /// cannot be read.
    pub async fn list(&self, query: &ListQuery) -> Result<Listed<PostRow>, PostStoreError> {
        self.posts.list(query).await
    }

    /// The post with identity `id`, with its schedule, or `None`.
    ///
    /// # Errors
    ///
    /// Returns [`PostStoreError`] when the post or its schedule cannot be
    /// read.
    pub async fn get(&self, id: &str) -> Result<Option<AdminPost>, PostStoreError> {
        let Some(post) = self.posts.get(id).await? else {
            return Ok(None);
        };
        Ok(Some(post.with_schedule(&self.schedule(id).await?)))
    }

    /// Applies `edit`, made by `by`, to post `id`, as
    /// [`PostStore::update`] does, and keeps the schedule it sets. When the
    /// edit takes the post live, takes it down, or gives it a new publish
    /// time, the receivers are told once it is saved. An edit the store
    /// refuses, or
    /// whose schedule cannot be kept, saves nothing and tells nobody.
    ///
    /// # Errors
    ///
    /// Returns the [`UpdatePostError`] of [`PostStore::update`],
    /// [`UpdatePostError::Schedule`] when the schedule cannot be kept, and
    /// [`UpdatePostError::Store`] when the post or its schedule cannot be
    /// read or written.
    pub async fn edit(
        &self,
        id: &str,
        edit: PostEdit,
        by: &str,
    ) -> Result<AdminPost, UpdatePostError> {
        let _editing = self.editing.lock().await;
        let before = self.posts.get(id).await?.ok_or(UpdatePostError::NotFound)?;
        let current = self.schedule(id).await?;
        let next = next_schedule(before.status(), current, &edit, self.clock.now())
            .map_err(UpdatePostError::Schedule)?;
        let schedules = match (&self.schedules, next == current) {
            (_, true) => None,
            (Some(schedules), false) => Some(schedules),
            (None, false) => return Err(UpdatePostError::Schedule(ScheduleError::NoSchedules)),
        };
        let post = self.posts.update(id, edit.without_schedule(), by).await?;
        if let Some(schedules) = schedules {
            schedules.set(id, next).await.map_err(PostStoreError::new)?;
        }
        let post = post.with_schedule(&next);
        let was_live = before.status() == PostStatus::Published;
        if !was_live && post.status() == PostStatus::Published {
            self.announce.published(&post);
        } else if was_live && post.status() == PostStatus::Draft {
            self.announce.unpublished(&post);
        } else if next.publish_at().is_some() && next.publish_at() != current.publish_at() {
            self.announce.scheduled(&post);
        }
        Ok(post)
    }

    /// Removes post `id` when it has not changed since `updated_at`, as
    /// [`PostStore::delete`] does, and drops its schedule. A post that was
    /// live is announced as taken down.
    ///
    /// # Errors
    ///
    /// Returns the [`DeletePostError`] of [`PostStore::delete`], and
    /// [`DeletePostError::Store`] when the schedule cannot be cleared.
    pub async fn delete(
        &self,
        id: &str,
        updated_at: DateTime<Utc>,
    ) -> Result<AdminPost, DeletePostError> {
        let _editing = self.editing.lock().await;
        let post = self.posts.delete(id, updated_at).await?;
        if let Some(schedules) = &self.schedules {
            let cleared = schedules.set(id, Schedule::default()).await;
            cleared.map_err(PostStoreError::new)?;
        }
        if post.status() == PostStatus::Published {
            self.announce.unpublished(&post);
        }
        Ok(post)
    }

    /// Acts on every time that has come, oldest first: publishes or takes
    /// down each post through [`PublishDoor::edit`], as [`SCHEDULER`], so
    /// the receivers are told as for any edit. A time is cleared only once
    /// its post is live or down, so running again after a restart does
    /// nothing twice. A post the linter refuses stays a draft, and its
    /// time is dropped rather than tried every round.
    ///
    /// # Errors
    ///
    /// Returns [`PostStoreError`] when the posts or the schedules cannot be
    /// read or written; times not reached yet wait for the next run.
    pub async fn run_due(&self) -> Result<Vec<Ran>, PostStoreError> {
        let Some(schedules) = &self.schedules else {
            return Ok(Vec::new());
        };
        let due = schedules.due(self.clock.now()).await;
        let mut ran = Vec::new();
        for (id, what) in due.map_err(PostStoreError::new)? {
            ran.push(self.run_one(id, what).await?);
        }
        Ok(ran)
    }

    /// Publishes post `id` or takes it down, as `what` asks.
    async fn run_one(&self, id: String, what: Due) -> Result<Ran, PostStoreError> {
        let Some(before) = self.posts.get(&id).await? else {
            self.drop_time(&id, what).await?;
            return Ok(Ran::Settled(id));
        };
        let status = match what {
            Due::Publish => PostStatus::Published,
            Due::Unpublish => PostStatus::Draft,
        };
        let edit = PostEdit::status_only(status, before.updated_at());
        match self.edit(&id, edit, SCHEDULER).await {
            Ok(after) if before.status() == after.status() => Ok(Ran::Settled(id)),
            Ok(_) if status == PostStatus::Published => Ok(Ran::Published(id)),
            Ok(_) => Ok(Ran::Unpublished(id)),
            Err(UpdatePostError::Refused(_)) => {
                self.drop_time(&id, what).await?;
                Ok(Ran::Refused(id))
            }
            Err(UpdatePostError::Stale { .. }) => Ok(Ran::Waiting(id)),
            Err(UpdatePostError::Store(error)) => Err(error),
            Err(_) => {
                self.drop_time(&id, what).await?;
                Ok(Ran::Settled(id))
            }
        }
    }

    /// Clears the `what` time of post `id`, keeping the other.
    async fn drop_time(&self, id: &str, what: Due) -> Result<(), PostStoreError> {
        let Some(schedules) = &self.schedules else {
            return Ok(());
        };
        let current = schedules.get(id).await.map_err(PostStoreError::new)?;
        let empty = Schedule::default();
        let kept = match what {
            Due::Publish => current.unpublish_at().map(|at| empty.unpublishing_at(at)),
            Due::Unpublish => current.publish_at().map(|at| empty.publishing_at(at)),
        };
        let next = kept.unwrap_or_default();
        schedules.set(id, next).await.map_err(PostStoreError::new)
    }

    /// The schedule of post `id`, empty when the door keeps none.
    async fn schedule(&self, id: &str) -> Result<Schedule, PostStoreError> {
        match &self.schedules {
            Some(schedules) => schedules.get(id).await.map_err(PostStoreError::new),
            None => Ok(Schedule::default()),
        }
    }
}

/// The schedule after `edit` to a post in `status` with schedule `current`,
/// judged at `now`: `scheduled` sets the publish time, `published` drops it
/// since the post goes live now, and `draft` drops both.
fn next_schedule(
    status: PostStatus,
    current: Schedule,
    edit: &PostEdit,
    now: DateTime<Utc>,
) -> Result<Schedule, ScheduleError> {
    let mut next = match (edit.status(), edit.published_at()) {
        (Some(PostStatus::Scheduled), None) => return Err(ScheduleError::NoTime),
        (Some(PostStatus::Scheduled), Some(_)) if status == PostStatus::Published => {
            return Err(ScheduleError::Live);
        }
        (Some(PostStatus::Scheduled), Some(at)) if at <= now => return Err(ScheduleError::Past),
        (Some(PostStatus::Scheduled), Some(at)) => current.publishing_at(at),
        (_, Some(_)) => return Err(ScheduleError::NotScheduled),
        (Some(PostStatus::Draft), None) => Schedule::default(),
        (Some(PostStatus::Published), None) => match current.unpublish_at() {
            Some(at) => Schedule::default().unpublishing_at(at),
            None => Schedule::default(),
        },
        _ => current,
    };
    if let Some(down) = edit.unpublish_at() {
        if down <= now {
            return Err(ScheduleError::Past);
        }
        if next.publish_at().is_some_and(|up| down <= up) {
            return Err(ScheduleError::DownBeforeUp);
        }
        next = next.unpublishing_at(down);
    }
    Ok(next)
}
