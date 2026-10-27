//! The comment form posts as plain HTML: a kept comment or bait goes back
//! to the post with 303, anything else gets a page saying why.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::connect_info::MockConnectInfo;
use axum::http::{Request, StatusCode, header};
use chrono::{TimeZone, Utc};
use khalid_comments::{
    CommentStatus, Comments, FormRoute, MemoryCommentStore, PostFile, PostPage, PostPages,
    RateLimits, RefusalPages, form_router,
};
use khalid_core::{FixedClock, Locale};
use tower::ServiceExt;

const POST: &str = "/blog/why-links-matter/";

/// The one post of the test site, in German.
#[derive(Debug)]
struct OnePost;

#[cfg(test)]
impl PostPages for OnePost {
    fn post_at(&self, page: &str) -> Option<PostPage> {
        (page == POST).then(|| PostPage {
            file: PostFile::new("content/de/posts/why-links-matter.md"),
            href: POST.to_owned(),
            language: "de".parse().unwrap(),
        })
    }
}

#[cfg(test)]
fn site() -> (Router, Arc<Comments<MemoryCommentStore>>) {
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap());
    let comments = Arc::new(Comments::new(MemoryCommentStore::default(), clock));
    let route = FormRoute {
        comments: Arc::clone(&comments),
        limits: RateLimits::new(clock),
        pages: Arc::new(OnePost),
        refusals: RefusalPages::load().unwrap(),
        action: "/blog/comments/".to_owned(),
        stylesheet: "/blog/assets/site.css".to_owned(),
        language: "en".parse::<Locale>().unwrap(),
    };
    let peer: SocketAddr = "203.0.113.7:50000".parse().unwrap();
    (form_router(route).layer(MockConnectInfo(peer)), comments)
}

/// Posts `body` as a browser posts a form, and returns the answer's
/// status, its `Location` or `Retry-After` header, and its text.
#[cfg(test)]
async fn send(router: &Router, body: &str) -> (StatusCode, String, String) {
    let request = Request::post("/blog/comments/")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(body.to_owned()))
        .unwrap();
    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let header = [header::LOCATION, header::RETRY_AFTER]
        .iter()
        .find_map(|name| response.headers().get(name))
        .map(|value| value.to_str().unwrap().to_owned())
        .unwrap_or_default();
    let text = to_bytes(response.into_body(), 1 << 20).await.unwrap();
    (status, header, String::from_utf8(text.to_vec()).unwrap())
}

const GOOD: &str = "page=%2Fblog%2Fwhy-links-matter%2F&name=Lena&email=&comment=Danke%21&website=";

#[tokio::test]
async fn a_comment_is_held_and_the_reader_goes_back_to_the_post() {
    let (router, comments) = site();
    let (status, location, _) = send(&router, GOOD).await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(location, "/blog/why-links-matter/#comment-sent");
    let held = comments.listed(None, 9).await.unwrap();
    assert_eq!(held.len(), 1);
    assert_eq!(held[0].status, CommentStatus::Pending);
    assert_eq!(
        held[0].post.as_str(),
        "content/de/posts/why-links-matter.md"
    );
}

#[tokio::test]
async fn bait_is_answered_like_a_comment_and_kept_nowhere() {
    let (router, comments) = site();
    let bait = format!("{GOOD}cheap-pills.example");
    let (status, location, _) = send(&router, &bait).await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(location, "/blog/why-links-matter/#comment-sent");
    assert_eq!(comments.listed(None, 9).await.unwrap().len(), 0);
}

#[tokio::test]
async fn mistakes_come_back_in_the_posts_language() {
    let (router, comments) = site();
    let body = "page=%2Fblog%2Fwhy-links-matter%2F&name=&comment=Danke";
    let (status, _, html) = send(&router, body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(html.contains("<li>Bitte gib deinen Namen an.</li>"));
    assert!(html.contains(">Danke</textarea>"));
    assert_eq!(comments.listed(None, 9).await.unwrap().len(), 0);
}

#[tokio::test]
async fn a_page_that_is_not_a_post_takes_no_comment_and_is_never_a_redirect() {
    let (router, _) = site();
    let body = "page=https%3A%2F%2Fevil.example%2F&name=Lena&comment=Hi";
    let (status, location, html) = send(&router, body).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(location, "");
    assert!(html.contains("This page does not take comments."));
}

#[tokio::test]
async fn a_reply_to_a_comment_that_is_not_shown_is_refused() {
    let (router, _) = site();
    let reply = format!("{GOOD}&parent=1");
    let (status, _, html) = send(&router, &reply.replace("&website=", "")).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(html.contains("Der beantwortete Kommentar wird nicht mehr gezeigt"));
}

#[tokio::test]
async fn a_flood_from_one_address_is_slowed_down() {
    let (router, comments) = site();
    for _ in 0..RateLimits::BURST {
        assert_eq!(send(&router, GOOD).await.0, StatusCode::SEE_OTHER);
    }
    let (status, retry_after, html) = send(&router, GOOD).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(retry_after, "600");
    assert!(html.contains("in 10 Minuten noch einmal"));
    assert_eq!(
        comments.listed(None, 9).await.unwrap().len(),
        RateLimits::BURST
    );
}
