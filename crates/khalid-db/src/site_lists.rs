//! Reading the site's authors, categories and tags back from the index.

use khalid_core::{Locale, Slug};

use crate::posts::{stored, stored_hash};
use crate::{Database, DbError, IndexedAuthor, IndexedTerm, SiteId};

impl Database {
    /// Every author of `site` in every language, in the order the last
    /// rebuild was given them.
    ///
    /// # Errors
    ///
    /// Returns [`DbError::Query`] when the database cannot be read, and
    /// [`DbError::Unreadable`] when a stored value is not one Khalid writes.
    pub async fn authors(&self, site: SiteId) -> Result<Vec<IndexedAuthor>, DbError> {
        let site = site.as_row();
        let rows = sqlx::query!(
            "SELECT path, locale, slug, name, content_hash FROM authors \
             WHERE site_id = ?1 ORDER BY id",
            site
        )
        .fetch_all(&self.readers)
        .await?;
        rows.into_iter()
            .map(|row| {
                Ok(IndexedAuthor::new(
                    &row.path,
                    stored("locale", &row.locale, Locale::try_new)?,
                    stored("slug", &row.slug, Slug::try_new_unicode)?,
                    &row.name,
                    stored_hash(&row.content_hash)?,
                ))
            })
            .collect()
    }

    /// Every category of `site` in every language, in the order the last
    /// rebuild was given them.
    ///
    /// # Errors
    ///
    /// As for [`Database::authors`].
    pub async fn categories(&self, site: SiteId) -> Result<Vec<IndexedTerm>, DbError> {
        let site = site.as_row();
        let rows = sqlx::query!(
            "SELECT locale, slug, name FROM categories WHERE site_id = ?1 ORDER BY id",
            site
        )
        .fetch_all(&self.readers)
        .await?;
        rows.into_iter()
            .map(|row| term(&row.locale, &row.slug, &row.name))
            .collect()
    }

    /// Every tag of `site` in every language, in the order the last rebuild
    /// was given them.
    ///
    /// # Errors
    ///
    /// As for [`Database::authors`].
    pub async fn tags(&self, site: SiteId) -> Result<Vec<IndexedTerm>, DbError> {
        let site = site.as_row();
        let rows = sqlx::query!(
            "SELECT locale, slug, name FROM tags WHERE site_id = ?1 ORDER BY id",
            site
        )
        .fetch_all(&self.readers)
        .await?;
        rows.into_iter()
            .map(|row| term(&row.locale, &row.slug, &row.name))
            .collect()
    }
}

/// Reads one stored category or tag back into its types.
fn term(locale: &str, slug: &str, name: &str) -> Result<IndexedTerm, DbError> {
    Ok(IndexedTerm::new(
        stored("locale", locale, Locale::try_new)?,
        stored("slug", slug, Slug::try_new_unicode)?,
        name,
    ))
}
