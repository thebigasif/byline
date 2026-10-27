//! The tools `khalid mcp` offers send the requests an HTTP client would,
//! through the API's own router (ADR "the MCP server calls the API in
//! process"), so every answer is the API's.

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
/// one draft.
#[cfg(test)]
fn tools(key: &str) -> ApiTools {
    let token: KeyToken = WRITER.parse().unwrap();
    let hashing = KeyHashing::fast_for_tests();
    let writer = StoredKey::for_integration(&token, vec![Scope::PostsWrite], &hashing).unwrap();
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
    let (router, _) = api::router(Arc::new(content), Arc::new(posts), Arc::new(keys));
    ApiTools::new(router, Secret::new(key.to_owned()), reader)
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
fn each_tool_takes_an_object() {
    let tools = tools(WRITER);
    let names: Vec<&str> = tools.list().iter().map(Tool::name).collect();
    assert_eq!(names, ["list_posts", "read_post"]);
    let listed = serde_json::to_value(tools.list()).unwrap();
    for tool in listed.as_array().unwrap() {
        assert_eq!(tool["inputSchema"]["type"], "object", "{tool}");
        assert!(
            tool["description"].as_str().unwrap().ends_with('.'),
            "{tool}"
        );
    }
}

#[tokio::test]
async fn list_posts_reads_the_content_api() {
    let tools = tools(WRITER);
    let result = call(&tools, "list_posts", &json!({ "limit": 5, "page": 1 })).await;
    assert!(!result.is_failure(), "{}", result.text());
    let listed = json(&result);
    assert_eq!(listed["posts"][0]["id"], "en.anchor-text");
    assert_eq!(listed["meta"]["pagination"]["total"], 1);
}

#[tokio::test]
async fn a_list_the_api_cannot_read_comes_back_as_its_problem() {
    let tools = tools(WRITER);
    let result = call(&tools, "list_posts", &json!({ "filter": "colour:red" })).await;
    assert!(result.is_failure());
    assert_eq!(json(&result)["status"], 400);
}

#[tokio::test]
async fn read_post_reads_a_draft_through_the_admin_api() {
    let tools = tools(WRITER);
    let read = call(&tools, "read_post", &json!({ "id": "en.internal-links" })).await;
    assert!(!read.is_failure(), "{}", read.text());
    let post = &json(&read)["posts"][0];
    assert_eq!(post["status"], "draft");
    assert_eq!(post["markdown"], "Link often.");
    assert_eq!(post["author"], "asif");
    assert_eq!(post["updated_at"], "2026-10-01T09:00:00Z");
}

#[tokio::test]
async fn a_key_the_site_does_not_hold_is_refused_by_the_api() {
    let tools = tools(STRANGER);
    let read = call(&tools, "read_post", &json!({ "id": "en.anchor-text" })).await;
    assert!(read.is_failure());
    assert_eq!(json(&read)["status"], 401);
}

#[tokio::test]
async fn an_identity_with_odd_characters_stays_one_path_segment() {
    let tools = tools(WRITER);
    let read = call(&tools, "read_post", &json!({ "id": "en.a/../b?c" })).await;
    assert_eq!(json(&read)["status"], 404);
}

#[tokio::test]
async fn a_missing_or_unknown_argument_is_named() {
    let tools = tools(WRITER);
    let read = call(&tools, "read_post", &json!({})).await;
    assert!(read.is_failure());
    assert!(read.text().contains("id"), "{}", read.text());
    let list = call(&tools, "list_posts", &json!({ "colour": "red" })).await;
    assert!(list.is_failure());
    assert!(list.text().contains("colour"), "{}", list.text());
}

#[tokio::test]
async fn an_unknown_tool_is_left_to_the_protocol() {
    let tools = tools(WRITER);
    assert!(tools.call("delete_site", Map::new()).await.is_none());
}
