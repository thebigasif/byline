//! Reading the JSON file that Settings, Import/Export in Ghost writes.

use chrono::NaiveDateTime;
use serde::Deserialize;
use serde_json::Value;

use crate::ghost::{Body, Export, Post, PostKind, PostStatus, Visibility};

/// Why [`read_export`] refused a file.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ExportError {
    /// The file is not JSON, often because the download stopped part of the
    /// way through, or a field holds JSON of the wrong kind.
    #[error("the file is not JSON that Ghost writes, at line {line}, column {column}: {source}")]
    NotJson {
        /// The file's line, counting from 1.
        line: usize,
        /// The column on that line, counting from 1.
        column: usize,
        /// What the JSON reader found.
        #[source]
        source: serde_json::Error,
    },
    /// The file is JSON, but has no `db` list with the data of a Ghost
    /// export in it.
    #[error("the file is not a Ghost export: it has no db list with data in it")]
    NotGhost,
    /// A field holds a value of the wrong kind, such as a date that is not a
    /// date.
    #[error("post {post}: {field} holds {value:?}, which is not what Ghost writes there")]
    BadValue {
        /// The post's id.
        post: String,
        /// The field, such as `published_at`.
        field: &'static str,
        /// The value as written.
        value: String,
    },
}

/// The file, which wraps the database in a `db` list.
#[derive(Deserialize)]
struct File {
    #[serde(default)]
    db: Vec<Database>,
}

#[derive(Deserialize)]
struct Database {
    meta: Option<Meta>,
    data: Data,
}

#[derive(Deserialize)]
struct Meta {
    version: Option<String>,
}

#[derive(Deserialize)]
struct Data {
    #[serde(default)]
    posts: Vec<RawPost>,
    #[serde(default)]
    settings: Vec<Setting>,
}

#[derive(Deserialize)]
struct RawPost {
    id: String,
    title: Option<String>,
    slug: Option<String>,
    lexical: Option<String>,
    mobiledoc: Option<String>,
    html: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
    status: Option<String>,
    visibility: Option<String>,
    published_at: Option<Value>,
}

#[derive(Deserialize)]
struct Setting {
    key: String,
    value: Option<Value>,
}

/// Reads a Ghost export file into an [`Export`].
///
/// # Errors
///
/// Returns [`ExportError::NotJson`] with the place when the text is not
/// JSON of the shape Ghost writes, [`ExportError::NotGhost`] when it is not
/// a Ghost export, and [`ExportError::BadValue`] when a post's date is not
/// a date.
pub fn read_export(file: &str) -> Result<Export, ExportError> {
    let file: File = serde_json::from_str(file).map_err(|source| ExportError::NotJson {
        line: source.line(),
        column: source.column(),
        source,
    })?;
    let Database { meta, data } = file.db.into_iter().next().ok_or(ExportError::NotGhost)?;
    let posts = data.posts.into_iter().map(post).collect::<Result<_, _>>()?;
    let title = data
        .settings
        .into_iter()
        .find(|setting| setting.key == "title")
        .and_then(|setting| setting.value)
        .and_then(|value| value.as_str().and_then(text));
    Ok(Export {
        title,
        version: meta.and_then(|meta| meta.version),
        posts,
    })
}

/// One post from its row.
fn post(raw: RawPost) -> Result<Post, ExportError> {
    let published = time(&raw.id, raw.published_at)?;
    let blank = |value: Option<String>| value.and_then(|t| text(&t));
    let body = match (blank(raw.lexical), blank(raw.mobiledoc), blank(raw.html)) {
        (Some(lexical), _, _) => Body::Lexical(lexical),
        (None, Some(mobiledoc), _) => Body::Mobiledoc(mobiledoc),
        (None, None, Some(html)) => Body::Html(html),
        (None, None, None) => Body::Empty,
    };
    let kind = if raw.kind.as_deref() == Some("page") {
        PostKind::Page
    } else {
        PostKind::Post
    };
    Ok(Post {
        kind,
        status: PostStatus::from_name(raw.status.as_deref().unwrap_or("draft")),
        visibility: Visibility::from_name(raw.visibility.as_deref().unwrap_or("public")),
        title: raw.title.unwrap_or_default(),
        slug: raw.slug.unwrap_or_default(),
        published,
        body,
        id: raw.id,
    })
}

/// `value`, unless it is blank.
fn text(value: &str) -> Option<String> {
    Some(value)
        .filter(|t| !t.trim().is_empty())
        .map(str::to_owned)
}

/// A post's date, such as `2024-03-12T09:30:00.000Z`. Ghost keeps dates
/// in UTC.
fn time(post: &str, value: Option<Value>) -> Result<Option<NaiveDateTime>, ExportError> {
    let bad = |value: String| ExportError::BadValue {
        post: post.to_owned(),
        field: "published_at",
        value,
    };
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => {
            let bare = text.trim().trim_end_matches('Z');
            match NaiveDateTime::parse_from_str(bare, "%Y-%m-%dT%H:%M:%S%.f") {
                Ok(at) => Ok(Some(at)),
                Err(_) => Err(bad(text)),
            }
        }
        Some(other) => Err(bad(other.to_string())),
    }
}
