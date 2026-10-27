//! The tools a server offers.

use std::fmt::Debug;

use serde::Serialize;
use serde_json::Value;

/// What the server can do, as `tools/list` describes it.
pub trait Tools: Debug + Send + Sync {
    /// Every tool, in the order an agent should read them.
    fn list(&self) -> &[Tool];
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
