//! Who may read a post, which template shows it, and the hash of its
//! password: settings that front matter, the Admin API and the server share.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Who may read a post's body (plan section 4.6, "access").
///
/// Paid tiers come later as more variants, so the enum is
/// `#[non_exhaustive]`. A reader who may not read the body sees the
/// title, the excerpt and a short note, never the body.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum PostAccess {
    /// Everyone.
    #[default]
    Public,
    /// Signed-in members only.
    Members,
}

impl PostAccess {
    /// Every kind of access, in the order the admin lists them.
    pub const ALL: [Self; 2] = [Self::Public, Self::Members];

    /// The name front matter and the API use, such as `members`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Members => "members",
        }
    }
}

impl fmt::Display for PostAccess {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The name of a theme template a post asks for in place of the default
/// post template, such as `wide`. The theme holds it as
/// `templates/post-<name>.html`.
///
/// A name is lowercase letters a to z, digits and single hyphens, at most
/// [`TemplateName::MAX_LEN`] bytes, so it can never point outside the
/// theme's `templates/` folder (RULES.md BL-SEC-4).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct TemplateName(String);

impl TemplateName {
    /// The longest name, in bytes.
    pub const MAX_LEN: usize = 40;

    /// Checks `name` and wraps it.
    ///
    /// # Errors
    ///
    /// Returns [`ParseTemplateNameError`] when `name` is empty, too long,
    /// or has anything but lowercase letters, digits and single hyphens
    /// inside it.
    pub fn try_new(name: &str) -> Result<Self, ParseTemplateNameError> {
        let allowed = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-';
        let fits = !name.is_empty() && name.len() <= Self::MAX_LEN;
        let shaped = !name.starts_with('-') && !name.ends_with('-') && !name.contains("--");
        if fits && shaped && name.chars().all(allowed) {
            Ok(Self(name.to_owned()))
        } else {
            Err(ParseTemplateNameError(name.to_owned()))
        }
    }

    /// The name.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The template's file name inside the theme's `templates/` folder.
    pub fn file_name(&self) -> String {
        format!("post-{}.html", self.0)
    }
}

impl fmt::Display for TemplateName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for TemplateName {
    type Err = ParseTemplateNameError;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        Self::try_new(name)
    }
}

impl TryFrom<String> for TemplateName {
    type Error = ParseTemplateNameError;

    fn try_from(name: String) -> Result<Self, Self::Error> {
        Self::try_new(&name)
    }
}

impl From<TemplateName> for String {
    fn from(name: TemplateName) -> Self {
        name.0
    }
}

/// A name that cannot be a template name.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "{0:?} is not a template name; use lowercase letters, digits and single hyphens, such as wide, for theme/templates/post-wide.html"
)]
pub struct ParseTemplateNameError(String);

/// The Argon2id hash of a post's password, in the PHC form
/// `$argon2id$v=19$m=...,t=...,p=...$<salt>$<hash>`.
///
/// Front matter holds only this hash, never the password, so a site
/// folder in a public repository gives nothing away. The hash is checked
/// by whoever unlocks the post; this type only checks its shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct PasswordHash(String);

impl PasswordHash {
    /// The start every hash has.
    const PREFIX: &'static str = "$argon2id$";

    /// Checks that `phc` looks like an Argon2id hash and wraps it.
    ///
    /// # Errors
    ///
    /// Returns [`ParsePasswordHashError`] when `phc` does not start with
    /// `$argon2id$` or has spaces in it.
    pub fn try_new(phc: &str) -> Result<Self, ParsePasswordHashError> {
        let rest = phc.strip_prefix(Self::PREFIX).unwrap_or_default();
        if rest.is_empty() || phc.contains(char::is_whitespace) {
            return Err(ParsePasswordHashError);
        }
        Ok(Self(phc.to_owned()))
    }

    /// The hash in its PHC form.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for PasswordHash {
    type Error = ParsePasswordHashError;

    fn try_from(phc: String) -> Result<Self, Self::Error> {
        Self::try_new(&phc)
    }
}

impl From<PasswordHash> for String {
    fn from(hash: PasswordHash) -> Self {
        hash.0
    }
}

/// Text that is not an Argon2id password hash.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "this is not a password hash; set a post's password through the admin or the Admin API, which store only its hash"
)]
pub struct ParsePasswordHashError;
