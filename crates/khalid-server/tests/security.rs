//! Every response carries the security headers and a Content Security
//! Policy with a nonce made fresh for that request (RULES.md BL-SEC-2).

use axum::body::Body;
use axum::http::header::{self, HeaderName};
use axum::http::{Method, Request, Response, StatusCode};
use khalid_server::{MediaType, Page, PagePath, Pages, SecurityHeaders, ServerError, router};
use tower::ServiceExt;

/// The players an embed may show, as `khalid serve` passes them.
const FRAMES: [&str; 2] = [
    "https://www.youtube-nocookie.com",
    "https://player.vimeo.com",
];

#[cfg(test)]
async fn send(method: Method, uri: &str, tag: Option<&str>) -> Response<Body> {
    let path = PagePath::try_new("/blog/").unwrap();
    let pages = Pages::new(
        vec![Page::new(path, MediaType::Html, "<h1>The blog</h1>")],
        "<h1>Page not found</h1>",
    )
    .unwrap();
    let security = SecurityHeaders::new(&FRAMES).unwrap();
    let mut request = Request::builder().method(method).uri(uri);
    if let Some(tag) = tag {
        request = request.header(header::IF_NONE_MATCH, tag);
    }
    let request = request.body(Body::empty()).unwrap();
    router(pages, security).oneshot(request).await.unwrap()
}

#[cfg(test)]
fn value<'r>(response: &'r Response<Body>, name: &HeaderName) -> &'r str {
    let value = response.headers().get(name).expect("the header is set");
    value.to_str().unwrap()
}

/// The policy's directives by name, such as `script-src`.
#[cfg(test)]
fn directive<'p>(policy: &'p str, name: &str) -> Vec<&'p str> {
    policy
        .split(';')
        .map(str::trim)
        .find_map(|part| part.strip_prefix(name)?.strip_prefix(' '))
        .unwrap_or_else(|| panic!("no {name} in {policy}"))
        .split(' ')
        .collect()
}

#[cfg(test)]
fn nonce(response: &Response<Body>) -> String {
    let policy = value(response, &header::CONTENT_SECURITY_POLICY);
    let sources = directive(policy, "script-src");
    let nonce = sources[0].strip_prefix("'nonce-").unwrap();
    nonce.strip_suffix('\'').unwrap().to_owned()
}

#[tokio::test]
async fn every_response_carries_the_policy_and_the_headers() {
    let tag = send(Method::GET, "/blog/", None).await;
    let tag = value(&tag, &header::ETAG).to_owned();
    let cases = [
        (Method::GET, "/blog/", None, StatusCode::OK),
        (Method::HEAD, "/blog/", None, StatusCode::OK),
        (
            Method::GET,
            "/blog/",
            Some(tag.as_str()),
            StatusCode::NOT_MODIFIED,
        ),
        (Method::GET, "/blog", None, StatusCode::PERMANENT_REDIRECT),
        (Method::GET, "/no-such-page/", None, StatusCode::NOT_FOUND),
        (Method::POST, "/blog/", None, StatusCode::METHOD_NOT_ALLOWED),
    ];
    for (method, uri, tag, status) in cases {
        let response = send(method.clone(), uri, tag).await;
        assert_eq!(response.status(), status, "{method} {uri}");
        assert_eq!(nonce(&response).len(), 32, "{method} {uri}");
        assert_eq!(value(&response, &header::X_CONTENT_TYPE_OPTIONS), "nosniff");
        assert_eq!(value(&response, &header::X_FRAME_OPTIONS), "SAMEORIGIN");
        assert_eq!(
            value(&response, &header::REFERRER_POLICY),
            "strict-origin-when-cross-origin"
        );
        let permissions = HeaderName::from_static("permissions-policy");
        assert!(value(&response, &permissions).contains("camera=()"));
        let opener = HeaderName::from_static("cross-origin-opener-policy");
        assert_eq!(value(&response, &opener), "same-origin");
    }
}

#[tokio::test]
async fn the_nonce_is_new_for_every_request() {
    let first = nonce(&send(Method::GET, "/blog/", None).await);
    let second = nonce(&send(Method::GET, "/blog/", None).await);
    assert_ne!(first, second);
    for nonce in [&first, &second] {
        assert!(nonce.bytes().all(|b| b.is_ascii_hexdigit()), "{nonce}");
    }
}

#[tokio::test]
async fn scripts_run_only_with_the_nonce() {
    let response = send(Method::GET, "/blog/", None).await;
    let policy = value(&response, &header::CONTENT_SECURITY_POLICY);
    assert!(!policy.contains("unsafe-inline"), "{policy}");
    assert!(!policy.contains("unsafe-eval"), "{policy}");
    let script = directive(policy, "script-src");
    assert_eq!(script.len(), 2, "{policy}");
    assert_eq!(script[1], "'strict-dynamic'");
    assert_eq!(directive(policy, "default-src"), ["'none'"]);
    assert_eq!(directive(policy, "object-src"), ["'none'"]);
    assert_eq!(directive(policy, "base-uri"), ["'none'"]);
    assert_eq!(directive(policy, "style-src"), ["'self'"]);
    assert_eq!(directive(policy, "frame-ancestors"), ["'self'"]);
    assert_eq!(directive(policy, "form-action"), ["'self'"]);
    assert_eq!(directive(policy, "frame-src"), FRAMES);
}

#[test]
fn a_site_with_no_players_allows_no_frames() {
    let headers = SecurityHeaders::new(&[]).unwrap();
    assert_eq!(headers, SecurityHeaders::default());
}

#[test]
fn refuses_an_origin_that_would_change_the_policy() {
    let bad = [
        "https://player.vimeo.com; script-src *",
        "http://player.vimeo.com",
        "https://",
        "https://player.vimeo.com/video/1",
        "https://player vimeo.com",
        "*",
    ];
    for origin in bad {
        let error = SecurityHeaders::new(&[origin]).unwrap_err();
        assert!(
            matches!(&error, ServerError::FrameOrigin { origin: got } if got == origin),
            "{error:?}"
        );
    }
}
