//! The tools `khalid mcp` offers: the Content and Admin API tools from
//! `khalid-mcp`, and `lint_post`, which runs the lint publish gate on a
//! post without publishing it. The Admin API has no lint route yet, so
//! `lint_post` calls the function the publish route calls, and checks the
//! key's scope the way a route would.

use khalid_api::keys::{Principal, Scope};
use khalid_api::post_store::PostStore;
use khalid_mcp::{ApiTools, BoxFuture, Tool, ToolResult, Tools};
use serde_json::{Map, Value, json};

use crate::site_posts::SitePosts;

/// The API tools and `lint_post`, acting as one key.
#[derive(Debug)]
pub(crate) struct SiteTools {
    api: ApiTools,
    posts: SitePosts,
    principal: Principal,
    tools: Vec<Tool>,
}

impl SiteTools {
    /// `api`'s tools, then `lint_post` over `posts`, for the key that
    /// `principal` names.
    pub(crate) fn new(api: ApiTools, posts: SitePosts, principal: Principal) -> Self {
        let mut tools = api.list().to_vec();
        tools.push(Tool::new(
            "lint_post",
            "Runs the checks publish_post runs, without publishing. A draft is checked as it would go live today.",
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "The post's identity, such as en.choosing-anchor-text." }
                },
                "required": ["id"],
                "additionalProperties": false
            }),
        ));
        Self {
            api,
            posts,
            principal,
            tools,
        }
    }

    async fn lint(&self, arguments: &Map<String, Value>) -> ToolResult {
        if !self.principal.access().allows(Scope::PostsWrite) {
            return ToolResult::failed(
                "This key does not have the posts:write scope, which reading drafts needs. \
                 Use a key made with --scope posts:write.",
            );
        }
        let Some(id) = arguments.get("id").and_then(Value::as_str) else {
            return ToolResult::failed(
                "lint_post needs id, the post's identity, such as en.choosing-anchor-text.",
            );
        };
        if let Some(other) = arguments.keys().find(|name| *name != "id") {
            return ToolResult::failed(&format!("lint_post does not take {other}. It takes id."));
        }
        match self.posts.lint(id).await {
            Ok(Some(report)) => match serde_json::to_string(&report) {
                Ok(text) => ToolResult::done(&text),
                Err(_) => ToolResult::failed("The findings could not be written out."),
            },
            Ok(None) => ToolResult::failed(
                "No post has this identity. Check it, or find it with list_posts.",
            ),
            Err(error) => {
                tracing::error!(%error, "a post could not be linted");
                ToolResult::failed("The post could not be read. Try again in a minute.")
            }
        }
    }
}

impl Tools for SiteTools {
    fn list(&self) -> &[Tool] {
        &self.tools
    }

    fn call<'a>(
        &'a self,
        name: &'a str,
        arguments: Map<String, Value>,
    ) -> BoxFuture<'a, Option<ToolResult>> {
        if name == "lint_post" {
            return Box::pin(async move { Some(self.lint(&arguments).await) });
        }
        self.api.call(name, arguments)
    }
}
