//! The value types a token can hold, each checked so it cannot break out of
//! its CSS declaration.

use std::fmt;

use serde::Deserialize;

/// A colour written `#rrggbb` or `#rgb`, kept as lowercase `#rrggbb`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub struct Color {
    text: String,
    channels: [u8; 3],
}

/// A CSS `font-family` list such as `Inter, "Segoe UI", sans-serif`: names,
/// commas, spaces, hyphens and double quotes, nothing that could end the
/// declaration.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub struct FontStack(String);

/// A CSS length such as `0.375rem` or `68ch`, in `px`, `rem`, `em` or `ch`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub struct Length(String);

/// Why a token value was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseTokenError(&'static str);

impl fmt::Display for ParseTokenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for ParseTokenError {}

impl Color {
    pub(crate) fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Self {
            text: format!("#{red:02x}{green:02x}{blue:02x}"),
            channels: [red, green, blue],
        }
    }

    /// The colour as `#rrggbb`.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// The WCAG contrast ratio between this colour and `other`, from 1 to 21.
    pub fn contrast(&self, other: &Self) -> f64 {
        let (a, b) = (self.luminance(), other.luminance());
        let (light, dark) = if a > b { (a, b) } else { (b, a) };
        (light + 0.05) / (dark + 0.05)
    }

    /// Relative luminance as WCAG 2.2 defines it.
    fn luminance(&self) -> f64 {
        let [red, green, blue] = self.channels.map(|channel| {
            let value = f64::from(channel) / 255.0;
            if value <= 0.040_45 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        });
        0.2126 * red + 0.7152 * green + 0.0722 * blue
    }
}

impl TryFrom<String> for Color {
    type Error = ParseTokenError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        const WRONG: ParseTokenError =
            ParseTokenError("a colour must be written #rrggbb or #rgb, such as #1f2328");
        let hex = text.strip_prefix('#').ok_or(WRONG)?;
        if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(WRONG);
        }
        let digits: Vec<u8> = hex
            .chars()
            .filter_map(|c| c.to_digit(16))
            .filter_map(|digit| u8::try_from(digit).ok())
            .collect();
        let channels = match digits.as_slice() {
            [r, g, b] => [r * 17, g * 17, b * 17],
            [r1, r2, g1, g2, b1, b2] => [r1 * 16 + r2, g1 * 16 + g2, b1 * 16 + b2],
            _ => return Err(WRONG),
        };
        let [red, green, blue] = channels;
        Ok(Self::rgb(red, green, blue))
    }
}

impl FontStack {
    /// A list this crate wrote itself, for the defaults.
    pub(crate) fn trusted(text: &str) -> Self {
        Self(text.to_owned())
    }

    /// The list as written.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for FontStack {
    type Error = ParseTokenError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        let allowed = |c: char| c.is_alphanumeric() || matches!(c, ' ' | ',' | '-' | '"');
        let balanced = text.matches('"').count().is_multiple_of(2);
        if text.trim().is_empty() || text.len() > 300 || !balanced || !text.chars().all(allowed) {
            return Err(ParseTokenError(
                "a font list may hold only names, commas, spaces, hyphens and paired double quotes",
            ));
        }
        Ok(Self(text))
    }
}

impl Length {
    /// A length this crate wrote itself, for the defaults.
    pub(crate) fn trusted(text: &str) -> Self {
        Self(text.to_owned())
    }

    /// The length as written.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Length {
    type Error = ParseTokenError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        let number = ["px", "rem", "em", "ch"]
            .into_iter()
            .find_map(|unit| text.strip_suffix(unit));
        let valid = number.is_some_and(|number| {
            !number.is_empty()
                && number.matches('.').count() <= 1
                && number.chars().all(|c| c.is_ascii_digit() || c == '.')
                && number.chars().any(|c| c.is_ascii_digit())
        });
        if valid {
            Ok(Self(text))
        } else {
            Err(ParseTokenError(
                "a length must be a number and px, rem, em or ch, such as 0.375rem",
            ))
        }
    }
}
