//! The checked values a token holds, with the rules the renderer applies
//! to `theme/tokens.toml`, so a file this crate writes always loads there.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::Rgb;
use crate::split::split_top;

/// The longest font list a token may hold, as in the renderer.
const MAX_FONT_LIST: usize = 300;

/// Generic CSS families. A font list without one gets `sans-serif` added,
/// so text always has a font to fall back to.
const GENERIC: [&str; 9] = [
    "serif",
    "sans-serif",
    "monospace",
    "cursive",
    "system-ui",
    "ui-sans-serif",
    "ui-serif",
    "ui-monospace",
    "math",
];

/// A CSS `font-family` list that cannot end its declaration: names,
/// commas, spaces, hyphens and paired double quotes only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct FontStack(String);

/// A CSS length such as `0.375rem` or `68ch`, in `px`, `rem`, `em` or `ch`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Length(String);

impl FontStack {
    /// The list as written.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// A list this crate wrote itself, for the defaults.
    pub(crate) fn trusted(text: &str) -> Self {
        Self(text.to_owned())
    }

    /// The safe part of a list found on a site: each name that keeps to the
    /// allowed characters, with `sans-serif` added when no generic family
    /// is left. `None` when no name is safe.
    pub(crate) fn from_found(list: &str) -> Option<Self> {
        let mut names: Vec<&str> = split_top(list, |c| c == ',')
            .into_iter()
            .filter(|name| is_safe_name(name))
            .collect();
        if names.is_empty() {
            return None;
        }
        if !names.iter().any(|name| GENERIC.contains(name)) {
            names.push("sans-serif");
        }
        Self::try_from(names.join(", ")).ok()
    }
}

/// Whether one family name keeps to the allowed characters: a bare name, or
/// one wrapped in a single pair of double quotes.
fn is_safe_name(name: &str) -> bool {
    let inner = name
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .unwrap_or(name);
    !inner.is_empty()
        && inner
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-'))
}

impl TryFrom<String> for FontStack {
    type Error = &'static str;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        let allowed = |c: char| c.is_alphanumeric() || matches!(c, ' ' | ',' | '-' | '"');
        let balanced = text.matches('"').count().is_multiple_of(2);
        if text.trim().is_empty()
            || text.len() > MAX_FONT_LIST
            || !balanced
            || !text.chars().all(allowed)
        {
            return Err(
                "a font list may hold only names, commas, spaces, hyphens and paired double quotes",
            );
        }
        Ok(Self(text))
    }
}

impl From<FontStack> for String {
    fn from(stack: FontStack) -> Self {
        stack.0
    }
}

impl Length {
    /// The length as written.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// A length this crate wrote itself, for the defaults.
    pub(crate) fn trusted(text: &str) -> Self {
        Self(text.to_owned())
    }
}

impl TryFrom<String> for Length {
    type Error = &'static str;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        let number = ["px", "rem", "em", "ch"]
            .into_iter()
            .find_map(|unit| text.strip_suffix(unit));
        let valid = number.is_some_and(|number| {
            number.matches('.').count() <= 1
                && number.chars().all(|c| c.is_ascii_digit() || c == '.')
                && number.chars().any(|c| c.is_ascii_digit())
        });
        if valid {
            Ok(Self(text))
        } else {
            Err("a length must be a number and px, rem, em or ch, such as 0.375rem")
        }
    }
}

impl From<Length> for String {
    fn from(length: Length) -> Self {
        length.0
    }
}

/// Colours in the file are `#rrggbb` or `#rgb`, nothing else, as the
/// renderer reads them.
pub(crate) mod hex {
    use super::{Deserialize, Deserializer, Rgb, Serializer};

    #[expect(
        clippy::trivially_copy_pass_by_ref,
        reason = "serde's `with` attribute passes the value by reference"
    )]
    pub(crate) fn serialize<S: Serializer>(color: &Rgb, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&color.to_hex())
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Rgb, D::Error> {
        let text = String::deserialize(deserializer)?;
        let digits = text.strip_prefix('#').map_or(0, str::len);
        match Rgb::from_css(&text) {
            Some(color) if matches!(digits, 3 | 6) => Ok(color),
            _ => Err(serde::de::Error::custom(
                "a colour must be written #rrggbb or #rgb, such as #1f2328",
            )),
        }
    }
}
