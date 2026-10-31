//! The tools `khalid mcp` offers send the requests an HTTP client would,
//! through the API's own router (ADR "the MCP server calls the API in
//! process"), so every answer is the API's. Each one comes from an
//! operation in the API's `OpenAPI` document, so the two cannot drift.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use khalid_api::api;
use khalid_api::content_store::MemoryContentStore;
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Scope, StoredKey};
use khalid_api::post_store::MemoryPostStore;
use khalid_api::resources::{AdminPost, Collection, Post};
use khalid_core::{AbsoluteUrl, Locale, Secret, Slug};
use khalid_mcp::{ApiTools, SessionKeys, Tool, ToolResult, Tools};
use serde_json::{Map, Value, json};

/// A key made up for these tests. It opens nothing real.
const WRITER: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";

/// Another made-up key of the right shape, which no store holds.
const STRANGER: &str =
    "ki_5555666677778888_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";

/// The tools, acting with `key`, over a site with one published post and
/// one draft, and the document they were made from.
#[cfg(test)]
fn site(key: &str) -> (ApiTools, Value) {
    let token: KeyToken = WRITER.parse().unwrap();
    let hashing = KeyHashing::fast_for_tests();
    let scopes = vec![Scope::PostsWrite, Scope::PostsPublish, Scope::SettingsWrite];
    let writer = StoredKey::for_integration(&token, scopes, &hashing).unwrap();
    let (keys, reader) = SessionKeys::new(MemoryKeyStore::new(vec![writer])).unwrap();
    let at: DateTime<Utc> = "2026-10-01T09:00:00Z".parse().unwrap();
    let published = Post::new(
        "en.anchor-text",
        Slug::try_new("anchor-text").unwrap(),
        "Choosing anchor text",
        AbsoluteUrl::parse("https://www.example.com/blog/anchor-text/").unwrap(),
        Locale::try_new("en").unwrap(),
        at,
    );
    let content = MemoryContentStore::new(vec![(Collection::Posts, published)]);
    let draft = AdminPost::new(
        "en.internal-links",
        Slug::try_new("internal-links").unwrap(),
        "Internal links",
        "Link often.",
        at,
    )
    .with_author(Some(Slug::try_new("asif").unwrap()));
    let posts = MemoryPostStore::new(vec![draft]);
    let (router, document) = api::router(Arc::new(content), Arc::new(posts), Arc::new(keys));
    let document = serde_json::to_value(&document).unwrap();
    let tools = ApiTools::new(router, &document, Secret::new(key.to_owned()), reader);
    (tools, document)
}

#[cfg(test)]
fn tools(key: &str) -> ApiTools {
    site(key).0
}

#[cfg(test)]
async fn call(tools: &ApiTools, name: &str, arguments: &Value) -> ToolResult {
    let arguments: Map<String, Value> = serde_json::from_value(arguments.clone()).unwrap();
    tools.call(name, arguments).await.unwrap()
}

#[cfg(test)]
fn json(result: &ToolResult) -> Value {
    serde_json::from_str(result.text()).unwrap()
}

#[test]
fn every_operation_a_key_may_call_is_a_tool_of_the_same_name() {
    let (tools, document) = site(WRITER);
    let names: Vec<&str> = tools.list().iter().map(Tool::name).collect();
    let mut operations = Vec::new();
    for item in document["paths"].as_object().unwrap().values() {
        for operation in item.as_object().unwrap().values() {
            let schemes = operation["security"].to_string();
            if schemes.contains("admin_key") || schemes.contains("content_key") {
                operations.push(operation["operationId"].as_str().unwrap());
            }
        }
    }
    assert!(operations.len() > 30, "{operations:?}");
    for operation in &operations {
        assert!(names.contains(operation), "no tool for {operation}");
    }
    let mut expected = operations.clone();
    expected.push("publish_post");
    assert_eq!(names, expected, "one tool per operation, then publish_post");
}

#[test]
fn each_tool_takes_an_object_described_without_outside_references() {
    let tools = tools(WRITER);
    let listed = serde_json::to_value(tools.list()).unwrap();
    for tool in listed.as_array().unwrap() {
        assert_eq!(tool["inputSchema"]["type"], "object", "{tool}");
        assert_eq!(tool["inputSchema"]["additionalProperties"], false, "{tool}");
        assert!(!tool["description"].as_str().unwrap().is_empty(), "{tool}");
        let schema = tool["inputSchema"].to_string();
        assert!(!schema.contains("#/components/"), "{schema}");
        assert!(!schema.contains("Accept-Version"), "{schema}");
    }
    let create = listed
        .as_array()
        .unwrap()
        .iter()
        .find(|tool| tool["name"] == "create_admin_post")
        .unwrap();
    let schema = &create["inputSchema"];
    assert_eq!(schema["required"], json!(["body"]), "{schema}");
    assert_eq!(schema["properties"]["body"]["$ref"], "#/$defs/NewPosts");
    assert!(schema["$defs"]["NewPost"].is_object(), "{schema}");
}

#[tokio::test]
async fn browse_posts_reads_the_content_api() {
    let tools = tools(WRITER);
    let result = call(&tools, "browse_posts", &json!({ "limit": 5, "page": 1 })).await;
    assert!(!result.is_failure(), "{}", result.text());
    let listed = json(&result);
    assert_eq!(listed["posts"][0]["id"], "en.anchor-text");
    assert_eq!(listed["meta"]["pagination"]["total"], 1);
}

#[tokio::test]
async fn a_list_the_api_cannot_read_comes_back_as_its_problem() {
    let tools = tools(WRITER);
    let result = call(&tools, "browse_posts", &json!({ "filter": "colour:red" })).await;
    assert!(result.is_failure());
    assert_eq!(json(&result)["status"], 400);
}

#[tokio::test]
async fn read_admin_post_reads_a_draft() {
    let tools = tools(WRITER);
    let read = call(
        &tools,
        "read_admin_post",
        &json!({ "id": "en.internal-links" }),
    )
    .await;
    assert!(!read.is_failure(), "{}", read.text());
    let post = &json(&read)["posts"][0];
    assert_eq!(post["status"], "draft");
    assert_eq!(post["markdown"], "Link often.");
    assert_eq!(post["updated_at"], "2026-10-01T09:00:00Z");
}

#[tokio::test]
async fn a_key_the_site_does_not_hold_is_refused_by_the_api() {
    let tools = tools(STRANGER);
    let read = call(
        &tools,
        "read_admin_post",
        &json!({ "id": "en.anchor-text" }),
    )
    .await;
    assert!(read.is_failure());
    assert_eq!(json(&read)["status"], 401);
}

#[tokio::test]
async fn an_identity_with_odd_characters_stays_one_path_segment() {
    let tools = tools(WRITER);
    let read = call(&tools, "read_admin_post", &json!({ "id": "en.a/../b?c" })).await;
    assert_eq!(json(&read)["status"], 404);
}

#[tokio::test]
async fn a_missing_or_unknown_argument_is_named() {
    let tools = tools(WRITER);
    let read = call(&tools, "read_admin_post", &json!({})).await;
    assert!(read.is_failure());
    assert!(read.text().contains("needs id"), "{}", read.text());
    let list = call(&tools, "browse_posts", &json!({ "colour": "red" })).await;
    assert!(list.is_failure());
    assert!(list.text().contains("colour"), "{}", list.text());
}

#[tokio::test]
async fn a_body_of_the_wrong_kind_is_refused_before_the_api() {
    let tools = tools(WRITER);
    let csv = call(&tools, "import_redirects", &json!({ "body": 7 })).await;
    assert!(csv.is_failure());
    assert!(csv.text().contains("body"), "{}", csv.text());
    let file = call(
        &tools,
        "upload_media",
        &json!({ "name": "a.png", "body": [300] }),
    )
    .await;
    assert!(file.is_failure());
    assert!(file.text().contains("0 to 255"), "{}", file.text());
}

#[tokio::test]
async fn a_parameter_the_document_places_in_the_path_alone_goes_in_the_query() {
    let tools = tools(WRITER);
    let misses = call(&tools, "list_misses", &json!({ "limit": 5 })).await;
    assert!(!misses.is_failure(), "{}", misses.text());
    let too_many = call(&tools, "list_misses", &json!({ "limit": 5000 })).await;
    assert_eq!(json(&too_many)["rule"], "limit", "the API read the limit");
}

#[tokio::test]
async fn an_unknown_tool_is_left_to_the_protocol() {
    let tools = tools(WRITER);
    assert!(tools.call("delete_site", Map::new()).await.is_none());
}

#[tokio::test]
async fn create_admin_post_saves_a_draft_once_per_idempotency_key() {
    let tools = tools(WRITER);
    let body = json!({ "posts": [{ "title": "Redirects", "author": "asif", "markdown": "Keep old links." }] });
    let arguments = json!({ "Idempotency-Key": "first try", "body": body });
    let created = call(&tools, "create_admin_post", &arguments).await;
    assert!(!created.is_failure(), "{}", created.text());
    let again = call(&tools, "create_admin_post", &arguments).await;
    assert_eq!(json(&again), json(&created), "a retry makes no second post");
    let read = call(&tools, "read_admin_post", &json!({ "id": "redirects" })).await;
    let post = &json(&read)["posts"][0];
    assert_eq!(post["status"], "draft");
    assert_eq!(post["markdown"], "Keep old links.");
}

#[tokio::test]
async fn a_draft_in_html_is_cleaned_and_stored_as_markdown() {
    let tools = tools(WRITER);
    let html = "<p>Read <strong>this</strong>.</p><script>steal()</script>";
    let body = json!({ "posts": [{ "title": "Html", "html": html }] });
    let arguments = json!({ "source": "html", "body": body });
    let created = call(&tools, "create_admin_post", &arguments).await;
    assert_eq!(json(&created)["posts"][0]["markdown"], "Read **this**.");
}

#[tokio::test]
async fn a_draft_the_api_refuses_comes_back_as_its_problem() {
    let tools = tools(WRITER);
    let body = json!({ "posts": [{ "title": " ", "markdown": "x" }] });
    let created = call(&tools, "create_admin_post", &json!({ "body": body })).await;
    assert!(created.is_failure());
    let problem = json(&created);
    assert_eq!(problem["status"], 422);
    assert!(problem["fix"].as_str().is_some(), "{problem}");
}

#[tokio::test]
async fn edit_admin_post_changes_only_the_fields_sent() {
    let tools = tools(WRITER);
    let edit =
        json!({ "markdown": "Link often, and say where.", "updated_at": "2026-10-01T09:00:00Z" });
    let arguments = json!({ "id": "en.internal-links", "body": { "posts": [edit] } });
    let edited = call(&tools, "edit_admin_post", &arguments).await;
    assert!(!edited.is_failure(), "{}", edited.text());
    let post = &json(&edited)["posts"][0];
    assert_eq!(post["markdown"], "Link often, and say where.");
    assert_eq!(post["title"], "Internal links");
    assert_eq!(post["status"], "draft");
}

#[tokio::test]
async fn an_edit_made_on_an_old_copy_is_refused_as_stale() {
    let tools = tools(WRITER);
    let edit = json!({ "title": "Old", "updated_at": "2026-09-01T09:00:00Z" });
    let arguments = json!({ "id": "en.internal-links", "body": { "posts": [edit] } });
    let edited = call(&tools, "edit_admin_post", &arguments).await;
    assert!(edited.is_failure());
    assert_eq!(json(&edited)["status"], 409);
}

#[tokio::test]
async fn publish_post_publishes_a_draft() {
    let tools = tools(WRITER);
    let publish = json!({ "id": "en.internal-links", "updated_at": "2026-10-01T09:00:00Z" });
    let published = call(&tools, "publish_post", &publish).await;
    assert!(!published.is_failure(), "{}", published.text());
    assert_eq!(json(&published)["posts"][0]["status"], "published");
}
