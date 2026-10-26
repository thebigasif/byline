//! Blocks that post to the server: forms.

use super::Attributes;
use crate::{BlockError, BlockKind, FieldKind, FormField};

/// The field name the spam trap uses, so no writer's field may take it.
pub(crate) const TRAP: &str = "homepage";

/// Reads a form: its id, its button's words and one field per line, such
/// as `- email reply_to: Your email address`.
pub(super) fn form(attributes: &Attributes, body: &[&str]) -> Result<BlockKind, BlockError> {
    attributes.only("form", &["id", "button"], "id and button")?;
    let id = attributes.need("id", "a form needs id, the name its answers are kept under")?;
    let button = attributes.need("button", "a form needs button, the words on its button")?;
    if id.is_empty()
        || !id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err(BlockError::Value {
            line: attributes.line,
            problem: "a form's id is lowercase letters, digits and -, such as contact",
        });
    }
    let mut fields: Vec<FormField> = Vec::new();
    for (number, text) in (attributes.line.saturating_add(1)..).zip(body) {
        if text.trim().is_empty() {
            continue;
        }
        let field = field(text).ok_or(BlockError::Value {
            line: number,
            problem: "each line of a form is a field, such as - email reply_to: Your email address",
        })?;
        if field.name == TRAP || fields.iter().any(|seen| seen.name == field.name) {
            return Err(BlockError::Value {
                line: number,
                problem: "each field of a form has its own name, and homepage is kept for the spam trap",
            });
        }
        fields.push(field);
    }
    if fields.is_empty() {
        return Err(BlockError::Value {
            line: attributes.line,
            problem: "a form needs at least one field, such as - email reply_to: Your email address",
        });
    }
    Ok(BlockKind::Form { id, button, fields })
}

/// Reads `- kind name: label`, with `?` after the name for an optional field.
fn field(text: &str) -> Option<FormField> {
    let (head, label) = text.trim().strip_prefix("- ")?.split_once(':')?;
    let mut words = head.split_whitespace();
    let (kind, name, None) = (words.next()?, words.next()?, words.next()) else {
        return None;
    };
    let (name, required) = name
        .strip_suffix('?')
        .map_or((name, true), |name| (name, false));
    let label = label.trim();
    let plain = |b: u8| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_';
    let readable = !name.is_empty() && name.bytes().all(plain) && !label.is_empty();
    Some(FormField {
        kind: FieldKind::named(kind).filter(|_| readable)?,
        name: name.to_owned(),
        label: label.to_owned(),
        required,
    })
}
