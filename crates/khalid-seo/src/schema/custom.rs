//! [`CustomSchema`]: JSON-LD a person writes by hand, for a type the CMS
//! does not write itself.

use std::str::FromStr;

use khalid_core::AbsoluteUrl;
use serde_json::{Map, Value};

use crate::schema::template::is_word;
use crate::{SchemaNode, SchemaType, SchemaValue, SeoError};

/// The longest custom JSON-LD field taken, in bytes. A page's whole graph
/// rarely reaches 10 KB, so this leaves room without letting one field
/// swell every page it is on.
const MAX_BYTES: usize = 64 * 1024;

/// The `@context` values taken at the top of the field.
const CONTEXTS: [&str; 4] = [
    "https://schema.org",
    "https://schema.org/",
    "http://schema.org",
    "http://schema.org/",
];

/// JSON-LD written by hand, read into schema nodes.
///
/// It takes one node, or a `@graph` of nodes, with the schema.org context or
/// none. Each node needs an `@type`; an object with only an `@id` points to a
/// node elsewhere. Reading it into nodes, rather than passing the text on,
/// means it is escaped like every other node and checked like every other
/// node.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CustomSchema {
    nodes: Vec<SchemaNode>,
}

impl CustomSchema {
    /// Reads the text of a custom JSON-LD field. Blank text gives no nodes.
    ///
    /// # Errors
    ///
    /// [`SeoError::CustomSchemaSyntax`] when the text is not JSON, and
    /// [`SeoError::CustomSchema`] when it is JSON but not JSON-LD this crate
    /// can write: too long, a node without a type, another vocabulary, an
    /// `@id` that is not a full web address, or an empty value.
    pub fn parse(text: &str) -> Result<Self, SeoError> {
        if text.len() > MAX_BYTES {
            return Err(refused("", "is longer than 64 KB"));
        }
        if text.trim().is_empty() {
            return Ok(Self::default());
        }
        let value: Value =
            serde_json::from_str(text).map_err(|error| SeoError::CustomSchemaSyntax {
                line: error.line(),
                column: error.column(),
            })?;
        let Value::Object(mut root) = value else {
            return Err(refused(
                "",
                "must be one object, such as {\"@type\": \"Event\"}",
            ));
        };
        if let Some(context) = root.shift_remove("@context")
            && !context.as_str().is_some_and(|c| CONTEXTS.contains(&c))
        {
            return Err(refused("@context", "must be https://schema.org"));
        }
        let nodes = match root.shift_remove("@graph") {
            Some(Value::Array(items)) if root.is_empty() => items
                .iter()
                .enumerate()
                .map(|(index, item)| match item {
                    Value::Object(object) => node(object, &format!("@graph[{index}]")),
                    _ => Err(refused(&format!("@graph[{index}]"), "must be an object")),
                })
                .collect::<Result<_, _>>()?,
            Some(_) => return Err(refused("@graph", "must be a list and stand alone")),
            None => vec![node(&root, "")?],
        };
        Ok(Self { nodes })
    }

    /// The nodes, in the order written.
    pub fn nodes(&self) -> &[SchemaNode] {
        &self.nodes
    }
}

impl FromStr for CustomSchema {
    type Err = SeoError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::parse(text)
    }
}

fn refused(at: &str, problem: &'static str) -> SeoError {
    SeoError::CustomSchema {
        at: at.to_owned(),
        problem,
    }
}

/// `property` under `parent`, as `parent.property`, or `property` at the top.
fn place(parent: &str, property: &str) -> String {
    if parent.is_empty() {
        property.to_owned()
    } else {
        format!("{parent}.{property}")
    }
}

fn web_address(value: &Value, at: &str) -> Result<AbsoluteUrl, SeoError> {
    value
        .as_str()
        .and_then(|text| AbsoluteUrl::parse(text).ok())
        .ok_or_else(|| refused(at, "must be a full web address such as https://..."))
}

/// The node an object at `at` describes.
fn node(object: &Map<String, Value>, at: &str) -> Result<SchemaNode, SeoError> {
    let type_at = place(at, "@type");
    let kind = match object.get("@type") {
        None => return Err(refused(at, "needs an @type, such as Event")),
        Some(Value::String(name)) => {
            SchemaType::try_new(name).map_err(|_| refused(&type_at, "is not a type name"))?
        }
        Some(_) => return Err(refused(&type_at, "must be one type name")),
    };
    let mut node = SchemaNode::new(kind.as_str());
    for (name, value) in object {
        let here = place(at, name);
        match name.as_str() {
            "@type" => {}
            "@id" => node = node.with_id(web_address(value, &here)?),
            "@context" => return Err(refused(&here, "may appear only at the top")),
            _ if name.starts_with('@') => {
                return Err(refused(
                    &here,
                    "is a JSON-LD keyword this field does not take",
                ));
            }
            _ if !is_word(name, char::is_ascii_lowercase) => {
                return Err(refused(&here, "is not a property name such as startDate"));
            }
            _ => node.set(name, property(value, &here)?),
        }
    }
    Ok(node)
}

/// The value of the property at `at`.
fn property(value: &Value, at: &str) -> Result<SchemaValue, SeoError> {
    Ok(match value {
        Value::Null => return Err(refused(at, "is empty; leave the property out instead")),
        Value::Bool(flag) => SchemaValue::Boolean(*flag),
        Value::Number(number) => match number.as_i64() {
            Some(whole) => SchemaValue::Integer(whole),
            None => SchemaValue::Number(number.as_f64().unwrap_or_default()),
        },
        Value::String(text) => SchemaValue::text(text),
        Value::Array(items) => SchemaValue::List(
            items
                .iter()
                .enumerate()
                .map(|(index, item)| property(item, &format!("{at}[{index}]")))
                .collect::<Result<_, _>>()?,
        ),
        Value::Object(object) => match object.get("@id") {
            Some(id) if object.len() == 1 => {
                SchemaValue::Reference(web_address(id, &place(at, "@id"))?)
            }
            _ if object.contains_key("@type") => SchemaValue::Node(Box::new(node(object, at)?)),
            _ => return Err(refused(at, "needs an @type, or only an @id")),
        },
    })
}
