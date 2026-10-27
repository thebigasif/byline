//! Readers' comments on SQLite, for `khalid-comments`.

use chrono::{DateTime, SecondsFormat, Utc};
use khalid_comments::{
    AuthorName, Comment, CommentAuthor, CommentId, CommentStatus, CommentStore, CommentStoreError,
    CommentText, NewComment, PostFile, ReaderEmail,
};
use sqlx::Row;
use sqlx::sqlite::SqliteRow;

use crate::{Database, DbError, SiteId};

/// Every column of `comments` but `site_id`, in the order [`comment`] reads.
macro_rules! columns {
    () => {
        "id, post_path, parent_id, author_kind, author_name, author_email, body, status, created_at"
    };
}

/// One site's comments in `comments`, a visitor table a rebuild never
/// touches. Every query is bound to the site, so a site never reads or
/// changes another's comments. Cloning shares the connections.
#[derive(Debug, Clone)]
pub struct CommentTable {
    db: Database,
    site: SiteId,
}

impl CommentTable {
    /// The comments of `site` in `db`.
    pub fn new(db: Database, site: SiteId) -> Self {
        Self { db, site }
    }

    async fn insert_now(
        &self,
        new: NewComment,
        status: CommentStatus,
        at: DateTime<Utc>,
    ) -> Result<Comment, DbError> {
        let (kind, name, email) = match &new.author {
            CommentAuthor::Reader { name, email } => ("reader", name, email.as_ref()),
            CommentAuthor::Staff { name } => ("staff", name, None),
        };
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO comments (site_id, post_path, parent_id, author_kind, author_name, \
             author_email, body, status, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9) RETURNING id",
        )
        .bind(self.site.as_row())
        .bind(new.post.as_str())
        .bind(new.parent.map(CommentId::get))
        .bind(kind)
        .bind(name.as_str())
        .bind(email.map(ReaderEmail::as_str))
        .bind(new.text.as_str())
        .bind(status.as_str())
        .bind(stamp(at))
        .fetch_one(&self.db.writer)
        .await?;
        Ok(Comment {
            id: CommentId::new(id),
            post: new.post,
            parent: new.parent,
            author: new.author,
            text: new.text,
            status,
            created_at: at,
        })
    }

    async fn get_now(&self, id: CommentId) -> Result<Option<Comment>, DbError> {
        let row = sqlx::query(concat!(
            "SELECT ",
            columns!(),
            " FROM comments WHERE site_id = ?1 AND id = ?2"
        ))
        .bind(self.site.as_row())
        .bind(id.get())
        .fetch_optional(&self.db.readers)
        .await?;
        row.as_ref().map(comment).transpose()
    }

    async fn set_status_now(
        &self,
        id: CommentId,
        status: CommentStatus,
    ) -> Result<Option<Comment>, DbError> {
        let row = sqlx::query(concat!(
            "UPDATE comments SET status = ?3 WHERE site_id = ?1 AND id = ?2 RETURNING ",
            columns!()
        ))
        .bind(self.site.as_row())
        .bind(id.get())
        .bind(status.as_str())
        .fetch_optional(&self.db.writer)
        .await?;
        row.as_ref().map(comment).transpose()
    }

    /// Deletes the comment; the foreign key deletes its replies with it.
    async fn delete_now(&self, id: CommentId) -> Result<Option<Comment>, DbError> {
        let row = sqlx::query(concat!(
            "DELETE FROM comments WHERE site_id = ?1 AND id = ?2 RETURNING ",
            columns!()
        ))
        .bind(self.site.as_row())
        .bind(id.get())
        .fetch_optional(&self.db.writer)
        .await?;
        row.as_ref().map(comment).transpose()
    }

    async fn with_status_now(
        &self,
        status: CommentStatus,
        post: Option<&PostFile>,
    ) -> Result<Vec<Comment>, DbError> {
        let rows = sqlx::query(concat!(
            "SELECT ",
            columns!(),
            " FROM comments WHERE site_id = ?1 AND status = ?2 \
             AND (?3 IS NULL OR post_path = ?3) ORDER BY created_at, id"
        ))
        .bind(self.site.as_row())
        .bind(status.as_str())
        .bind(post.map(PostFile::as_str))
        .fetch_all(&self.db.readers)
        .await?;
        rows.iter().map(comment).collect()
    }

    async fn newest_now(
        &self,
        status: Option<CommentStatus>,
        limit: u32,
    ) -> Result<Vec<Comment>, DbError> {
        let rows = sqlx::query(concat!(
            "SELECT ",
            columns!(),
            " FROM comments WHERE site_id = ?1 AND (?2 IS NULL OR status = ?2) \
             ORDER BY created_at DESC, id DESC LIMIT ?3"
        ))
        .bind(self.site.as_row())
        .bind(status.map(CommentStatus::as_str))
        .bind(i64::from(limit))
        .fetch_all(&self.db.readers)
        .await?;
        rows.iter().map(comment).collect()
    }
}

/// The comment in `row`, which holds every column of `comments` but
/// `site_id`.
fn comment(row: &SqliteRow) -> Result<Comment, DbError> {
    let text = |column: &'static str| row.try_get::<String, _>(column);
    let (kind, name) = (text("author_kind")?, text("author_name")?);
    let email: Option<String> = row.try_get("author_email")?;
    let name = AuthorName::parse(&name).map_err(|_| unreadable("author_name", &name))?;
    let checked =
        |email: String| ReaderEmail::parse(&email).map_err(|_| unreadable("author_email", &email));
    let author = match (kind.as_str(), email) {
        ("reader", email) => {
            let email = email.map(checked).transpose()?;
            CommentAuthor::Reader { name, email }
        }
        ("staff", None) => CommentAuthor::Staff { name },
        _ => return Err(unreadable("author_kind", &kind)),
    };
    let body = text("body")?;
    let status = text("status")?;
    let created_at = text("created_at")?;
    let parent: Option<i64> = row.try_get("parent_id")?;
    Ok(Comment {
        id: CommentId::new(row.try_get("id")?),
        post: PostFile::new(text("post_path")?),
        parent: parent.map(CommentId::new),
        author,
        text: CommentText::parse(&body).map_err(|_| unreadable("body", &body))?,
        status: status.parse().map_err(|_| unreadable("status", &status))?,
        created_at: DateTime::parse_from_rfc3339(&created_at)
            .map(|time| time.with_timezone(&Utc))
            .map_err(|_| unreadable("created_at", &created_at))?,
    })
}

/// `time` as the text the table orders by: UTC to the microsecond.
fn stamp(time: DateTime<Utc>) -> String {
    time.to_rfc3339_opts(SecondsFormat::Micros, true)
}

fn unreadable(column: &'static str, value: &str) -> DbError {
    DbError::Unreadable {
        column,
        value: value.to_owned(),
    }
}

fn read_failed(error: DbError) -> CommentStoreError {
    CommentStoreError::Read {
        source: Box::new(error),
    }
}

fn write_failed(error: DbError) -> CommentStoreError {
    CommentStoreError::Write {
        source: Box::new(error),
    }
}

impl CommentStore for CommentTable {
    async fn insert(
        &self,
        new: NewComment,
        status: CommentStatus,
        at: DateTime<Utc>,
    ) -> Result<Comment, CommentStoreError> {
        self.insert_now(new, status, at).await.map_err(write_failed)
    }

    async fn get(&self, id: CommentId) -> Result<Option<Comment>, CommentStoreError> {
        self.get_now(id).await.map_err(read_failed)
    }

    async fn set_status(
        &self,
        id: CommentId,
        status: CommentStatus,
    ) -> Result<Option<Comment>, CommentStoreError> {
        self.set_status_now(id, status).await.map_err(write_failed)
    }

    async fn delete(&self, id: CommentId) -> Result<Option<Comment>, CommentStoreError> {
        self.delete_now(id).await.map_err(write_failed)
    }

    async fn with_status(
        &self,
        status: CommentStatus,
        post: Option<&PostFile>,
    ) -> Result<Vec<Comment>, CommentStoreError> {
        self.with_status_now(status, post)
            .await
            .map_err(read_failed)
    }

    async fn newest(
        &self,
        status: Option<CommentStatus>,
        limit: u32,
    ) -> Result<Vec<Comment>, CommentStoreError> {
        self.newest_now(status, limit).await.map_err(read_failed)
    }
}
