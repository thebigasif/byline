//! Sending changed addresses to `IndexNow` through `Outbound`, with batches
//! and retries (KH-SEO-11). Every server here is a `wiremock` stand-in.

#![cfg(test)]

use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;

use khalid_core::{AbsoluteUrl, IndexNowKey, IndexNowSettings, Locale};
use khalid_outbound::{Limits, Outbound, OutboundError, Reach, Resolver};
use khalid_seo::{IndexNow, Retries, SendIndexNowError, Sent, SeoError, SeoSite, UrlPath};
use serde_json::json;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const KEY: &str = "3f9c0b5e2a8d4c71";

fn url(text: &str) -> AbsoluteUrl {
    AbsoluteUrl::parse(text).unwrap()
}

fn changed() -> Vec<AbsoluteUrl> {
    vec![
        url("https://www.example.com/blog/why-links-matter/"),
        url("https://www.example.com/blog/de/warum-links-zaehlen/"),
    ]
}

/// Sends `changed` for the demo site to `endpoint`, with two retries and no
/// pause. The client may reach the local stand-in, and resolves
/// `indexnow.internal` to a private address.
async fn send(
    endpoint: &str,
    limits: Limits,
    changed: &[AbsoluteUrl],
) -> Result<Sent, SendIndexNowError> {
    let mut site = SeoSite::new(
        "Field Notes on SEO",
        url("https://www.example.com"),
        Locale::try_new("en").unwrap(),
    );
    site.mount = UrlPath::try_new("/blog/").unwrap();
    let mut settings = IndexNowSettings::new(IndexNowKey::try_new(KEY).unwrap());
    settings.endpoint = Some(url(endpoint));
    let indexnow = IndexNow::new(&site, &settings).unwrap();
    let private = vec![IpAddr::V4(Ipv4Addr::new(10, 0, 0, 7))];
    let hosts = vec![("indexnow.internal".to_owned(), private)];
    let resolver = Resolver::for_tests(hosts, Reach::PublicAndLoopback);
    let outbound = Outbound::with_resolver(limits, resolver).unwrap();
    let retries = Retries::new(vec![Duration::ZERO, Duration::ZERO]);
    indexnow.send(&outbound, &retries, changed).await
}

/// A stand-in that answers `status` exactly `times` times.
async fn answer(server: &MockServer, status: u16, times: u64) {
    Mock::given(method("POST"))
        .and(path("/indexnow"))
        .respond_with(ResponseTemplate::new(status))
        .up_to_n_times(times)
        .expect(times)
        .mount(server)
        .await;
}

/// A stand-in that fails the test if anything reaches it.
async fn never_called(server: &MockServer) {
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(server)
        .await;
}

fn counts(sent: &Sent) -> (usize, usize, usize) {
    (sent.requests, sent.urls, sent.attempts)
}

#[tokio::test]
async fn posts_the_changed_addresses_as_json_to_the_endpoint() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/indexnow"))
        .and(header("content-type", "application/json; charset=utf-8"))
        .and(body_json(json!({
            "host": "www.example.com",
            "key": KEY,
            "keyLocation": "https://www.example.com/blog/3f9c0b5e2a8d4c71.txt",
            "urlList": [
                "https://www.example.com/blog/why-links-matter/",
                "https://www.example.com/blog/de/warum-links-zaehlen/",
            ],
        })))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    let endpoint = format!("{}/indexnow", server.uri());
    let sent = send(&endpoint, Limits::default(), &changed()).await;
    assert_eq!(counts(&sent.unwrap()), (1, 2, 1));
}

#[tokio::test]
async fn too_many_requests_is_tried_again_and_accepted_for_later_counts_as_sent() {
    let server = MockServer::start().await;
    answer(&server, 429, 1).await;
    answer(&server, 202, 1).await;
    let endpoint = format!("{}/indexnow", server.uri());
    let sent = send(&endpoint, Limits::default(), &changed()).await;
    assert_eq!(counts(&sent.unwrap()), (1, 2, 2));
}

#[tokio::test]
async fn a_server_error_every_time_gives_up_after_the_last_retry() {
    let server = MockServer::start().await;
    answer(&server, 503, 3).await;
    let endpoint = format!("{}/indexnow", server.uri());
    let error = send(&endpoint, Limits::default(), &changed()).await;
    let Err(SendIndexNowError::Status { status, attempts }) = error else {
        panic!("{error:?}");
    };
    assert_eq!((status, attempts), (503, 3));
}

#[tokio::test]
async fn a_refused_key_or_address_is_not_tried_again() {
    for refused in [400, 403, 422] {
        let server = MockServer::start().await;
        answer(&server, refused, 1).await;
        let endpoint = format!("{}/indexnow", server.uri());
        let error = send(&endpoint, Limits::default(), &changed()).await;
        let Err(SendIndexNowError::Status { status, attempts }) = error else {
            panic!("{refused}: {error:?}");
        };
        assert_eq!((status, attempts), (refused, 1));
    }
}

#[tokio::test]
async fn a_request_that_times_out_is_tried_again() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(5)))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    answer(&server, 200, 1).await;
    let endpoint = format!("{}/indexnow", server.uri());
    let limits = Limits::default().with_timeout(Duration::from_millis(200));
    let sent = send(&endpoint, limits, &changed()).await;
    assert_eq!(counts(&sent.unwrap()), (1, 2, 2));
}

#[tokio::test]
async fn an_endpoint_on_a_private_address_is_refused_and_not_tried_again() {
    let endpoint = "http://indexnow.internal/indexnow";
    let error = send(endpoint, Limits::default(), &changed()).await;
    let Err(SendIndexNowError::Request { attempts, source }) = error else {
        panic!("{error:?}");
    };
    assert_eq!(attempts, 1);
    assert!(
        matches!(source, OutboundError::Blocked { .. }),
        "{source:?}"
    );
}

#[tokio::test]
async fn an_address_outside_the_blog_sends_nothing() {
    let server = MockServer::start().await;
    never_called(&server).await;
    let endpoint = format!("{}/indexnow", server.uri());
    let mut changed = changed();
    changed.push(url("https://www.example.com/pricing/"));
    let error = send(&endpoint, Limits::default(), &changed).await;
    let Err(SendIndexNowError::Payload { source }) = error else {
        panic!("{error:?}");
    };
    assert!(
        matches!(source, SeoError::IndexNowScope { .. }),
        "{source:?}"
    );
}

#[tokio::test]
async fn more_than_ten_thousand_addresses_go_in_two_requests() {
    let server = MockServer::start().await;
    answer(&server, 200, 2).await;
    let endpoint = format!("{}/indexnow", server.uri());
    let changed: Vec<AbsoluteUrl> = (0..10_001)
        .map(|n| url(&format!("https://www.example.com/blog/post-{n}/")))
        .collect();
    let sent = send(&endpoint, Limits::default(), &changed).await;
    assert_eq!(counts(&sent.unwrap()), (2, 10_001, 2));
}

#[tokio::test]
async fn nothing_changed_sends_nothing() {
    let server = MockServer::start().await;
    never_called(&server).await;
    let endpoint = format!("{}/indexnow", server.uri());
    let sent = send(&endpoint, Limits::default(), &[]).await;
    assert_eq!(counts(&sent.unwrap()), (0, 0, 0));
}

#[test]
fn the_default_retries_wait_longer_each_time() {
    let pauses = [10, 60, 300].map(Duration::from_secs);
    assert_eq!(Retries::default().pauses(), pauses);
}
