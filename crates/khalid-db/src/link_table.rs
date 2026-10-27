//! Sign-in links on SQLite, for `khalid-auth`.

use chrono::{DateTime, SecondsFormat, Utc};
use khalid_api::content_store::BoxFuture;
use khalid_auth::{AccountId, LINK_LIFETIME, LinkStore, LinkStoreError, SessionHash};

use crate::{Database, DbError, SiteId};

/// One site's sign-in links in `magic_links`, each stored as the SHA-256
/// of its token (RULES.md BL-SEC-3). Cloning shares the connections.
#[derive(Debug, Clone)]
pub struct LinkTable {
    db: Database,
    site: SiteId,
}

impl LinkTable {
    /// The links of `site` in `db`.
    pub fn new(db: Database, site: SiteId) -> Self {
        Self { db, site }
    }

    async fn create_now(
        &self,
        hash: SessionHash,
        account: AccountId,
        at: DateTime<Utc>,
    ) -> Result<(), DbError> {
        sqlx::query(
            "INSERT INTO magic_links (site_id, user_id, token_hash, created_at, expires_at) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(self.site.as_row())
        .bind(account.as_row())
        .bind(hash.as_bytes().as_slice())
        .bind(text(at))
        .bind(text(at + LINK_LIFETIME))
        .execute(&self.db.writer)
        .await?;
        Ok(())
    }

    /// Marks the link used and returns its account in one statement, so
    /// two requests racing with one link cannot both win.
    async fn redeem_now(
        &self,
        hash: SessionHash,
        at: DateTime<Utc>,
    ) -> Result<Option<AccountId>, DbError> {
        let account: Option<i64> = sqlx::query_scalar(
            "UPDATE magic_links SET used_at = ?1 \
             WHERE site_id = ?2 AND token_hash = ?3 AND used_at IS NULL \
             AND julianday(expires_at) > julianday(?1) RETURNING user_id",
        )
        .bind(text(at))
        .bind(self.site.as_row())
        .bind(hash.as_bytes().as_slice())
        .fetch_optional(&self.db.writer)
        .await?;
        Ok(account.map(AccountId::from_row))
    }
}

fn text(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Secs, true)
}

impl LinkStore for LinkTable {
    fn create(
        &self,
        hash: SessionHash,
        account: AccountId,
        at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<(), LinkStoreError>> {
        Box::pin(async move {
            self.create_now(hash, account, at)
                .await
                .map_err(LinkStoreError::new)
        })
    }

    fn redeem(
        &self,
        hash: SessionHash,
        at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<Option<AccountId>, LinkStoreError>> {
        Box::pin(async move { self.redeem_now(hash, at).await.map_err(LinkStoreError::new) })
    }
}
