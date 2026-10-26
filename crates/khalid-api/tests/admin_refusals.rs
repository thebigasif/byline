//! Every Admin API endpoint refuses a missing, wrong or weak key the same
//! way, and no key ever reaches a log line or an answer (RULES.md BL-SEC-3,
//! BL-RUN-5, task 1.18's done line).

mod common;

use std::error::Error;
use std::fmt::{Debug, Write as _};
use std::sync::{Arc, Mutex, PoisonError};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use chrono::{TimeZone, Utc};
use khalid_api::api;
use khalid_api::content_store::BoxFuture;
use khalid_api::keys::{
    CreateKeyError, KeyHashing, KeyId, KeyStore, KeyStoreError, KeyToken, MemoryKeyStore, Scope,
    StoredKey,
};
use khalid_api::post_store::MemoryPostStore;
use khalid_api::resources::AdminPost;
use khalid_core::Slug;
use serde_json::{Value, json};
use tower::ServiceExt;
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

/// Keys made up for these tests. They open nothing real.
const WRITER: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const UPLOADER: &str =
    "ki_2222333344445555_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
/// The writer's identity with another secret.
const WRONG_SECRET: &str =
    "ki_1111222233334444_9999888877776666555544443333222299998888777766665555444433332222";
const CONTENT_KEY: &str =
    "kc_3333444455556666_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";

/// One request to each Admin API endpoint, with a body that would succeed.
#[cfg(test)]
fn endpoints() -> Vec<(&'static str, &'static str, Value)> {
    let edit = json!({"posts": [{
        "title": "Choosing anchor text",
        "markdown": "Name the page a link opens.",
        "updated_at": "2026-10-20T08:30:00Z",
    }]});
    let new = json!({"posts": [{"title": "Internal links", "markdown": "Link with care."}]});
    vec![
        ("GET", "/api/admin/v1/posts/anchor-text-draft/", Value::Null),
        ("PUT", "/api/admin/v1/posts/anchor-text-draft/", edit),
        ("POST", "/api/admin/v1/posts/", new),
    ]
}

#[cfg(test)]
fn router(keys: Arc<dyn KeyStore>) -> Router {
    let draft = AdminPost::new(
        "anchor-text-draft",
        Slug::try_new("choosing-anchor-text").unwrap(),
        "Choosing anchor text",
        "Anchor text tells readers where a link goes.",
        Utc.with_ymd_and_hms(2026, 10, 20, 8, 30, 0).unwrap(),
    );
    let posts = MemoryPostStore::new(vec![draft]);
    let (router, _) = api::router(Arc::new(common::store()), Arc::new(posts), keys);
    router
}

#[cfg(test)]
fn keys() -> Arc<dyn KeyStore> {
    let hashing = KeyHashing::fast_for_tests();
    let token = |text: &str| text.parse::<KeyToken>().unwrap();
    Arc::new(MemoryKeyStore::new(vec![
        StoredKey::for_integration(&token(WRITER), vec![Scope::PostsWrite], &hashing).unwrap(),
        StoredKey::for_integration(&token(UPLOADER), vec![Scope::MediaWrite], &hashing).unwrap(),
        StoredKey::for_token(&token(CONTENT_KEY)),
    ]))
}

/// Sends one request and returns the status, the `WWW-Authenticate`
/// header and the whole answer as text.
#[cfg(test)]
async fn send(
    router: &Router,
    (method, uri, body): &(&str, &str, Value),
    authorization: Option<&str>,
) -> (StatusCode, Option<String>, String) {
    let mut request = Request::builder()
        .method(*method)
        .uri(*uri)
        .header("content-type", "application/json");
    if let Some(value) = authorization {
        request = request.header("authorization", value);
    }
    let body = if body.is_null() {
        Body::empty()
    } else {
        Body::from(body.to_string())
    };
    let response = router
        .clone()
        .oneshot(request.body(body).unwrap())
        .await
        .unwrap();
    let challenge = response
        .headers()
        .get("www-authenticate")
        .map(|value| value.to_str().unwrap().to_owned());
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (
        status,
        challenge,
        String::from_utf8(bytes.to_vec()).unwrap(),
    )
}

#[tokio::test]
async fn every_endpoint_answers_its_writer() {
    let router = router(keys());
    let writer = format!("Khalid {WRITER}");
    let expected = [StatusCode::OK, StatusCode::OK, StatusCode::CREATED];
    for (endpoint, status) in endpoints().iter().zip(expected) {
        let (got, _, body) = send(&router, endpoint, Some(&writer)).await;
        assert_eq!(got, status, "{} {}: {body}", endpoint.0, endpoint.1);
    }
}

#[tokio::test]
async fn every_endpoint_refuses_a_missing_wrong_or_content_key_with_a_401() {
    let router = router(keys());
    let refused = [
        None,
        Some(format!("Khalid {WRONG_SECRET}")),
        Some(format!("Khalid {CONTENT_KEY}")),
        Some(format!("Bearer {WRITER}")),
        Some("Khalid not-a-key".to_owned()),
    ];
    for endpoint in &endpoints() {
        for authorization in &refused {
            let (status, challenge, body) = send(&router, endpoint, authorization.as_deref()).await;
            let case = format!("{} {} with {authorization:?}", endpoint.0, endpoint.1);
            assert_eq!(status, StatusCode::UNAUTHORIZED, "{case}");
            assert_eq!(challenge.as_deref(), Some("Khalid"), "{case}");
            let problem: Value = serde_json::from_str(&body).unwrap();
            assert_eq!(
                problem["type"], "https://khalid.dev/problems/unauthorized",
                "{case}"
            );
        }
    }
}

#[tokio::test]
async fn every_endpoint_refuses_a_key_without_posts_write_with_a_403() {
    let router = router(keys());
    let uploader = format!("Khalid {UPLOADER}");
    for endpoint in &endpoints() {
        let (status, _, body) = send(&router, endpoint, Some(&uploader)).await;
        let problem: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(
            (status, problem["rule"].clone()),
            (StatusCode::FORBIDDEN, json!("posts:write")),
            "{} {}",
            endpoint.0,
            endpoint.1
        );
    }
}

#[tokio::test(flavor = "current_thread")]
async fn no_key_reaches_a_log_line_or_an_answer() {
    let log = Arc::new(Mutex::new(String::new()));
    let _recording = tracing::subscriber::set_default(Recorder(Arc::clone(&log)));
    let mut answers = String::new();
    let working = router(keys());
    let broken = router(Arc::new(BrokenKeys));
    for endpoint in &endpoints() {
        for key in [WRITER, UPLOADER, WRONG_SECRET, CONTENT_KEY] {
            let authorization = format!("Khalid {key}");
            for router in [&working, &broken] {
                let (_, _, body) = send(router, endpoint, Some(&authorization)).await;
                answers.push_str(&body);
            }
        }
    }
    let log = log.lock().unwrap_or_else(PoisonError::into_inner).clone();
    assert!(
        log.contains("the keys could not be read"),
        "the broken store was logged: {log}"
    );
    for key in [WRITER, UPLOADER, WRONG_SECRET, CONTENT_KEY] {
        let (_, secret) = key.rsplit_once('_').unwrap();
        assert!(!log.contains(secret), "a secret reached the log");
        assert!(!answers.contains(secret), "a secret reached an answer");
    }
}

/// A key store whose disk has gone away.
#[derive(Debug)]
struct BrokenKeys;

impl KeyStore for BrokenKeys {
    fn find<'a>(
        &'a self,
        _id: &'a KeyId,
    ) -> BoxFuture<'a, Result<Option<StoredKey>, KeyStoreError>> {
        Box::pin(std::future::ready(Err(disk_gone())))
    }

    fn create(&self, _key: StoredKey) -> BoxFuture<'_, Result<(), CreateKeyError>> {
        let source = disk_gone();
        Box::pin(std::future::ready(Err(CreateKeyError::Unavailable {
            source,
        })))
    }

    fn revoke<'a>(&'a self, _id: &'a KeyId) -> BoxFuture<'a, Result<bool, KeyStoreError>> {
        Box::pin(std::future::ready(Err(disk_gone())))
    }
}

#[cfg(test)]
fn disk_gone() -> KeyStoreError {
    KeyStoreError::new(std::io::Error::other("disk I/O error"))
}

/// Writes every event and span field, with each error's whole chain of
/// causes, into one string.
#[cfg(test)]
struct Recorder(Arc<Mutex<String>>);

#[cfg(test)]
impl Recorder {
    fn write(&self, record: impl FnOnce(&mut FieldText)) {
        let mut text = FieldText(String::new());
        record(&mut text);
        let mut log = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        log.push_str(&text.0);
        log.push('\n');
    }
}

#[cfg(test)]
impl Subscriber for Recorder {
    fn enabled(&self, _: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, span: &Attributes<'_>) -> Id {
        self.write(|text| span.record(text));
        Id::from_u64(1)
    }

    fn record(&self, _: &Id, values: &Record<'_>) {
        self.write(|text| values.record(text));
    }

    fn record_follows_from(&self, _: &Id, _: &Id) {}

    fn event(&self, event: &Event<'_>) {
        self.write(|text| event.record(text));
    }

    fn enter(&self, _: &Id) {}

    fn exit(&self, _: &Id) {}
}

#[cfg(test)]
struct FieldText(String);

#[cfg(test)]
impl Visit for FieldText {
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        write!(self.0, " {}={value:?}", field.name()).unwrap();
    }

    fn record_error(&mut self, field: &Field, value: &(dyn Error + 'static)) {
        write!(self.0, " {}={value}", field.name()).unwrap();
        let mut cause = value.source();
        while let Some(error) = cause {
            write!(self.0, ": {error}").unwrap();
            cause = error.source();
        }
    }
}
