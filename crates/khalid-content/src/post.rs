//! A post: its front matter and its markdown body.

use std::path::Path;

use chrono::NaiveDate;
use khalid_core::{AbsoluteUrl, RobotsDirectives, Slug};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer};

use crate::front_matter::{Fence, split};
use crate::{ContentError, parse_blocks};

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
    /// Old paths that redirect here, such as `/2019/05/old-title/`, so a
    /// moved post keeps its links and rankings.
    #[serde(default, deserialize_with = "aliases")]
    pub aliases: Vec<String>,
    /// Whether the post is featured, for the featured channel and lists.
    #[serde(default)]
    pub featured: bool,
    /// The post's own robots directives, such as `noindex, nofollow`, in
    /// place of the site's defaults.
    pub robots: Option<RobotsDirectives>,
    /// The address search engines should credit instead of this post, such
    /// as the original of a post first published elsewhere.
    pub canonical: Option<AbsoluteUrl>,
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
/// closed, when a field is unknown, has the wrong kind of value, or the YAML
/// is broken, or when a block in the body is wrong. Field and block errors
/// carry the file's line.
pub fn parse_post(path: &Path, text: &str) -> Result<Post, ContentError> {
    let (front, body) = parse_front(path, text)?;
    Ok(Post { front, body })
}

/// Reads any kind of front matter from `text`, naming `path` in errors, and
/// returns it with the body exactly as written once its blocks are checked.
pub(crate) fn parse_front<F: DeserializeOwned + Default>(
    path: &Path,
    text: &str,
) -> Result<(F, String), ContentError> {
    let parts = split(text).map_err(|fence| match fence {
        Fence::Missing => ContentError::NoFrontMatter {
            path: path.to_path_buf(),
        },
        Fence::Unclosed => ContentError::UnclosedFrontMatter {
            path: path.to_path_buf(),
        },
    })?;
    let front = if parts.yaml.trim().is_empty() {
        F::default()
    } else {
        serde_saphyr::from_str(parts.yaml).map_err(|source| located(path, parts.yaml, source))?
    };
    parse_blocks(parts.body).map_err(|source| {
        // The body starts on the line after the closing ---.
        let before = text
            .get(..text.len().saturating_sub(parts.body.len()))
            .unwrap_or_default()
            .matches('\n')
            .count();
        ContentError::Block {
            path: path.to_path_buf(),
            line: before.saturating_add(source.line()),
            source,
        }
    })?;
    Ok((front, parts.body.to_owned()))
}

/// Turns a YAML error into one that names the file's line and field.
fn located(path: &Path, yaml: &str, source: serde_saphyr::Error) -> ContentError {
    let yaml_line = source
        .location()
        .and_then(|location| usize::try_from(location.line()).ok())
        .unwrap_or(1);
    // The key on the error's line, or, for an item of a list such as
    // `  - old-title`, the nearest key above it that starts a line.
    let lines: Vec<&str> = yaml.lines().take(yaml_line).collect();
    let own = lines
        .last()
        .and_then(|line| line.split_once(':'))
        .map(|(key, _)| key);
    let field = own
        .filter(|key| !key.trim_start().starts_with('-'))
        .or_else(|| {
            lines
                .iter()
                .rev()
                .find(|line| !line.starts_with([' ', '-']) && line.contains(':'))
                .and_then(|line| line.split_once(':'))
                .map(|(key, _)| key)
        })
        .map_or_else(String::new, |key| key.trim().to_owned());
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
pub(crate) fn date<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<NaiveDate>, D::Error> {
    let text = String::deserialize(deserializer)?;
    NaiveDate::parse_from_str(&text, "%Y-%m-%d")
        .map(Some)
        .map_err(|_| {
            serde::de::Error::custom(
                "the date must be a real day written as YYYY-MM-DD, such as 2026-09-14",
            )
        })
}

/// Reads old paths, each starting with `/`, with a message a writer
/// understands.
fn aliases<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<String>, D::Error> {
    let paths = Vec::<String>::deserialize(deserializer)?;
    match paths
        .iter()
        .find(|path| !path.starts_with('/') || path.contains(char::is_whitespace))
    {
        Some(bad) => Err(serde::de::Error::custom(format!(
            "{bad:?} is not a path; an alias is an old address on this site starting with /, such as /2019/05/old-title/"
        ))),
        None => Ok(paths),
    }
}
