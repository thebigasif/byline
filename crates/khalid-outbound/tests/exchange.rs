//! `Outbound::exchange`: a request sent as given, and the whole answer with
//! its headers, for protocols such as ACME that read them. The same guard
//! and limits apply as for `get` and `post`.

use khalid_outbound::{Limits, Outbound, OutboundError, Reach, Resolver};
use wiremock::matchers::{body_string, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// A client that may reach `reach` and resolves no names.
#[cfg(test)]
fn client(limits: Limits, reach: Reach) -> Outbound {
    Outbound::with_resolver(limits, Resolver::for_tests(Vec::new(), reach)).unwrap()
}

#[cfg(test)]
fn request(method: &str, url: &str, body: &str) -> http::Request<Vec<u8>> {
    http::Request::builder()
        .method(method)
        .uri(url)
        .header("content-type", "application/jose+json")
        .body(body.as_bytes().to_vec())
        .unwrap()
}

#[tokio::test]
async fn a_head_request_answers_with_its_headers() {
    let server = MockServer::start().await;
    Mock::given(method("HEAD"))
        .and(path("/acme/new-nonce"))
        .respond_with(ResponseTemplate::new(200).insert_header("replay-nonce", "nonce-one"))
        .mount(&server)
        .await;
    let outbound = client(Limits::default(), Reach::PublicAndLoopback);

    let url = format!("{}/acme/new-nonce", server.uri());
    let answer = outbound.exchange(request("HEAD", &url, "")).await.unwrap();

    assert_eq!(answer.status(), 200);
    assert_eq!(answer.headers()["replay-nonce"], "nonce-one");
    assert_eq!(answer.body().len(), 0);
}

#[tokio::test]
async fn a_post_sends_its_headers_and_body_and_reads_the_answer() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/acme/new-order"))
        .and(header("content-type", "application/jose+json"))
        .and(body_string("signed"))
        .respond_with(
            ResponseTemplate::new(201)
                .insert_header("location", "https://ca.example/order/1")
                .set_body_string("the order"),
        )
        .mount(&server)
        .await;
    let outbound = client(Limits::default(), Reach::PublicAndLoopback);

    let url = format!("{}/acme/new-order", server.uri());
    let answer = outbound
        .exchange(request("POST", &url, "signed"))
        .await
        .unwrap();

    assert_eq!(answer.status(), 201);
    assert_eq!(answer.headers()["location"], "https://ca.example/order/1");
    assert_eq!(answer.body(), b"the order");
}

#[tokio::test]
async fn a_literal_inward_address_is_refused_before_anything_is_sent() {
    let outbound = client(Limits::default(), Reach::Public);

    let error = outbound
        .exchange(request("POST", "http://10.0.0.1/acme", "signed"))
        .await
        .unwrap_err();

    assert!(
        matches!(&error, OutboundError::Blocked { host, .. } if host == "10.0.0.1"),
        "{error:?}"
    );
}

#[tokio::test]
async fn an_answer_over_the_size_limit_is_refused() {
    let server = MockServer::start().await;
    let big = "x".repeat(Limits::default().max_body_bytes() + 1);
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string(big))
        .mount(&server)
        .await;
    let outbound = client(Limits::default(), Reach::PublicAndLoopback);

    let url = format!("{}/big", server.uri());
    let error = outbound
        .exchange(request("GET", &url, ""))
        .await
        .unwrap_err();

    assert!(matches!(error, OutboundError::TooLarge { .. }), "{error:?}");
}
