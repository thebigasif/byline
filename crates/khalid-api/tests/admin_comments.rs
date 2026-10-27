//! The Admin API's comment routes: list, approve, reply and delete, each
//! behind `posts:write`, with problem details when something is wrong.

mod common;

use std::sync::{Arc, Mutex, PoisonError};

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use chrono::{TimeZone, Utc};
use khalid_api::api;
use khalid_api::comment_moderation::{CommentModeration, ModerationError};
use khalid_api::content_store::BoxFuture;
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Scope, StoredKey};
use khalid_api::post_store::MemoryPostStore;
use khalid_api::resources::{AdminComment, CommentReply, CommentStatus, CommentWriter};
use serde_json::{Value, json};
use tower::ServiceExt;

/// Keys made up for these tests. They open nothing real.
const WRITER: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const UPLOADER: &str =
    "ki_2222333344445555_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";

/// Comment 1 waits on a post; anything else is not found.
#[derive(Debug, Default)]
struct OneComment(Mutex<Vec<String>>);

#[cfg(test)]
fn comment(id: i64, status: CommentStatus) -> AdminComment {
    AdminComment {
        id,
        post: "content/en/posts/why-links-matter.md".to_owned(),
        parent: None,
        author_name: "Lena".to_owned(),
        author_kind: CommentWriter::Reader,
        author_email: Some("lena@example.com".to_owned()),
        text: "Great post.".to_owned(),
        status,
        created_at: Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap(),
    }
}

#[cfg(test)]
impl OneComment {
    fn heard(&self, call: String) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(call);
    }
}

#[cfg(test)]
impl CommentModeration for OneComment {
    fn list(
        &self,
        status: Option<CommentStatus>,
        limit: u32,
    ) -> BoxFuture<'_, Result<Vec<AdminComment>, ModerationError>> {
        self.heard(format!("list {status:?} {limit}"));
        Box::pin(async { Ok(vec![comment(1, CommentStatus::Pending)]) })
    }

    fn approve(&self, id: i64) -> BoxFuture<'_, Result<AdminComment, ModerationError>> {
        let found = (id == 1).then(|| comment(1, CommentStatus::Approved));
        Box::pin(async move { found.ok_or(ModerationError::NotFound) })
    }

    fn reply(
        &self,
        id: i64,
        reply: CommentReply,
    ) -> BoxFuture<'_, Result<AdminComment, ModerationError>> {
        Box::pin(async move {
            if reply.text.trim().is_empty() {
                let reason = "the text is empty".to_owned();
                return Err(ModerationError::Invalid {
                    field: "text",
                    reason,
                });
            }
            let mut answer = comment(2, CommentStatus::Approved);
            (answer.parent, answer.author_kind) = (Some(id), CommentWriter::Staff);
            (answer.author_name, answer.text, answer.author_email) =
                (reply.author_name, reply.text, None);
            Ok(answer)
        })
    }

    fn delete(&self, id: i64) -> BoxFuture<'_, Result<(), ModerationError>> {
        Box::pin(async move {
            if id == 1 {
                Ok(())
            } else {
                Err(ModerationError::NotFound)
            }
        })
    }
}

#[cfg(test)]
async fn call(
    moderation: Arc<OneComment>,
    (method, uri): (&str, &str),
    (key, body): (&str, Option<Value>),
) -> (StatusCode, Value) {
    let hashing = KeyHashing::fast_for_tests();
    let token = |text: &str| text.parse::<KeyToken>().unwrap();
    let keys = MemoryKeyStore::new(vec![
        StoredKey::for_integration(&token(WRITER), vec![Scope::PostsWrite], &hashing).unwrap(),
        StoredKey::for_integration(&token(UPLOADER), vec![Scope::MediaWrite], &hashing).unwrap(),
    ]);
    let posts = Arc::new(MemoryPostStore::default());
    let (router, _) =
        api::router_with_comments(Arc::new(common::store()), posts, Arc::new(keys), moderation);
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Khalid {key}"))
        .header("content-type", "application/json");
    let body = body.map_or_else(Body::empty, |json| Body::from(json.to_string()));
    let response = router.oneshot(request.body(body).unwrap()).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1 << 20).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

const BASE: &str = "/api/admin/v1/comments/";

#[tokio::test]
async fn the_queue_lists_pending_comments_with_the_readers_email() {
    let moderation = Arc::new(OneComment::default());
    let uri = format!("{BASE}?status=pending&limit=500");
    let (status, body) = call(Arc::clone(&moderation), ("GET", &uri), (WRITER, None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["comments"][0]["status"], "pending");
    assert_eq!(body["comments"][0]["author_email"], "lena@example.com");
    let heard = moderation.0.lock().unwrap().clone();
    assert_eq!(heard, ["list Some(Pending) 100"], "the limit is capped");
}

#[tokio::test]
async fn an_unknown_status_is_a_problem_naming_the_choices() {
    let uri = format!("{BASE}?status=hidden");
    let (status, body) = call(Arc::default(), ("GET", &uri), (WRITER, None)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        body["detail"]
            .as_str()
            .unwrap()
            .contains("pending, approved, spam or trash")
    );
}

#[tokio::test]
async fn approving_and_deleting_answer_with_the_comment_or_nothing() {
    let approve = ("POST", "/api/admin/v1/comments/1/approve/");
    let (status, body) = call(Arc::default(), approve, (WRITER, None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["comments"][0]["status"], "approved");
    let delete = ("DELETE", "/api/admin/v1/comments/1/");
    assert_eq!(
        call(Arc::default(), delete, (WRITER, None)).await.0,
        StatusCode::NO_CONTENT
    );
    for (method, uri) in [
        ("POST", "/api/admin/v1/comments/9/approve/"),
        ("DELETE", "/api/admin/v1/comments/x/"),
    ] {
        let (status, body) = call(Arc::default(), (method, uri), (WRITER, None)).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{method} {uri}");
        assert_eq!(body["instance"], uri);
    }
}

#[tokio::test]
async fn a_staff_reply_is_created_and_an_empty_one_is_refused() {
    let reply = ("POST", "/api/admin/v1/comments/1/replies/");
    let body = json!({"comments": [{"author_name": "Asif", "text": "Thanks, Lena."}]});
    let (status, answer) = call(Arc::default(), reply, (WRITER, Some(body))).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(answer["comments"][0]["author_kind"], "staff");
    assert_eq!(answer["comments"][0]["parent"], 1);
    assert!(answer["comments"][0].get("author_email").is_none());
    let empty = json!({"comments": [{"author_name": "Asif", "text": " "}]});
    let (status, answer) = call(Arc::default(), reply, (WRITER, Some(empty))).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(answer["rule"], "text");
}

#[tokio::test]
async fn a_key_without_posts_write_cannot_moderate() {
    let (status, body) = call(Arc::default(), ("GET", BASE), (UPLOADER, None)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["rule"], "posts:write");
}
