//! The delivery store contract, shown on the in-memory store: identifiers,
//! what is due, updates after an attempt, and the log per webhook.

use chrono::{DateTime, TimeZone, Utc};
use khalid_core::AbsoluteUrl;
use khalid_webhooks::{
    Attempt, DeliveryId, DeliveryStore, Event, MemoryDeliveryStore, NewDelivery, Outcome,
    RetrySchedule, WebhookId,
};

/// When the first attempt is made in these tests.
const START: i64 = 1_793_001_600;

#[cfg(test)]
fn at(unix: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(unix, 0).single().unwrap()
}

#[cfg(test)]
fn new_delivery(webhook: i64, first: i64) -> NewDelivery {
    NewDelivery::new(
        WebhookId::new(webhook),
        Event::PostPublished,
        AbsoluteUrl::parse("https://hooks.example.com/khalid").unwrap(),
        br#"{"event":"post.published"}"#.to_vec(),
        at(first),
    )
}

#[tokio::test]
async fn insert_gives_each_delivery_a_new_identifier() {
    let store = MemoryDeliveryStore::default();
    let first = store.insert(new_delivery(1, START)).await.unwrap();
    let second = store.insert(new_delivery(1, START)).await.unwrap();
    assert_eq!(first.id(), DeliveryId::new(1));
    assert_eq!(second.id(), DeliveryId::new(2));
    assert_eq!(first.id().get(), 1);
}

#[tokio::test]
async fn due_lists_pending_deliveries_whose_time_has_come_oldest_first() {
    let store = MemoryDeliveryStore::default();
    let early = store.insert(new_delivery(1, START)).await.unwrap();
    let later = store.insert(new_delivery(2, START + 30)).await.unwrap();
    assert_eq!(store.due(at(START - 1)).await.unwrap(), []);
    assert_eq!(
        store.due(at(START)).await.unwrap(),
        std::slice::from_ref(&early)
    );
    assert_eq!(store.due(at(START + 30)).await.unwrap(), [early, later]);
}

#[tokio::test]
async fn an_update_after_an_attempt_is_kept() {
    let store = MemoryDeliveryStore::default();
    let schedule = RetrySchedule::default();
    let mut delivery = store.insert(new_delivery(1, START)).await.unwrap();
    let failed = Attempt::new(at(START), Outcome::Answered { status: 503 });
    delivery.record(failed, &schedule);
    store.update(&delivery).await.unwrap();
    assert_eq!(store.due(at(START + 59)).await.unwrap(), []);
    assert_eq!(store.due(at(START + 60)).await.unwrap(), [delivery.clone()]);
    let answered = Attempt::new(at(START + 60), Outcome::Answered { status: 200 });
    delivery.record(answered, &schedule);
    store.update(&delivery).await.unwrap();
    assert_eq!(store.due(at(START + 86_400)).await.unwrap(), []);
    assert_eq!(store.log(WebhookId::new(1)).await.unwrap(), [delivery]);
}

#[tokio::test]
async fn the_log_lists_one_webhook_newest_first() {
    let store = MemoryDeliveryStore::default();
    let first = store.insert(new_delivery(1, START)).await.unwrap();
    store.insert(new_delivery(2, START)).await.unwrap();
    let second = store.insert(new_delivery(1, START)).await.unwrap();
    let log = store.log(WebhookId::new(1)).await.unwrap();
    assert_eq!(log, [second, first]);
    assert_eq!(store.log(WebhookId::new(3)).await.unwrap(), []);
}

#[tokio::test]
async fn updating_an_unknown_delivery_is_an_error() {
    let elsewhere = MemoryDeliveryStore::default();
    let delivery = elsewhere.insert(new_delivery(1, START)).await.unwrap();
    let store = MemoryDeliveryStore::default();
    let error = store.update(&delivery).await.unwrap_err();
    assert_eq!(error.to_string(), "the delivery log could not be written");
    let cause = std::error::Error::source(&error).map(ToString::to_string);
    assert_eq!(cause.as_deref(), Some("there is no delivery 1"));
}
