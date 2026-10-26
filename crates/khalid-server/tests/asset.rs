//! A file whose name carries a hash of its content, such as the stylesheet,
//! is sent with its media type and cached for a year without a check.
//! An old name, from a page cached before the file changed, finds nothing.

use std::io::Read;

use axum::body::Body;
use axum::http::header::{ACCEPT_ENCODING, CACHE_CONTROL, CONTENT_ENCODING, CONTENT_TYPE};
use axum::http::{Request, Response, StatusCode};
use http_body_util::BodyExt;
use khalid_server::{MediaType, Page, PagePath, Pages, SecurityHeaders, router};
use tower::ServiceExt;

/// The hashed stylesheet's address.
const CSS: &str = "/blog/assets/site.3f2a9c1e.css";

/// A stylesheet long enough to be worth compressing.
#[cfg(test)]
fn css() -> String {
    "a { color: var(--color-link); }\n".repeat(40)
}

#[cfg(test)]
async fn send(uri: &str, accept: &str) -> Response<Body> {
    let path = PagePath::try_new(CSS).unwrap();
    let pages = Pages::new(vec![Page::immutable(path, MediaType::Css, css())], "").unwrap();
    let request = Request::get(uri)
        .header(ACCEPT_ENCODING, accept)
        .body(Body::empty())
        .unwrap();
    router(pages, SecurityHeaders::default())
        .oneshot(request)
        .await
        .unwrap()
}

#[cfg(test)]
fn header(response: &Response<Body>, name: &axum::http::HeaderName) -> String {
    response.headers()[name].to_str().unwrap().to_owned()
}

#[tokio::test]
async fn a_hashed_stylesheet_is_css_cached_for_a_year_and_compressed() {
    let response = send(CSS, "gzip, br").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(header(&response, &CONTENT_TYPE), "text/css; charset=utf-8");
    assert_eq!(
        header(&response, &CACHE_CONTROL),
        "public, max-age=31536000, immutable"
    );
    assert_eq!(header(&response, &CONTENT_ENCODING), "br");
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let mut text = String::new();
    brotli::Decompressor::new(&body[..], 4096)
        .read_to_string(&mut text)
        .unwrap();
    assert_eq!(text, css());
}

#[tokio::test]
async fn an_old_stylesheet_name_is_not_found_and_not_cached_for_a_year() {
    let response = send("/blog/assets/site.00000000.css", "br").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert!(response.headers().get(CACHE_CONTROL).is_none());
}
