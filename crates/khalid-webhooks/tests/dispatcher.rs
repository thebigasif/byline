//! Sending deliveries: each event reaches each subscribed webhook once,
//! signed, through the outbound guard, and a failure is retried when due.
//! The receiver is a wiremock server on loopback, never the network.

use std::net::IpAddr;
use std::sync::{Arc, Mutex, PoisonError};

use chrono::{DateTime, TimeZone, Utc};
use khalid_core::{AbsoluteUrl, Clock, FixedClock, Secret};
use khalid_outbound::{Limits, Outbound, Reach, Resolver};
use khalid_webhooks::{
    DeliveryState, Dispatcher, Event, MemoryDeliveryStore, Outcome, SigningKey, Verifier, Webhook,
    WebhookId,
};
use serde_json::{Value, json};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// A key made up for these tests. It protects nothing.
const KEY: &str = "test-key-generated-for-dispatcher-tests";

/// When the event happens in these tests.
const START: i64 = 1_793_001_600;

/// A clock the test moves by hand.
#[derive(Debug, Clone)]
struct HandClock(Arc<Mutex<DateTime<Utc>>>);

impl Clock for HandClock {
    fn now(&self) -> DateTime<Utc> {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
fn at(unix: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(unix, 0).single().unwrap()
}

#[cfg(test)]
fn webhook(id: i64, url: &str, events: Vec<Event>) -> Webhook {
    let key = SigningKey::try_new(&Secret::new(KEY.to_owned())).unwrap();
    Webhook::new(
        WebhookId::new(id),
        AbsoluteUrl::parse(url).unwrap(),
        key,
        events,
    )
}

#[cfg(test)]
fn dispatcher(hosts: Vec<(String, Vec<IpAddr>)>) -> (Dispatcher<MemoryDeliveryStore>, HandClock) {
    let resolver = Resolver::for_tests(hosts, Reach::PublicAndLoopback);
    let outbound = Outbound::with_resolver(Limits::default(), resolver).unwrap();
    let clock = HandClock(Arc::new(Mutex::new(at(START))));
    let site = AbsoluteUrl::parse("https://example.com/blog/").unwrap();
    let store = MemoryDeliveryStore::default();
    (Dispatcher::new(outbound, store, clock.clone(), site), clock)
}

#[cfg(test)]
fn post() -> Value {
    json!({"post": {"slug": "why-internal-links-matter", "title": "Why internal links matter"}})
}

#[tokio::test]
async fn an_event_reaches_a_subscriber_once_signed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/hooks"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let (dispatcher, _) = dispatcher(vec![]);
    let url = format!("{}/hooks", server.uri());
    let hooks = [webhook(1, &url, vec![Event::PostPublished])];
    let sent = dispatcher
        .publish(&hooks, Event::PostPublished, &post())
        .await
        .unwrap();
    assert_eq!(sent[0].state(), &DeliveryState::Delivered { at: at(START) });

    let received = server.received_requests().await.unwrap();
    let request = &received[0];
    let header = |name: &str| request.headers.get(name).unwrap().to_str().unwrap();
    assert_eq!(header("content-type"), "application/json");
    assert_eq!(header("khalid-event"), "post.published");
    assert_eq!(header("khalid-delivery"), "1");
    let key = SigningKey::try_new(&Secret::new(KEY.to_owned())).unwrap();
    let verified = Verifier::new(key).verify(
        header("khalid-signature"),
        &request.body,
        &FixedClock::new(at(START)),
    );
    assert_eq!(verified, Ok(()));
    let body: Value = serde_json::from_slice(&request.body).unwrap();
    assert_eq!(
        body,
        json!({
            "event": "post.published",
            "occurred_at": "2026-10-26T08:00:00Z",
            "site": "https://example.com/blog/",
            "post": {"slug": "why-internal-links-matter", "title": "Why internal links matter"},
        })
    );
}

#[tokio::test]
async fn a_webhook_that_did_not_subscribe_gets_nothing() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;
    let (dispatcher, _) = dispatcher(vec![]);
    let hooks = [webhook(1, &server.uri(), vec![Event::PostDeleted])];
    let sent = dispatcher
        .publish(&hooks, Event::PostPublished, &post())
        .await
        .unwrap();
    assert_eq!(sent, []);
}

#[tokio::test]
async fn an_inward_address_is_refused_and_logged_as_unreachable() {
    let hosts = vec![(
        "hooks.example".to_owned(),
        vec!["10.0.0.8".parse().unwrap()],
    )];
    let (dispatcher, _) = dispatcher(hosts);
    let hooks = [webhook(
        1,
        "http://hooks.example/",
        vec![Event::SiteChanged],
    )];
    let sent = dispatcher
        .publish(&hooks, Event::SiteChanged, &json!({"site": {}}))
        .await
        .unwrap();
    let outcome = sent[0].attempts()[0].outcome();
    assert!(
        matches!(outcome, Outcome::Unreachable { reason } if reason.contains("10.0.0.8")),
        "{outcome:?}"
    );
    assert!(matches!(sent[0].state(), DeliveryState::Pending { .. }));
}

#[tokio::test]
async fn every_event_in_the_catalogue_fires_once() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(u64::try_from(Event::ALL.len()).unwrap())
        .mount(&server)
        .await;
    let (dispatcher, _) = dispatcher(vec![]);
    let hooks = [webhook(1, &server.uri(), Event::ALL.to_vec())];
    for event in Event::ALL {
        let sent = dispatcher.publish(&hooks, event, &post()).await.unwrap();
        assert_eq!(sent.len(), 1, "{event}");
    }
    let received = server.received_requests().await.unwrap();
    let names: Vec<&str> = received
        .iter()
        .map(|request| {
            request
                .headers
                .get("khalid-event")
                .unwrap()
                .to_str()
                .unwrap()
        })
        .collect();
    let want: Vec<&str> = Event::ALL.iter().map(|event| event.as_str()).collect();
    assert_eq!(names, want);
}
