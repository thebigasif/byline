//! The tools `khalid mcp` offers, each one request to the Content or
//! Admin API through the router `khalid serve` mounts (ADR "the MCP server
//! calls the API in process"). There is one tool for each operation in the
//! API's `OpenAPI` document, named by its `operationId`, and `publish_post`.

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use axum::http::{HeaderName, HeaderValue, Method, Request};
use khalid_core::Secret;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde_json::{Map, Value, json};
use tower::ServiceExt;

use crate::call::{Call, Key, Payload};
use crate::operations::{self, Operation};
use crate::tool::{BoxFuture, Tool, ToolResult, Tools};

/// The largest API answer passed on to an agent, in bytes.
const MAX_ANSWER: usize = 32 * 1024 * 1024;

/// The Content and Admin APIs as MCP tools, acting with one key.
#[derive(Debug)]
pub struct ApiTools {
    api: Router,
    key: Secret<String>,
    reader: Secret<String>,
    operations: Vec<Operation>,
    tools: Vec<Tool>,
}

impl ApiTools {
    /// The tools over `api`, the router from `khalid_api::api::router`,
    /// one for each operation `document`, the router's `OpenAPI` document
    /// as JSON, lets a key call. They act with `key` on the Admin API and
    /// read with `reader`, the content key from
    /// [`SessionKeys::new`](crate::SessionKeys::new).
    pub fn new(api: Router, document: &Value, key: Secret<String>, reader: Secret<String>) -> Self {
        let operations = operations::derive(document);
        let mut tools: Vec<Tool> = operations.iter().map(|o| o.tool().clone()).collect();
        tools.push(publish_post_tool());
        Self {
            api,
            key,
            reader,
            operations,
            tools,
        }
    }

    /// Sends `call` to the API and passes its answer on: the body, marked
    /// as a failure when the API answered with a problem.
    async fn send(&self, call: Call) -> ToolResult {
        let Some(request) = self.request(call) else {
            return ToolResult::failed("The request could not be made from these arguments.");
        };
        let Ok(response) = self.api.clone().oneshot(request).await;
        let succeeded = response.status().is_success();
        let Ok(body) = to_bytes(response.into_body(), MAX_ANSWER).await else {
            return ToolResult::failed("The answer was too long. Ask for fewer with limit.");
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
        let mut builder = Request::builder()
            .method(call.method)
            .uri(call.uri)
            .header(AUTHORIZATION, authorization);
        for (name, value) in call.headers {
            let name = HeaderName::try_from(name).ok()?;
            builder = builder.header(name, HeaderValue::try_from(value).ok()?);
        }
        let request = match call.body {
            Some(body) => builder
                .header(CONTENT_TYPE, body.media)
                .body(Body::from(body.bytes)),
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
            let call = if name == "publish_post" {
                publish_post(&arguments)
            } else {
                let operation = self.operations.iter().find(|o| o.tool().name() == name)?;
                operation.call(&arguments)
            };
            Some(match call {
                Ok(call) => self.send(call).await,
                Err(failed) => failed,
            })
        })
    }
}

/// Asks the Admin API to publish, which runs the lint publish gate. It is
/// `edit_admin_post` with only the status, named for what an agent wants.
fn publish_post(arguments: &Map<String, Value>) -> Result<Call, ToolResult> {
    if let Some(name) = arguments
        .keys()
        .find(|name| !["id", "updated_at"].contains(&name.as_str()))
    {
        return Err(ToolResult::failed(&format!(
            "publish_post does not take {name}. It takes id, updated_at."
        )));
    }
    let Some(id) = arguments.get("id").and_then(Value::as_str) else {
        return Err(ToolResult::failed(
            "publish_post needs id, the post's identity, such as en.choosing-anchor-text.",
        ));
    };
    let id = utf8_percent_encode(id, NON_ALPHANUMERIC);
    let edit = json!({ "status": "published", "updated_at": arguments.get("updated_at") });
    let body = json!({ "posts": [edit] }).to_string().into_bytes();
    Ok(Call {
        method: Method::PUT,
        uri: format!("/api/admin/v1/posts/{id}/"),
        key: Key::Writer,
        headers: Vec::new(),
        body: Some(Payload {
            media: "application/json".to_owned(),
            bytes: body,
        }),
    })
}

/// `publish_post`, the one tool that is not an operation of its own.
fn publish_post_tool() -> Tool {
    Tool::new(
        "publish_post",
        "Publishes a draft once the linter lets it through. A refusal lists every finding to fix.",
        json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "The post's identity, such as en.choosing-anchor-text." },
                "updated_at": { "type": "string", "description": "As read_admin_post gave it." }
            },
            "required": ["id", "updated_at"],
            "additionalProperties": false
        }),
    )
}
