//! One webhook's delivery log as the admin reads it from SQLite: the
//! newest first, each with every attempt, and no other webhook's
//! (task 4.10, KH-API-3).

use std::path::PathBuf;

use chrono::{TimeDelta, TimeZone, Utc};
use khalid_api::webhook_store::WebhookStore;
use khalid_core::{AbsoluteUrl, FixedClock, Slug};
use khalid_db::{Database, DeliveryLog, WebhookTable};
use khalid_webhooks::{
    Attempt, DeliveryState, DeliveryStore, Event, NewDelivery, Outcome, RetrySchedule,
};

#[tokio::test]
async fn the_newest_deliveries_of_one_webhook_come_back_with_their_attempts() {
    let dir = tempfile::tempdir().unwrap();
    let mut path = PathBuf::from(dir.path());
    path.push("site.db");
    let db = Database::open(&path).await.unwrap();
    db.migrate().await.unwrap();
    let nine = Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap();
    let site = db.site(&Slug::try_new("demo").unwrap()).await.unwrap();
    let table = WebhookTable::new(db.clone(), site, FixedClock::new(nine));
    let log = DeliveryLog::new(db, site);
    let url = AbsoluteUrl::parse("https://hooks.example.com/khalid").unwrap();
    let (mine, _) = table.create(&url, Event::CommentAdded).await.unwrap();
    let (other, _) = table.create(&url, Event::PostPublished).await.unwrap();

    let mut ids = Vec::new();
    for (minute, webhook) in [
        (0, mine.id()),
        (1, other.id()),
        (2, mine.id()),
        (3, mine.id()),
    ] {
        let at = nine + TimeDelta::minutes(minute);
        let new = NewDelivery::new(
            webhook,
            Event::CommentAdded,
            url.clone(),
            b"{}".to_vec(),
            at,
        );
        let mut delivery = log.insert(new).await.unwrap();
        let refused = Outcome::Unreachable {
            reason: "the connection timed out".to_owned(),
        };
        delivery.record(Attempt::new(at, refused), &RetrySchedule::default());
        delivery.record(
            Attempt::new(
                at + TimeDelta::minutes(1),
                Outcome::Answered { status: 200 },
            ),
            &RetrySchedule::default(),
        );
        log.update(&delivery).await.unwrap();
        ids.push(delivery.id().get());
    }

    let newest = table.deliveries(mine.id().get(), 2).await.unwrap();
    let listed: Vec<i64> = newest.iter().map(|d| d.id().get()).collect();
    assert_eq!(listed, [ids[3], ids[2]], "the two newest of this webhook");
    let delivery = &newest[0];
    assert_eq!(delivery.webhook(), mine.id());
    assert_eq!(delivery.attempts().len(), 2);
    assert_eq!(
        delivery.state(),
        &DeliveryState::Delivered {
            at: nine + TimeDelta::minutes(4)
        }
    );
    let unknown = table.deliveries(99, 50).await.unwrap();
    assert_eq!(unknown, []);
}
