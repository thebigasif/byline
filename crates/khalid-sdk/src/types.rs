//! JSON Schema, as the API's `OpenAPI` document writes it, as TypeScript
//! types.

use std::fmt::Write as _;

use serde_json::{Map, Value};

use crate::SdkError;

/// Where every `$ref` the SDK follows points.
const SCHEMAS: &str = "#/components/schemas/";

/// One `export type` per schema, each with its description as a comment.
pub(crate) fn declarations(schemas: &Map<String, Value>) -> Result<String, SdkError> {
    let mut out = String::new();
    for (name, schema) in schemas {
        out.push_str(&doc(schema, ""));
        let name = checked_name(name)?;
        let kind = type_of(schema, 0)?;
        let _ = write!(out, "export type {name} = {kind};\n\n");
    }
    Ok(out)
}

/// The TypeScript type for `schema`. Lines of a nested object are indented
/// one level deeper than `depth`.
pub(crate) fn type_of(schema: &Value, depth: usize) -> Result<String, SdkError> {
    let Some(object) = schema.as_object() else {
        let any = schema.as_bool().unwrap_or(true);
        return Ok(if any { "unknown" } else { "never" }.to_owned());
    };
    if let Some(reference) = object.get("$ref") {
        return reference_name(reference).map(str::to_owned);
    }
    if let Some(Value::Array(choices)) = object.get("oneOf").or_else(|| object.get("anyOf")) {
        return joined(choices, " | ", depth);
    }
    if let Some(Value::Array(parts)) = object.get("allOf") {
        return joined(parts, " & ", depth);
    }
    if let Some(Value::Array(values)) = object.get("enum") {
        let literals: Vec<String> = values.iter().map(Value::to_string).collect();
        return Ok(literals.join(" | "));
    }
    if let Some(value) = object.get("const") {
        return Ok(value.to_string());
    }
    match object.get("type") {
        Some(Value::String(kind)) => named(kind, object, depth),
        Some(Value::Array(kinds)) => {
            let parts = kinds
                .iter()
                .map(|kind| {
                    kind.as_str()
                        .map_or(Ok("unknown".to_owned()), |kind| named(kind, object, depth))
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(parts.join(" | "))
        }
        _ => Ok("unknown".to_owned()),
    }
}

/// A `/** ... */` line with the schema's description, or nothing.
pub(crate) fn doc(schema: &Value, pad: &str) -> String {
    schema
        .get("description")
        .and_then(Value::as_str)
        .map_or_else(String::new, |text| comment(text, pad))
}

/// `text` as a one-line `/** ... */` comment, or nothing when it is blank.
pub(crate) fn comment(text: &str, pad: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return String::new();
    }
    format!("{pad}/** {} */\n", words.join(" ").replace("*/", "*\\/"))
}

/// The key for a property or parameter `name`, quoted when it is not a
/// plain TypeScript name.
pub(crate) fn key(name: &str) -> String {
    if is_name(name) {
        name.to_owned()
    } else {
        Value::from(name).to_string()
    }
}

/// `name` when it can name a TypeScript type or member.
pub(crate) fn checked_name(name: &str) -> Result<&str, SdkError> {
    if is_name(name) {
        Ok(name)
    } else {
        Err(SdkError::Name(name.to_owned()))
    }
}

/// Whether `name` is a plain TypeScript name.
fn is_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_' || first == '$')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

/// The schema name a `$ref` points at.
fn reference_name(reference: &Value) -> Result<&str, SdkError> {
    let text = reference.as_str().unwrap_or_default();
    let name = text
        .strip_prefix(SCHEMAS)
        .ok_or_else(|| SdkError::Reference(text.to_owned()))?;
    checked_name(name)
}

/// The types of `schemas` joined by `glue`, each in brackets when it has
/// operators of its own.
fn joined(schemas: &[Value], glue: &str, depth: usize) -> Result<String, SdkError> {
    let parts = schemas
        .iter()
        .map(|schema| {
            let part = type_of(schema, depth)?;
            Ok(if part.contains(['|', '&']) {
                format!("({part})")
            } else {
                part
            })
        })
        .collect::<Result<Vec<_>, SdkError>>()?;
    Ok(parts.join(glue))
}

/// The type for one JSON Schema `type` name.
fn named(kind: &str, object: &Map<String, Value>, depth: usize) -> Result<String, SdkError> {
    Ok(match kind {
        "string" => "string".to_owned(),
        "integer" | "number" => "number".to_owned(),
        "boolean" => "boolean".to_owned(),
        "null" => "null".to_owned(),
        "array" => {
            let items = object
                .get("items")
                .map_or(Ok("unknown".to_owned()), |items| type_of(items, depth))?;
            if items.contains(['|', '&', ' ']) && !items.starts_with('{') {
                format!("({items})[]")
            } else {
                format!("{items}[]")
            }
        }
        "object" => object_type(object, depth)?,
        _ => "unknown".to_owned(),
    })
}

/// An object literal type with one member per property.
fn object_type(object: &Map<String, Value>, depth: usize) -> Result<String, SdkError> {
    let properties = object.get("properties").and_then(Value::as_object);
    let extra = object.get("additionalProperties");
    let Some(properties) = properties.filter(|properties| !properties.is_empty()) else {
        return Ok(match extra {
            Some(Value::Bool(false)) => "Record<string, never>".to_owned(),
            Some(schema @ Value::Object(_)) => {
                format!("Record<string, {}>", type_of(schema, depth)?)
            }
            _ => "Record<string, unknown>".to_owned(),
        });
    };
    let required: Vec<&str> = object
        .get("required")
        .and_then(Value::as_array)
        .map(|names| names.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let pad = "  ".repeat(depth + 1);
    let mut out = String::from("{\n");
    for (name, schema) in properties {
        let optional = if required.contains(&name.as_str()) {
            ""
        } else {
            "?"
        };
        out.push_str(&doc(schema, &pad));
        let (name, kind) = (key(name), type_of(schema, depth + 1)?);
        let _ = writeln!(out, "{pad}{name}{optional}: {kind};");
    }
    out.push_str(&"  ".repeat(depth));
    out.push('}');
    Ok(out)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn ts(schema: &Value) -> String {
        type_of(schema, 0).unwrap()
    }

    #[test]
    fn plain_types_become_their_typescript_names() {
        assert_eq!(ts(&json!({"type": "integer", "format": "int32"})), "number");
        assert_eq!(ts(&json!({"type": ["string", "null"]})), "string | null");
        assert_eq!(
            ts(&json!({"type": "array", "items": {"type": "string"}})),
            "string[]"
        );
        assert_eq!(
            ts(&json!({"type": "array", "items": {"type": ["string", "null"]}})),
            "(string | null)[]"
        );
        assert_eq!(
            ts(&json!({"enum": ["draft", "published"]})),
            r#""draft" | "published""#
        );
        assert_eq!(ts(&json!({})), "unknown");
    }

    #[test]
    fn references_and_choices_name_the_schemas() {
        let meta = json!({"oneOf": [{"$ref": "#/components/schemas/Meta"}, {"type": "null"}]});
        assert_eq!(ts(&meta), "Meta | null");
        let post = json!({"allOf": [{"$ref": "#/components/schemas/Bodies"}, {"oneOf": [{"type": "string"}, {"type": "null"}]}]});
        assert_eq!(ts(&post), "Bodies & (string | null)");
    }

    #[test]
    fn a_reference_outside_the_schemas_is_refused() {
        let got = type_of(&json!({"$ref": "#/components/parameters/Slug"}), 0);
        assert!(
            matches!(got, Err(SdkError::Reference(at)) if at == "#/components/parameters/Slug")
        );
    }

    #[test]
    fn objects_list_their_properties_with_docs_and_optional_marks() {
        let schema = json!({
            "type": "object",
            "required": ["slug"],
            "properties": {
                "slug": {"type": "string", "description": "The address.\nShort."},
                "x-count": {"type": "object", "properties": {"n": {"type": "integer"}}}
            }
        });
        let want = "{\n  /** The address. Short. */\n  slug: string;\n  \"x-count\"?: {\n    n?: number;\n  };\n}";
        assert_eq!(ts(&schema), want);
    }

    #[test]
    fn maps_and_closed_empty_objects_become_records() {
        let map = json!({"type": "object", "additionalProperties": {"type": "integer"}});
        assert_eq!(ts(&map), "Record<string, number>");
        let closed = json!({"type": "object", "additionalProperties": false});
        assert_eq!(ts(&closed), "Record<string, never>");
        assert_eq!(
            ts(&json!({"type": ["object", "null"]})),
            "Record<string, unknown> | null"
        );
    }

    #[test]
    fn declarations_carry_docs_and_refuse_bad_names() {
        let schemas = json!({"Tag": {"type": "string", "description": "A tag, not */ closed."}});
        let got = declarations(schemas.as_object().unwrap()).unwrap();
        assert_eq!(
            got,
            "/** A tag, not *\\/ closed. */\nexport type Tag = string;\n\n"
        );
        let bad = json!({"Two words": {"type": "string"}});
        assert!(matches!(
            declarations(bad.as_object().unwrap()),
            Err(SdkError::Name(_))
        ));
    }
}
