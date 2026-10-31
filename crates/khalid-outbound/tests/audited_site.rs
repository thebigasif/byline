//! The client `khalid audit` crawls a running blog with: it may reach the
//! blog on this machine, as `khalid serve` answers it, and still nothing
//! else inward, and it hands each redirect back instead of following it.

use std::net::IpAddr;

use khalid_outbound::{Limits, Outbound, OutboundError, Reach, Resolver};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[cfg(test)]
fn ip(text: &str) -> IpAddr {
    text.parse().unwrap()
}

#[test]
fn the_site_under_audit_may_be_on_loopback_and_nothing_else_inward() {
    let reach = Resolver::for_site_under_audit().reach();
    assert_eq!(reach, Reach::SiteUnderAudit);
    for allowed in ["127.0.0.1", "::1", "93.184.215.14"] {
        assert!(reach.allows(ip(allowed)), "{allowed}");
    }
    for refused in ["10.0.0.1", "192.168.1.5", "169.254.169.254", "fd00::1"] {
        assert!(!reach.allows(ip(refused)), "{refused}");
    }
}

#[tokio::test]
async fn with_no_redirects_allowed_a_redirect_comes_back_as_the_answer() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/blog/old/"))
        .respond_with(ResponseTemplate::new(301).insert_header("location", "/blog/new/"))
        .mount(&server)
        .await;
    let outbound = Outbound::with_resolver(
        Limits::default().with_redirects(0),
        Resolver::for_site_under_audit(),
    )
    .unwrap();
    let request = http::Request::builder()
        .uri(format!("{}/blog/old/", server.uri()))
        .body(Vec::new())
        .unwrap();
    let answer = outbound.exchange(request).await.unwrap();
    assert_eq!(answer.status(), 301);
    assert_eq!(answer.headers()["location"], "/blog/new/");
}

#[tokio::test]
async fn the_public_client_still_refuses_loopback() {
    let outbound = Outbound::new(Limits::default().with_redirects(0)).unwrap();
    let request = http::Request::builder()
        .uri("http://127.0.0.1:9/")
        .body(Vec::new())
        .unwrap();
    let error = outbound.exchange(request).await.unwrap_err();
    assert!(matches!(error, OutboundError::Blocked { .. }), "{error:?}");
}
