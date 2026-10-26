//! `routes.toml`, read once when Khalid starts.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use toml::Spanned;

use crate::{Permalink, RoutesError};

/// Every route setting of a site. Built from `routes.toml`, or from the
/// defaults when the site has no such file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Routes {
    collections: BTreeMap<String, Collection>,
}

/// A collection: the folder its posts live in, and their address pattern.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Collection {
    permalink: Permalink,
}

impl Collection {
    /// The address pattern of the collection's posts.
    pub fn permalink(&self) -> &Permalink {
        &self.permalink
    }
}

impl Default for Routes {
    /// The posts collection at `/{slug}/`.
    fn default() -> Self {
        Self {
            collections: BTreeMap::from([("posts".to_owned(), Collection::default())]),
        }
    }
}

/// `routes.toml` as written, with the place of each value kept for messages.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RoutesFile {
    #[serde(default)]
    collections: BTreeMap<String, CollectionFile>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CollectionFile {
    permalink: Spanned<String>,
}

impl Routes {
    /// Checks the text of a routes file, named `path` in messages. A
    /// collection the file does not name keeps its default.
    ///
    /// # Errors
    ///
    /// Returns [`RoutesError::Parse`] for broken TOML, an unknown field or a
    /// value of the wrong kind, and [`RoutesError::Field`] for a value that
    /// breaks a rule, each with the line and field of the first mistake.
    pub fn from_toml(path: &Path, text: &str) -> Result<Self, RoutesError> {
        let file: RoutesFile = toml::from_str(text).map_err(|source| RoutesError::Parse {
            path: path.to_path_buf(),
            line: line_of(text, source.span().map_or(0, |span| span.start)),
            field: field_of(text, &source),
            source: Box::new(source),
        })?;
        let at = Located { path, text };
        let mut routes = Self::default();
        for (name, collection) in file.collections {
            let permalink = at.check("permalink", &collection.permalink, |text| {
                Permalink::parse(text)
            })?;
            routes.collections.insert(name, Collection { permalink });
        }
        Ok(routes)
    }

    /// The collection named after its folder, such as `posts`.
    pub fn collection(&self, name: &str) -> Option<&Collection> {
        self.collections.get(name)
    }
}

/// The file a value came from, so a rule it breaks can name its line.
struct Located<'a> {
    path: &'a Path,
    text: &'a str,
}

impl Located<'_> {
    /// Runs `rule` on a value, and on failure names the value's line.
    fn check<T, U>(
        &self,
        field: &str,
        value: &Spanned<T>,
        rule: impl FnOnce(&T) -> Result<U, String>,
    ) -> Result<U, RoutesError> {
        rule(value.get_ref()).map_err(|problem| RoutesError::Field {
            path: PathBuf::from(self.path),
            line: line_of(self.text, value.span().start),
            field: field.to_owned(),
            problem,
        })
    }
}

/// The 1-based line a byte offset falls on.
fn line_of(text: &str, offset: usize) -> usize {
    text.bytes()
        .take(offset)
        .filter(|&b| b == b'\n')
        .count()
        .saturating_add(1)
}

/// The field a parse error is about: the one named in the message, or the
/// key on the line the error points at.
fn field_of(text: &str, error: &toml::de::Error) -> String {
    let message = error.message();
    if let Some((_, rest)) = message.split_once("field `")
        && let Some((name, _)) = rest.split_once('`')
    {
        return name.to_owned();
    }
    let line = line_of(text, error.span().map_or(0, |span| span.start));
    text.lines()
        .nth(line.saturating_sub(1))
        .and_then(|line| line.split_once('='))
        .map_or_else(String::new, |(key, _)| key.trim().to_owned())
}
