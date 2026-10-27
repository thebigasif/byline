//! The newsletter over HTTP with scripts off: the plain form signs up,
//! the email's links confirm and unsubscribe through one button, a mail
//! program's one-click post unsubscribes, and opening a link changes
//! nothing (task 5.5). Against SQLite and the fake transport.

use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode, header};
use khalid_core::{AbsoluteUrl, Locale, Slug, SystemClock};
use khalid_db::{Database, SubscriberTable};
use khalid_newsletter::{MemoryMailer, Newsletter, NewsletterSite, Pages, SiteLinks, router};
use tower::ServiceExt;

#[cfg(test)]
fn site() -> NewsletterSite {
    let address = AbsoluteUrl::parse("https://www.example.com").unwrap();
    NewsletterSite {
        name: "Field Notes".to_owned(),
        locale: Locale::try_new("en").unwrap(),
        links: SiteLinks::new(&address, "/blog/"),
    }
}

/// The routes over a fresh database and `mailer`, kept alive by the folder.
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
    (dir, router(Pages::new(site()).unwrap(), Some(newsletter)))
}

/// The status and page `router` answers.
#[cfg(test)]
async fn ask(router: &Router, method: Method, uri: &str, form: &str) -> (StatusCode, String) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(form.to_owned()))
        .unwrap();
    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), 1 << 16).await.unwrap();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

/// The token after `page?token=` in the last email `mailer` sent.
#[cfg(test)]
fn token_in(mailer: &MemoryMailer, page: &str) -> String {
    let text = mailer.sent().last().unwrap().text.clone();
    let marker = format!("/blog/{page}?token=");
    let start = text.find(&marker).unwrap() + marker.len();
    text.get(start..start + 64).unwrap().to_owned()
}

#[tokio::test]
async fn the_plain_form_signs_up_and_the_link_s_button_confirms() {
    let mailer = MemoryMailer::default();
    let (_dir, app) = routes(&mailer).await;
    let form = "email=grace%40example.com&homepage=";
    let (status, html) = ask(&app, Method::POST, "/blog/subscribe", form).await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("<h1>Check your inbox</h1>"), "{html}");
    let token = token_in(&mailer, "subscribe/confirm");
    let link = format!("/blog/subscribe/confirm?token={token}");
    let (status, html) = ask(&app, Method::GET, &link, "").await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("<h1>Confirm your email</h1>"), "{html}");
    let (_, again) = ask(&app, Method::GET, &link, "").await;
    assert!(
        again.contains("<h1>Confirm your email</h1>"),
        "opening twice changed it"
    );
    let button = format!("token={token}");
    let (status, html) = ask(&app, Method::POST, "/blog/subscribe/confirm", &button).await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("<h1>You are subscribed</h1>"), "{html}");
}

#[tokio::test]
async fn a_mail_program_s_one_click_post_unsubscribes() {
    let mailer = MemoryMailer::default();
    let (_dir, app) = routes(&mailer).await;
    ask(
        &app,
        Method::POST,
        "/blog/subscribe",
        "email=grace%40example.com",
    )
    .await;
    let link = format!(
        "/blog/unsubscribe?token={}",
        token_in(&mailer, "unsubscribe")
    );
    let (status, html) = ask(&app, Method::GET, &link, "").await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        html.contains("<button type=\"submit\">Unsubscribe</button>"),
        "{html}"
    );
    let one_click = "List-Unsubscribe=One-Click";
    let (status, html) = ask(&app, Method::POST, &link, one_click).await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("<h1>You are unsubscribed</h1>"), "{html}");
}

#[tokio::test]
async fn the_block_s_form_at_the_site_root_works_under_a_mount_path() {
    let mailer = MemoryMailer::default();
    let (_dir, app) = routes(&mailer).await;
    let (status, _) = ask(
        &app,
        Method::POST,
        "/subscribe",
        "email=grace%40example.com",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(mailer.sent().len(), 1);
}

#[tokio::test]
async fn a_wrong_address_or_a_filled_trap_sends_nothing() {
    let mailer = MemoryMailer::default();
    let (_dir, app) = routes(&mailer).await;
    for form in ["email=grace", "", "email="] {
        let (status, html) = ask(&app, Method::POST, "/blog/subscribe", form).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{form}");
        assert!(html.contains("action=\"/blog/subscribe\""), "{html}");
    }
    let bot = "email=grace%40example.com&homepage=https%3A%2F%2Fspam.example";
    let (status, html) = ask(&app, Method::POST, "/blog/subscribe", bot).await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("<h1>Check your inbox</h1>"), "{html}");
    assert_eq!(mailer.sent(), []);
}

#[tokio::test]
async fn links_that_do_not_work_say_so() {
    let mailer = MemoryMailer::default();
    let (_dir, app) = routes(&mailer).await;
    let never = format!("token={}", "0".repeat(64));
    for uri in ["/blog/subscribe/confirm", "/blog/unsubscribe"] {
        let (status, html) = ask(&app, Method::POST, uri, &never).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
        assert!(html.contains("<h1>This link does not work</h1>"), "{html}");
        let (status, _) = ask(&app, Method::GET, &format!("{uri}?token=cut"), "").await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
    }
}

#[tokio::test]
async fn a_site_that_cannot_send_email_takes_no_sign_ups() {
    let app = router(Pages::new(site()).unwrap(), None);
    let (status, html) = ask(
        &app,
        Method::POST,
        "/blog/subscribe",
        "email=grace%40example.com",
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(
        html.contains("<h1>Signing up is not open yet</h1>"),
        "{html}"
    );
}

#[tokio::test]
async fn pages_are_never_kept_framed_or_sent_on_as_a_referrer() {
    let mailer = MemoryMailer::default();
    let (_dir, app) = routes(&mailer).await;
    let request = Request::get("/blog/unsubscribe?token=cut")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    let headers = response.headers();
    assert_eq!(headers[header::CACHE_CONTROL], "no-store");
    assert_eq!(headers[header::REFERRER_POLICY], "no-referrer");
    assert_eq!(headers["x-robots-tag"], "noindex");
    let policy = headers[header::CONTENT_SECURITY_POLICY].to_str().unwrap();
    assert!(policy.contains("frame-ancestors 'none'"), "{policy}");
    assert_eq!(headers[header::CONTENT_TYPE], "text/html; charset=utf-8");
}
