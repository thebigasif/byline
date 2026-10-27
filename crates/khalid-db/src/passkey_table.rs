//! Staff passkeys on SQLite, for `khalid-auth`.

use chrono::{DateTime, SecondsFormat, Utc};
use khalid_api::content_store::BoxFuture;
use khalid_auth::{
    AccountId, PasskeyStore, PasskeyStoreError, PublicKey, Registration, StoredPasskey,
};
use sqlx::Row;
use sqlx::sqlite::SqliteRow;

use crate::{Database, DbError, SiteId};

/// One site's passkeys in `passkeys`: public keys only, never anything
/// that could sign. Cloning shares the connections.
#[derive(Debug, Clone)]
pub struct PasskeyTable {
    db: Database,
    site: SiteId,
}

impl PasskeyTable {
    /// The passkeys of `site` in `db`.
    pub fn new(db: Database, site: SiteId) -> Self {
        Self { db, site }
    }

    async fn add_now(
        &self,
        account: AccountId,
        registration: &Registration,
        name: &str,
        at: DateTime<Utc>,
    ) -> Result<bool, DbError> {
        let added = sqlx::query(
            "INSERT INTO passkeys (site_id, user_id, credential_id, public_key, sign_count, name, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) ON CONFLICT (site_id, credential_id) DO NOTHING",
        )
        .bind(self.site.as_row())
        .bind(account.as_row())
        .bind(registration.credential_id.as_slice())
        .bind(registration.public_key.to_stored())
        .bind(i64::from(registration.sign_count))
        .bind(name)
        .bind(text(at))
        .execute(&self.db.writer)
        .await?;
        Ok(added.rows_affected() > 0)
    }

    async fn find_now(&self, credential_id: &[u8]) -> Result<Option<StoredPasskey>, DbError> {
        let row = sqlx::query(
            "SELECT user_id, credential_id, public_key, sign_count, name FROM passkeys \
             WHERE site_id = ?1 AND credential_id = ?2",
        )
        .bind(self.site.as_row())
        .bind(credential_id)
        .fetch_optional(&self.db.readers)
        .await?;
        row.map(|row| stored(&row)).transpose()
    }

    async fn of_now(&self, account: AccountId) -> Result<Vec<StoredPasskey>, DbError> {
        let rows = sqlx::query(
            "SELECT user_id, credential_id, public_key, sign_count, name FROM passkeys \
             WHERE site_id = ?1 AND user_id = ?2 ORDER BY id",
        )
        .bind(self.site.as_row())
        .bind(account.as_row())
        .fetch_all(&self.db.readers)
        .await?;
        rows.iter().map(stored).collect()
    }

    async fn used_now(
        &self,
        credential_id: &[u8],
        sign_count: u32,
        at: DateTime<Utc>,
    ) -> Result<(), DbError> {
        sqlx::query(
            "UPDATE passkeys SET sign_count = ?1, last_used_at = ?2 \
             WHERE site_id = ?3 AND credential_id = ?4",
        )
        .bind(i64::from(sign_count))
        .bind(text(at))
        .bind(self.site.as_row())
        .bind(credential_id)
        .execute(&self.db.writer)
        .await?;
        Ok(())
    }
}

/// A passkey from a row of `passkeys`.
fn stored(row: &SqliteRow) -> Result<StoredPasskey, DbError> {
    let unreadable = |column| DbError::Unreadable {
        column,
        value: "a value khalid-auth did not write".to_owned(),
    };
    let key: Vec<u8> = row.try_get("public_key")?;
    let count: i64 = row.try_get("sign_count")?;
    Ok(StoredPasskey {
        account: AccountId::from_row(row.try_get("user_id")?),
        credential_id: row.try_get("credential_id")?,
        public_key: PublicKey::from_stored(&key).map_err(|_| unreadable("public_key"))?,
        sign_count: u32::try_from(count).map_err(|_| unreadable("sign_count"))?,
        name: row.try_get("name")?,
    })
}

fn text(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Secs, true)
}

impl From<DbError> for PasskeyStoreError {
    fn from(error: DbError) -> Self {
        Self::new(error)
    }
}

impl PasskeyStore for PasskeyTable {
    fn add<'a>(
        &'a self,
        account: AccountId,
        registration: &'a Registration,
        name: &'a str,
        at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<bool, PasskeyStoreError>> {
        Box::pin(async move { Ok(self.add_now(account, registration, name, at).await?) })
    }

    fn find<'a>(
        &'a self,
        credential_id: &'a [u8],
    ) -> BoxFuture<'a, Result<Option<StoredPasskey>, PasskeyStoreError>> {
        Box::pin(async move { Ok(self.find_now(credential_id).await?) })
    }

    fn of(
        &self,
        account: AccountId,
    ) -> BoxFuture<'_, Result<Vec<StoredPasskey>, PasskeyStoreError>> {
        Box::pin(async move { Ok(self.of_now(account).await?) })
    }

    fn used<'a>(
        &'a self,
        credential_id: &'a [u8],
        sign_count: u32,
        at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<(), PasskeyStoreError>> {
        Box::pin(async move { Ok(self.used_now(credential_id, sign_count, at).await?) })
    }
}
