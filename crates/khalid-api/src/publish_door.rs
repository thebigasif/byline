//! The one door a post goes live through. The Admin API's edit calls it,
//! and so does every other way to publish, such as the scheduler in
//! `khalid serve`, so a post that goes live is always announced (ADR 0003,
//! CTO answers 2026-10-27 section 4).

use std::fmt;
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::post_store::{PostStore, UpdatePostError};
use crate::resources::{AdminPost, PostEdit, PostStatus};

/// Tells the site's receivers what happened to a post: the webhooks that
/// asked for the event and, where it is switched on, `IndexNow`.
///
/// Each method must return at once and send on a task of its own, so a
/// slow receiver never holds up the edit that caused it.
pub trait Announce: Send + Sync {
    /// `post` went live just now. Called once per post going live.
    fn published(&self, post: &AdminPost);
}

/// Tells nobody, for a site with nowhere to send, and for tests.
#[derive(Debug, Clone, Copy, Default)]
pub struct Silent;

impl Announce for Silent {
    fn published(&self, _post: &AdminPost) {}
}

/// The posts, and who to tell when one goes live.
pub struct PublishDoor {
    posts: Arc<dyn PostStore>,
    announce: Arc<dyn Announce>,
    /// Held through a publishing edit, so two at once cannot both see the
    /// same draft go live and announce it twice.
    publishing: Mutex<()>,
}

impl fmt::Debug for PublishDoor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PublishDoor").finish_non_exhaustive()
    }
}

impl PublishDoor {
    /// The door to `posts`, telling `announce` about each post that goes
    /// live.
    pub fn new(posts: Arc<dyn PostStore>, announce: Arc<dyn Announce>) -> Self {
        Self {
            posts,
            announce,
            publishing: Mutex::new(()),
        }
    }

    /// The posts behind the door, for reading and for new drafts, which
    /// nobody is told about.
    pub fn posts(&self) -> &dyn PostStore {
        self.posts.as_ref()
    }

    /// Applies `edit`, made by `by`, to post `id`, as
    /// [`PostStore::update`] does. When the edit takes the post live, the
    /// receivers are told once it is saved; an edit to a post already live,
    /// or one the store refuses, tells nobody.
    ///
    /// # Errors
    ///
    /// Returns the [`UpdatePostError`] of [`PostStore::update`], and
    /// [`UpdatePostError::Store`] when the post cannot be read first.
    pub async fn edit(
        &self,
        id: &str,
        edit: PostEdit,
        by: &str,
    ) -> Result<AdminPost, UpdatePostError> {
        if !edit.publishes() {
            return self.posts.update(id, edit, by).await;
        }
        let _publishing = self.publishing.lock().await;
        let was_live = self
            .posts
            .get(id)
            .await?
            .is_some_and(|post| post.status() == PostStatus::Published);
        let post = self.posts.update(id, edit, by).await?;
        if !was_live && post.status() == PostStatus::Published {
            self.announce.published(&post);
        }
        Ok(post)
    }
}
