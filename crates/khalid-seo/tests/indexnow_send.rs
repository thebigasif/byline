//! Sending changed addresses to `IndexNow` through `Outbound`, with batches
//! and retries (KH-SEO-11). Every server here is a `wiremock` stand-in.

#![cfg(test)]

use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;

use khalid_core::{AbsoluteUrl, IndexNowKey, IndexNowSettings, Locale};
use khalid_outbound::{Limits, Outbound, OutboundError, Reach, Resolver};
use khalid_seo::{IndexNow, Retries, SendIndexNowError, SeoError, SeoSite, UrlPath};
use serde_json::json;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const KEY: &str = "3f9c0b5e2a8d4c71";

fn url(text: &str) -> AbsoluteUrl {
    AbsoluteUrl::parse(text).unwrap()
}

fn site() -> SeoSite {
    let mut site = SeoSite::new(
        "Field Notes on SEO",
        url("https://www.example.com"),
        Locale::try_new("en").unwrap(),
    );
    site.mount = UrlPath::try_new("/blog/").unwrap();
    site
}

/// `IndexNow` sending to `endpoint`.
fn indexnow(endpoint: &str) -> IndexNow {
    let mut settings = IndexNowSettings::new(IndexNowKey::try_new(KEY).unwrap());
    settings.endpoint = Some(url(endpoint));
    IndexNow::new(&site(), &settings).unwrap()
}

/// A client that may reach the local stand-in, and resolves
/// `indexnow.internal` to a private address.
fn outbound(limits: Limits) -> Outbound {
    let hosts = vec![(
        "indexnow.internal".to_owned(),
        vec![IpAddr::V4(Ipv4Addr::new(10, 0, 0, 7))],
    )];
    Outbound::with_resolver(limits, Resolver::for_tests(hosts, Reach::PublicAndLoopback)).unwrap()
}

/// Two retries with no pause, so the tests run at once.
fn retries() -> Retries {
    Retries::new(vec![Duration::ZERO, Duration::ZERO])
}

fn changed() -> Vec<AbsoluteUrl> {
    vec![
        url("https://www.example.com/blog/why-links-matter/"),
        url("https://www.example.com/blog/de/warum-links-zaehlen/"),
    ]
}

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
    let indexnow = indexnow(&format!("{}/indexnow", server.uri()));
    let sent = indexnow
        .send(&outbound(Limits::default()), &retries(), &changed())
        .await
        .unwrap();
    assert_eq!((sent.requests(), sent.urls(), sent.attempts()), (1, 2, 1));
}

#[tokio::test]
async fn accepted_for_later_counts_as_sent() {
    let server = MockServer::start().await;
    answer(&server, 202, 1).await;
    let indexnow = indexnow(&format!("{}/indexnow", server.uri()));
    let sent = indexnow
        .send(&outbound(Limits::default()), &retries(), &changed())
        .await
        .unwrap();
    assert_eq!(sent.attempts(), 1);
}

#[tokio::test]
async fn too_many_requests_is_tried_again_after_a_pause() {
    let server = MockServer::start().await;
    answer(&server, 429, 1).await;
    answer(&server, 200, 1).await;
    let indexnow = indexnow(&format!("{}/indexnow", server.uri()));
    let sent = indexnow
        .send(&outbound(Limits::default()), &retries(), &changed())
        .await
        .unwrap();
    assert_eq!((sent.requests(), sent.attempts()), (1, 2));
}

#[tokio::test]
async fn a_server_error_every_time_gives_up_after_the_last_retry() {
    let server = MockServer::start().await;
    answer(&server, 503, 3).await;
    let indexnow = indexnow(&format!("{}/indexnow", server.uri()));
    let error = indexnow
        .send(&outbound(Limits::default()), &retries(), &changed())
        .await
        .unwrap_err();
    assert!(
        matches!(
            error,
            SendIndexNowError::Status {
                status: 503,
                attempts: 3
            }
        ),
        "{error:?}"
    );
}

#[tokio::test]
async fn a_refused_key_or_address_is_not_tried_again() {
    for status in [400, 403, 422] {
        let server = MockServer::start().await;
        answer(&server, status, 1).await;
        let indexnow = indexnow(&format!("{}/indexnow", server.uri()));
        let error = indexnow
            .send(&outbound(Limits::default()), &retries(), &changed())
            .await
            .unwrap_err();
        assert!(
            matches!(error, SendIndexNowError::Status { status: got, attempts: 1 } if got == status),
            "{status}: {error:?}"
        );
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
    let indexnow = indexnow(&format!("{}/indexnow", server.uri()));
    let limits = Limits::default().with_timeout(Duration::from_millis(200));
    let sent = indexnow
        .send(&outbound(limits), &retries(), &changed())
        .await
        .unwrap();
    assert_eq!(sent.attempts(), 2);
}

#[tokio::test]
async fn an_endpoint_on_a_private_address_is_refused_and_not_tried_again() {
    let indexnow = indexnow("http://indexnow.internal/indexnow");
    let error = indexnow
        .send(&outbound(Limits::default()), &retries(), &changed())
        .await
        .unwrap_err();
    assert!(
        matches!(
            error,
            SendIndexNowError::Request {
                attempts: 1,
                source: OutboundError::Blocked { .. }
            }
        ),
        "{error:?}"
    );
}

#[tokio::test]
async fn an_address_outside_the_blog_sends_nothing() {
    let server = MockServer::start().await;
    never_called(&server).await;
    let indexnow = indexnow(&format!("{}/indexnow", server.uri()));
    let mut changed = changed();
    changed.push(url("https://www.example.com/pricing/"));
    let error = indexnow
        .send(&outbound(Limits::default()), &retries(), &changed)
        .await
        .unwrap_err();
    assert!(
        matches!(
            error,
            SendIndexNowError::Payload {
                source: SeoError::IndexNowScope { .. }
            }
        ),
        "{error:?}"
    );
}

#[tokio::test]
async fn more_than_ten_thousand_addresses_go_in_two_requests() {
    let server = MockServer::start().await;
    answer(&server, 200, 2).await;
    let indexnow = indexnow(&format!("{}/indexnow", server.uri()));
    let changed: Vec<AbsoluteUrl> = (0..10_001)
        .map(|n| url(&format!("https://www.example.com/blog/post-{n}/")))
        .collect();
    let sent = indexnow
        .send(&outbound(Limits::default()), &retries(), &changed)
        .await
        .unwrap();
    assert_eq!(
        (sent.requests(), sent.urls(), sent.attempts()),
        (2, 10_001, 2)
    );
}

#[tokio::test]
async fn nothing_changed_sends_nothing() {
    let server = MockServer::start().await;
    never_called(&server).await;
    let indexnow = indexnow(&format!("{}/indexnow", server.uri()));
    let sent = indexnow
        .send(&outbound(Limits::default()), &retries(), &[])
        .await
        .unwrap();
    assert_eq!((sent.requests(), sent.urls(), sent.attempts()), (0, 0, 0));
}

#[test]
fn the_default_retries_wait_longer_each_time() {
    let pauses = Retries::default();
    assert_eq!(
        pauses.pauses(),
        [
            Duration::from_secs(10),
            Duration::from_mins(1),
            Duration::from_mins(5)
        ]
    );
}
