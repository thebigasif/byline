//! [`SchemaGraph`]: every schema.org node of one page, as one JSON-LD graph.

use serde_json::{Map, Value, json};

use crate::SchemaNode;

/// Every schema.org node of one page, written as one `@graph` so nodes can
/// point to each other by `@id`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SchemaGraph {
    nodes: Vec<SchemaNode>,
}

impl SchemaGraph {
    /// A graph of `nodes`, in order.
    pub fn new(nodes: Vec<SchemaNode>) -> Self {
        Self { nodes }
    }

    /// The nodes, in order.
    pub fn nodes(&self) -> &[SchemaNode] {
        &self.nodes
    }

    /// Adds a node at the end.
    pub fn push(&mut self, node: SchemaNode) {
        self.nodes.push(node);
    }

    fn to_value(&self) -> Value {
        let mut object = Map::new();
        object.insert("@context".to_owned(), json!("https://schema.org"));
        object.insert(
            "@graph".to_owned(),
            Value::Array(self.nodes.iter().map(SchemaNode::to_json).collect()),
        );
        Value::Object(object)
    }

    /// The graph as pretty-printed JSON-LD.
    pub fn to_json(&self) -> String {
        format!("{:#}", self.to_value())
    }

    /// The graph in a `<script type="application/ld+json">` tag, compact,
    /// with every `<` written as `<` so no text can close the tag.
    pub fn to_script_tag(&self) -> String {
        let json = self.to_value().to_string().replace('<', "\\u003c");
        format!("<script type=\"application/ld+json\">{json}</script>\n")
    }
}
