//! The document's operations: what each takes and answers, as the
//! `Operations` interface, and where each is sent, as the route table.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use serde_json::{Map, Value};

use crate::SdkError;
use crate::types::{checked_name, comment, doc, key, type_of};

/// The HTTP methods an `OpenAPI` path item can hold, in the order read.
const METHODS: [&str; 7] = ["get", "put", "post", "delete", "patch", "head", "options"];

/// The parameter places the client fills, with the input member for each.
/// Cookies are left to the browser.
const PLACES: [(&str, &str); 3] = [("path", "path"), ("query", "query"), ("header", "headers")];

/// One operation of the document.
#[derive(Debug)]
pub(crate) struct Operation<'doc> {
    id: &'doc str,
    method: &'static str,
    path: &'doc str,
    parameters: Vec<&'doc Value>,
    spec: &'doc Map<String, Value>,
}

/// Every operation in the document, in the document's order.
pub(crate) fn read(document: &Value) -> Result<Vec<Operation<'_>>, SdkError> {
    let paths = document
        .get("paths")
        .and_then(Value::as_object)
        .ok_or(SdkError::Missing("paths"))?;
    let mut seen = BTreeSet::new();
    let mut found = Vec::new();
    for (path, item) in paths {
        let shared = item.get("parameters").and_then(Value::as_array);
        for method in METHODS {
            let Some(spec) = item.get(method).and_then(Value::as_object) else {
                continue;
            };
            let id = spec
                .get("operationId")
                .and_then(Value::as_str)
                .ok_or_else(|| SdkError::NoOperationId {
                    method: method.to_owned(),
                    path: path.clone(),
                })?;
            if !seen.insert(checked_name(id)?) {
                return Err(SdkError::DuplicateOperation(id.to_owned()));
            }
            let own = spec.get("parameters").and_then(Value::as_array);
            let parameters = shared.into_iter().chain(own).flatten().collect();
            found.push(Operation {
                id,
                method,
                path,
                parameters,
                spec,
            });
        }
    }
    Ok(found)
}

/// The `Operations` interface: for each operation, its `input` and the
/// `output` of its success answer.
pub(crate) fn interface(operations: &[Operation<'_>]) -> Result<String, SdkError> {
    let mut out =
        String::from("/** What each operation takes and what it answers on success. */\n");
    out.push_str("export interface Operations {\n");
    for operation in operations {
        if let Some(summary) = operation.spec.get("summary").and_then(Value::as_str) {
            out.push_str(&comment(summary, "  "));
        }
        let (input, output) = (input(operation)?, output(operation)?);
        let id = operation.id;
        let _ = write!(
            out,
            "  {id}: {{\n    input: {input};\n    output: {output};\n  }};\n"
        );
    }
    out.push_str("}\n");
    Ok(out)
}

/// The route table the client sends each operation by.
pub(crate) fn routes(operations: &[Operation<'_>]) -> String {
    let mut out =
        String::from("/** @type {import(\"./index.js\").Routes} */\nexport const operations = {\n");
    for operation in operations {
        let method = operation.method.to_ascii_uppercase();
        let path = Value::from(operation.path);
        let (body, key) = (body_kind(operation.spec), key_kind(operation.spec));
        let id = operation.id;
        let _ = writeln!(
            out,
            "  {id}: {{ method: \"{method}\", path: {path}, key: \"{key}\", body: {body} }},"
        );
    }
    out.push_str("};\n");
    out
}

/// The input type: path, query and header values in their own members, and
/// the body.
fn input(operation: &Operation<'_>) -> Result<String, SdkError> {
    let mut out = String::new();
    for (place, member) in PLACES {
        let mut fields = String::new();
        let mut any_required = false;
        for parameter in &operation.parameters {
            if let Some(at) = parameter.get("$ref") {
                let at = at.as_str().unwrap_or_default();
                return Err(SdkError::Reference(at.to_owned()));
            }
            if parameter.get("in").and_then(Value::as_str) != Some(place) {
                continue;
            }
            let name = parameter
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let required = parameter.get("required").and_then(Value::as_bool) == Some(true);
            any_required |= required;
            let schema = parameter.get("schema").unwrap_or(&Value::Null);
            fields.push_str(&doc(parameter, "        "));
            let optional = if required { "" } else { "?" };
            let (name, kind) = (key(name), type_of(schema, 4)?);
            let _ = writeln!(fields, "        {name}{optional}: {kind};");
        }
        if !fields.is_empty() {
            let optional = if any_required { "" } else { "?" };
            let _ = writeln!(out, "      {member}{optional}: {{\n{fields}      }};");
        }
    }
    if let Some(body) = operation.spec.get("requestBody") {
        let required = body.get("required").and_then(Value::as_bool) == Some(true);
        let optional = if required { "" } else { "?" };
        let kind = body_type(operation, body)?;
        let _ = writeln!(out, "      body{optional}: {kind};");
    }
    if out.is_empty() {
        return Ok("Record<string, never>".to_owned());
    }
    Ok(format!("{{\n{out}    }}"))
}

/// The type of a request body: JSON as its schema, text as a string, bytes
/// as a `Blob`.
fn body_type(operation: &Operation<'_>, body: &Value) -> Result<String, SdkError> {
    let content = body.get("content").and_then(Value::as_object);
    let Some((media, spec)) = content.and_then(|content| content.iter().next()) else {
        return Ok("never".to_owned());
    };
    match media.as_str() {
        "application/json" => type_of(spec.get("schema").unwrap_or(&Value::Null), 3),
        "application/octet-stream" => Ok("Blob".to_owned()),
        text if text.starts_with("text/") => Ok("string".to_owned()),
        _ => Err(SdkError::MediaType {
            operation: operation.id.to_owned(),
            media: media.clone(),
        }),
    }
}

/// The type of the first success answer: JSON as its schema, text as a
/// string, other bytes as a `Blob`, and `null` when it has no body.
fn output(operation: &Operation<'_>) -> Result<String, SdkError> {
    let responses = operation.spec.get("responses").and_then(Value::as_object);
    let success = responses.and_then(|all| all.iter().find(|(code, _)| code.starts_with('2')));
    let content = success
        .and_then(|(_, answer)| answer.get("content"))
        .and_then(Value::as_object);
    let Some((media, spec)) = content.and_then(|content| content.iter().next()) else {
        return Ok("null".to_owned());
    };
    if media.contains("json") {
        type_of(spec.get("schema").unwrap_or(&Value::Null), 2)
    } else if media.starts_with("text/") {
        Ok("string".to_owned())
    } else {
        Ok("Blob".to_owned())
    }
}

/// The body's media type as a JavaScript string, or `null` when the
/// operation takes no body. The client sends JSON encoded and anything else
/// as given, with this type.
fn body_kind(spec: &Map<String, Value>) -> String {
    let body = spec.get("requestBody");
    let content = body
        .and_then(|body| body.get("content"))
        .and_then(Value::as_object);
    content
        .and_then(|content| content.keys().next())
        .map_or_else(
            || "null".to_owned(),
            |media| Value::from(media.as_str()).to_string(),
        )
}

/// Which key the client signs the operation with: the Admin API key when
/// the operation takes one, else the content key when it takes one.
fn key_kind(spec: &Map<String, Value>) -> &'static str {
    let schemes: Vec<&str> = spec
        .get("security")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_object)
        .flat_map(Map::keys)
        .map(String::as_str)
        .collect();
    if schemes.contains(&"admin_key") {
        "admin"
    } else if schemes
        .iter()
        .any(|scheme| scheme.starts_with("content_key"))
    {
        "content"
    } else {
        "none"
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn document() -> Value {
        json!({
            "paths": {
                "/api/content/v1/posts/slug/{slug}/": {
                    "get": {
                        "operationId": "read_post_by_slug",
                        "summary": "Reads one post.",
                        "parameters": [
                            {"name": "slug", "in": "path", "required": true, "schema": {"type": "string"}},
                            {"name": "Accept-Version", "in": "header", "schema": {"type": "string"}},
                            {"name": "session", "in": "cookie", "schema": {"type": "string"}}
                        ],
                        "responses": {
                            "200": {"content": {"application/json": {"schema": {"$ref": "#/components/schemas/Posts"}}}},
                            "404": {"content": {"application/problem+json": {"schema": {"$ref": "#/components/schemas/Problem"}}}}
                        },
                        "security": [{"content_key": []}, {"content_key_query": []}]
                    }
                },
                "/api/admin/v1/media/": {
                    "post": {
                        "operationId": "upload_media",
                        "requestBody": {"required": true, "content": {"application/octet-stream": {}}},
                        "responses": {"204": {"description": "Saved"}},
                        "security": [{"admin_key": []}, {"staff_session": []}]
                    },
                    "get": {
                        "operationId": "export_media",
                        "responses": {"200": {"content": {"text/csv": {}}}}
                    }
                }
            }
        })
    }

    #[test]
    fn the_interface_names_each_input_and_success_output() {
        let document = document();
        let got = interface(&read(&document).unwrap()).unwrap();
        let want = "  /** Reads one post. */
  read_post_by_slug: {
    input: {
      path: {
        slug: string;
      };
      headers?: {
        \"Accept-Version\"?: string;
      };
    };
    output: Posts;
  };
";
        assert!(got.contains(want), "{got}");
        let upload =
            "  upload_media: {\n    input: {\n      body: Blob;\n    };\n    output: null;\n";
        assert!(got.contains(upload), "{got}");
        let export = "  export_media: {\n    input: Record<string, never>;\n    output: string;\n";
        assert!(got.contains(export), "{got}");
        assert!(
            !got.contains("session"),
            "cookies are left to the browser: {got}"
        );
    }

    #[test]
    fn the_route_table_holds_method_path_key_and_body() {
        let document = document();
        let got = routes(&read(&document).unwrap());
        let read_post = "  read_post_by_slug: { method: \"GET\", path: \"/api/content/v1/posts/slug/{slug}/\", key: \"content\", body: null },\n";
        assert!(got.contains(read_post), "{got}");
        let upload = "  upload_media: { method: \"POST\", path: \"/api/admin/v1/media/\", key: \"admin\", body: \"application/octet-stream\" },\n";
        assert!(got.contains(upload), "{got}");
        assert!(got.contains("key: \"none\", body: null },\n"), "{got}");
    }

    #[test]
    fn operations_need_one_name_each() {
        let unnamed = json!({"paths": {"/a": {"get": {}}}});
        assert!(matches!(
            read(&unnamed),
            Err(SdkError::NoOperationId { .. })
        ));
        let twice = json!({"paths": {
            "/a": {"get": {"operationId": "a"}},
            "/b": {"get": {"operationId": "a"}}
        }});
        assert!(matches!(read(&twice), Err(SdkError::DuplicateOperation(name)) if name == "a"));
    }

    #[test]
    fn a_form_body_is_refused() {
        let form = json!({"paths": {"/a": {"post": {
            "operationId": "a",
            "requestBody": {"content": {"application/x-www-form-urlencoded": {}}}
        }}}});
        let got = interface(&read(&form).unwrap());
        let media = "application/x-www-form-urlencoded";
        assert!(matches!(got, Err(SdkError::MediaType { media: m, .. }) if m == media));
    }
}
