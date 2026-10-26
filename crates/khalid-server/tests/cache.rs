//! Each page carries an entity tag made from its bytes, and a reader whose copy
//! is current gets 304 with no body.

use axum::body::Body;
use axum::http::{Method, Request, Response, StatusCode, header};
use http_body_util::BodyExt;
use khalid_server::{MediaType, Page, PagePath, Pages, SecurityHeaders, router};
use tower::ServiceExt;

/// The post every test asks for.
const POST: &str = "/blog/what-a-sitemap-does/";

#[cfg(test)]
fn pages(post_body: &'static str) -> Pages {
    let page = |path: &str, body: &'static str| {
        Page::new(PagePath::try_new(path).unwrap(), MediaType::Html, body)
    };
    Pages::new(
        vec![
            page(POST, post_body),
            page(
                "/de/blog/was-eine-sitemap-tut/",
                "<h1>Was eine Sitemap tut</h1>",
            ),
        ],
        "<h1>Page not found</h1>",
    )
    .unwrap()
}

#[cfg(test)]
async fn send(pages: Pages, method: Method, uri: &str, tag: Option<&str>) -> Response<Body> {
    let mut request = Request::builder().method(method).uri(uri);
    if let Some(tag) = tag {
        request = request.header(header::IF_NONE_MATCH, tag);
    }
    let request = request.body(Body::empty()).unwrap();
    router(pages, SecurityHeaders::default())
        .oneshot(request)
        .await
        .unwrap()
}

#[cfg(test)]
async fn etag(pages: Pages, uri: &str) -> String {
    let response = send(pages, Method::GET, uri, None).await;
    let tag = response.headers().get(header::ETAG).expect("an ETag");
    tag.to_str().unwrap().to_owned()
}

#[cfg(test)]
async fn body(response: Response<Body>) -> Vec<u8> {
    response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes()
        .to_vec()
}

#[tokio::test]
async fn a_page_carries_a_strong_etag_from_its_bytes() {
    let first = etag(pages("<h1>What a sitemap does</h1>"), POST).await;
    let again = etag(pages("<h1>What a sitemap does</h1>"), POST).await;
    let edited = etag(pages("<h1>What a sitemap is for</h1>"), POST).await;
    let other = etag(
        pages("<h1>What a sitemap does</h1>"),
        "/de/blog/was-eine-sitemap-tut/",
    )
    .await;
    assert!(first.starts_with('"') && first.ends_with('"'), "{first}");
    assert_eq!(first.len(), 34, "{first}");
    assert_eq!(first, again, "the same bytes give the same tag");
    assert_ne!(first, edited, "an edit changes the tag");
    assert_ne!(first, other, "another page has another tag");
}

#[tokio::test]
async fn a_page_asks_browsers_to_check_before_reusing_it() {
    let response = send(
        pages("<h1>What a sitemap does</h1>"),
        Method::GET,
        POST,
        None,
    )
    .await;
    let control = response.headers().get(header::CACHE_CONTROL).unwrap();
    assert_eq!(control, "no-cache");
}

#[tokio::test]
async fn a_current_copy_gets_304_with_no_body() {
    let tag = etag(pages("<h1>What a sitemap does</h1>"), POST).await;
    let weak = format!("W/{tag}");
    let listed = format!("\"0123\", {tag}");
    for sent in [tag.as_str(), weak.as_str(), listed.as_str(), "*"] {
        for method in [Method::GET, Method::HEAD] {
            let pages = pages("<h1>What a sitemap does</h1>");
            let response = send(pages, method.clone(), POST, Some(sent)).await;
            assert_eq!(
                response.status(),
                StatusCode::NOT_MODIFIED,
                "{method} {sent}"
            );
            assert_eq!(response.headers().get(header::ETAG).unwrap(), tag.as_str());
            assert_eq!(
                response.headers().get(header::CACHE_CONTROL).unwrap(),
                "no-cache"
            );
            assert!(response.headers().get(header::CONTENT_LENGTH).is_none());
            assert!(body(response).await.is_empty(), "{method} {sent}");
        }
    }
}

#[tokio::test]
async fn an_old_copy_gets_the_edited_page() {
    let old = etag(pages("<h1>What a sitemap does</h1>"), POST).await;
    let edited = pages("<h1>What a sitemap is for</h1>");
    let response = send(edited, Method::GET, POST, Some(&old)).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body(response).await, b"<h1>What a sitemap is for</h1>");
}

#[tokio::test]
async fn the_not_found_page_is_never_answered_with_304() {
    let response = send(
        pages("<h1>What a sitemap does</h1>"),
        Method::GET,
        "/blog/no-such-post/",
        Some("*"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert!(response.headers().get(header::ETAG).is_none());
    assert_eq!(body(response).await, b"<h1>Page not found</h1>");
}
