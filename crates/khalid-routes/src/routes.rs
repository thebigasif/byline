//! `routes.toml`, read once when Khalid starts.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use khalid_core::Slug;
use serde::Deserialize;
use toml::Spanned;

use crate::permalink::{folder_shape, plain};
use crate::{Channel, ChannelFilter, Permalink, RoutesError};

/// Every route setting of a site: collections, channels and taxonomy
/// prefixes. Built from `routes.toml`, or from the defaults when the site has
/// no such file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Routes {
    collections: BTreeMap<String, Collection>,
    channels: Vec<Channel>,
    tag_prefix: String,
    category_prefix: String,
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
    /// The posts collection at `/{slug}/`, no channels, and `tag` and
    /// `category` as the taxonomy prefixes.
    fn default() -> Self {
        Self {
            collections: BTreeMap::from([("posts".to_owned(), Collection::default())]),
            channels: Vec::new(),
            tag_prefix: "tag".to_owned(),
            category_prefix: "category".to_owned(),
        }
    }
}

/// `routes.toml` as written, with the place of each value kept for messages.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RoutesFile {
    #[serde(default)]
    taxonomies: TaxonomiesFile,
    #[serde(default)]
    collections: BTreeMap<String, CollectionFile>,
    #[serde(default)]
    channels: BTreeMap<String, ChannelFile>,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct TaxonomiesFile {
    tag: Option<Spanned<String>>,
    category: Option<Spanned<String>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CollectionFile {
    permalink: Spanned<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChannelFile {
    path: Spanned<String>,
    filter: Spanned<ChannelFilter>,
}

impl Routes {
    /// The routes file's name inside a site folder.
    pub const FILE_NAME: &str = "routes.toml";

    /// Reads `routes.toml` from the `site` folder, or gives the defaults when
    /// there is none.
    ///
    /// # Errors
    ///
    /// Returns [`RoutesError::Read`] when the file exists but cannot be read,
    /// and the errors of [`Routes::from_toml`] when its content is wrong.
    pub fn load(site: &Path) -> Result<Self, RoutesError> {
        let mut path = site.to_path_buf();
        path.push(Self::FILE_NAME);
        match fs::read_to_string(&path) {
            Ok(text) => Self::from_toml(&path, &text),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(source) => Err(RoutesError::Read { path, source }),
        }
    }

    /// Checks the text of a routes file, named `path` in messages. A
    /// collection or prefix the file does not name keeps its default.
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
        if let Some(tag) = &file.taxonomies.tag {
            routes.tag_prefix = at.check("tag", tag, |text| prefix(text))?;
        }
        if let Some(category) = &file.taxonomies.category {
            routes.category_prefix = at.check("category", category, |text| prefix(text))?;
        }
        for (name, collection) in file.collections {
            let permalink = at.check("permalink", &collection.permalink, |text| {
                Permalink::parse(text)
            })?;
            routes.collections.insert(name, Collection { permalink });
        }
        for (name, channel) in file.channels {
            let path = at.check("path", &channel.path, |text| {
                folder_shape(text)
                    .and_then(|()| plain(text))
                    .map(|()| text.to_owned())
            })?;
            let filter = at.check("filter", &channel.filter, |filter| {
                if filter.is_empty() {
                    return Err(
                        "a channel needs at least one filter, such as author or tag".to_owned()
                    );
                }
                Ok(filter.clone())
            })?;
            routes.channels.push(Channel { name, path, filter });
        }
        Ok(routes)
    }

    /// The collection named after its folder, such as `posts`.
    pub fn collection(&self, name: &str) -> Option<&Collection> {
        self.collections.get(name)
    }

    /// Every channel, in name order.
    pub fn channels(&self) -> &[Channel] {
        &self.channels
    }

    /// The folder tag pages sit in, such as `tag` in `/tag/links/`.
    pub fn tag_prefix(&self) -> &str {
        &self.tag_prefix
    }

    /// The folder category pages sit in, such as `category`.
    pub fn category_prefix(&self) -> &str {
        &self.category_prefix
    }
}

/// Checks a taxonomy prefix: one slug, such as `topic`.
fn prefix(text: &str) -> Result<String, String> {
    Slug::try_new(text)
        .map(|slug| slug.as_str().to_owned())
        .map_err(|_| "a prefix is one lowercase word or hyphenated words, such as topic".to_owned())
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
