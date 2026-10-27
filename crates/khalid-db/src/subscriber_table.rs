//! Newsletter subscribers on SQLite, for `khalid-newsletter` (task 5.5).

use chrono::{DateTime, SecondsFormat, Utc};
use khalid_newsletter::{
    BoxFuture, Confirmed, SignUp, SignedUp, StoreError, SubscriberId, SubscriberStore, TokenHash,
};
use sqlx::{Sqlite, Transaction};

use crate::{Database, DbError, SiteId};

/// One site's subscribers and their links. Only token hashes are stored, so
/// the file never holds a link that works (RULES.md BL-SEC-3).
#[derive(Debug, Clone)]
pub struct SubscriberTable {
    db: Database,
    site: SiteId,
}

impl SubscriberTable {
    /// The subscribers of `site` in `db`.
    pub fn new(db: Database, site: SiteId) -> Self {
        Self { db, site }
    }

    async fn sign_up_now(&self, sign_up: &SignUp) -> Result<SignedUp, DbError> {
        let at = stamp(sign_up.at);
        let mut tx = self.db.writer.begin().await?;
        let found: Option<(i64, String)> =
            sqlx::query_as("SELECT id, status FROM subscribers WHERE site_id = ?1 AND email = ?2")
                .bind(self.site.as_row())
                .bind(sign_up.email.as_str())
                .fetch_optional(&mut *tx)
                .await?;
        let (row, status) = if let Some(found) = found {
            found
        } else {
            let row = sqlx::query_scalar(
                "INSERT INTO subscribers (site_id, email, status, created_at) \
                 VALUES (?1, ?2, 'pending', ?3) RETURNING id",
            )
            .bind(self.site.as_row())
            .bind(sign_up.email.as_str())
            .bind(&at)
            .fetch_one(&mut *tx)
            .await?;
            (row, "pending".to_owned())
        };
        sqlx::query(
            "INSERT INTO subscriber_lists (site_id, subscriber_id, list, joined_at) \
             VALUES (?1, ?2, ?3, ?4) ON CONFLICT DO NOTHING",
        )
        .bind(self.site.as_row())
        .bind(row)
        .bind(sign_up.list.as_str())
        .bind(&at)
        .execute(&mut *tx)
        .await?;
        let signed_up = match status.as_str() {
            "confirmed" => SignedUp::AlreadyConfirmed,
            "pending" | "unsubscribed" => self.ask(&mut tx, row, sign_up).await?,
            _ => return Err(unreadable("status", &status)),
        };
        tx.commit().await?;
        Ok(signed_up)
    }

    /// Makes subscriber `row` wait for its confirmation, and stores the
    /// token of `sign_up` unless a confirmation went out a moment ago.
    async fn ask(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        row: i64,
        sign_up: &SignUp,
    ) -> Result<SignedUp, DbError> {
        sqlx::query(
            "UPDATE subscribers SET status = 'pending', unsubscribed_at = NULL \
             WHERE id = ?1 AND status = 'unsubscribed'",
        )
        .bind(row)
        .execute(&mut **tx)
        .await?;
        let recent: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM subscriber_tokens \
             WHERE subscriber_id = ?1 AND purpose = 'confirm' AND used_at IS NULL \
             AND julianday(created_at) > julianday(?2))",
        )
        .bind(row)
        .bind(stamp(sign_up.quiet_since))
        .fetch_one(&mut **tx)
        .await?;
        if recent {
            return Ok(SignedUp::RecentlyAsked);
        }
        sqlx::query(
            "INSERT INTO subscriber_tokens \
             (site_id, subscriber_id, purpose, token_hash, created_at, expires_at) \
             VALUES (?1, ?2, 'confirm', ?3, ?4, ?5)",
        )
        .bind(self.site.as_row())
        .bind(row)
        .bind(sign_up.confirm.as_bytes().as_slice())
        .bind(stamp(sign_up.at))
        .bind(stamp(sign_up.expires_at))
        .execute(&mut **tx)
        .await?;
        Ok(SignedUp::ConfirmationDue(SubscriberId::from_row(row)))
    }

    async fn confirm_now(
        &self,
        token: &TokenHash,
        at: DateTime<Utc>,
    ) -> Result<Confirmed, DbError> {
        let at = stamp(at);
        let mut tx = self.db.writer.begin().await?;
        let found: Option<(i64, i64, bool, bool, String)> = sqlx::query_as(
            "SELECT t.id, t.subscriber_id, t.used_at IS NOT NULL, \
             julianday(t.expires_at) <= julianday(?3), s.status \
             FROM subscriber_tokens t JOIN subscribers s ON s.id = t.subscriber_id \
             WHERE t.site_id = ?1 AND t.token_hash = ?2 AND t.purpose = 'confirm'",
        )
        .bind(self.site.as_row())
        .bind(token.as_bytes().as_slice())
        .bind(&at)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((token_row, row, used, expired, status)) = found else {
            return Ok(Confirmed::Unknown);
        };
        let confirmed = match (status.as_str(), used, expired) {
            ("confirmed", _, _) => Confirmed::Before,
            ("pending", true, _) | ("unsubscribed", _, _) => Confirmed::Unknown,
            ("pending", false, true) => Confirmed::Expired,
            ("pending", false, false) => {
                sqlx::query(
                    "UPDATE subscribers SET status = 'confirmed', confirmed_at = ?1 WHERE id = ?2",
                )
                .bind(&at)
                .bind(row)
                .execute(&mut *tx)
                .await?;
                Confirmed::Now(SubscriberId::from_row(row))
            }
            _ => return Err(unreadable("status", &status)),
        };
        sqlx::query("UPDATE subscriber_tokens SET used_at = ?1 WHERE id = ?2 AND used_at IS NULL")
            .bind(&at)
            .bind(token_row)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(confirmed)
    }
}

impl SubscriberStore for SubscriberTable {
    fn sign_up<'a>(&'a self, sign_up: &'a SignUp) -> BoxFuture<'a, Result<SignedUp, StoreError>> {
        Box::pin(async move { self.sign_up_now(sign_up).await.map_err(StoreError::new) })
    }

    fn confirm<'a>(
        &'a self,
        token: &'a TokenHash,
        at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<Confirmed, StoreError>> {
        Box::pin(async move { self.confirm_now(token, at).await.map_err(StoreError::new) })
    }
}

/// `at` as the RFC 3339 text the schema stores.
fn stamp(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn unreadable(column: &'static str, value: &str) -> DbError {
    DbError::Unreadable {
        column,
        value: value.to_owned(),
    }
}
