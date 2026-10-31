//! API keys on SQLite, for `khalid-api` (ADR 0011).

use std::collections::BTreeSet;
use std::sync::Arc;

use chrono::{DateTime, SecondsFormat, Utc};
use khalid_api::content_store::BoxFuture;
use khalid_api::keys::{
    Access, CreateKeyError, KeyHash, KeyId, KeyStore, KeyStoreError, ListedKey, Role, Scope,
    StoredKey,
};
use khalid_core::Clock;
use sqlx::Row;

use crate::{Database, DbError, SiteId};

/// One site's API keys in `api_keys` and `api_key_scopes`. Only hashes are
/// stored, so the file never holds a usable key (RULES.md BL-SEC-3).
/// Cloning shares the connections.
#[derive(Debug, Clone)]
pub struct KeyTable {
    db: Database,
    site: SiteId,
    clock: Arc<dyn Clock>,
}

impl KeyTable {
    /// The keys of `site` in `db`, stamping creation and revocation with
    /// `clock`.
    pub fn new(db: Database, site: SiteId, clock: impl Clock + 'static) -> Self {
        Self {
            db,
            site,
            clock: Arc::new(clock),
        }
    }

    async fn find_now(&self, id: &KeyId) -> Result<Option<StoredKey>, DbError> {
        let row = sqlx::query(
            "SELECT id, kind, role, key_hash FROM api_keys \
             WHERE site_id = ?1 AND key_id = ?2 AND revoked_at IS NULL",
        )
        .bind(self.site.as_row())
        .bind(id.as_str())
        .fetch_optional(&self.db.readers)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let (row_id, kind): (i64, String) = (row.try_get("id")?, row.try_get("kind")?);
        let (role, hash): (Option<String>, String) =
            (row.try_get("role")?, row.try_get("key_hash")?);
        let access = self.access(row_id, &kind, role).await?;
        let hash = if kind == "content" {
            KeyHash::Sha256(
                from_hex(&hash).ok_or_else(|| unreadable("key_hash", "not 64 hex digits"))?,
            )
        } else {
            KeyHash::Argon2id(hash)
        };
        Ok(Some(StoredKey::new(id.clone(), access, hash)))
    }

    /// What key row `row_id` of `kind` may do, with its staff `role`.
    async fn access(
        &self,
        row_id: i64,
        kind: &str,
        role: Option<String>,
    ) -> Result<Access, DbError> {
        Ok(match (kind, role) {
            ("content", None) => Access::Content,
            ("integration", None) => Access::Integration(self.scopes(row_id).await?),
            ("staff", Some(role)) => Access::Staff(
                role.parse::<Role>()
                    .map_err(|_| unreadable("role", &role))?,
            ),
            _ => return Err(unreadable("kind", kind)),
        })
    }

    /// Every key that works, oldest first.
    async fn list_now(&self) -> Result<Vec<ListedKey>, DbError> {
        let rows = sqlx::query(
            "SELECT id, key_id, kind, role, name, created_at FROM api_keys \
             WHERE site_id = ?1 AND revoked_at IS NULL ORDER BY created_at, key_id",
        )
        .bind(self.site.as_row())
        .fetch_all(&self.db.readers)
        .await?;
        let mut keys = Vec::with_capacity(rows.len());
        for row in rows {
            let (row_id, key_id): (i64, String) = (row.try_get("id")?, row.try_get("key_id")?);
            let (kind, role): (String, Option<String>) =
                (row.try_get("kind")?, row.try_get("role")?);
            let (name, created_at): (String, String) =
                (row.try_get("name")?, row.try_get("created_at")?);
            let id = KeyId::try_new(&key_id).map_err(|_| unreadable("key_id", &key_id))?;
            let created_at = DateTime::parse_from_rfc3339(&created_at)
                .map_err(|_| unreadable("created_at", &created_at))?
                .with_timezone(&Utc);
            let access = self.access(row_id, &kind, role).await?;
            keys.push(ListedKey {
                id,
                name,
                access,
                created_at,
            });
        }
        Ok(keys)
    }

    /// The scopes of key row `key`, once each, in the order the API
    /// standard lists them.
    async fn scopes(&self, key: i64) -> Result<Vec<Scope>, DbError> {
        let names: Vec<String> =
            sqlx::query_scalar("SELECT scope FROM api_key_scopes WHERE api_key_id = ?1")
                .bind(key)
                .fetch_all(&self.db.readers)
                .await?;
        let scopes: Result<BTreeSet<Scope>, DbError> = names
            .iter()
            .map(|name| name.parse().map_err(|_| unreadable("scope", name)))
            .collect();
        Ok(scopes?.into_iter().collect())
    }

    async fn create_now(&self, key: &StoredKey, name: &str) -> Result<bool, DbError> {
        let (kind, role, scopes) = match key.access() {
            Access::Content => ("content", None, BTreeSet::new()),
            Access::Integration(scopes) => ("integration", None, scopes.iter().copied().collect()),
            Access::Staff(role) => ("staff", Some(role.as_str()), BTreeSet::new()),
            _ => {
                return Err(unreadable(
                    "kind",
                    "a kind of key this version cannot store",
                ));
            }
        };
        let hash = match key.hash() {
            KeyHash::Sha256(digest) => to_hex(digest),
            KeyHash::Argon2id(phc) => phc.clone(),
            _ => return Err(unreadable("key_hash", "a hash this version cannot store")),
        };
        let created_at = self.now();
        let mut tx = self.db.writer.begin().await?;
        let inserted = sqlx::query_scalar::<_, i64>(
            "INSERT INTO api_keys (site_id, key_id, kind, role, name, key_hash, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) RETURNING id",
        )
        .bind(self.site.as_row())
        .bind(key.id().as_str())
        .bind(kind)
        .bind(role)
        .bind(name)
        .bind(hash)
        .bind(created_at)
        .fetch_one(&mut *tx)
        .await;
        let row_id = match inserted {
            Ok(id) => id,
            Err(sqlx::Error::Database(error)) if error.is_unique_violation() => return Ok(false),
            Err(error) => return Err(error.into()),
        };
        for scope in scopes {
            sqlx::query(
                "INSERT INTO api_key_scopes (site_id, api_key_id, scope) VALUES (?1, ?2, ?3)",
            )
            .bind(self.site.as_row())
            .bind(row_id)
            .bind(scope.as_str())
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(true)
    }

    async fn revoke_now(&self, id: &KeyId) -> Result<bool, DbError> {
        let revoked = sqlx::query(
            "UPDATE api_keys SET revoked_at = ?1 \
             WHERE site_id = ?2 AND key_id = ?3 AND revoked_at IS NULL",
        )
        .bind(self.now())
        .bind(self.site.as_row())
        .bind(id.as_str())
        .execute(&self.db.writer)
        .await?;
        Ok(revoked.rows_affected() > 0)
    }

    fn now(&self) -> String {
        self.clock.now().to_rfc3339_opts(SecondsFormat::Secs, true)
    }
}

impl KeyStore for KeyTable {
    fn find<'a>(
        &'a self,
        id: &'a KeyId,
    ) -> BoxFuture<'a, Result<Option<StoredKey>, KeyStoreError>> {
        Box::pin(async move { self.find_now(id).await.map_err(KeyStoreError::new) })
    }

    fn create<'a>(
        &'a self,
        key: StoredKey,
        name: &'a str,
    ) -> BoxFuture<'a, Result<(), CreateKeyError>> {
        Box::pin(async move {
            match self.create_now(&key, name).await {
                Ok(true) => Ok(()),
                Ok(false) => Err(CreateKeyError::IdTaken(key.id().clone())),
                Err(error) => Err(CreateKeyError::Unavailable {
                    source: KeyStoreError::new(error),
                }),
            }
        })
    }

    fn revoke<'a>(&'a self, id: &'a KeyId) -> BoxFuture<'a, Result<bool, KeyStoreError>> {
        Box::pin(async move { self.revoke_now(id).await.map_err(KeyStoreError::new) })
    }

    fn list(&self) -> BoxFuture<'_, Result<Vec<ListedKey>, KeyStoreError>> {
        Box::pin(async move { self.list_now().await.map_err(KeyStoreError::new) })
    }
}

/// `bytes` as lowercase hex digits.
fn to_hex(bytes: &[u8; 32]) -> String {
    bytes
        .iter()
        .flat_map(|byte| [byte >> 4, byte & 0x0f])
        .filter_map(|nibble| char::from_digit(u32::from(nibble), 16))
        .collect()
}

/// 64 lowercase hex digits as 32 bytes, or `None`.
fn from_hex(text: &str) -> Option<[u8; 32]> {
    if text.len() != 64 {
        return None;
    }
    let mut bytes = [0; 32];
    for (byte, pair) in bytes.iter_mut().zip(text.as_bytes().chunks(2)) {
        *byte = u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()?;
    }
    Some(bytes)
}

fn unreadable(column: &'static str, value: &str) -> DbError {
    DbError::Unreadable {
        column,
        value: value.to_owned(),
    }
}
