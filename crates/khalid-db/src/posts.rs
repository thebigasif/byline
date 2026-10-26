//! Reading posts back from the index.

use khalid_core::{Locale, Slug};

use crate::{Database, DbError, IndexedPost, SiteId};

impl Database {
    /// Up to `limit` posts of one collection in one language, newest first.
    /// Undated posts come last, and posts of one day are in slug order. The
    /// query reads the `posts_by_date` index and never sorts.
    ///
    /// # Errors
    ///
    /// Returns [`DbError::Query`] when the database cannot be read, and
    /// [`DbError::Unreadable`] when a stored value is not one Khalid writes.
    pub async fn posts(
        &self,
        site: SiteId,
        locale: &Locale,
        collection: &str,
        limit: u32,
    ) -> Result<Vec<IndexedPost>, DbError> {
        let site = site.as_row();
        let language = locale.as_str();
        let limit = i64::from(limit);
        let rows = sqlx::query_file!(
            "queries/posts_newest_first.sql",
            site,
            language,
            collection,
            limit
        )
        .fetch_all(&self.readers)
        .await?;
        rows.into_iter()
            .map(|row| {
                let mut post = IndexedPost::new(
                    &row.path,
                    stored("locale", &row.locale, Locale::try_new)?,
                    &row.collection,
                    stored("slug", &row.slug, Slug::try_new_unicode)?,
                    stored_hash(&row.content_hash)?,
                );
                post.title = row.title;
                post.description = row.description;
                post.date = row.date;
                post.author = optional("author", row.author.as_deref())?;
                post.category = optional("category", row.category.as_deref())?;
                post.translation = row.translation;
                Ok(post)
            })
            .collect()
    }
}

/// Reads a stored value back into its type.
fn stored<T, E>(
    column: &'static str,
    value: &str,
    parse: impl FnOnce(&str) -> Result<T, E>,
) -> Result<T, DbError> {
    parse(value).map_err(|_| DbError::Unreadable {
        column,
        value: value.to_owned(),
    })
}

/// Reads a stored slug that may be missing.
fn optional(column: &'static str, value: Option<&str>) -> Result<Option<Slug>, DbError> {
    value
        .map(|value| stored(column, value, Slug::try_new_unicode))
        .transpose()
}

/// Reads a stored content hash, which is always 32 bytes.
fn stored_hash(bytes: &[u8]) -> Result<[u8; 32], DbError> {
    <[u8; 32]>::try_from(bytes).map_err(|_| DbError::Unreadable {
        column: "content_hash",
        value: format!("{} bytes", bytes.len()),
    })
}
