//! Staff comments on drafts on SQLite, for the Admin API (KH-EDT-4).

use std::sync::Arc;

use chrono::{DateTime, SecondsFormat, Utc};
use khalid_api::content_store::BoxFuture;
use khalid_api::draft_comments::{
    DraftComment, DraftCommentStore, DraftCommentStoreError, DraftCommentText, NewDraftComment,
    ThreadState,
};
use khalid_core::Clock;
use sqlx::Row;
use sqlx::sqlite::SqliteRow;

use crate::{Database, DbError, SiteId};

/// One site's comments on drafts in `draft_comments`, a visitor table a
/// rebuild never touches. Every query is bound to the site. Cloning
/// shares the connections.
#[derive(Debug, Clone)]
pub struct DraftCommentTable {
    db: Database,
    site: SiteId,
    clock: Arc<dyn Clock>,
}

impl DraftCommentTable {
    /// The comments on drafts of `site` in `db`, dating new ones with
    /// `clock`.
    pub fn new(db: Database, site: SiteId, clock: impl Clock + 'static) -> Self {
        let clock = Arc::new(clock);
        Self { db, site, clock }
    }

    async fn list_now(&self, post: &str) -> Result<Vec<DraftComment>, DbError> {
        let rows = sqlx::query(
            "SELECT id, post_id, parent_id, block, author, body, resolved, created_at \
             FROM draft_comments WHERE site_id = ?1 AND post_id = ?2 ORDER BY id",
        )
        .bind(self.site.as_row())
        .bind(post)
        .fetch_all(&self.db.readers)
        .await?;
        rows.iter().map(comment).collect()
    }

    async fn add_now(&self, new: NewDraftComment) -> Result<DraftComment, DbError> {
        let at = self.clock.now();
        let stamp = at.to_rfc3339_opts(SecondsFormat::Secs, true);
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO draft_comments (site_id, post_id, parent_id, block, author, body, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) RETURNING id",
        )
        .bind(self.site.as_row())
        .bind(&new.post)
        .bind(new.parent)
        .bind(new.block)
        .bind(&new.author)
        .bind(new.text.as_str())
        .bind(&stamp)
        .fetch_one(&self.db.writer)
        .await?;
        // The time as stored, to the second, so a list reads back the same.
        let at = DateTime::parse_from_rfc3339(&stamp).map_or(at, |at| at.with_timezone(&Utc));
        Ok(DraftComment::saved(id, new, at))
    }

    async fn resolve_now(&self, post: &str, id: i64, state: ThreadState) -> Result<bool, DbError> {
        let done = sqlx::query(
            "UPDATE draft_comments SET resolved = ?1 \
             WHERE site_id = ?2 AND post_id = ?3 AND id = ?4 AND parent_id IS NULL",
        )
        .bind(state.is_resolved())
        .bind(self.site.as_row())
        .bind(post)
        .bind(id)
        .execute(&self.db.writer)
        .await?;
        Ok(done.rows_affected() > 0)
    }
}

/// The comment in `row`.
fn comment(row: &SqliteRow) -> Result<DraftComment, DbError> {
    let unreadable = |column, value: &str| DbError::Unreadable {
        column,
        value: value.to_owned(),
    };
    let body: String = row.try_get("body")?;
    let text = DraftCommentText::try_from(body.clone())
        .map_err(|_| unreadable("draft_comments.body", &body))?;
    let block: Option<i64> = row.try_get("block")?;
    let block = block
        .map(|block| {
            u32::try_from(block).map_err(|_| unreadable("draft_comments.block", &block.to_string()))
        })
        .transpose()?;
    let created_at: String = row.try_get("created_at")?;
    let created_at = DateTime::parse_from_rfc3339(&created_at)
        .map_err(|_| unreadable("draft_comments.created_at", &created_at))?
        .with_timezone(&Utc);
    let (post, author): (String, String) = (row.try_get("post_id")?, row.try_get("author")?);
    let mut new = NewDraftComment::new(&post, &author, text);
    (new.parent, new.block) = (row.try_get("parent_id")?, block);
    let state = ThreadState::from(row.try_get::<bool, _>("resolved")?);
    Ok(DraftComment::saved(row.try_get("id")?, new, created_at).in_state(state))
}

impl DraftCommentStore for DraftCommentTable {
    fn list<'a>(
        &'a self,
        post: &'a str,
    ) -> BoxFuture<'a, Result<Vec<DraftComment>, DraftCommentStoreError>> {
        Box::pin(async move {
            self.list_now(post)
                .await
                .map_err(DraftCommentStoreError::storage)
        })
    }

    fn add(
        &self,
        comment: NewDraftComment,
    ) -> BoxFuture<'_, Result<DraftComment, DraftCommentStoreError>> {
        Box::pin(async move {
            self.add_now(comment)
                .await
                .map_err(DraftCommentStoreError::storage)
        })
    }

    fn resolve<'a>(
        &'a self,
        post: &'a str,
        id: i64,
        state: ThreadState,
    ) -> BoxFuture<'a, Result<bool, DraftCommentStoreError>> {
        Box::pin(async move {
            let resolved = self.resolve_now(post, id, state).await;
            resolved.map_err(DraftCommentStoreError::storage)
        })
    }
}
