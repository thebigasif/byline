//! The site's API keys through the Admin API: listed without secrets,
//! made with the secret shown once, and revoked, by an owner only
//! (KH-API-2, RULES.md BL-SEC-3).

use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use khalid_api::api::{self, AdminDesks};
use khalid_api::content_store::MemoryContentStore;
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Scope, StoredKey};
use khalid_api::post_store::MemoryPostStore;
use khalid_api::publish_door::{PublishDoor, Silent};
use serde_json::{Value, json};
use tower::ServiceExt;

/// Keys made up for these tests. They open nothing real.
const OWNER: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const EDITOR: &str =
    "ki_2222333344445555_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";

const KEYS: &str = "/api/admin/v1/keys/";

/// A site with an owner's key and an editor's.
#[cfg(test)]
fn site() -> Router {
    let hashing = KeyHashing::fast_for_tests();
    let token = |text: &str| text.parse::<KeyToken>().unwrap();
    let key = |text, scopes| StoredKey::for_integration(&token(text), scopes, &hashing).unwrap();
    let keys = MemoryKeyStore::new(vec![
        key(OWNER, vec![Scope::AdminAll]),
        key(EDITOR, vec![Scope::PostsWrite, Scope::PostsPublish]),
    ]);
    let door = PublishDoor::new(Arc::new(MemoryPostStore::default()), Arc::new(Silent));
    let content = Arc::new(MemoryContentStore::new(Vec::new()));
    let desks = AdminDesks::none();
    let (router, _) = api::router_with_desks(content, Arc::new(door), Arc::new(keys), desks);
    router
}

#[cfg(test)]
async fn send(
    router: &Router,
    method: &str,
    uri: &str,
    key: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Khalid {key}"))
        .header("content-type", "application/json");
    let body = body.map_or_else(Body::empty, |body| Body::from(body.to_string()));
    let request = request.body(body).unwrap();
    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn an_owner_lists_the_keys_without_their_secrets() {
    let (status, json) = send(&site(), "GET", KEYS, OWNER, None).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["keys"][0]["id"], "1111222233334444", "{json}");
    assert_eq!(json["keys"][0]["scopes"][0], "admin:all", "{json}");
    assert_eq!(json["keys"][1]["kind"], "integration", "{json}");
    let text = json.to_string();
    assert!(
        !text.contains("aaaabbbbcccc"),
        "no secret is ever listed: {text}"
    );
}

#[tokio::test]
async fn a_new_integration_key_is_shown_once_and_works_with_its_scopes() {
    let router = site();
    let new =
        json!({"keys": [{"name": "SerpDad", "kind": "integration", "scopes": ["posts:write"]}]});
    let (status, json) = send(&router, "POST", KEYS, OWNER, Some(new)).await;
    assert_eq!(status, StatusCode::CREATED, "{json}");
    assert_eq!(json["keys"][0]["name"], "SerpDad", "{json}");
    let key = json["keys"][0]["key"].as_str().unwrap().to_owned();
    assert!(key.starts_with("ki_"), "an integration key");
    let (status, json) = send(&router, "GET", KEYS, &key, None).await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "it opens, without admin:all: {json}"
    );
    let (_, json) = send(&router, "GET", KEYS, OWNER, None).await;
    assert_eq!(json["keys"][2]["name"], "SerpDad", "{json}");
    assert_eq!(
        json["keys"][2]["key"],
        Value::Null,
        "never shown again: {json}"
    );
}

#[tokio::test]
async fn a_new_content_key_takes_no_scopes() {
    let new = json!({"keys": [{"name": "Main site", "kind": "content"}]});
    let (status, json) = send(&site(), "POST", KEYS, OWNER, Some(new)).await;
    assert_eq!(status, StatusCode::CREATED, "{json}");
    assert_eq!(json["keys"][0]["kind"], "content", "{json}");
    assert_eq!(json["keys"][0]["scopes"], json!([]), "{json}");
}

#[tokio::test]
async fn a_key_that_does_not_fit_its_kind_is_not_made() {
    let router = site();
    let cases = [
        json!({"keys": [{"name": " ", "kind": "content"}]}),
        json!({"keys": [{"name": "Site", "kind": "content", "scopes": ["posts:write"]}]}),
        json!({"keys": [{"name": "Tool", "kind": "integration"}]}),
        json!({"keys": [{"name": "Tool", "kind": "integration", "scopes": ["posts:everything"]}]}),
        json!({"keys": [{"name": "Lena", "kind": "staff"}]}),
    ];
    for body in cases {
        let (status, json) = send(&router, "POST", KEYS, OWNER, Some(body.clone())).await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{body} gave {json}"
        );
    }
    let new = json!({"keys": [{"name": "Mine", "kind": "content"}]});
    let (status, _) = send(&router, "POST", KEYS, EDITOR, Some(new)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (_, json) = send(&router, "GET", KEYS, OWNER, None).await;
    assert_eq!(
        json["keys"].as_array().map(Vec::len),
        Some(2),
        "nothing made: {json}"
    );
}

#[tokio::test]
async fn listing_keys_needs_admin_all() {
    let (status, json) = send(&site(), "GET", KEYS, EDITOR, None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{json}");
    assert_eq!(json["rule"], "admin:all", "{json}");
}
