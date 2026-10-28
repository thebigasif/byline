//! Reading a folder of markdown files as Hugo, Jekyll, Astro and Eleventy
//! write them: YAML front matter between `---` lines, or TOML between
//! `+++` lines, then the body.

use chrono::NaiveDate;
use serde_json::{Map, Value};

use crate::folder::{Export, Front, SourceFile, UnreadFile};

/// Why [`read_folder`] refused a folder.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum FolderError {
    /// No file in it is a markdown post.
    #[error("no file in the folder is a markdown file with front matter")]
    NoPosts,
}

/// Why one file was not read as a post.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum FileError {
    /// It is MDX, whose components Khalid cannot run.
    #[error("it is MDX, whose components Khalid cannot run")]
    Mdx,
    /// Its front matter is not YAML or TOML: what the reader said.
    #[error("its front matter cannot be read: {0}")]
    FrontMatter(String),
    /// Its date is not a date.
    #[error("its date {0:?} is not a date")]
    Date(String),
}

/// Reads every markdown file of a folder, each given as its path inside
/// the folder, with `/` between parts, and its text. Files come in path
/// order. A file that is not markdown, such as an image or a template, is
/// not read; a markdown file that cannot be read is listed in
/// [`Export::unread`] with the reason.
///
/// # Errors
///
/// Returns [`FolderError::NoPosts`] when no file is a markdown post.
pub fn read_folder<P, T>(files: impl IntoIterator<Item = (P, T)>) -> Result<Export, FolderError>
where
    P: AsRef<str>,
    T: AsRef<str>,
{
    let mut files: Vec<(P, T)> = files.into_iter().collect();
    files.sort_by(|(a, _), (b, _)| a.as_ref().cmp(b.as_ref()));
    let mut export = Export::default();
    for (path, text) in &files {
        let path = path.as_ref();
        let Some(ext) = path.rsplit_once('.').map(|(_, ext)| ext) else {
            continue;
        };
        let read = match ext {
            "md" | "markdown" => read_file(path, text.as_ref()),
            "mdx" => Err(FileError::Mdx),
            _ => continue,
        };
        match read {
            Ok(file) => export.files.push(file),
            Err(error) => export.unread.push(UnreadFile {
                path: path.to_owned(),
                error,
            }),
        }
    }
    if export.files.is_empty() {
        return Err(FolderError::NoPosts);
    }
    Ok(export)
}

/// One markdown file: its front matter and body.
fn read_file(path: &str, text: &str) -> Result<SourceFile, FileError> {
    let text = text.trim_start_matches('\u{feff}');
    let (fields, body) = if let Some(rest) = text.strip_prefix("---\n") {
        let (yaml, body) = fenced(rest, "---");
        let value: Value = serde_saphyr::from_str(yaml)
            .map_err(|error| FileError::FrontMatter(error.to_string()))?;
        (value, body)
    } else if let Some(rest) = text.strip_prefix("+++\n") {
        let (front, body) = fenced(rest, "+++");
        let table: toml::Table =
            toml::from_str(front).map_err(|error| FileError::FrontMatter(error.to_string()))?;
        (json(toml::Value::Table(table)), body)
    } else {
        (Value::Null, text)
    };
    let fields = match fields {
        Value::Object(map) => map,
        _ => Map::new(),
    };
    Ok(SourceFile {
        path: path.to_owned(),
        front: front(&fields)?,
        body: body.trim_start_matches('\n').to_owned(),
    })
}

/// The front matter before the closing `fence` line and the body after it.
/// A file whose fence never closes is all front matter.
fn fenced<'a>(rest: &'a str, fence: &str) -> (&'a str, &'a str) {
    let mut at = 0;
    for line in rest.split_inclusive('\n') {
        if line.trim_end() == fence {
            let (front, after) = rest.split_at(at);
            return (front, after.get(line.len()..).unwrap_or(""));
        }
        at += line.len();
    }
    (rest, "")
}

/// A TOML value as the JSON value YAML would give, dates as their text.
fn json(value: toml::Value) -> Value {
    match value {
        toml::Value::String(text) => Value::String(text),
        toml::Value::Integer(n) => Value::from(n),
        toml::Value::Float(n) => Value::from(n),
        toml::Value::Boolean(yes) => Value::Bool(yes),
        toml::Value::Datetime(at) => Value::String(at.to_string()),
        toml::Value::Array(items) => Value::Array(items.into_iter().map(json).collect()),
        toml::Value::Table(table) => Value::Object(
            table
                .into_iter()
                .map(|(key, value)| (key, json(value)))
                .collect(),
        ),
    }
}

/// The fields Khalid reads from front matter, under the names the four
/// generators use.
fn front(fields: &Map<String, Value>) -> Result<Front, FileError> {
    let text = |names: &[&str]| {
        names.iter().find_map(|name| match fields.get(*name) {
            Some(Value::String(text)) if !text.trim().is_empty() => Some(text.trim().to_owned()),
            _ => None,
        })
    };
    let list = |name: &str| match fields.get(name) {
        Some(Value::String(text)) => text.split_whitespace().map(str::to_owned).collect(),
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| item.as_str().map(str::to_owned))
            .collect(),
        _ => Vec::new(),
    };
    let date = match text(&["date", "pubDate", "publishDate"]) {
        Some(written) => Some(date_of(&written).ok_or(FileError::Date(written))?),
        None => None,
    };
    let yes = |name: &str| fields.get(name).and_then(Value::as_bool);
    Ok(Front {
        title: text(&["title"]),
        date,
        slug: text(&["slug"]),
        draft: yes("draft") == Some(true) || yes("published") == Some(false),
        description: text(&["description", "summary", "excerpt"]),
        tags: list("tags"),
        categories: list("categories"),
        aliases: list("aliases"),
        permalink: text(&["permalink", "url"]),
        author: text(&["author"]),
    })
}

/// The day a date names, from its first ten characters, such as
/// `2024-03-12` in `2024-03-12 09:30:00 +0100`.
pub(crate) fn date_of(written: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(written.get(..10)?, "%Y-%m-%d").ok()
}
