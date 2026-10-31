//! The form route answers as plain HTML: a kept submission, and bait too,
//! go on to the thank-you page with 303, anything else gets a page saying
//! why, and nothing but a kept submission is stored (KH-GRO-1, BL-SEC-5).

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::connect_info::MockConnectInfo;
use axum::http::{Request, StatusCode, header};
use chrono::{DateTime, TimeZone, Utc};
use khalid_comments::RateLimits;
use khalid_core::FixedClock;
use khalid_forms::{
    FormId, FormRoute, FormStore, FormStoreError, SiteForms, Submission, form_router,
};
use tower::ServiceExt;

const ACTION: &str = "/blog/forms/contact";
const PAGE: &str = "/blog/write-to-us/";

/// The site's one form, `contact`, on [`PAGE`].
#[derive(Debug)]
struct OneForm;

#[cfg(test)]
impl SiteForms for OneForm {
    fn page_of(&self, form: &FormId) -> Option<String> {
        (form.as_str() == "contact").then(|| PAGE.to_owned())
    }
}

/// Keeps submissions in memory, or fails every write when `broken`.
#[derive(Debug, Default)]
struct Kept {
    rows: Mutex<Vec<(Submission, DateTime<Utc>)>>,
    broken: bool,
}

#[cfg(test)]
impl FormStore for Kept {
    fn insert(
        &self,
        submission: &Submission,
        at: DateTime<Utc>,
    ) -> impl Future<Output = Result<(), FormStoreError>> + Send {
        let kept = if self.broken {
            Err(FormStoreError::Write {
                source: "the disk is full".into(),
            })
        } else {
            self.rows.lock().unwrap().push((submission.clone(), at));
            Ok(())
        };
        std::future::ready(kept)
    }
}

#[cfg(test)]
fn site(store: Kept) -> (Router, Arc<Kept>) {
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 10, 31, 9, 0, 0).unwrap());
    let store = Arc::new(store);
    let route = FormRoute {
        store: Arc::clone(&store),
        forms: Arc::new(OneForm),
        limits: RateLimits::new(clock),
        clock: Box::new(clock),
        mount: "/blog/".to_owned(),
        stylesheet: "/blog/assets/site.css".to_owned(),
    };
    let peer: SocketAddr = "203.0.113.7:50000".parse().unwrap();
    (form_router(route).layer(MockConnectInfo(peer)), store)
}

/// Sends `method` to `path` with `body` as a browser posts a form, and
/// returns the status, the `Location` or `Retry-After` header and the text.
#[cfg(test)]
async fn send(
    router: &Router,
    (method, path): (&str, &str),
    body: &str,
) -> (StatusCode, String, String) {
    let request = Request::builder()
        .method(method)
        .uri(path)
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

#[tokio::test]
async fn a_filled_form_is_kept_and_thanked_for() {
    let (router, store) = site(Kept::default());
    let body = "name=Ada&reply_to=ada%40example.com&message=Hello+there&homepage=";
    let (status, location, _) = send(&router, ("POST", ACTION), body).await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(location, "/blog/forms/contact/sent");
    let rows = store.rows.lock().unwrap().clone();
    let (kept, at) = rows.first().unwrap();
    assert_eq!(kept.form.as_str(), "contact");
    assert_eq!(kept.page, PAGE);
    let fields: Vec<(&str, &str)> = kept
        .fields
        .iter()
        .map(|(n, v)| (n.as_str(), v.as_str()))
        .collect();
    assert_eq!(
        fields,
        [
            ("name", "Ada"),
            ("reply_to", "ada@example.com"),
            ("message", "Hello there")
        ]
    );
    assert_eq!(at.to_rfc3339(), "2026-10-31T09:00:00+00:00");

    let (status, _, page) = send(&router, ("GET", "/blog/forms/contact/sent"), "").await;
    assert_eq!(status, StatusCode::OK);
    assert!(page.contains("Thank you. Your message was sent."), "{page}");
    assert!(page.contains(&format!("href=\"{PAGE}\"")), "{page}");
    assert!(page.contains("href=\"/blog/assets/site.css\""), "{page}");
}

#[tokio::test]
async fn a_robot_that_fills_the_trap_is_thanked_and_kept_nowhere() {
    let (router, store) = site(Kept::default());
    let body = "name=Bot&message=Buy+now&homepage=https%3A%2F%2Fspam.example";
    let (status, location, _) = send(&router, ("POST", ACTION), body).await;
    assert_eq!(
        (status, location.as_str()),
        (StatusCode::SEE_OTHER, "/blog/forms/contact/sent")
    );
    assert!(store.rows.lock().unwrap().is_empty());
}

#[tokio::test]
async fn a_form_the_site_does_not_have_is_not_found() {
    let (router, store) = site(Kept::default());
    for path in ["/blog/forms/survey", "/blog/forms/Not_An_Id"] {
        let (status, _, page) = send(&router, ("POST", path), "name=Ada").await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{path}");
        assert!(page.contains("This form is not on the site."), "{page}");
        assert!(page.contains("href=\"/blog/\""), "{page}");
    }
    let (status, _, _) = send(&router, ("GET", "/blog/forms/survey/sent"), "").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(store.rows.lock().unwrap().is_empty());
}

#[tokio::test]
async fn an_empty_or_unreadable_form_says_why_and_keeps_nothing() {
    let (router, store) = site(Kept::default());
    let long = "a".repeat(5_001);
    let cases = [
        ("name=&message=+&homepage=", "Every field was empty."),
        (&*format!("message={long}"), "longer than 5000 characters"),
        ("Name=Ada", "The form could not be read."),
    ];
    for (body, why) in cases {
        let (status, _, page) = send(&router, ("POST", ACTION), body).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        assert!(page.contains(why), "{page}");
        assert!(page.contains(&format!("href=\"{PAGE}\"")), "{page}");
    }
    assert!(store.rows.lock().unwrap().is_empty());
}

#[tokio::test]
async fn one_address_sending_too_often_is_asked_to_wait() {
    let (router, store) = site(Kept::default());
    for _ in 0..RateLimits::BURST {
        let (status, _, _) = send(&router, ("POST", ACTION), "name=Ada").await;
        assert_eq!(status, StatusCode::SEE_OTHER);
    }
    let (status, retry, page) = send(&router, ("POST", ACTION), "name=Ada").await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(retry, "600");
    assert!(page.contains("10 minutes"), "{page}");
    assert_eq!(store.rows.lock().unwrap().len(), RateLimits::BURST);
}

#[tokio::test]
async fn a_store_that_cannot_write_says_to_try_again() {
    let (router, _) = site(Kept {
        broken: true,
        ..Kept::default()
    });
    let (status, _, page) = send(&router, ("POST", ACTION), "name=Ada").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(
        page.contains("Please try again in a few minutes."),
        "{page}"
    );
    assert!(!page.contains("disk"), "{page}");
}
