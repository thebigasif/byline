//! A post: its front matter and its markdown body.

use std::path::Path;

use chrono::NaiveDate;
use khalid_core::Slug;
use serde::{Deserialize, Deserializer};

use crate::front_matter::{Fence, split};
use crate::{Collection, ContentError, ContentTree};

/// What a post's front matter may say.
///
/// Title, date and author may be missing: the linter blocks a post without
/// them and explains why, which teaches more than a refusal to read the file.
/// Unknown fields and values of the wrong kind are refused here.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct PostFront {
    /// The headline.
    pub title: Option<String>,
    /// The last part of the address. Made from the title when missing.
    pub slug: Option<Slug>,
    /// The day the post is dated.
    #[serde(default, deserialize_with = "date")]
    pub date: Option<NaiveDate>,
    /// The author's slug, matching a file in `authors/`.
    pub author: Option<Slug>,
    /// The category's slug, matching one in `categories.toml`.
    pub category: Option<Slug>,
    /// The summary for search results and feeds.
    pub description: Option<String>,
    /// The key shared by the versions of this post in other languages.
    pub translation: Option<String>,
}

/// A post as read from its file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Post {
    /// The front matter.
    pub front: PostFront,
    /// The markdown after the front matter, exactly as written.
    pub body: String,
}

/// Reads a post from `text`, naming `path` in errors.
///
/// # Errors
///
/// Returns a [`ContentError`] when the front matter fence is missing or never
/// closed, or when a field is unknown, has the wrong kind of value, or the
/// YAML is broken. Field errors carry the file's line.
pub fn parse_post(path: &Path, text: &str) -> Result<Post, ContentError> {
    let parts = split(text).map_err(|fence| match fence {
        Fence::Missing => ContentError::NoFrontMatter {
            path: path.to_path_buf(),
        },
        Fence::Unclosed => ContentError::UnclosedFrontMatter {
            path: path.to_path_buf(),
        },
    })?;
    let front = if parts.yaml.trim().is_empty() {
        PostFront::default()
    } else {
        serde_saphyr::from_str(parts.yaml).map_err(|source| located(path, parts.yaml, source))?
    };
    Ok(Post {
        front,
        body: parts.body.to_owned(),
    })
}

/// Turns a YAML error into one that names the file's line and field.
fn located(path: &Path, yaml: &str, source: serde_saphyr::Error) -> ContentError {
    let yaml_line = source
        .location()
        .and_then(|location| usize::try_from(location.line()).ok())
        .unwrap_or(1);
    let field = yaml
        .lines()
        .nth(yaml_line.saturating_sub(1))
        .and_then(|line| line.split_once(':'))
        .map_or_else(String::new, |(key, _)| key.trim().to_owned());
    let rendered = source.to_string();
    let first = rendered.lines().next().unwrap_or_default();
    // The reader writes "error: line 2 column 7: <problem>"; keep the problem.
    let without_error = first.strip_prefix("error: ").unwrap_or(first);
    let problem = match without_error.split_once(": ") {
        Some((place, problem)) if place.starts_with("line ") => problem,
        _ => without_error,
    }
    .to_owned();
    // Two of the reader's phrases mean nothing to a writer; say what happened.
    let problem = match problem.as_str() {
        "expected string scalar" => {
            "this field must be plain text, not a list or a group".to_owned()
        }
        "mapping values are not allowed in this context" => {
            "the line has a second colon; put the value in quotes if it needs one".to_owned()
        }
        _ => problem,
    };
    ContentError::Field {
        path: path.to_path_buf(),
        // The YAML starts on the file's second line, after the opening ---.
        line: yaml_line.saturating_add(1),
        field,
        problem,
        source: Box::new(source),
    }
}

/// Reads a date written as `YYYY-MM-DD`, with a message a writer understands.
fn date<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<NaiveDate>, D::Error> {
    let text = String::deserialize(deserializer)?;
    NaiveDate::parse_from_str(&text, "%Y-%m-%d")
        .map(Some)
        .map_err(|_| {
            serde::de::Error::custom(
                "the date must be a real day written as YYYY-MM-DD, such as 2026-09-14",
            )
        })
}

/// Reads the front matter of every entry in every `posts` collection, in the
/// tree's path order. Other collections get their own schemas.
///
/// # Errors
///
/// Returns the first [`ContentError`] from [`parse_post`].
pub fn read_posts(tree: &ContentTree) -> Result<Vec<Post>, ContentError> {
    tree.languages()
        .iter()
        .filter_map(|language| language.collection("posts"))
        .flat_map(Collection::entries)
        .map(|entry| parse_post(entry.path(), entry.text()))
        .collect()
}
