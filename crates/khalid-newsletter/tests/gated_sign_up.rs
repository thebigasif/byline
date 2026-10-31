//! Signing up from gated content with scripts off: the form names the page,
//! the confirmation brings the reader back to it with the reader cookie,
//! and a subscriber on a new device gets a pass link instead (G41, task
//! 5.11). Against SQLite and the fake transport.

use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, Method, Request, StatusCode, header};
use khalid_core::{AbsoluteUrl, Locale, Secret, Slug, SystemClock};
use khalid_db::{Database, SubscriberTable};
use khalid_newsletter::{
    MemoryMailer, Newsletter, NewsletterSite, Pages, ReaderPass, SiteLinks, router,
};
use tower::ServiceExt;

const FORM: &str = "email=grace%40example.com&homepage=&back=%2Fblog%2Fguides%2Fchecklist%2F";

#[cfg(test)]
fn site() -> NewsletterSite {
    let address = AbsoluteUrl::parse("https://www.example.com").unwrap();
    NewsletterSite {
        name: "Field Notes".to_owned(),
        locale: Locale::try_new("en").unwrap(),
        links: SiteLinks::new(&address, "/blog/"),
    }
}

#[cfg(test)]
fn reader_pass() -> ReaderPass {
    ReaderPass::new(&Secret::new([3; 32]), &site().links, Arc::new(SystemClock))
}

/// The routes over a fresh database and `mailer`, with the reader pass,
/// kept alive by the folder.
#[cfg(test)]
async fn routes(mailer: &MemoryMailer) -> (tempfile::TempDir, Router) {
    let dir = tempfile::tempdir().unwrap();
    let mut path = PathBuf::from(dir.path());
    path.push("site.db");
    let db = Database::open(&path).await.unwrap();
    db.migrate().await.unwrap();
    let id = db.site(&Slug::try_new("demo").unwrap()).await.unwrap();
    let store = Arc::new(SubscriberTable::new(db, id));
    let mail = Arc::new(mailer.clone());
    let newsletter = Newsletter::new(store, mail, Arc::new(SystemClock), site()).unwrap();
    let newsletter = newsletter.with_pass(reader_pass());
    (dir, router(Pages::new(site()).unwrap(), Some(newsletter)))
}

/// The status and headers `router` answers.
#[cfg(test)]
async fn ask(router: &Router, method: Method, uri: &str, form: &str) -> (StatusCode, HeaderMap) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(form.to_owned()))
        .unwrap();
    let response = router.clone().oneshot(request).await.unwrap();
    (response.status(), response.headers().clone())
}

/// The link starting `/blog/{page}?` in the last email `mailer` sent.
#[cfg(test)]
fn link_in(mailer: &MemoryMailer, page: &str) -> String {
    let text = mailer.sent().last().unwrap().text.clone();
    let start = text.find(&format!("/blog/{page}?")).unwrap();
    let rest = text.get(start..).unwrap();
    rest.split(char::is_whitespace).next().unwrap().to_owned()
}

/// Whether `headers` send the reader to the checklist with a cookie the
/// site made.
#[cfg(test)]
fn lets_in(headers: &HeaderMap) -> bool {
    let cookie = headers[header::SET_COOKIE].to_str().unwrap();
    let sent = cookie.split(';').next().unwrap();
    headers[header::LOCATION] == "/blog/guides/checklist/" && reader_pass().admits(sent)
}

#[tokio::test]
async fn confirming_brings_the_reader_back_with_the_cookie() {
    let mailer = MemoryMailer::default();
    let (_dir, app) = routes(&mailer).await;
    let (status, _) = ask(&app, Method::POST, "/blog/subscribe", FORM).await;
    assert_eq!(status, StatusCode::OK);
    let link = link_in(&mailer, "subscribe/confirm");
    assert!(
        link.ends_with("&back=%2Fblog%2Fguides%2Fchecklist%2F"),
        "{link}"
    );
    let token = link.split("token=").nth(1).unwrap().get(..64).unwrap();
    let button = format!("token={token}&back=%2Fblog%2Fguides%2Fchecklist%2F");
    let (status, headers) = ask(&app, Method::POST, "/blog/subscribe/confirm", &button).await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert!(lets_in(&headers), "{headers:?}");
}

#[tokio::test]
async fn a_subscriber_on_a_new_device_gets_a_pass_link() {
    let mailer = MemoryMailer::default();
    let (_dir, app) = routes(&mailer).await;
    ask(&app, Method::POST, "/blog/subscribe", FORM).await;
    let token = link_in(&mailer, "subscribe/confirm");
    let token = token.split("token=").nth(1).unwrap().get(..64).unwrap();
    let button = format!("token={token}");
    let (status, _) = ask(&app, Method::POST, "/blog/subscribe/confirm", &button).await;
    assert_eq!(status, StatusCode::OK);
    ask(&app, Method::POST, "/blog/subscribe", FORM).await;
    assert_eq!(mailer.sent().len(), 2);
    assert_eq!(
        mailer.sent().last().unwrap().subject,
        "Your link to read Field Notes"
    );
    let link = link_in(&mailer, "subscribe/pass");
    let (status, headers) = ask(&app, Method::GET, &link, "").await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert!(lets_in(&headers), "{headers:?}");
    let forged = link.replace("code=", "code=1");
    let (status, headers) = ask(&app, Method::GET, &forged, "").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(!headers.contains_key(header::SET_COOKIE));
}

#[tokio::test]
async fn a_page_off_the_blog_is_never_where_a_reader_is_sent() {
    let mailer = MemoryMailer::default();
    let (_dir, app) = routes(&mailer).await;
    let away = "email=grace%40example.com&back=https%3A%2F%2Fevil.example%2F";
    ask(&app, Method::POST, "/blog/subscribe", away).await;
    let link = link_in(&mailer, "subscribe/confirm");
    assert!(!link.contains("back="), "{link}");
    let token = link.split("token=").nth(1).unwrap().get(..64).unwrap();
    for back in ["%2F%2Fevil.example%2F", "%2Fblog%2F%2Fevil.example%2F"] {
        let button = format!("token={token}&back={back}");
        let (status, headers) = ask(&app, Method::POST, "/blog/subscribe/confirm", &button).await;
        assert_eq!(status, StatusCode::OK, "{back}");
        assert!(!headers.contains_key(header::LOCATION), "{back}");
        assert!(headers.contains_key(header::SET_COOKIE), "{back}");
    }
}
