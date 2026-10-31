//! The audit log through the Admin API (KH-EDT-6, brief G52): newest first,
//! a page at a time, and only for a key with `admin:all`.

use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use chrono::{TimeZone, Utc};
use khalid_api::api::{self, AuditDesk};
use khalid_api::audit_log::MemoryAuditLog;
use khalid_api::content_store::MemoryContentStore;
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Scope, StoredKey};
use khalid_api::post_store::MemoryPostStore;
use khalid_api::publish_door::{PublishDoor, Silent};
use khalid_api::resources::AdminPost;
use khalid_core::{FixedClock, Slug};
use serde_json::{Value, json};
use tower::ServiceExt;

/// Keys made up for these tests. They open nothing real.
const WRITER: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const OWNER: &str =
    "ki_2222333344445555_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";

/// When every post last changed, and when every change is logged.
const LAST_CHANGE: &str = "2026-10-20T08:30:00Z";

#[cfg(test)]
fn router() -> Router {
    let hashing = KeyHashing::fast_for_tests();
    let token = |text: &str| text.parse::<KeyToken>().unwrap();
    let key = |text, scopes| StoredKey::for_integration(&token(text), scopes, &hashing).unwrap();
    let keys = MemoryKeyStore::new(vec![
        key(WRITER, vec![Scope::PostsWrite]),
        key(OWNER, vec![Scope::AdminAll]),
    ]);
    let at = Utc.with_ymd_and_hms(2026, 10, 20, 8, 30, 0).unwrap();
    let post = |id, slug| AdminPost::new(id, Slug::try_new(slug).unwrap(), "A post", "Text.", at);
    let posts = MemoryPostStore::new(vec![post("en.first", "first"), post("en.second", "second")]);
    let door = PublishDoor::new(Arc::new(posts), Arc::new(Silent));
    let content = MemoryContentStore::new(Vec::new());
    let audit = AuditDesk::new(Arc::new(MemoryAuditLog::default()), FixedClock::new(at));
    let (router, _) =
        api::router_with_audit(Arc::new(content), Arc::new(door), Arc::new(keys), audit);
    router
}

#[cfg(test)]
async fn send(router: &Router, method: &str, path: &str, key: &str) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(method)
        .uri(format!("/api/admin/v1{path}"))
        .header("authorization", format!("Khalid {key}"))
        .body(Body::empty())
        .unwrap();
    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// The log entry for `WRITER` removing post `id`.
#[cfg(test)]
fn removed(id: u32, post: &str) -> Value {
    json!({"id": id, "at": LAST_CHANGE, "by": "1111222233334444", "action": "removed",
        "route": "/api/admin/v1/posts/{id}/", "path": format!("/api/admin/v1/posts/{post}/")})
}

#[tokio::test]
async fn the_log_lists_changes_newest_first_a_page_at_a_time() {
    let router = router();
    for id in ["en.first", "en.second"] {
        let delete = format!("/posts/{id}/?updated_at={LAST_CHANGE}");
        let gone = send(&router, "DELETE", &delete, WRITER).await;
        assert_eq!(gone.0, StatusCode::NO_CONTENT);
    }
    let pages = [
        (
            "",
            json!([removed(2, "en.second"), removed(1, "en.first")]),
            json!(null),
        ),
        ("?limit=1", json!([removed(2, "en.second")]), json!(2)),
        (
            "?limit=1&before=2",
            json!([removed(1, "en.first")]),
            json!(1),
        ),
    ];
    for (query, changes, next) in pages {
        let (status, json) = send(&router, "GET", &format!("/audit/{query}"), OWNER).await;
        assert_eq!(status, StatusCode::OK, "{json}");
        assert_eq!(json, json!({"changes": changes, "next": next}), "{query}");
    }
}

#[tokio::test]
async fn only_a_key_with_admin_all_reads_the_log_and_a_bad_limit_is_refused() {
    let router = router();
    let (status, json) = send(&router, "GET", "/audit/", WRITER).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{json}");
    for bad in ["limit=0", "limit=101", "before=soon"] {
        let (status, json) = send(&router, "GET", &format!("/audit/?{bad}"), OWNER).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad}: {json}");
        assert!(json["detail"].as_str().unwrap().contains("limit"), "{json}");
    }
}
