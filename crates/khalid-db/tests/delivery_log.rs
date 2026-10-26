//! The webhook delivery log on SQLite: what goes in comes back the same,
//! due deliveries come oldest first, and each site sees only its own.

use std::path::PathBuf;

use chrono::{DateTime, TimeDelta, TimeZone, Utc};
use khalid_core::{AbsoluteUrl, Slug};
use khalid_db::{Database, DeliveryLog};
use khalid_webhooks::{
    Attempt, Delivery, DeliveryId, DeliveryState, DeliveryStore, DeliveryStoreError, NewDelivery,
    Outcome, RetrySchedule, WebhookId,
};
use sqlx::ConnectOptions;
use sqlx::sqlite::SqliteConnectOptions;

#[cfg(test)]
fn at(minutes: i64) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 26, 9, 0, 0).unwrap() + TimeDelta::minutes(minutes)
}

/// A migrated database with the site `demo` and webhook 7 for it.
#[cfg(test)]
async fn opened() -> (tempfile::TempDir, Database, DeliveryLog) {
    let dir = tempfile::tempdir().unwrap();
    let mut path = PathBuf::from(dir.path());
    path.push("site.db");
    let db = Database::open(&path).await.unwrap();
    db.migrate().await.unwrap();
    let site = db.site(&Slug::try_new("demo").unwrap()).await.unwrap();
    let mut connection = SqliteConnectOptions::new()
        .filename(&path)
        .connect()
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO webhooks (id, site_id, event, url, secret, created_at) VALUES \
         (7, 1, 'post.published', 'https://hooks.example.com/khalid', zeroblob(32), \
         '2026-10-26T09:00:00Z')",
    )
    .execute(&mut connection)
    .await
    .unwrap();
    let log = DeliveryLog::new(db.clone(), site);
    (dir, db, log)
}

#[cfg(test)]
fn new_delivery(minutes: i64) -> NewDelivery {
    NewDelivery::new(
        WebhookId::new(7),
        "post.published".parse().unwrap(),
        AbsoluteUrl::parse("https://hooks.example.com/khalid").unwrap(),
        br#"{"post":{"title":"What a sitemap does"}}"#.to_vec(),
        at(minutes),
    )
}

#[cfg(test)]
fn attempt(delivery: &mut Delivery, minutes: i64, outcome: Outcome) {
    delivery.record(
        Attempt::new(at(minutes), outcome),
        &RetrySchedule::default(),
    );
}

#[tokio::test]
async fn a_delivery_comes_back_as_it_went_in_and_is_due_on_time() {
    let (_dir, _db, log) = opened().await;
    let mut delivery = log.insert(new_delivery(0)).await.unwrap();
    assert_eq!(
        log.log(WebhookId::new(7)).await.unwrap(),
        [delivery.clone()]
    );
    assert_eq!(log.due(at(0)).await.unwrap(), [delivery.clone()]);

    attempt(&mut delivery, 0, Outcome::Answered { status: 503 });
    log.update(&delivery).await.unwrap();
    assert_eq!(log.due(at(0)).await.unwrap(), []);
    assert_eq!(log.due(at(1)).await.unwrap(), [delivery.clone()]);

    let reason = "the address is on a private network".to_owned();
    attempt(&mut delivery, 1, Outcome::Unreachable { reason });
    attempt(&mut delivery, 6, Outcome::Answered { status: 204 });
    log.update(&delivery).await.unwrap();
    let stored = log.log(WebhookId::new(7)).await.unwrap();
    assert_eq!(stored, [delivery]);
    assert!(matches!(stored[0].state(), DeliveryState::Delivered { .. }));
    assert_eq!(log.due(at(1_000)).await.unwrap(), []);
}

#[tokio::test]
async fn due_deliveries_come_oldest_first_and_only_for_their_site() {
    let (_dir, db, log) = opened().await;
    let later = log.insert(new_delivery(2)).await.unwrap();
    let earlier = log.insert(new_delivery(1)).await.unwrap();
    let mut given_up = log.insert(new_delivery(0)).await.unwrap();
    for round in 0..10 {
        if matches!(given_up.state(), DeliveryState::Pending { .. }) {
            attempt(
                &mut given_up,
                round * 1_000,
                Outcome::Answered { status: 500 },
            );
        }
    }
    assert!(matches!(given_up.state(), DeliveryState::GaveUp { .. }));
    log.update(&given_up).await.unwrap();

    let due = log.due(at(100_000)).await.unwrap();
    assert_eq!(due, [earlier.clone(), later.clone()]);
    let logged = log.log(WebhookId::new(7)).await.unwrap();
    let ids: Vec<DeliveryId> = logged.iter().map(Delivery::id).collect();
    assert_eq!(
        ids,
        [given_up.id(), earlier.id(), later.id()],
        "newest first"
    );

    let other = db.site(&Slug::try_new("other").unwrap()).await.unwrap();
    let other_log = DeliveryLog::new(db.clone(), other);
    assert_eq!(other_log.due(at(100_000)).await.unwrap(), []);
    assert_eq!(other_log.log(WebhookId::new(7)).await.unwrap(), []);
    let error = other_log.update(&earlier).await.unwrap_err();
    assert!(
        matches!(error, DeliveryStoreError::Write { .. }),
        "{error:?}"
    );
}
