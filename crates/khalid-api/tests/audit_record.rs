//! The Admin API's key check logs every write that succeeds, with who made
//! it, and nothing else (KH-EDT-6, brief G52).

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::{TimeZone, Utc};
use khalid_api::api::{self, AuditDesk};
use khalid_api::audit_log::{Action, AuditLog, Change, MemoryAuditLog};
use khalid_api::content_store::MemoryContentStore;
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Scope, StoredKey};
use khalid_api::post_store::MemoryPostStore;
use khalid_api::publish_door::{PublishDoor, Silent};
use khalid_api::resources::AdminPost;
use khalid_core::{FixedClock, Slug};
use tower::ServiceExt;

/// A key made up for these tests. It opens nothing real.
const WRITER: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";

/// When every post last changed, and when every change is logged.
const LAST_CHANGE: &str = "2026-10-20T08:30:00Z";

/// The API over two drafts, logging into `log`.
#[cfg(test)]
fn router(log: Arc<MemoryAuditLog>) -> Router {
    let hashing = KeyHashing::fast_for_tests();
    let token = WRITER.parse::<KeyToken>().unwrap();
    let key = StoredKey::for_integration(&token, vec![Scope::PostsWrite], &hashing).unwrap();
    let at = Utc.with_ymd_and_hms(2026, 10, 20, 8, 30, 0).unwrap();
    let post = |id, slug| AdminPost::new(id, Slug::try_new(slug).unwrap(), "A post", "Text.", at);
    let posts = MemoryPostStore::new(vec![post("en.first", "first"), post("en.second", "second")]);
    let door = PublishDoor::new(Arc::new(posts), Arc::new(Silent));
    let content = MemoryContentStore::new(Vec::new());
    let keys = MemoryKeyStore::new(vec![key]);
    let audit = AuditDesk::new(log, FixedClock::new(at));
    let (router, _) =
        api::router_with_audit(Arc::new(content), Arc::new(door), Arc::new(keys), audit);
    router
}

#[cfg(test)]
async fn send(router: &Router, method: &str, path: &str) -> StatusCode {
    let request = Request::builder()
        .method(method)
        .uri(format!("/api/admin/v1{path}"))
        .header("authorization", format!("Khalid {WRITER}"))
        .body(Body::empty())
        .unwrap();
    router.clone().oneshot(request).await.unwrap().status()
}

#[tokio::test]
async fn each_write_that_succeeds_is_logged_and_reads_and_refusals_are_not() {
    let log = Arc::new(MemoryAuditLog::default());
    let router = router(Arc::clone(&log));
    assert_eq!(send(&router, "GET", "/posts/").await, StatusCode::OK);
    let stale = "/posts/en.first/?updated_at=2026-10-19T08:30:00Z";
    assert_eq!(send(&router, "DELETE", stale).await, StatusCode::CONFLICT);
    let delete = format!("/posts/en.first/?updated_at={LAST_CHANGE}");
    assert_eq!(
        send(&router, "DELETE", &delete).await,
        StatusCode::NO_CONTENT
    );

    let logged = log.changes(None, 10).await.unwrap();
    let changes: Vec<&Change> = logged.iter().map(|logged| &logged.change).collect();
    let removed = Change::new(
        LAST_CHANGE.parse().unwrap(),
        "1111222233334444",
        Action::Removed,
        "/api/admin/v1/posts/{id}/",
        "/api/admin/v1/posts/en.first/",
    );
    assert_eq!(changes, [&removed]);
}
