//! Staff accounts on SQLite, for `khalid-auth`.

use chrono::{DateTime, SecondsFormat, Utc};
use khalid_api::content_store::BoxFuture;
use khalid_api::keys::Role;
use khalid_auth::{
    Account, AccountId, AccountStore, AccountStoreError, CreateAccountError, Email, PasswordHash,
    StoredAccount,
};
use sqlx::Row;
use sqlx::sqlite::SqliteRow;

use crate::{Database, DbError, SiteId};

/// One site's staff accounts in `users`. Passwords are stored only as
/// Argon2id hashes (RULES.md BL-SEC-3). Cloning shares the connections.
#[derive(Debug, Clone)]
pub struct AccountTable {
    db: Database,
    site: SiteId,
}

impl AccountTable {
    /// The accounts of `site` in `db`.
    pub fn new(db: Database, site: SiteId) -> Self {
        Self { db, site }
    }

    async fn create_now(
        &self,
        email: &Email,
        name: &str,
        role: Role,
        at: DateTime<Utc>,
    ) -> Result<Result<Account, CreateAccountError>, DbError> {
        let inserted = sqlx::query_scalar::<_, i64>(
            "INSERT INTO users (site_id, email, name, role, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5) RETURNING id",
        )
        .bind(self.site.as_row())
        .bind(email.as_str())
        .bind(name)
        .bind(role.as_str())
        .bind(at.to_rfc3339_opts(SecondsFormat::Secs, true))
        .fetch_one(&self.db.writer)
        .await;
        let id = match inserted {
            Ok(id) => id,
            // The email's constraint names the email column; the one-owner
            // index names only the site.
            Err(sqlx::Error::Database(error)) if error.is_unique_violation() => {
                return Ok(Err(if error.message().contains("users.email") {
                    CreateAccountError::EmailTaken
                } else {
                    CreateAccountError::OwnerTaken
                }));
            }
            Err(error) => return Err(error.into()),
        };
        let account = Account::new(
            AccountId::from_row(id),
            email.clone(),
            name.to_owned(),
            role,
        );
        Ok(Ok(account))
    }

    async fn find_by_email_now(&self, email: &Email) -> Result<Option<StoredAccount>, DbError> {
        let row = sqlx::query(
            "SELECT id, email, name, role, password_hash FROM users WHERE site_id = ?1 AND email = ?2",
        )
        .bind(self.site.as_row())
        .bind(email.as_str())
        .fetch_optional(&self.db.readers)
        .await?;
        row.map(|row| stored(&row)).transpose()
    }

    async fn find_now(&self, id: AccountId) -> Result<Option<Account>, DbError> {
        let row = sqlx::query(
            "SELECT id, email, name, role, password_hash FROM users WHERE site_id = ?1 AND id = ?2",
        )
        .bind(self.site.as_row())
        .bind(id.as_row())
        .fetch_optional(&self.db.readers)
        .await?;
        row.map(|row| stored(&row).map(|stored| stored.account))
            .transpose()
    }

    async fn list_now(&self) -> Result<Vec<Account>, DbError> {
        let rows = sqlx::query(
            "SELECT id, email, name, role, password_hash FROM users WHERE site_id = ?1 ORDER BY email",
        )
        .bind(self.site.as_row())
        .fetch_all(&self.db.readers)
        .await?;
        rows.iter()
            .map(|row| stored(row).map(|stored| stored.account))
            .collect()
    }

    /// Runs `sql`, which binds the site as `?1` and the account as `?2`
    /// and then `value` as `?3`, and says whether it changed a row.
    async fn change(&self, sql: &'static str, id: AccountId, value: &str) -> Result<bool, DbError> {
        let done = sqlx::query(sql)
            .bind(self.site.as_row())
            .bind(id.as_row())
            .bind(value)
            .execute(&self.db.writer)
            .await?;
        Ok(done.rows_affected() > 0)
    }

    async fn transfer_now(&self, from: AccountId, to: AccountId) -> Result<bool, DbError> {
        if from == to {
            return Ok(false);
        }
        let mut tx = self.db.writer.begin().await?;
        // The old owner steps down first, so the one-owner index holds.
        let steps = [
            (
                "UPDATE users SET role = 'admin' WHERE site_id = ?1 AND id = ?2 AND role = 'owner'",
                from,
            ),
            (
                "UPDATE users SET role = 'owner' WHERE site_id = ?1 AND id = ?2",
                to,
            ),
        ];
        for (sql, id) in steps {
            let done = sqlx::query(sql)
                .bind(self.site.as_row())
                .bind(id.as_row())
                .execute(&mut *tx)
                .await?;
            if done.rows_affected() == 0 {
                return Ok(false);
            }
        }
        tx.commit().await?;
        Ok(true)
    }
}

/// An account and its password hash from a row of `users`.
fn stored(row: &SqliteRow) -> Result<StoredAccount, DbError> {
    let unreadable = |column, value: &str| DbError::Unreadable {
        column,
        value: value.to_owned(),
    };
    let email: String = row.try_get("email")?;
    let email: Email = email
        .parse()
        .map_err(|_| unreadable("email", "not an email address"))?;
    let role: String = row.try_get("role")?;
    let role: Role = role.parse().map_err(|_| unreadable("role", &role))?;
    let password = row
        .try_get::<Option<String>, _>("password_hash")?
        .map(|phc| {
            PasswordHash::from_phc(phc).map_err(|_| unreadable("password_hash", "not Argon2id"))
        })
        .transpose()?;
    let account = Account::new(
        AccountId::from_row(row.try_get("id")?),
        email,
        row.try_get("name")?,
        role,
    );
    Ok(StoredAccount { account, password })
}

impl From<DbError> for AccountStoreError {
    fn from(error: DbError) -> Self {
        Self::new(error)
    }
}

impl AccountStore for AccountTable {
    fn create<'a>(
        &'a self,
        email: &'a Email,
        name: &'a str,
        role: Role,
        at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<Account, CreateAccountError>> {
        Box::pin(async move {
            let unavailable = |error: DbError| CreateAccountError::Unavailable {
                source: error.into(),
            };
            self.create_now(email, name, role, at)
                .await
                .map_err(unavailable)?
        })
    }

    fn find_by_email<'a>(
        &'a self,
        email: &'a Email,
    ) -> BoxFuture<'a, Result<Option<StoredAccount>, AccountStoreError>> {
        Box::pin(async move { Ok(self.find_by_email_now(email).await?) })
    }

    fn find(&self, id: AccountId) -> BoxFuture<'_, Result<Option<Account>, AccountStoreError>> {
        Box::pin(async move { Ok(self.find_now(id).await?) })
    }

    fn list(&self) -> BoxFuture<'_, Result<Vec<Account>, AccountStoreError>> {
        Box::pin(async move { Ok(self.list_now().await?) })
    }

    fn set_role(
        &self,
        id: AccountId,
        role: Role,
    ) -> BoxFuture<'_, Result<bool, AccountStoreError>> {
        let sql = "UPDATE users SET role = ?3 \
                   WHERE site_id = ?1 AND id = ?2 AND role != 'owner' AND ?3 != 'owner'";
        Box::pin(async move { Ok(self.change(sql, id, role.as_str()).await?) })
    }

    fn set_password<'a>(
        &'a self,
        id: AccountId,
        hash: &'a PasswordHash,
    ) -> BoxFuture<'a, Result<bool, AccountStoreError>> {
        let sql = "UPDATE users SET password_hash = ?3 WHERE site_id = ?1 AND id = ?2";
        Box::pin(async move { Ok(self.change(sql, id, hash.as_str()).await?) })
    }

    fn remove(&self, id: AccountId) -> BoxFuture<'_, Result<bool, AccountStoreError>> {
        Box::pin(async move {
            let done = sqlx::query("DELETE FROM users WHERE site_id = ?1 AND id = ?2")
                .bind(self.site.as_row())
                .bind(id.as_row())
                .execute(&self.db.writer)
                .await
                .map_err(DbError::from)?;
            Ok(done.rows_affected() > 0)
        })
    }

    fn transfer(
        &self,
        from: AccountId,
        to: AccountId,
    ) -> BoxFuture<'_, Result<bool, AccountStoreError>> {
        Box::pin(async move { Ok(self.transfer_now(from, to).await?) })
    }
}
