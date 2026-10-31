//! Newsletter sends on SQLite: each issue once, with its totals (tasks 5.8
//! and 5.12).

use chrono::{DateTime, Utc};
use khalid_core::Slug;
use khalid_newsletter::{SendId, SendTotals, SubscriberId, TokenHash};

use crate::subscriber_table::stamp;
use crate::{DbError, SubscriberTable};

impl SubscriberTable {
    pub(crate) async fn start_send_now(
        &self,
        key: &str,
        list: &Slug,
        at: DateTime<Utc>,
    ) -> Result<Option<SendId>, DbError> {
        let row: Option<i64> = sqlx::query_scalar(
            "INSERT INTO newsletter_sends (site_id, send_key, list, started_at) \
             VALUES (?1, ?2, ?3, ?4) ON CONFLICT DO NOTHING RETURNING id",
        )
        .bind(self.site.as_row())
        .bind(key)
        .bind(list.as_str())
        .bind(stamp(at))
        .fetch_optional(&self.db.writer)
        .await?;
        Ok(row.map(SendId::from_row))
    }

    pub(crate) async fn add_send_unsubscribe_now(
        &self,
        send: SendId,
        subscriber: SubscriberId,
        token: &TokenHash,
        at: DateTime<Utc>,
    ) -> Result<(), DbError> {
        let added = sqlx::query(
            "INSERT INTO subscriber_tokens \
             (site_id, subscriber_id, purpose, token_hash, created_at, send_id) \
             SELECT site_id, id, 'unsubscribe', ?3, ?4, ?5 FROM subscribers \
             WHERE site_id = ?1 AND id = ?2",
        )
        .bind(self.site.as_row())
        .bind(subscriber.as_row())
        .bind(token.as_bytes().as_slice())
        .bind(stamp(at))
        .bind(send.as_row())
        .execute(&self.db.writer)
        .await?;
        if added.rows_affected() == 1 {
            Ok(())
        } else {
            Err(DbError::NoSuchSubscriber {
                id: subscriber.as_row(),
            })
        }
    }

    pub(crate) async fn finish_send_now(
        &self,
        send: SendId,
        totals: SendTotals,
        at: DateTime<Utc>,
    ) -> Result<(), DbError> {
        sqlx::query(
            "UPDATE newsletter_sends SET sent = ?3, failed = ?4, finished_at = ?5 \
             WHERE site_id = ?1 AND id = ?2",
        )
        .bind(self.site.as_row())
        .bind(send.as_row())
        .bind(totals.sent)
        .bind(totals.failed)
        .bind(stamp(at))
        .execute(&self.db.writer)
        .await?;
        Ok(())
    }
}
