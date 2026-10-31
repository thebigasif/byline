//! The schema template routes of the Admin API (G54, KH-SEO-4): read and
//! replace the node every post in a collection gets.

mod common;

use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use chrono::{TimeZone, Utc};
use khalid_api::api::{self, AdminDesks};
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Scope, StoredKey};
use khalid_api::post_store::MemoryPostStore;
use khalid_api::publish_door::{PublishDoor, Silent};
use khalid_api::resources::NonBlank;
use khalid_api::schema_template_store::{
    AdminSchemaTemplate, MemorySchemaTemplates, SchemaTemplates, TemplateTarget,
};
use serde_json::{Value, json};
use tower::ServiceExt;

/// Keys made up for these tests. They open nothing real.
const ADMIN: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const WRITER: &str =
    "ki_5555666677778888_0000111122223333444455556666777788889999aaaabbbbccccddddeeeeffff";

/// When the templates last changed, before any test changes them.
const LAST_CHANGE: &str = "2026-10-30T09:00:00Z";

/// An API whose site has a `Course` template for its guides.
#[cfg(test)]
fn router() -> axum::Router {
    let hashing = KeyHashing::fast_for_tests();
    let stored = |text: &str, scope| {
        let token: KeyToken = text.parse().unwrap();
        StoredKey::for_integration(&token, vec![scope], &hashing).unwrap()
    };
    let keys = MemoryKeyStore::new(vec![
        stored(ADMIN, Scope::SettingsWrite),
        stored(WRITER, Scope::PostsWrite),
    ]);
    let guides = NonBlank::try_new("guides").unwrap();
    let mut course = AdminSchemaTemplate::new(TemplateTarget::Collection, guides, "Course");
    course
        .properties
        .insert("provider".to_owned(), "{site}".to_owned());
    let templates = SchemaTemplates {
        templates: vec![course],
        collections: vec!["posts".to_owned(), "guides".to_owned()],
        updated_at: Utc.with_ymd_and_hms(2026, 10, 30, 9, 0, 0).unwrap(),
        restart_needed: false,
    };
    let desks = AdminDesks {
        schema_templates: Some(Arc::new(MemorySchemaTemplates::new(templates))),
        ..AdminDesks::none()
    };
    let door = PublishDoor::new(Arc::new(MemoryPostStore::default()), Arc::new(Silent));
    let content = Arc::new(common::store());
    let (router, _) = api::router_with_desks(content, Arc::new(door), Arc::new(keys), desks);
    router
}

/// Sends `body`, or nothing for a read, to the templates route with `key`.
#[cfg(test)]
async fn send(router: &axum::Router, key: &str, body: Option<Value>) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(if body.is_some() { "PUT" } else { "GET" })
        .uri("/api/admin/v1/schema-templates/")
        .header("authorization", format!("Khalid {key}"))
        .header("content-type", "application/json")
        .body(body.map_or_else(Body::empty, |json| Body::from(json.to_string())))
        .unwrap();
    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[cfg(test)]
fn recipe() -> Value {
    json!({"applies_to": "collection", "name": "posts", "type": "Recipe",
        "properties": {"name": "{title}", "author": "{author}"}})
}

#[tokio::test]
async fn the_templates_are_read_with_the_collections_to_pick_from() {
    let (status, json) = send(&router(), ADMIN, None).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    let course = json!({"applies_to": "collection", "name": "guides", "type": "Course",
        "properties": {"provider": "{site}"}});
    assert_eq!(json["templates"], json!([course]));
    assert_eq!(json["collections"], json!(["posts", "guides"]));
    assert_eq!(json["restart_needed"], false);
}

#[tokio::test]
async fn a_new_list_replaces_them_all_and_asks_for_a_restart() {
    let router = router();
    let change = json!({"templates": [recipe()], "updated_at": LAST_CHANGE});
    let (status, json) = send(&router, ADMIN, Some(change)).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["templates"], json!([recipe()]));
    assert_eq!(json["restart_needed"], true);
    let (_, read) = send(&router, ADMIN, None).await;
    assert_eq!(read["templates"], json!([recipe()]));
}

#[tokio::test]
async fn a_change_on_an_old_copy_is_a_409() {
    let router = router();
    let change = json!({"templates": [], "updated_at": LAST_CHANGE});
    let (status, _) = send(&router, ADMIN, Some(change.clone())).await;
    assert_eq!(status, StatusCode::OK);
    let (status, json) = send(&router, ADMIN, Some(change)).await;
    assert_eq!(status, StatusCode::CONFLICT, "{json}");
    assert_eq!(json["rule"], "updated_at");
}

#[tokio::test]
async fn a_template_google_could_not_read_is_a_422_and_nothing_is_saved() {
    let router = router();
    let cases = [
        json!({"applies_to": "collection", "name": "posts", "type": "recipe"}),
        json!({"applies_to": "collection", "name": "posts", "type": "Recipe",
            "properties": {"Name": "{title}"}}),
        json!({"applies_to": "collection", "name": "posts", "type": "Recipe",
            "properties": {"name": "{headline}"}}),
        json!({"applies_to": "collection", "name": " ", "type": "Recipe"}),
        json!({"applies_to": "folder", "name": "posts", "type": "Recipe"}),
    ];
    for template in cases {
        let change = json!({"templates": [recipe(), template], "updated_at": LAST_CHANGE});
        let (status, json) = send(&router, ADMIN, Some(change)).await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{template}: {json}"
        );
    }
    let (_, read) = send(&router, ADMIN, None).await;
    assert_eq!(read["templates"][0]["type"], "Course");
    let named = json!({"templates": [recipe(), {"applies_to": "collection", "name": "posts",
        "type": "recipe"}], "updated_at": LAST_CHANGE});
    let (_, json) = send(&router, ADMIN, Some(named)).await;
    let what = json["detail"].as_str().unwrap();
    assert!(what.contains("template 2:"), "{json}");
}

#[tokio::test]
async fn a_key_without_settings_write_is_a_403() {
    let (status, json) = send(&router(), WRITER, None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{json}");
}
