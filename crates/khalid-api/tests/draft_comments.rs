//! Comments on drafts through the Admin API: threaded, attached to a
//! block, resolved, and told to whoever listens (KH-EDT-4).

mod common;

use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use chrono::{TimeZone, Utc};
use khalid_api::api;
use khalid_api::content_store::BoxFuture;
use khalid_api::draft_comments::{
    DraftComment, DraftCommentNotice, DraftComments, MemoryDraftComments,
};
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Role, StoredKey};
use khalid_api::post_store::MemoryPostStore;
use khalid_api::publish_door::{PublishDoor, Silent};
use khalid_api::resources::AdminPost;
use khalid_core::{FixedClock, Slug};
use serde_json::{Value, json};
use tower::ServiceExt;

/// Staff tokens made up for these tests. They open nothing real.
const WRITER: &str =
    "ks_2222333344445555_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const EDITOR: &str =
    "ks_3333444455556666_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";

/// Each comment told, with the authors of the comments before it.
#[derive(Default)]
struct Told(Mutex<Vec<(String, i64, Vec<String>)>>);

impl DraftCommentNotice for Told {
    fn added<'a>(
        &'a self,
        post: &'a AdminPost,
        comment: &'a DraftComment,
        earlier: &'a [DraftComment],
    ) -> BoxFuture<'a, ()> {
        let authors = earlier.iter().map(|c| c.author.clone()).collect();
        let told = (post.id().to_owned(), comment.id, authors);
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(told);
        Box::pin(async {})
    }
}

/// The API over one draft, `en.notes`, and who was told about comments.
#[cfg(test)]
fn site() -> (Router, Arc<Told>) {
    let hashing = KeyHashing::fast_for_tests();
    let staff = |text: &str, role| {
        StoredKey::for_staff(&text.parse::<KeyToken>().unwrap(), role, &hashing).unwrap()
    };
    let keys = vec![staff(WRITER, Role::Writer), staff(EDITOR, Role::Editor)];
    let at = Utc.with_ymd_and_hms(2026, 10, 20, 8, 30, 0).unwrap();
    let slug = Slug::try_new("notes").unwrap();
    let draft = AdminPost::new("en.notes", slug, "Notes", "One.\n\nTwo.", at);
    let posts = Arc::new(MemoryPostStore::new(vec![draft]));
    let door = Arc::new(PublishDoor::new(posts, Arc::new(Silent)));
    let told = Arc::new(Told::default());
    let store = MemoryDraftComments::with_clock(FixedClock::new(at));
    let comments = DraftComments::new(Arc::new(store)).with_notice(Arc::<Told>::clone(&told));
    let keys = Arc::new(MemoryKeyStore::new(keys));
    let (router, _) =
        api::router_with_draft_comments(Arc::new(common::store()), door, keys, comments);
    (router, told)
}

#[cfg(test)]
async fn send(
    router: &Router,
    token: &str,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(method)
        .uri(path)
        .header("authorization", format!("Khalid {token}"))
        .header("content-type", "application/json")
        .body(body.map_or_else(Body::empty, |body| Body::from(body.to_string())))
        .unwrap();
    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

const COMMENTS: &str = "/api/admin/v1/posts/en.notes/draft-comments/";

#[tokio::test]
async fn a_reply_joins_its_thread_on_the_same_block_and_everyone_is_told() {
    let (router, told) = site();
    let first = json!({"draft_comments": [{"text": "Name the tool here?", "block": 2}]});
    let (status, json) = send(&router, EDITOR, "POST", COMMENTS, Some(first)).await;
    assert_eq!(status, StatusCode::CREATED, "{json}");
    let saved = &json["draft_comments"][0];
    assert_eq!(saved["block"], 2);
    assert_eq!(saved["resolved"], false);
    assert_eq!(saved["author"], "3333444455556666");
    let thread = saved["id"].clone();
    let whole = json!({"draft_comments": [{"text": "Good start."}]});
    let (status, _) = send(&router, EDITOR, "POST", COMMENTS, Some(whole)).await;
    assert_eq!(status, StatusCode::CREATED);
    let reply = json!({"draft_comments": [{"text": "Done, it is Ahrefs.", "parent": thread}]});
    let (status, json) = send(&router, WRITER, "POST", COMMENTS, Some(reply)).await;
    assert_eq!(status, StatusCode::CREATED, "{json}");
    assert_eq!(
        json["draft_comments"][0]["block"], 2,
        "a reply takes its thread's block"
    );
    let told = told.0.lock().unwrap().clone();
    assert_eq!(told.len(), 3);
    let editor = "3333444455556666".to_owned();
    assert_eq!(
        told[2],
        ("en.notes".to_owned(), 3, vec![editor.clone(), editor])
    );
}

#[tokio::test]
async fn bad_comments_are_refused_with_what_to_send() {
    let (router, told) = site();
    let first = json!({"draft_comments": [{"text": "Check this."}]});
    send(&router, EDITOR, "POST", COMMENTS, Some(first)).await;
    let reply = json!({"draft_comments": [{"text": "Ok.", "parent": 1}]});
    send(&router, WRITER, "POST", COMMENTS, Some(reply)).await;
    for body in [
        json!({"draft_comments": [{"text": "   "}]}),
        json!({"draft_comments": [{"text": "Hi.", "block": 0}]}),
        json!({"draft_comments": [{"text": "Hi.", "parent": 2}]}),
        json!({"draft_comments": [{"text": "Hi.", "parent": 9}]}),
        json!({"draft_comments": []}),
    ] {
        let (status, json) = send(&router, WRITER, "POST", COMMENTS, Some(body.clone())).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}: {json}");
        assert!(json["fix"].as_str().is_some(), "{json}");
    }
    let missing = "/api/admin/v1/posts/en.nothing/draft-comments/";
    let body = json!({"draft_comments": [{"text": "Hi."}]});
    let (status, _) = send(&router, WRITER, "POST", missing, Some(body)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        told.0.lock().unwrap().len(),
        2,
        "a refused comment tells nobody"
    );
}

/// The texts of the post's comments, in the order the list sends them,
/// and whether the first is resolved.
#[cfg(test)]
async fn listed(router: &Router) -> (Vec<String>, bool) {
    let (status, json) = send(router, WRITER, "GET", COMMENTS, None).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    let all = json["draft_comments"].as_array().unwrap();
    let texts = all.iter().map(|c| c["text"].as_str().unwrap().to_owned());
    (texts.collect(), all[0]["resolved"] == true)
}

#[tokio::test]
async fn threads_list_together_and_resolve_by_their_first_comment() {
    let (router, _) = site();
    for (token, body) in [
        (EDITOR, json!({"text": "Shorter title?"})),
        (EDITOR, json!({"text": "Good start."})),
        (WRITER, json!({"text": "Yes.", "parent": 1})),
    ] {
        let body = Some(json!({"draft_comments": [body]}));
        send(&router, token, "POST", COMMENTS, body).await;
    }
    let order = ["Shorter title?", "Yes.", "Good start."];
    assert_eq!(
        listed(&router).await,
        (order.map(str::to_owned).to_vec(), false)
    );
    let thread = |id: u8| format!("{COMMENTS}{id}/");
    let resolve = |resolved| Some(json!({"draft_comments": [{"resolved": resolved}]}));
    let (status, json) = send(&router, WRITER, "PUT", &thread(1), resolve(true)).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "{json}");
    assert!(listed(&router).await.1);
    let (status, _) = send(&router, WRITER, "PUT", &thread(1), resolve(false)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(!listed(&router).await.1);
    let (status, json) = send(&router, WRITER, "PUT", &thread(3), resolve(true)).await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "a reply begins no thread: {json}"
    );
}
