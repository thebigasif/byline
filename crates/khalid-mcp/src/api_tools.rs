//! The tools `khalid mcp` offers, each one request to the Content or
//! Admin API through the router `khalid serve` mounts (ADR "the MCP server
//! calls the API in process").

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::header::AUTHORIZATION;
use axum::http::{HeaderValue, Request};
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

/// One request, with the key it carries.
struct Call {
    uri: String,
    key: Key,
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
        Request::get(call.uri)
            .header(AUTHORIZATION, authorization)
            .body(Body::empty())
            .ok()
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
    let mut query = form_urlencoded::Serializer::new(String::new());
    for (name, value) in arguments {
        if !LIST_ARGUMENTS.contains(&name.as_str()) {
            return Err(ToolResult::failed(&format!(
                "list_posts does not take {name}. It takes {}.",
                LIST_ARGUMENTS.join(", ")
            )));
        }
        match value {
            Value::String(text) => query.append_pair(name, text),
            other => query.append_pair(name, &other.to_string()),
        };
    }
    Ok(Call {
        uri: format!("/api/content/v1/posts/?{}", query.finish()),
        key: Key::Reader,
    })
}

fn read_post(arguments: &Map<String, Value>) -> Result<Call, ToolResult> {
    Ok(Call {
        uri: post_address(arguments, "read_post")?,
        key: Key::Writer,
    })
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
    ]
}
