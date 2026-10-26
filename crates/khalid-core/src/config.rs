//! Settings, read once when Khalid starts.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::{AbsoluteUrl, Locale, ParseUrlError};

/// Every setting Khalid reads, checked once at start and then passed by
/// reference (RULES.md BL-RUN-2).
///
/// It comes from `site.toml` in the site folder, then the environment, which
/// wins. No other module reads either source itself.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Config {
    name: String,
    address: AbsoluteUrl,
    mount: String,
    mode: Mode,
    slugs: SlugMode,
    default_language: Locale,
    languages: Vec<Locale>,
}

/// Whether Khalid runs a server or writes plain HTML.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Khalid serves the site and every server feature works.
    #[default]
    Server,
    /// Khalid writes plain HTML to `public/`, and server features are locked.
    Static,
}

/// How slugs are written (`docs/decisions/2026-10-26-cto-answers.md`, section 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SlugMode {
    /// Lowercase a to z, digits and hyphens, transliterated by language.
    #[default]
    Ascii,
    /// Letters from any script, for languages where ASCII is unreadable.
    Unicode,
}

/// `site.toml` as written, before the checks that span several fields.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SiteFile {
    name: String,
    address: AbsoluteUrl,
    #[serde(default = "root_mount")]
    mount: String,
    #[serde(default)]
    mode: Mode,
    #[serde(default)]
    slugs: SlugMode,
    default_language: Locale,
    languages: Vec<Locale>,
}

fn root_mount() -> String {
    "/".to_owned()
}

impl Config {
    /// The settings file's name inside a site folder.
    pub const FILE_NAME: &str = "site.toml";

    /// Reads `site.toml` from the `site` folder and applies `env` on top.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::Read`] when the file cannot be read, and the
    /// errors of [`Config::from_toml`] when its content is wrong.
    pub fn load(site: &Path, env: &Environment) -> Result<Self, ConfigError> {
        let mut path = site.to_path_buf();
        path.push(Self::FILE_NAME);
        let text = fs::read_to_string(&path).map_err(|source| ConfigError::Read {
            path: path.clone(),
            source,
        })?;
        Self::from_toml(&path, &text, env)
    }

    /// Checks the text of a settings file, named `path` in messages, and
    /// applies `env` on top.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::Parse`] or [`ConfigError::Field`] with the line
    /// and field of the first mistake, and [`ConfigError::Environment`] when an
    /// environment variable holds a value that is not allowed.
    pub fn from_toml(path: &Path, text: &str, env: &Environment) -> Result<Self, ConfigError> {
        let file: SiteFile = toml::from_str(text).map_err(|source| ConfigError::Parse {
            path: path.to_path_buf(),
            line: line_of_span(text, source.span()),
            field: field_of(text, &source),
            source: Box::new(source),
        })?;
        let field = |field: &str, problem: &str| ConfigError::Field {
            path: path.to_path_buf(),
            line: line_of_key(text, field),
            field: field.to_owned(),
            problem: problem.to_owned(),
        };
        let mount = &file.mount;
        if !mount.starts_with('/') || !mount.ends_with('/') || mount.contains("//") {
            return Err(field(
                "mount",
                "the mount path must start and end with /, like /blog/",
            ));
        }
        if file.languages.is_empty() {
            return Err(field("languages", "the site needs at least one language"));
        }
        if !file.languages.contains(&file.default_language) {
            return Err(field(
                "default_language",
                "the default language must be one of languages",
            ));
        }
        let address = match env.get(Environment::ADDRESS) {
            Some(text) => AbsoluteUrl::parse(text).map_err(|source| ConfigError::Environment {
                variable: Environment::ADDRESS.to_owned(),
                source,
            })?,
            None => file.address,
        };
        Ok(Self {
            name: file.name,
            address,
            mount: file.mount,
            mode: file.mode,
            slugs: file.slugs,
            default_language: file.default_language,
            languages: file.languages,
        })
    }

    /// The site's name, shown in titles and feeds.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Where the site lives, such as `https://www.example.com/`.
    pub fn address(&self) -> &AbsoluteUrl {
        &self.address
    }

    /// The folder the blog sits under on that site, such as `/blog/`.
    pub fn mount(&self) -> &str {
        &self.mount
    }

    /// Server or static.
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// How slugs are written.
    pub fn slugs(&self) -> SlugMode {
        self.slugs
    }

    /// The language a page is in when nothing says otherwise.
    pub fn default_language(&self) -> &Locale {
        &self.default_language
    }

    /// Every language the site publishes in, in the order written.
    pub fn languages(&self) -> &[Locale] {
        &self.languages
    }
}

/// The 1-based line a byte offset falls on.
fn line_of_span(text: &str, span: Option<std::ops::Range<usize>>) -> usize {
    let start = span.map_or(0, |span| span.start);
    text.bytes()
        .take(start)
        .filter(|&b| b == b'\n')
        .count()
        .saturating_add(1)
}

/// The 1-based line where `key = ...` is written, or the first line.
fn line_of_key(text: &str, key: &str) -> usize {
    text.lines()
        .position(|line| {
            line.split_once('=')
                .is_some_and(|(left, _)| left.trim() == key)
        })
        .map_or(1, |index| index.saturating_add(1))
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
    let line = line_of_span(text, error.span());
    text.lines()
        .nth(line.saturating_sub(1))
        .and_then(|line| line.split_once('='))
        .map_or_else(String::new, |(key, _)| key.trim().to_owned())
}

/// The environment variables Khalid reads, captured once at start.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Environment(BTreeMap<String, String>);

impl Environment {
    /// Overrides `address` in `site.toml`, for deploys that move the site.
    pub const ADDRESS: &str = "KHALID_ADDRESS";

    /// Every variable Khalid reads.
    const ALL: [&str; 1] = [Self::ADDRESS];

    /// Reads Khalid's variables from this process's environment.
    pub fn from_process() -> Self {
        let mut found = BTreeMap::new();
        for name in Self::ALL {
            #[expect(
                clippy::disallowed_methods,
                reason = "the one place that reads the environment (BL-RUN-2)"
            )]
            let value = std::env::var(name);
            if let Ok(value) = value {
                found.insert(name.to_owned(), value);
            }
        }
        Self(found)
    }

    /// An environment with no variables set.
    pub fn empty() -> Self {
        Self::default()
    }

    /// An environment with exactly these variables, for tests and tools.
    pub fn from_pairs(pairs: &[(&str, &str)]) -> Self {
        Self(
            pairs
                .iter()
                .map(|&(name, value)| (name.to_owned(), value.to_owned()))
                .collect(),
        )
    }

    fn get(&self, name: &str) -> Option<&str> {
        self.0.get(name).map(String::as_str)
    }
}

/// Why the settings could not be read.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ConfigError {
    /// The settings file could not be read.
    #[error("could not read {}", path.display())]
    Read {
        /// The file.
        path: PathBuf,
        /// What the file system reported.
        #[source]
        source: io::Error,
    },
    /// The file is not valid, or a field has the wrong kind of value.
    #[error("{}, line {line}, field {field:?}: {}", path.display(), source.message())]
    Parse {
        /// The file.
        path: PathBuf,
        /// The 1-based line of the mistake.
        line: usize,
        /// The field, or empty when the mistake is not in one field.
        field: String,
        /// What the TOML reader found.
        #[source]
        source: Box<toml::de::Error>,
    },
    /// A field is readable but breaks a rule that spans the file.
    #[error("{}, line {line}, field {field:?}: {problem}", path.display())]
    Field {
        /// The file.
        path: PathBuf,
        /// The 1-based line of the field.
        line: usize,
        /// The field.
        field: String,
        /// The rule it breaks.
        problem: String,
    },
    /// An environment variable holds a value that is not allowed.
    #[error("the environment variable {variable} is not a valid address")]
    Environment {
        /// The variable.
        variable: String,
        /// Why its value was refused.
        #[source]
        source: ParseUrlError,
    },
}
