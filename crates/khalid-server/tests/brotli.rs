//! Pages are compressed with Brotli once, when they are built, and sent
//! compressed to every browser that accepts it.

use std::io::Read;

use axum::body::Body;
use axum::http::header::{
    ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_LENGTH, ETAG, HeaderName, IF_NONE_MATCH, VARY,
};
use axum::http::{Method, Request, Response, StatusCode};
use http_body_util::BodyExt;
use khalid_server::{MediaType, Page, PagePath, Pages, router};
use tower::ServiceExt;

/// A post long enough to be worth compressing.
const POST: &str = "/blog/what-a-sitemap-does/";

/// A file too short to shrink.
const TINY: &str = "/ok.txt";

#[cfg(test)]
fn post_html() -> String {
    "<p>A sitemap lists the pages a site wants search engines to find.</p>\n".repeat(40)
}

#[cfg(test)]
async fn send(method: Method, uri: &str, headers: &[(HeaderName, &str)]) -> Response<Body> {
    let page =
        |path, media_type, body| Page::new(PagePath::try_new(path).unwrap(), media_type, body);
    let pages = Pages::new(
        vec![
            page(POST, MediaType::Html, post_html()),
            page(TINY, MediaType::PlainText, "ok".to_owned()),
        ],
        "<h1>Page not found</h1>",
    )
    .unwrap();
    let mut request = Request::builder().method(method).uri(uri);
    for (name, value) in headers {
        request = request.header(name, *value);
    }
    let request = request.body(Body::empty()).unwrap();
    router(pages).oneshot(request).await.unwrap()
}

#[cfg(test)]
fn value(response: &Response<Body>, name: &HeaderName) -> Option<String> {
    let value = response.headers().get(name)?;
    Some(value.to_str().unwrap().to_owned())
}

#[cfg(test)]
async fn bytes(response: Response<Body>) -> Vec<u8> {
    let collected = response.into_body().collect().await.unwrap();
    collected.to_bytes().to_vec()
}

#[tokio::test]
async fn a_browser_that_takes_brotli_gets_the_page_compressed() {
    for accepted in ["gzip, deflate, br", "br", "br;q=1.0, gzip;q=0.5", "*"] {
        let response = send(Method::GET, POST, &[(ACCEPT_ENCODING, accepted)]).await;
        assert_eq!(response.status(), StatusCode::OK, "{accepted}");
        assert_eq!(value(&response, &CONTENT_ENCODING).as_deref(), Some("br"));
        assert_eq!(value(&response, &VARY).as_deref(), Some("accept-encoding"));
        let length = value(&response, &CONTENT_LENGTH).unwrap();
        let tag = value(&response, &ETAG).unwrap();
        assert!(tag.ends_with("-br\""), "{tag}");
        let body = bytes(response).await;
        assert_eq!(body.len().to_string(), length);
        assert!(body.len() < post_html().len() / 4, "{} bytes", body.len());
        let mut text = String::new();
        brotli::Decompressor::new(body.as_slice(), 4096)
            .read_to_string(&mut text)
            .unwrap();
        assert_eq!(text, post_html(), "{accepted}");
    }
}

#[tokio::test]
async fn a_browser_that_refuses_brotli_gets_the_plain_page() {
    for accepted in ["", "gzip, deflate", "br;q=0", "br;q=0.000, *"] {
        let response = send(Method::GET, POST, &[(ACCEPT_ENCODING, accepted)]).await;
        assert_eq!(value(&response, &CONTENT_ENCODING), None, "{accepted}");
        assert_eq!(value(&response, &VARY).as_deref(), Some("accept-encoding"));
        assert!(!value(&response, &ETAG).unwrap().ends_with("-br\""));
        assert_eq!(bytes(response).await, post_html().as_bytes(), "{accepted}");
    }
}

#[tokio::test]
async fn a_file_that_does_not_shrink_is_sent_as_it_is() {
    let response = send(Method::GET, TINY, &[(ACCEPT_ENCODING, "br")]).await;
    assert_eq!(value(&response, &CONTENT_ENCODING), None);
    assert_eq!(bytes(response).await, b"ok");
}

#[tokio::test]
async fn head_reports_the_compressed_length() {
    let get = send(Method::GET, POST, &[(ACCEPT_ENCODING, "br")]).await;
    let head = send(Method::HEAD, POST, &[(ACCEPT_ENCODING, "br")]).await;
    assert_eq!(value(&head, &CONTENT_LENGTH), value(&get, &CONTENT_LENGTH));
    assert_eq!(value(&head, &CONTENT_ENCODING).as_deref(), Some("br"));
    assert_eq!(bytes(head).await, b"");
}

#[tokio::test]
async fn either_copy_is_current_for_a_304() {
    let plain = send(Method::GET, POST, &[]).await;
    let plain_tag = value(&plain, &ETAG).unwrap();
    let compressed = send(Method::GET, POST, &[(ACCEPT_ENCODING, "br")]).await;
    let br_tag = value(&compressed, &ETAG).unwrap();
    for sent in [&plain_tag, &br_tag] {
        let asked = [(ACCEPT_ENCODING, "br"), (IF_NONE_MATCH, sent.as_str())];
        let response = send(Method::GET, POST, &asked).await;
        assert_eq!(response.status(), StatusCode::NOT_MODIFIED, "{sent}");
        assert_eq!(value(&response, &ETAG), Some(br_tag.clone()));
        assert_eq!(value(&response, &VARY).as_deref(), Some("accept-encoding"));
        assert_eq!(bytes(response).await, b"", "{sent}");
    }
}
