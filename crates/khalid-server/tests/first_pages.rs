//! A server that starts with its first pages answers those at once, and
//! holds a request for any other address until the whole site is served.

use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use khalid_server::{LivePages, MediaType, Page, PagePath, Pages, SecurityHeaders, router};
use tower::ServiceExt;

/// Long enough that a request which does not wait has long answered.
const SETTLE: Duration = Duration::from_millis(200);

#[cfg(test)]
fn page(path: &str, body: &'static str) -> Page {
    Page::new(PagePath::try_new(path).unwrap(), MediaType::Html, body)
}

#[cfg(test)]
fn home() -> Page {
    page("/blog/", "<h1>Generated Field Notes</h1>")
}

#[cfg(test)]
fn whole() -> Pages {
    let post = page("/blog/what-a-sitemap-does/", "<h1>What a sitemap does</h1>");
    Pages::new(vec![home(), post], "<h1>Page not found</h1>").unwrap()
}

#[cfg(test)]
async fn get(live: &LivePages, path: &str) -> (StatusCode, String) {
    let request = Request::get(path).body(Body::empty()).unwrap();
    let response = router(live.clone(), SecurityHeaders::default())
        .oneshot(request)
        .await
        .unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

#[tokio::test]
async fn a_first_page_answers_at_once() {
    let live = LivePages::new(Pages::first(vec![home()]).unwrap());
    assert!(!live.current().is_whole());
    let (status, body) = tokio::time::timeout(SETTLE, get(&live, "/blog/"))
        .await
        .expect("the home page does not wait");
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "<h1>Generated Field Notes</h1>");
}

#[tokio::test]
async fn any_other_address_waits_for_the_whole_site() {
    let live = LivePages::new(Pages::first(vec![home()]).unwrap());
    let asking = tokio::spawn({
        let live = live.clone();
        async move { get(&live, "/blog/what-a-sitemap-does/").await }
    });
    tokio::time::sleep(SETTLE).await;
    assert!(!asking.is_finished(), "a first set never answers 404");

    let first = live.current();
    assert!(live.replace_if_current(&first, whole()));
    let (status, body) = asking.await.unwrap();
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "<h1>What a sitemap does</h1>");

    let (status, body) = tokio::time::timeout(SETTLE, get(&live, "/blog/nothing-here/"))
        .await
        .expect("the whole site answers a missing address at once");
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body, "<h1>Page not found</h1>");
}

#[tokio::test]
async fn a_saved_page_keeps_a_first_set_first() {
    let live = LivePages::new(Pages::first(vec![home()]).unwrap());
    live.show_saved(vec![page("/blog/what-a-sitemap-does/", "<p>Saved.</p>")]);
    assert!(
        !live.current().is_whole(),
        "the rest of the site still waits"
    );
    let (status, _) = get(&live, "/blog/what-a-sitemap-does/").await;
    assert_eq!(status, StatusCode::OK);
}
