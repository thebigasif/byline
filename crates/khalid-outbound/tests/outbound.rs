//! The outbound client: the guard in front of every request, and the time
//! and size limits. Names resolve through a test table, never real DNS.

use std::net::IpAddr;
use std::time::Duration;

use khalid_core::AbsoluteUrl;
use khalid_outbound::{Limits, Outbound, OutboundError, Reach, Resolver};
use wiremock::matchers::path;
use wiremock::{Mock, MockServer, ResponseTemplate};

#[cfg(test)]
fn url(text: &str) -> AbsoluteUrl {
    AbsoluteUrl::parse(text).unwrap()
}

/// A client whose names resolve through `hosts` and may reach `reach`.
#[cfg(test)]
fn client(limits: Limits, hosts: &[(&str, &str)], reach: Reach) -> Outbound {
    let hosts: Vec<(String, Vec<IpAddr>)> = hosts
        .iter()
        .map(|(name, ip)| ((*name).to_owned(), vec![ip.parse().unwrap()]))
        .collect();
    Outbound::with_resolver(limits, Resolver::for_tests(hosts, reach)).unwrap()
}

#[tokio::test]
async fn a_request_to_a_name_that_resolves_inward_is_refused() {
    let outbound = client(
        Limits::default(),
        &[("metadata.example", "169.254.169.254")],
        Reach::Public,
    );
    let error = outbound
        .get(&url("http://metadata.example/latest/"))
        .await
        .unwrap_err();
    assert!(
        matches!(&error, OutboundError::Blocked { host, .. } if host == "metadata.example"),
        "{error:?}"
    );
}

#[tokio::test]
async fn a_literal_inward_address_is_refused_without_a_lookup() {
    let outbound = client(Limits::default(), &[], Reach::Public);
    for text in [
        "http://10.0.0.1/",
        "http://127.0.0.1:8080/",
        "http://[fd00::1]/",
    ] {
        let error = outbound.get(&url(text)).await.unwrap_err();
        assert!(
            matches!(error, OutboundError::Blocked { .. }),
            "{text}: {error:?}"
        );
    }
}

#[tokio::test]
async fn fetches_a_page_and_its_body() {
    let server = MockServer::start().await;
    Mock::given(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string("hello"))
        .mount(&server)
        .await;
    let outbound = client(Limits::default(), &[], Reach::PublicAndLoopback);
    let page = outbound.get(&url(&server.uri())).await.unwrap();
    assert_eq!(page.status(), 200);
    assert_eq!(page.body(), b"hello");
}

#[tokio::test]
async fn a_body_over_the_limit_is_refused() {
    let server = MockServer::start().await;
    let big = vec![b'x'; Limits::default().max_body_bytes() + 1];
    Mock::given(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(big))
        .mount(&server)
        .await;
    let outbound = client(Limits::default(), &[], Reach::PublicAndLoopback);
    let error = outbound.get(&url(&server.uri())).await.unwrap_err();
    assert!(matches!(error, OutboundError::TooLarge { .. }), "{error:?}");
}

#[tokio::test]
async fn a_slow_server_times_out() {
    let server = MockServer::start().await;
    Mock::given(path("/"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(5)))
        .mount(&server)
        .await;
    let limits = Limits::default().with_timeout(Duration::from_millis(200));
    let error = client(limits, &[], Reach::PublicAndLoopback)
        .get(&url(&server.uri()))
        .await
        .unwrap_err();
    assert!(matches!(error, OutboundError::TimedOut), "{error:?}");
}

#[tokio::test]
async fn a_redirect_to_a_public_page_is_followed() {
    let server = MockServer::start().await;
    let target = format!("{}/moved", server.uri());
    Mock::given(path("/"))
        .respond_with(ResponseTemplate::new(301).insert_header("location", target.as_str()))
        .mount(&server)
        .await;
    Mock::given(path("/moved"))
        .respond_with(ResponseTemplate::new(200).set_body_string("here"))
        .mount(&server)
        .await;
    let outbound = client(Limits::default(), &[], Reach::PublicAndLoopback);
    let page = outbound.get(&url(&server.uri())).await.unwrap();
    assert_eq!((page.status(), page.body()), (200, &b"here"[..]));
}

#[tokio::test]
async fn too_many_redirects_are_refused() {
    let server = MockServer::start().await;
    let again = format!("{}/", server.uri());
    Mock::given(path("/"))
        .respond_with(ResponseTemplate::new(302).insert_header("location", again.as_str()))
        .mount(&server)
        .await;
    let outbound = client(Limits::default(), &[], Reach::PublicAndLoopback);
    let error = outbound.get(&url(&server.uri())).await.unwrap_err();
    assert!(
        matches!(error, OutboundError::TooManyRedirects { limit: 5 }),
        "{error:?}"
    );
}

#[tokio::test]
async fn a_redirect_to_a_literal_inward_address_is_refused() {
    let server = MockServer::start().await;
    Mock::given(path("/"))
        .respond_with(ResponseTemplate::new(302).insert_header("location", "http://10.0.0.1/admin"))
        .mount(&server)
        .await;
    let outbound = client(Limits::default(), &[], Reach::PublicAndLoopback);
    let error = outbound.get(&url(&server.uri())).await.unwrap_err();
    assert!(
        matches!(&error, OutboundError::Blocked { host, .. } if host == "10.0.0.1"),
        "{error:?}"
    );
}

#[tokio::test]
async fn a_redirect_to_a_name_that_resolves_inward_is_refused() {
    let server = MockServer::start().await;
    Mock::given(path("/"))
        .respond_with(
            ResponseTemplate::new(302).insert_header("location", "http://router.example/"),
        )
        .mount(&server)
        .await;
    let outbound = client(
        Limits::default(),
        &[("router.example", "192.168.1.1")],
        Reach::PublicAndLoopback,
    );
    let error = outbound.get(&url(&server.uri())).await.unwrap_err();
    assert!(
        matches!(&error, OutboundError::Blocked { host, .. } if host == "router.example"),
        "{error:?}"
    );
}
