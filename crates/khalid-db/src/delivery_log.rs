//! The webhook delivery log on SQLite, for `khalid-webhooks`.

use chrono::{DateTime, SecondsFormat, Utc};
use khalid_core::AbsoluteUrl;
use khalid_webhooks::{
    Attempt, Delivery, DeliveryId, DeliveryState, DeliveryStore, DeliveryStoreError, Event,
    NewDelivery, Outcome, RetrySchedule, WebhookId,
};

use crate::{Database, DbError, SiteId};

/// One site's webhook delivery log in `webhook_deliveries` and
/// `webhook_attempts`.
///
/// A delivery is read back by replaying its attempts on the dispatcher's
/// default [`RetrySchedule`], so the state read is the state written.
#[derive(Debug, Clone)]
pub struct DeliveryLog {
    db: Database,
    site: SiteId,
    schedule: RetrySchedule,
}

/// One row of a delivery joined with one of its attempts, if it has any.
struct Row {
    id: i64,
    webhook_id: i64,
    event: String,
    url: String,
    body: Vec<u8>,
    first_at: String,
    number: Option<i64>,
    at: Option<String>,
    status: Option<i64>,
    reason: Option<String>,
}

impl DeliveryLog {
    /// The delivery log of `site` in `db`.
    pub fn new(db: Database, site: SiteId) -> Self {
        Self {
            db,
            site,
            schedule: RetrySchedule::default(),
        }
    }

    async fn insert_now(&self, new: NewDelivery) -> Result<Delivery, DbError> {
        // A stand-in id until the row gives the real one.
        let draft = new.into_delivery(DeliveryId::new(0));
        let DeliveryState::Pending { next_at } = *draft.state() else {
            return Err(unreadable("state", "a new delivery that is not pending"));
        };
        let (site, webhook) = (self.site.as_row(), draft.webhook().get());
        let (event, url, body) = (draft.event().as_str(), draft.url().as_str(), draft.body());
        let first_at = stamp(next_at);
        let id = sqlx::query_scalar!(
            "INSERT INTO webhook_deliveries \
             (site_id, webhook_id, event, url, body, first_at, state, state_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending', ?6) RETURNING id AS \"id!\"",
            site,
            webhook,
            event,
            url,
            body,
            first_at,
        )
        .fetch_one(&self.db.writer)
        .await?;
        let stored = NewDelivery::new(
            draft.webhook(),
            draft.event(),
            draft.url().clone(),
            draft.body().to_vec(),
            next_at,
        );
        Ok(stored.into_delivery(DeliveryId::new(id)))
    }

    async fn update_now(&self, delivery: &Delivery) -> Result<(), DbError> {
        let (state, state_at) = match *delivery.state() {
            DeliveryState::Pending { next_at } => ("pending", next_at),
            DeliveryState::Delivered { at } => ("delivered", at),
            DeliveryState::GaveUp { at } => ("gave_up", at),
            _ => return Err(unreadable("state", "a state this version cannot store")),
        };
        let (id, site, state_at) = (delivery.id().get(), self.site.as_row(), stamp(state_at));
        let mut tx = self.db.writer.begin().await?;
        let changed = sqlx::query!(
            "UPDATE webhook_deliveries SET state = ?1, state_at = ?2 WHERE id = ?3 AND site_id = ?4",
            state,
            state_at,
            id,
            site,
        )
        .execute(&mut *tx)
        .await?;
        if changed.rows_affected() == 0 {
            return Err(DbError::NoSuchDelivery { id });
        }
        for (number, attempt) in (1_i64..).zip(delivery.attempts()) {
            let at = stamp(attempt.at());
            let (status, reason) = match attempt.outcome() {
                Outcome::Answered { status } => (Some(i64::from(*status)), None),
                Outcome::Unreachable { reason } => (None, Some(reason.as_str())),
                _ => return Err(unreadable("outcome", "a new kind of outcome")),
            };
            sqlx::query!(
                "INSERT INTO webhook_attempts (site_id, delivery_id, number, at, status, reason) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6) ON CONFLICT (delivery_id, number) DO NOTHING",
                site,
                id,
                number,
                at,
                status,
                reason,
            )
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn due_now(&self, now: DateTime<Utc>) -> Result<Vec<Delivery>, DbError> {
        let (site, now) = (self.site.as_row(), stamp(now));
        let rows = sqlx::query_file_as!(Row, "queries/deliveries_due.sql", site, now)
            .fetch_all(&self.db.readers)
            .await?;
        self.deliveries(rows)
    }

    async fn log_now(&self, webhook: WebhookId) -> Result<Vec<Delivery>, DbError> {
        let (site, webhook) = (self.site.as_row(), webhook.get());
        let rows = sqlx::query_as!(
            Row,
            r#"SELECT d.id AS "id!", d.webhook_id, d.event, d.url, d.body, d.first_at,
                      a.number AS "number?", a.at AS "at?", a.status AS "status?", a.reason AS "reason?"
               FROM webhook_deliveries d
               LEFT JOIN webhook_attempts a ON a.delivery_id = d.id
               WHERE d.site_id = ?1 AND d.webhook_id = ?2
               ORDER BY d.id DESC, a.number"#,
            site,
            webhook,
        )
        .fetch_all(&self.db.readers)
        .await?;
        self.deliveries(rows)
    }

    /// Groups joined rows into deliveries, in row order, replaying each
    /// delivery's attempts in order.
    fn deliveries(&self, rows: Vec<Row>) -> Result<Vec<Delivery>, DbError> {
        let mut deliveries: Vec<Delivery> = Vec::new();
        for row in rows {
            let attempt = attempt(&row)?;
            let same = deliveries
                .last()
                .is_some_and(|last| last.id().get() == row.id);
            if !same {
                deliveries.push(delivery(row)?);
            }
            if let (Some(attempt), Some(current)) = (attempt, deliveries.last_mut()) {
                current.record(attempt, &self.schedule);
            }
        }
        Ok(deliveries)
    }
}

/// The delivery a row describes, before its attempts.
fn delivery(row: Row) -> Result<Delivery, DbError> {
    let event: Event = row
        .event
        .parse()
        .map_err(|_| unreadable("event", &row.event))?;
    let url = AbsoluteUrl::parse(&row.url).map_err(|_| unreadable("url", &row.url))?;
    let new = NewDelivery::new(
        WebhookId::new(row.webhook_id),
        event,
        url,
        row.body,
        time("first_at", &row.first_at)?,
    );
    Ok(new.into_delivery(DeliveryId::new(row.id)))
}

/// The attempt in a joined row, or `None` for a delivery with none yet.
fn attempt(row: &Row) -> Result<Option<Attempt>, DbError> {
    let (Some(_), Some(at)) = (row.number, &row.at) else {
        return Ok(None);
    };
    let outcome = match (row.status, &row.reason) {
        (Some(status), None) => Outcome::Answered {
            status: u16::try_from(status).map_err(|_| unreadable("status", &status.to_string()))?,
        },
        (None, Some(reason)) => Outcome::Unreachable {
            reason: reason.clone(),
        },
        _ => return Err(unreadable("status", "a status and a reason")),
    };
    Ok(Some(Attempt::new(time("at", at)?, outcome)))
}

/// A time as the log writes it: UTC to the microsecond, so text order is
/// time order.
fn stamp(time: DateTime<Utc>) -> String {
    time.to_rfc3339_opts(SecondsFormat::Micros, true)
}

fn time(column: &'static str, text: &str) -> Result<DateTime<Utc>, DbError> {
    DateTime::parse_from_rfc3339(text)
        .map(|time| time.with_timezone(&Utc))
        .map_err(|_| unreadable(column, text))
}

fn unreadable(column: &'static str, value: &str) -> DbError {
    DbError::Unreadable {
        column,
        value: value.to_owned(),
    }
}

fn read_failed(error: DbError) -> DeliveryStoreError {
    DeliveryStoreError::Read {
        source: Box::new(error),
    }
}

fn write_failed(error: DbError) -> DeliveryStoreError {
    DeliveryStoreError::Write {
        source: Box::new(error),
    }
}

impl DeliveryStore for DeliveryLog {
    async fn insert(&self, new: NewDelivery) -> Result<Delivery, DeliveryStoreError> {
        self.insert_now(new).await.map_err(write_failed)
    }

    async fn update(&self, delivery: &Delivery) -> Result<(), DeliveryStoreError> {
        self.update_now(delivery).await.map_err(write_failed)
    }

    async fn due(&self, now: DateTime<Utc>) -> Result<Vec<Delivery>, DeliveryStoreError> {
        self.due_now(now).await.map_err(read_failed)
    }

    async fn log(&self, webhook: WebhookId) -> Result<Vec<Delivery>, DeliveryStoreError> {
        self.log_now(webhook).await.map_err(read_failed)
    }
}
