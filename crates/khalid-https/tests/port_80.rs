//! Plain HTTP on port 80 when the site serves HTTPS: the answer to Let's
//! Encrypt's file check (HTTP-01), and a move to HTTPS for everything else.
//! And the domain name those addresses are built from.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use khalid_https::{Challenges, Domain, port_80};
use tower::ServiceExt;

/// Port 80's answer to `path` for `blog.test`: its status, its
/// `Location`, and its body.
#[cfg(test)]
async fn get(challenges: &Challenges, path: &str) -> (StatusCode, Option<String>, String) {
    let request = Request::get(path).body(Body::empty()).unwrap();
    let domain = Domain::try_new("blog.test").unwrap();
    let response = port_80(&domain, challenges.clone())
        .oneshot(request)
        .await
        .unwrap();
    let status = response.status();
    let location = response
        .headers()
        .get("location")
        .map(|value| value.to_str().unwrap().to_owned());
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, location, String::from_utf8(body.to_vec()).unwrap())
}

#[tokio::test]
async fn an_open_check_is_answered_with_its_key_authorization() {
    let challenges = Challenges::new();
    challenges.answer_http("token-1", "token-1.thumbprint");

    let answered = get(&challenges, "/.well-known/acme-challenge/token-1").await;

    assert_eq!(answered.0, StatusCode::OK);
    assert_eq!(answered.2, "token-1.thumbprint");
}

#[tokio::test]
async fn a_check_that_is_not_open_is_not_found() {
    let challenges = Challenges::new();
    challenges.answer_http("token-1", "token-1.thumbprint");
    challenges.clear();

    let answered = get(&challenges, "/.well-known/acme-challenge/token-1").await;

    assert_eq!(answered.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn every_other_address_moves_to_https() {
    let answered = get(&Challenges::new(), "/blog/hello/?ref=mail").await;

    assert_eq!(answered.0, StatusCode::PERMANENT_REDIRECT);
    assert_eq!(
        answered.1.as_deref(),
        Some("https://blog.test/blog/hello/?ref=mail")
    );
}

#[test]
fn a_domain_is_kept_in_lower_case_without_its_last_dot() {
    let domain = Domain::try_new("Blog.Example.COM.").unwrap();

    assert_eq!(domain.as_str(), "blog.example.com");
}

#[test]
fn addresses_single_words_and_broken_names_are_not_domains() {
    for text in [
        "",
        "localhost",
        "203.0.113.7",
        "blog example.com",
        "-blog.example.com",
        "blog..example.com",
        "blog.example.com/path",
        "https://blog.example.com",
    ] {
        assert!(Domain::try_new(text).is_err(), "{text:?}");
    }
}
