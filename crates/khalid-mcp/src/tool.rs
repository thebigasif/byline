//! The tools a server offers, and what a call answers.

use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;

use serde::Serialize;
use serde_json::{Map, Value, json};

/// A future a tool returns, boxed so a server can hold any `dyn Tools`.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// What the server can do, as `tools/list` describes it and `tools/call`
/// runs it.
pub trait Tools: Debug + Send + Sync {
    /// Every tool, in the order an agent should read them.
    fn list(&self) -> &[Tool];

    /// Runs the tool called `name` with `arguments`, or returns `None` when
    /// no tool has that name. Arguments the tool cannot use are a failed
    /// [`ToolResult`] that says why, so the agent can correct them.
    fn call<'a>(
        &'a self,
        name: &'a str,
        arguments: Map<String, Value>,
    ) -> BoxFuture<'a, Option<ToolResult>>;
}

/// One tool: its name, what it does, and the JSON Schema of its arguments.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    name: String,
    description: String,
    input_schema: Value,
}

impl Tool {
    /// A tool called `name`, described for an agent by `description`,
    /// taking arguments that match the JSON Schema `input_schema`.
    pub fn new(name: &str, description: &str, input_schema: Value) -> Self {
        Self {
            name: name.to_owned(),
            description: description.to_owned(),
            input_schema,
        }
    }

    /// The name an agent calls it by.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// What a tool call answers: text for the agent, and whether the call
/// failed in a way the agent can act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolResult {
    text: String,
    outcome: Outcome,
}

/// Whether a call did what it was asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Done,
    Failed,
}

impl ToolResult {
    /// The call did what it was asked, and `text` says what came of it.
    pub fn done(text: &str) -> Self {
        Self {
            text: text.to_owned(),
            outcome: Outcome::Done,
        }
    }

    /// The call did not do what it was asked, and `text` says why and what
    /// to change.
    pub fn failed(text: &str) -> Self {
        Self {
            text: text.to_owned(),
            outcome: Outcome::Failed,
        }
    }

    /// The text the agent reads.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Whether the call failed.
    pub fn is_failure(&self) -> bool {
        self.outcome == Outcome::Failed
    }

    /// The result as `tools/call` answers it.
    pub(crate) fn to_json(&self) -> Value {
        json!({
            "content": [{ "type": "text", "text": self.text }],
            "isError": self.is_failure(),
        })
    }
}
