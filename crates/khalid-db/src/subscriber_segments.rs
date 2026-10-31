//! Subscriber labels and segments on SQLite (G39).

use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use khalid_core::Slug;
use khalid_newsletter::{EmailAddress, Labelled, Recipient, Segment, SubscriberId};

use crate::subscriber_table::stamp;
use crate::{DbError, SubscriberTable};

impl SubscriberTable {
    pub(crate) async fn label_now(
        &self,
        email: &EmailAddress,
        label: &Slug,
        at: DateTime<Utc>,
    ) -> Result<Labelled, DbError> {
        sqlx::query(
            "INSERT INTO subscriber_labels (site_id, subscriber_id, label, added_at) \
             SELECT site_id, id, ?3, ?4 FROM subscribers \
             WHERE site_id = ?1 AND email = ?2 AND status != 'unsubscribed' \
             ON CONFLICT DO NOTHING",
        )
        .bind(self.site.as_row())
        .bind(email.as_str())
        .bind(label.as_str())
        .bind(stamp(at))
        .execute(&self.db.writer)
        .await?;
        self.labelled(email).await
    }

    pub(crate) async fn unlabel_now(
        &self,
        email: &EmailAddress,
        label: &Slug,
    ) -> Result<Labelled, DbError> {
        sqlx::query(
            "DELETE FROM subscriber_labels WHERE label = ?3 AND subscriber_id = \
             (SELECT id FROM subscribers WHERE site_id = ?1 AND email = ?2)",
        )
        .bind(self.site.as_row())
        .bind(email.as_str())
        .bind(label.as_str())
        .execute(&self.db.writer)
        .await?;
        self.labelled(email).await
    }

    /// `Done` when `email` subscribes, confirmed or not yet.
    async fn labelled(&self, email: &EmailAddress) -> Result<Labelled, DbError> {
        let subscribed: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM subscribers \
             WHERE site_id = ?1 AND email = ?2 AND status != 'unsubscribed')",
        )
        .bind(self.site.as_row())
        .bind(email.as_str())
        .fetch_one(&self.db.writer)
        .await?;
        Ok(if subscribed {
            Labelled::Done
        } else {
            Labelled::NotSubscribed
        })
    }

    /// The confirmed members of the segment's newsletter, then, one label
    /// at a time, only those who carry it.
    pub(crate) async fn audience_now(&self, segment: &Segment) -> Result<Vec<Recipient>, DbError> {
        let mut members: Vec<(i64, String)> = sqlx::query_as(
            "SELECT s.id, s.email FROM subscribers s \
             JOIN subscriber_lists l ON l.subscriber_id = s.id AND l.list = ?2 \
             WHERE s.site_id = ?1 AND s.status = 'confirmed' ORDER BY s.id",
        )
        .bind(self.site.as_row())
        .bind(segment.list().as_str())
        .fetch_all(&self.db.readers)
        .await?;
        for label in segment.labels() {
            let carrying: BTreeSet<i64> = sqlx::query_scalar(
                "SELECT subscriber_id FROM subscriber_labels WHERE site_id = ?1 AND label = ?2",
            )
            .bind(self.site.as_row())
            .bind(label.as_str())
            .fetch_all(&self.db.readers)
            .await?
            .into_iter()
            .collect();
            members.retain(|(id, _)| carrying.contains(id));
        }
        members
            .into_iter()
            .map(|(id, email)| match EmailAddress::parse(&email) {
                Ok(email) => Ok(Recipient::new(SubscriberId::from_row(id), email)),
                Err(_) => Err(DbError::Unreadable {
                    column: "email",
                    value: email,
                }),
            })
            .collect()
    }
}
