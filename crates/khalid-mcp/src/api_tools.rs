//! The tools `khalid mcp` offers, each one request to the Content or
//! Admin API through the router `khalid serve` mounts (ADR "the MCP server
//! calls the API in process").

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use axum::http::{HeaderValue, Method, Request};
use khalid_core::Secret;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde_json::{Map, Value, json};
use tower::ServiceExt;

use crate::tool::{BoxFuture, Tool, ToolResult, Tools};

/// The largest API answer passed on to an agent, in bytes.
const MAX_ANSWER: usize = 32 * 1024 * 1024;

/// The arguments `list_posts` passes to the Content API's list grammar.
const LIST_ARGUMENTS: [&str; 4] = ["filter", "order", "limit", "page"];

/// The Content and Admin APIs as MCP tools, acting with one key.
#[derive(Debug)]
pub struct ApiTools {
    api: Router,
    key: Secret<String>,
    reader: Secret<String>,
    tools: Vec<Tool>,
}

/// One request, with the key it carries and its JSON body, if any.
struct Call {
    method: Method,
    uri: String,
    key: Key,
    body: Option<Value>,
}

impl Call {
    fn get(uri: String, key: Key) -> Self {
        let method = Method::GET;
        Self {
            method,
            uri,
            key,
            body: None,
        }
    }

    /// A write with the server's key and a JSON `body`.
    fn write(method: Method, uri: String, body: Value) -> Self {
        let key = Key::Writer;
        Self {
            method,
            uri,
            key,
            body: Some(body),
        }
    }
}

/// Which key a request carries.
#[derive(Clone, Copy)]
enum Key {
    /// The server's own content key, for the Content API.
    Reader,
    /// The integration key or staff token the server acts with.
    Writer,
}

impl ApiTools {
    /// The tools over `api`, the router from `khalid_api::api::router`,
    /// acting with `key` on the Admin API and reading with `reader`, the
    /// content key from [`SessionKeys::new`](crate::SessionKeys::new).
    pub fn new(api: Router, key: Secret<String>, reader: Secret<String>) -> Self {
        Self {
            api,
            key,
            reader,
            tools: definitions(),
        }
    }

    /// Sends `call` to the API and passes its answer on: the JSON body,
    /// marked as a failure when the API answered with a problem.
    async fn send(&self, call: Call) -> ToolResult {
        let Some(request) = self.request(call) else {
            return ToolResult::failed("The request could not be made from these arguments.");
        };
        let Ok(response) = self.api.clone().oneshot(request).await;
        let succeeded = response.status().is_success();
        let Ok(body) = to_bytes(response.into_body(), MAX_ANSWER).await else {
            return ToolResult::failed("The answer was too long. Ask for fewer posts with limit.");
        };
        let text = String::from_utf8_lossy(&body);
        if succeeded {
            ToolResult::done(&text)
        } else {
            ToolResult::failed(&text)
        }
    }

    fn request(&self, call: Call) -> Option<Request<Body>> {
        let (scheme, key) = match call.key {
            Key::Reader => ("Khalid-Content", &self.reader),
            Key::Writer => ("Khalid", &self.key),
        };
        let mut authorization = HeaderValue::try_from(format!("{scheme} {}", key.expose())).ok()?;
        authorization.set_sensitive(true);
        let builder = Request::builder()
            .method(call.method)
            .uri(call.uri)
            .header(AUTHORIZATION, authorization);
        let request = match call.body {
            Some(body) => builder
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string())),
            None => builder.body(Body::empty()),
        };
        request.ok()
    }
}

impl Tools for ApiTools {
    fn list(&self) -> &[Tool] {
        &self.tools
    }

    fn call<'a>(
        &'a self,
        name: &'a str,
        arguments: Map<String, Value>,
    ) -> BoxFuture<'a, Option<ToolResult>> {
        Box::pin(async move {
            let call = match name {
                "list_posts" => list_posts(&arguments),
                "read_post" => read_post(&arguments),
                "create_draft" => Ok(create_draft(&arguments)),
                "edit_post" => edit_post(&arguments),
                "publish_post" => publish_post(&arguments),
                _ => return None,
            };
            Some(match call {
                Ok(call) => self.send(call).await,
                Err(failed) => failed,
            })
        })
    }
}

fn list_posts(arguments: &Map<String, Value>) -> Result<Call, ToolResult> {
    only(arguments, "list_posts", &LIST_ARGUMENTS)?;
    let mut query = form_urlencoded::Serializer::new(String::new());
    for (name, value) in arguments {
        match value {
            Value::String(text) => query.append_pair(name, text),
            other => query.append_pair(name, &other.to_string()),
        };
    }
    let uri = format!("/api/content/v1/posts/?{}", query.finish());
    Ok(Call::get(uri, Key::Reader))
}

fn read_post(arguments: &Map<String, Value>) -> Result<Call, ToolResult> {
    let uri = post_address(arguments, "read_post")?;
    Ok(Call::get(uri, Key::Writer))
}

/// Sends the arguments as the new post, so the Admin API checks every
/// field, and turns HTML into markdown when `html` is given.
fn create_draft(arguments: &Map<String, Value>) -> Call {
    let query = if arguments.contains_key("html") {
        "?source=html"
    } else {
        ""
    };
    let uri = format!("/api/admin/v1/posts/{query}");
    Call::write(Method::POST, uri, json!({ "posts": [arguments] }))
}

/// Sends the fields to change. `status` is not one of them, so an edit
/// never publishes; `publish_post` does.
fn edit_post(arguments: &Map<String, Value>) -> Result<Call, ToolResult> {
    only(
        arguments,
        "edit_post",
        &["id", "title", "markdown", "real_update", "updated_at"],
    )?;
    let uri = post_address(arguments, "edit_post")?;
    let mut edit = arguments.clone();
    edit.remove("id");
    Ok(Call::write(Method::PUT, uri, json!({ "posts": [edit] })))
}

/// Asks the Admin API to publish, which runs the lint publish gate.
fn publish_post(arguments: &Map<String, Value>) -> Result<Call, ToolResult> {
    only(arguments, "publish_post", &["id", "updated_at"])?;
    let uri = post_address(arguments, "publish_post")?;
    let updated_at = arguments.get("updated_at");
    let edit = json!({ "status": "published", "updated_at": updated_at });
    Ok(Call::write(Method::PUT, uri, json!({ "posts": [edit] })))
}

/// Fails, naming the first argument `tool` does not take.
fn only(arguments: &Map<String, Value>, tool: &str, takes: &[&str]) -> Result<(), ToolResult> {
    match arguments
        .keys()
        .find(|name| !takes.contains(&name.as_str()))
    {
        Some(name) => Err(ToolResult::failed(&format!(
            "{tool} does not take {name}. It takes {}.",
            takes.join(", ")
        ))),
        None => Ok(()),
    }
}

/// The Admin API address of the post the `id` argument names, with the
/// identity encoded so it stays one path segment.
fn post_address(arguments: &Map<String, Value>, tool: &str) -> Result<String, ToolResult> {
    let Some(id) = arguments.get("id").and_then(Value::as_str) else {
        return Err(ToolResult::failed(&format!(
            "{tool} needs id, the post's identity, such as en.choosing-anchor-text."
        )));
    };
    let id = utf8_percent_encode(id, NON_ALPHANUMERIC);
    Ok(format!("/api/admin/v1/posts/{id}/"))
}

/// Every tool, with what it does and the arguments it takes.
fn definitions() -> Vec<Tool> {
    vec![
        Tool::new(
            "list_posts",
            "Lists published posts, newest first, through the Content API. Drafts are not listed.",
            json!({
                "type": "object",
                "properties": {
                    "filter": { "type": "string", "description": "Such as tag:news+featured:true." },
                    "order": { "type": "string", "description": "Such as published_at desc." },
                    "limit": { "type": ["integer", "string"], "description": "Posts per page, or all." },
                    "page": { "type": "integer", "description": "Which page, from 1." }
                },
                "additionalProperties": false
            }),
        ),
        Tool::new(
            "read_post",
            "Reads one post in any status, with its markdown and the updated_at an edit sends back.",
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "The post's identity, such as en.choosing-anchor-text." }
                },
                "required": ["id"],
                "additionalProperties": false
            }),
        ),
        Tool::new(
            "create_draft",
            "Saves a new draft from markdown, or from HTML that is cleaned and turned into markdown.",
            json!({
                "type": "object",
                "properties": {
                    "title": { "type": "string" },
                    "markdown": { "type": "string", "description": "The body. Send this or html." },
                    "html": { "type": "string", "description": "The body as HTML. Send this or markdown." },
                    "slug": { "type": "string", "description": "Made from the title when left out." },
                    "author": { "type": "string", "description": "An author's slug. Publishing needs one." }
                },
                "required": ["title"],
                "additionalProperties": false
            }),
        ),
        Tool::new(
            "edit_post",
            "Changes a post's title or markdown. Fields left out stay. Refused if the post changed after updated_at. Send real_update only when the new markdown really changes what a live post says: its updated date becomes today.",
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string" },
                    "title": { "type": "string" },
                    "markdown": { "type": "string", "description": "The whole new body." },
                    "real_update": {
                        "type": "object",
                        "description": "Marks a real update to a live post's text.",
                        "properties": {
                            "note": { "type": "string", "description": "What changed, in one short sentence." }
                        },
                        "required": ["note"],
                        "additionalProperties": false
                    },
                    "updated_at": { "type": "string", "description": "As read_post gave it." }
                },
                "required": ["id", "updated_at"],
                "additionalProperties": false
            }),
        ),
        Tool::new(
            "publish_post",
            "Publishes a draft once the linter lets it through. A refusal lists every finding to fix.",
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string" },
                    "updated_at": { "type": "string", "description": "As read_post gave it." }
                },
                "required": ["id", "updated_at"],
                "additionalProperties": false
            }),
        ),
    ]
}
