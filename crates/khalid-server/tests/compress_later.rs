//! The server answers before its pages are compressed: plain pages first,
//! Brotli copies added in the background, and an edit made meanwhile kept.

use axum::body::Body;
use axum::http::header::{ACCEPT_ENCODING, CONTENT_ENCODING};
use axum::http::{Request, Response};
use khalid_server::{LivePages, MediaType, Page, PagePath, Pages, SecurityHeaders, router};
use tower::ServiceExt;

const POST: &str = "/blog/what-a-sitemap-does/";

#[cfg(test)]
fn post(sentence: &str) -> Page {
    let path = PagePath::try_new(POST).unwrap();
    Page::new(
        path,
        MediaType::Html,
        format!("<p>{sentence}</p>\n").repeat(40),
    )
}

#[cfg(test)]
async fn get(live: &LivePages) -> Response<Body> {
    let request = Request::get(POST)
        .header(ACCEPT_ENCODING, "br")
        .body(Body::empty())
        .unwrap();
    router(live.clone(), SecurityHeaders::default())
        .oneshot(request)
        .await
        .unwrap()
}

#[cfg(test)]
fn encoding(response: &Response<Body>) -> Option<&str> {
    let value = response.headers().get(CONTENT_ENCODING)?;
    Some(value.to_str().unwrap())
}

#[tokio::test]
async fn plain_pages_answer_at_once_and_brotli_follows() {
    let plain = Pages::plain(vec![post("A sitemap lists pages.")], "").unwrap();
    let live = LivePages::new(plain);
    assert_eq!(encoding(&get(&live).await), None, "not compressed yet");
    live.compress().await.unwrap();
    assert_eq!(encoding(&get(&live).await), Some("br"));
}

#[tokio::test]
async fn compressing_never_puts_back_pages_an_edit_replaced() {
    let live = LivePages::new(Pages::plain(vec![post("Before the edit.")], "").unwrap());
    let before = live.current();
    let edited = Pages::new(vec![post("After the edit.")], "").unwrap();
    let compressed = before.compressed();
    assert!(live.replace_if_current(&before, edited), "the edit lands");
    assert!(
        !live.replace_if_current(&before, compressed),
        "the old set does not"
    );
    let page = live.current();
    let body = page.get(POST).unwrap().body().unwrap();
    assert!(String::from_utf8_lossy(&body).contains("After the edit."));
}
