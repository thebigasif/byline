//! Tools made from the API's `OpenAPI` document: one for each operation a
//! key may call, named by its `operationId`, so a new route is a new tool
//! and a changed route changes its tool (KH-API-5).

use std::collections::BTreeSet;

use axum::http::Method;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde_json::{Map, Value, json};

use crate::call::{Call, Key, Payload};
use crate::tool::{Tool, ToolResult};

/// Where the schemas an operation names live in the document.
const COMPONENTS: &str = "#/components/schemas/";

/// Where they live in a tool's input schema, which must stand alone.
const DEFS: &str = "#/$defs/";

/// The header the API reads the version from. The server sends none, so
/// the API answers with its current version.
const VERSION_HEADER: &str = "Accept-Version";

/// One operation, ready to turn arguments into a request.
#[derive(Debug, Clone)]
pub(crate) struct Operation {
    tool: Tool,
    method: Method,
    path: String,
    key: Key,
    parameters: Vec<Parameter>,
    body: Option<BodyShape>,
}

/// One argument the operation reads from the address or a header.
#[derive(Debug, Clone)]
struct Parameter {
    name: String,
    place: Place,
    required: bool,
}

/// Where a parameter goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Place {
    Path,
    Query,
    Header,
}

/// The body an operation takes, as its `body` argument.
#[derive(Debug, Clone)]
struct BodyShape {
    media: String,
    kind: BodyKind,
    required: bool,
}

/// How the `body` argument becomes bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BodyKind {
    /// Any JSON value, sent as written.
    Json,
    /// A string, sent as its text.
    Text,
    /// An array of numbers from 0 to 255, sent as those bytes.
    Bytes,
}

/// Every operation in `document` that a key may call, in the document's
/// order. An operation open to anyone, or only to a signed-in person,
/// such as signing in, is left out: an agent acts with a key.
pub(crate) fn derive(document: &Value) -> Vec<Operation> {
    let empty = Map::new();
    let schemas = document
        .pointer("/components/schemas")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    let Some(paths) = document.get("paths").and_then(Value::as_object) else {
        return Vec::new();
    };
    let mut operations = Vec::new();
    for (path, item) in paths {
        let Some(item) = item.as_object() else {
            continue;
        };
        for (method, operation) in item {
            if let Some(operation) = Operation::read(path, method, operation, schemas) {
                operations.push(operation);
            }
        }
    }
    operations
}

impl Operation {
    /// The operation `method` on `path`, or `None` when no key may call it
    /// or it has no `operationId`.
    fn read(
        path: &str,
        method: &str,
        operation: &Value,
        schemas: &Map<String, Value>,
    ) -> Option<Self> {
        let method = match method {
            "get" => Method::GET,
            "post" => Method::POST,
            "put" => Method::PUT,
            "patch" => Method::PATCH,
            "delete" => Method::DELETE,
            _ => return None,
        };
        let name = operation.get("operationId")?.as_str()?;
        let key = key(operation)?;
        let mut properties = Map::new();
        let mut required = Vec::new();
        let mut parameters = Vec::new();
        for parameter in operation
            .get("parameters")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let Some((parameter, schema)) = Parameter::read(path, parameter) else {
                continue;
            };
            if parameter.required {
                required.push(parameter.name.clone());
            }
            properties.insert(parameter.name.clone(), schema);
            parameters.push(parameter);
        }
        let body = operation.get("requestBody").and_then(|body| {
            let (shape, schema) = BodyShape::read(body)?;
            if shape.required {
                required.push("body".to_owned());
            }
            properties.insert("body".to_owned(), schema);
            Some(shape)
        });
        let mut properties = Value::Object(properties);
        let defs = localise(&mut properties, schemas);
        let mut input = Map::new();
        input.insert("type".to_owned(), json!("object"));
        input.insert("properties".to_owned(), properties);
        input.insert("required".to_owned(), json!(required));
        input.insert("additionalProperties".to_owned(), json!(false));
        if !defs.is_empty() {
            input.insert("$defs".to_owned(), Value::Object(defs));
        }
        let input = Value::Object(input);
        let tool = Tool::new(name, &describe(operation, &method, path), input);
        Some(Self {
            tool,
            method,
            path: path.to_owned(),
            key,
            parameters,
            body,
        })
    }

    /// The tool an agent sees.
    pub(crate) fn tool(&self) -> &Tool {
        &self.tool
    }

    /// The request `arguments` ask for, or a failure that names the
    /// argument to fix.
    pub(crate) fn call(&self, arguments: &Map<String, Value>) -> Result<Call, ToolResult> {
        self.check(arguments)?;
        let mut uri = self.path.clone();
        let mut query = form_urlencoded::Serializer::new(String::new());
        let mut headers = Vec::new();
        for parameter in &self.parameters {
            let Some(value) = arguments.get(&parameter.name).and_then(text) else {
                continue;
            };
            match parameter.place {
                Place::Path => {
                    let segment = utf8_percent_encode(&value, NON_ALPHANUMERIC).to_string();
                    uri = uri.replace(&format!("{{{}}}", parameter.name), &segment);
                }
                Place::Query => {
                    query.append_pair(&parameter.name, &value);
                }
                Place::Header => headers.push((parameter.name.clone(), value)),
            }
        }
        let query = query.finish();
        if !query.is_empty() {
            uri = format!("{uri}?{query}");
        }
        let body = match (&self.body, arguments.get("body")) {
            (Some(shape), Some(value)) => Some(self.payload(shape, value)?),
            _ => None,
        };
        Ok(Call {
            method: self.method.clone(),
            uri,
            key: self.key,
            headers,
            body,
        })
    }

    /// Fails on the first argument the tool does not take, then on the
    /// first one it needs and did not get.
    fn check(&self, arguments: &Map<String, Value>) -> Result<(), ToolResult> {
        let name = self.tool.name();
        let mut takes: Vec<&str> = self.parameters.iter().map(|p| p.name.as_str()).collect();
        if self.body.is_some() {
            takes.push("body");
        }
        if let Some(other) = arguments
            .keys()
            .find(|given| !takes.contains(&given.as_str()))
        {
            let takes = if takes.is_empty() {
                "nothing".to_owned()
            } else {
                takes.join(", ")
            };
            return Err(ToolResult::failed(&format!(
                "{name} does not take {other}. It takes {takes}."
            )));
        }
        let needed = self
            .parameters
            .iter()
            .filter(|p| p.required)
            .map(|p| p.name.as_str())
            .chain(self.body.iter().filter(|b| b.required).map(|_| "body"));
        for needed in needed {
            if arguments.get(needed).is_none_or(Value::is_null) {
                return Err(ToolResult::failed(&format!(
                    "{name} needs {needed}. Its input schema says what it holds."
                )));
            }
        }
        Ok(())
    }

    /// `value` as the bytes of the body `shape` describes.
    fn payload(&self, shape: &BodyShape, value: &Value) -> Result<Payload, ToolResult> {
        let name = self.tool.name();
        let bytes = match shape.kind {
            BodyKind::Json => value.to_string().into_bytes(),
            BodyKind::Text => match value.as_str() {
                Some(text) => text.as_bytes().to_vec(),
                None => {
                    return Err(ToolResult::failed(&format!(
                        "{name} takes body as text, {}. Send it as one string.",
                        shape.media
                    )));
                }
            },
            BodyKind::Bytes => serde_json::from_value::<Vec<u8>>(value.clone()).map_err(|_| {
                ToolResult::failed(&format!(
                    "{name} takes body as the file's bytes. Send a list of numbers from 0 to 255."
                ))
            })?,
        };
        Ok(Payload {
            media: shape.media.clone(),
            bytes,
        })
    }
}

impl Parameter {
    /// The parameter and its schema, or `None` for the version header and
    /// cookies, which the server never sends. A parameter the document
    /// places in the path that the address has no slot for goes in the
    /// query, where the API reads it.
    fn read(path: &str, parameter: &Value) -> Option<(Self, Value)> {
        let name = parameter.get("name")?.as_str()?;
        let slot = path.contains(&format!("{{{name}}}"));
        let documented = parameter.get("required").and_then(Value::as_bool) == Some(true);
        let (place, required) = match parameter.get("in")?.as_str()? {
            "path" if slot => (Place::Path, true),
            "path" => (Place::Query, false),
            "query" => (Place::Query, documented),
            "header" if name != VERSION_HEADER => (Place::Header, documented),
            _ => return None,
        };
        let mut schema = parameter
            .get("schema")
            .cloned()
            .unwrap_or_else(|| json!({}));
        if let (Some(described), Some(schema)) =
            (parameter.get("description"), schema.as_object_mut())
        {
            schema.insert("description".to_owned(), described.clone());
        }
        Some((
            Self {
                name: name.to_owned(),
                place,
                required,
            },
            schema,
        ))
    }
}

impl BodyShape {
    /// The shape of `body`, an operation's `requestBody`, from its first
    /// media type, and the schema of the `body` argument.
    fn read(body: &Value) -> Option<(Self, Value)> {
        let (media, content) = body.get("content")?.as_object()?.iter().next()?;
        let kind = if media == "application/json" || media.ends_with("+json") {
            BodyKind::Json
        } else if media.starts_with("text/") {
            BodyKind::Text
        } else {
            BodyKind::Bytes
        };
        let mut schema = content.get("schema").cloned().unwrap_or_else(|| json!({}));
        if let (Some(described), Some(schema)) = (body.get("description"), schema.as_object_mut()) {
            schema.insert("description".to_owned(), described.clone());
        }
        let required = body.get("required").and_then(Value::as_bool) == Some(true);
        Some((
            Self {
                media: media.clone(),
                kind,
                required,
            },
            schema,
        ))
    }
}

/// Which key an operation takes: the integration key when the Admin API's
/// key is accepted, the content key when only that is.
fn key(operation: &Value) -> Option<Key> {
    let schemes: BTreeSet<&str> = operation
        .get("security")?
        .as_array()?
        .iter()
        .filter_map(Value::as_object)
        .flat_map(Map::keys)
        .map(String::as_str)
        .collect();
    if schemes.contains("admin_key") {
        Some(Key::Writer)
    } else if schemes.contains("content_key") {
        Some(Key::Reader)
    } else {
        None
    }
}

/// What the operation does, from its summary, on one line.
fn describe(operation: &Value, method: &Method, path: &str) -> String {
    let said = ["summary", "description"]
        .iter()
        .find_map(|field| operation.get(*field).and_then(Value::as_str))
        .map(|text| text.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|text| !text.is_empty());
    said.unwrap_or_else(|| format!("Sends {method} {path} to the API."))
}

/// An argument as the text an address or header carries, or `None` for
/// `null`, which leaves the argument out.
fn text(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(text) => Some(text.clone()),
        other => Some(other.to_string()),
    }
}

/// Points every reference in `schema` at `$defs` and returns the schemas
/// it needs from `schemas`, each pointing at `$defs` too.
fn localise(schema: &mut Value, schemas: &Map<String, Value>) -> Map<String, Value> {
    let mut wanted = BTreeSet::new();
    point_home(schema, &mut wanted);
    let mut defs = Map::new();
    while let Some(name) = wanted
        .iter()
        .find(|name| !defs.contains_key(*name))
        .cloned()
    {
        let mut def = schemas.get(&name).cloned().unwrap_or_else(|| json!({}));
        point_home(&mut def, &mut wanted);
        defs.insert(name, def);
    }
    defs
}

/// Rewrites each `$ref` under `value` from the document's components to
/// `$defs`, noting the name in `wanted`.
fn point_home(value: &mut Value, wanted: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            for (field, inner) in map.iter_mut() {
                let named = inner.as_str().and_then(|r| r.strip_prefix(COMPONENTS));
                match (field.as_str(), named) {
                    ("$ref", Some(name)) => {
                        wanted.insert(name.to_owned());
                        *inner = Value::String(format!("{DEFS}{name}"));
                    }
                    _ => point_home(inner, wanted),
                }
            }
        }
        Value::Array(items) => items.iter_mut().for_each(|item| point_home(item, wanted)),
        _ => {}
    }
}
