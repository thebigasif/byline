//! Keys a site makes for itself and keeps in `site_keys`.

use chrono::SecondsFormat;
use khalid_core::{Clock, Secret};

use crate::{Database, DbError, SiteId};

impl Database {
    /// The key that signs `site`'s preview links: 256 random bits, made
    /// the first time it is asked for and the same ever after, so links
    /// outlast a restart. `clock` dates a new key.
    ///
    /// # Errors
    ///
    /// Returns [`DbError::NoRandomness`] when a new key cannot be made,
    /// [`DbError::Query`] when the table cannot be read or written, and
    /// [`DbError::Unreadable`] when the stored key is not 32 bytes.
    pub async fn preview_key(
        &self,
        site: SiteId,
        clock: &dyn Clock,
    ) -> Result<Secret<[u8; 32]>, DbError> {
        let mut fresh = [0_u8; 32];
        getrandom::fill(&mut fresh).map_err(|source| DbError::NoRandomness { source })?;
        let created_at = clock.now().to_rfc3339_opts(SecondsFormat::Secs, true);
        sqlx::query(
            "INSERT INTO site_keys (site_id, purpose, secret, created_at) \
             VALUES (?1, 'preview', ?2, ?3) ON CONFLICT (site_id, purpose) DO NOTHING",
        )
        .bind(site.as_row())
        .bind(fresh.as_slice())
        .bind(created_at)
        .execute(&self.writer)
        .await?;
        let stored: Vec<u8> = sqlx::query_scalar(
            "SELECT secret FROM site_keys WHERE site_id = ?1 AND purpose = 'preview'",
        )
        .bind(site.as_row())
        .fetch_one(&self.writer)
        .await?;
        let key = <[u8; 32]>::try_from(stored).map_err(|_| DbError::Unreadable {
            column: "site_keys.secret",
            value: "a key that is not 32 bytes".to_owned(),
        })?;
        Ok(Secret::new(key))
    }
}
