//! The retry timer sends a failed webhook delivery again once it is due,
//! and not before, to the webhooks in the list as it reads it each round.
//! The receiver is a wiremock server on loopback.

use std::convert::Infallible;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use chrono::{DateTime, TimeZone, Utc};
use khalid_core::{AbsoluteUrl, Clock, Secret};
use khalid_outbound::{Limits, Outbound, Reach, Resolver};
use khalid_server::retry_deliveries;
use khalid_webhooks::{Dispatcher, Event, MemoryDeliveryStore, SigningKey, Webhook, WebhookId};
use serde_json::json;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

/// How often the timer runs in the test.
const EVERY: Duration = Duration::from_millis(20);

/// A clock the test moves by hand.
#[derive(Debug, Clone)]
struct HandClock(Arc<Mutex<DateTime<Utc>>>);

impl Clock for HandClock {
    fn now(&self) -> DateTime<Utc> {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
fn at(seconds: i64) -> DateTime<Utc> {
    let start = Utc.with_ymd_and_hms(2026, 10, 26, 9, 0, 0).unwrap();
    start + chrono::TimeDelta::seconds(seconds)
}

#[cfg(test)]
async fn received(server: &MockServer) -> usize {
    server.received_requests().await.unwrap().len()
}

/// Waits up to ten seconds for `server` to have `count` requests.
#[cfg(test)]
async fn reaches(server: &MockServer, count: usize) -> bool {
    let reached = tokio::time::timeout(Duration::from_secs(10), async {
        while received(server).await < count {
            tokio::time::sleep(EVERY).await;
        }
    });
    reached.await.is_ok()
}

/// A receiver that fails the first delivery, a dispatcher on a hand clock
/// at `at(0)`, and the one webhook whose first delivery failed.
#[cfg(test)]
async fn failed_once() -> (
    MockServer,
    Arc<Dispatcher<MemoryDeliveryStore>>,
    HandClock,
    Webhook,
) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;
    let resolver = Resolver::for_tests(vec![], Reach::PublicAndLoopback);
    let outbound = Outbound::with_resolver(Limits::default(), resolver).unwrap();
    let clock = HandClock(Arc::new(Mutex::new(at(0))));
    let site = AbsoluteUrl::parse("https://example.com/blog/").unwrap();
    let store = MemoryDeliveryStore::default();
    let dispatcher = Arc::new(Dispatcher::new(outbound, store, clock.clone(), site));
    // A key made up for this test. It protects nothing.
    let key = SigningKey::try_new(&Secret::new(
        "test-key-generated-for-the-retry-timer-test".to_owned(),
    ))
    .unwrap();
    let url = AbsoluteUrl::parse(&server.uri()).unwrap();
    let hook = Webhook::new(WebhookId::new(1), url, key, vec![Event::PostPublished]);
    let post = json!({"post": {"title": "What a sitemap does"}});
    dispatcher
        .publish(std::slice::from_ref(&hook), Event::PostPublished, &post)
        .await
        .unwrap();
    assert_eq!(received(&server).await, 1);
    (server, dispatcher, clock, hook)
}

#[tokio::test]
async fn a_failed_delivery_is_sent_again_once_it_is_due() {
    let (server, dispatcher, clock, hook) = failed_once().await;
    let hooks = vec![hook];
    let list = move || std::future::ready(Ok::<_, Infallible>(hooks.clone()));
    let retrying = retry_deliveries(Arc::clone(&dispatcher), list, EVERY);
    tokio::time::sleep(EVERY * 5).await;
    assert_eq!(received(&server).await, 1, "not due for a minute");

    *clock.0.lock().unwrap() = at(60);
    assert!(
        reaches(&server, 2).await,
        "the due delivery was never sent again"
    );
    tokio::time::sleep(EVERY * 5).await;
    retrying.stop().await;
    assert_eq!(
        received(&server).await,
        2,
        "delivered once, then left alone"
    );
}

#[tokio::test]
async fn the_timer_reads_the_webhook_list_afresh_each_round() {
    let (server, dispatcher, clock, hook) = failed_once().await;
    let listed = Arc::new(AtomicBool::new(false));
    let list = {
        let listed = Arc::clone(&listed);
        move || {
            let hooks = if listed.load(Ordering::SeqCst) {
                vec![hook.clone()]
            } else {
                Vec::new()
            };
            std::future::ready(Ok::<_, Infallible>(hooks))
        }
    };
    let retrying = retry_deliveries(Arc::clone(&dispatcher), list, EVERY);
    *clock.0.lock().unwrap() = at(60);
    tokio::time::sleep(EVERY * 5).await;
    assert_eq!(
        received(&server).await,
        1,
        "a webhook missing from the list is not sent to"
    );

    listed.store(true, Ordering::SeqCst);
    assert!(
        reaches(&server, 2).await,
        "a webhook added to the list was never retried"
    );
    retrying.stop().await;
}
