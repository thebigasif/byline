//! Newsletter sends on SQLite: each issue once, with its totals (tasks 5.8
//! and 5.12).

use chrono::{DateTime, Utc};
use khalid_core::Slug;
use khalid_newsletter::{SendId, SendNumbers, SendTotals, SubscriberId, TokenHash};

use crate::subscriber_table::stamp;
use crate::{DbError, SubscriberTable};

/// One row of `newsletter_sends`: key, list, start, sent, failed and
/// unsubscribed.
type SendRow = (String, String, DateTime<Utc>, u32, u32, u32);

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

    pub(crate) async fn sends_now(&self) -> Result<Vec<SendNumbers>, DbError> {
        let rows: Vec<SendRow> = sqlx::query_as(
            "SELECT send_key, list, started_at, sent, failed, unsubscribed \
             FROM newsletter_sends WHERE site_id = ?1 ORDER BY started_at DESC, id DESC",
        )
        .bind(self.site.as_row())
        .fetch_all(&self.db.readers)
        .await?;
        rows.into_iter()
            .map(|(key, list, started_at, sent, failed, unsubscribed)| {
                let list = Slug::try_new(&list).map_err(|_| DbError::Unreadable {
                    column: "list",
                    value: list.clone(),
                })?;
                let totals = SendTotals { sent, failed };
                let numbers = SendNumbers::new(key, list, started_at, totals);
                Ok(numbers.with_unsubscribed(unsubscribed))
            })
            .collect()
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
