//! [`SchemaNode`]: one schema.org thing, such as an Article or a Person.

use khalid_core::AbsoluteUrl;
use serde_json::{Map, Value, json};

use crate::SchemaValue;

/// One schema.org thing: its type, an optional `@id` other nodes can point
/// to, and its properties in the order they were set.
#[derive(Debug, Clone, PartialEq)]
pub struct SchemaNode {
    kind: String,
    id: Option<AbsoluteUrl>,
    properties: Vec<(String, SchemaValue)>,
}

impl SchemaNode {
    /// A node of the schema.org type `kind`, such as `BlogPosting`.
    pub fn new(kind: &str) -> Self {
        Self {
            kind: kind.to_owned(),
            id: None,
            properties: Vec::new(),
        }
    }

    /// The node with an `@id`, such as `https://www.example.com/blog/#organization`.
    #[must_use]
    pub fn with_id(mut self, id: AbsoluteUrl) -> Self {
        self.id = Some(id);
        self
    }

    /// The node with `name` set to `value`.
    #[must_use]
    pub fn with(mut self, name: &str, value: SchemaValue) -> Self {
        self.set(name, value);
        self
    }

    /// Sets `name` to `value`. A property set before keeps its place and
    /// takes the new value.
    pub fn set(&mut self, name: &str, value: SchemaValue) {
        match self
            .properties
            .iter_mut()
            .find(|(existing, _)| existing == name)
        {
            Some((_, slot)) => *slot = value,
            None => self.properties.push((name.to_owned(), value)),
        }
    }

    /// The schema.org type.
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// The `@id`, if the node has one.
    pub fn id(&self) -> Option<&AbsoluteUrl> {
        self.id.as_ref()
    }

    /// The value of `name`, if set.
    pub fn get(&self, name: &str) -> Option<&SchemaValue> {
        self.properties
            .iter()
            .find(|(existing, _)| existing == name)
            .map(|(_, value)| value)
    }

    /// Every property, in the order set.
    pub fn properties(&self) -> &[(String, SchemaValue)] {
        &self.properties
    }

    /// The node as a JSON object: `@type`, then `@id`, then the properties.
    pub(crate) fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.insert("@type".to_owned(), json!(self.kind));
        if let Some(id) = &self.id {
            object.insert("@id".to_owned(), json!(id.as_str()));
        }
        for (name, value) in &self.properties {
            object.insert(name.clone(), value.to_json());
        }
        Value::Object(object)
    }
}
