//! The comments in memory, for tests.

use std::collections::BTreeMap;
use std::future::{self, Future};
use std::sync::{Mutex, MutexGuard};

use chrono::{DateTime, Utc};

use crate::{
    Comment, CommentId, CommentStatus, CommentStore, CommentStoreError, NewComment, PostFile,
};

/// The comments in memory, for tests, numbered from 1 like database rows.
#[derive(Debug, Default)]
pub struct MemoryCommentStore {
    rows: Mutex<BTreeMap<CommentId, Comment>>,
}

/// A lock was poisoned by a panic in another test thread.
#[derive(Debug, thiserror::Error)]
#[error("the in-memory comments were poisoned by a panic")]
struct Poisoned;

impl MemoryCommentStore {
    fn rows(&self) -> Result<MutexGuard<'_, BTreeMap<CommentId, Comment>>, CommentStoreError> {
        self.rows.lock().map_err(|_| CommentStoreError::Write {
            source: Box::new(Poisoned),
        })
    }

    fn insert_now(
        &self,
        new: NewComment,
        status: CommentStatus,
        at: DateTime<Utc>,
    ) -> Result<Comment, CommentStoreError> {
        let mut rows = self.rows()?;
        let next = rows.last_key_value().map_or(1, |(id, _)| id.get() + 1);
        let comment = Comment {
            id: CommentId::new(next),
            post: new.post,
            parent: new.parent,
            author: new.author,
            text: new.text,
            status,
            created_at: at,
        };
        rows.insert(comment.id, comment.clone());
        Ok(comment)
    }

    fn set_status_now(
        &self,
        id: CommentId,
        status: CommentStatus,
    ) -> Result<Option<Comment>, CommentStoreError> {
        let mut rows = self.rows()?;
        Ok(rows.get_mut(&id).map(|row| {
            row.status = status;
            row.clone()
        }))
    }

    fn delete_now(&self, id: CommentId) -> Result<Option<Comment>, CommentStoreError> {
        let mut rows = self.rows()?;
        rows.retain(|_, row| row.parent != Some(id));
        Ok(rows.remove(&id))
    }

    fn select(&self, keep: impl Fn(&Comment) -> bool) -> Result<Vec<Comment>, CommentStoreError> {
        Ok(self
            .rows()?
            .values()
            .filter(|row| keep(row))
            .cloned()
            .collect())
    }
}

impl CommentStore for MemoryCommentStore {
    fn insert(
        &self,
        new: NewComment,
        status: CommentStatus,
        at: DateTime<Utc>,
    ) -> impl Future<Output = Result<Comment, CommentStoreError>> + Send {
        future::ready(self.insert_now(new, status, at))
    }

    fn get(
        &self,
        id: CommentId,
    ) -> impl Future<Output = Result<Option<Comment>, CommentStoreError>> + Send {
        future::ready(self.rows().map(|rows| rows.get(&id).cloned()))
    }

    fn set_status(
        &self,
        id: CommentId,
        status: CommentStatus,
    ) -> impl Future<Output = Result<Option<Comment>, CommentStoreError>> + Send {
        future::ready(self.set_status_now(id, status))
    }

    fn delete(
        &self,
        id: CommentId,
    ) -> impl Future<Output = Result<Option<Comment>, CommentStoreError>> + Send {
        future::ready(self.delete_now(id))
    }

    fn with_status(
        &self,
        status: CommentStatus,
        post: Option<&PostFile>,
    ) -> impl Future<Output = Result<Vec<Comment>, CommentStoreError>> + Send {
        let on = |row: &Comment| post.is_none_or(|post| &row.post == post);
        future::ready(self.select(|row| row.status == status && on(row)))
    }

    fn newest(
        &self,
        status: Option<CommentStatus>,
        limit: u32,
    ) -> impl Future<Output = Result<Vec<Comment>, CommentStoreError>> + Send {
        let found = self.select(|row| status.is_none_or(|status| row.status == status));
        let limit = usize::try_from(limit).unwrap_or(usize::MAX);
        future::ready(found.map(|rows| rows.into_iter().rev().take(limit).collect()))
    }
}
