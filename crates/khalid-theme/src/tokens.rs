//! `theme/tokens.toml`: the synced theme in the shape the renderer loads.

use serde::{Deserialize, Serialize};

use crate::token_values::{FontStack, Length, hex};
use crate::{Rgb, TokensError};

/// The comment at the top of every file this crate writes.
const HEADER: &str = "# The blog's colours, fonts and shape, read by the templates as CSS
# variables. `khalid theme sync` writes this file from the main site.
# Every value may be edited by hand.

";

/// The theme tokens: every field the renderer reads from
/// `theme/tokens.toml`, and no other.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct ThemeTokens {
    /// The colours.
    pub colors: Colors,
    /// The font stacks.
    pub fonts: Fonts,
    /// Corners and line length.
    pub shape: Shape,
}

/// The colours, each written `#rrggbb`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct Colors {
    /// The page background.
    #[serde(with = "hex")]
    pub background: Rgb,
    /// Cards and code blocks, a shade off the background.
    #[serde(with = "hex")]
    pub surface: Rgb,
    /// Body text.
    #[serde(with = "hex")]
    pub text: Rgb,
    /// Dates, captions and other quiet text.
    #[serde(with = "hex")]
    pub muted: Rgb,
    /// Links.
    #[serde(with = "hex")]
    pub link: Rgb,
    /// Buttons and highlights.
    #[serde(with = "hex")]
    pub accent: Rgb,
    /// Lines between parts of the page.
    #[serde(with = "hex")]
    pub border: Rgb,
}

/// The font stacks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct Fonts {
    /// Body text.
    pub body: FontStack,
    /// Headings.
    pub heading: FontStack,
    /// Code.
    pub mono: FontStack,
}

/// Corners and line length.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct Shape {
    /// How round corners are.
    pub radius: Length,
    /// The widest a line of text may run.
    pub measure: Length,
}

impl Default for ThemeTokens {
    /// The renderer's light defaults, value for value.
    fn default() -> Self {
        let sans =
            r#"system-ui, -apple-system, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif"#;
        Self {
            colors: Colors {
                background: Rgb::new(0xff, 0xff, 0xff),
                surface: Rgb::new(0xf6, 0xf7, 0xf9),
                text: Rgb::new(0x1f, 0x23, 0x28),
                muted: Rgb::new(0x57, 0x60, 0x6a),
                link: Rgb::new(0x0b, 0x57, 0xd0),
                accent: Rgb::new(0x0b, 0x57, 0xd0),
                border: Rgb::new(0xd0, 0xd7, 0xde),
            },
            fonts: Fonts {
                body: FontStack::trusted(sans),
                heading: FontStack::trusted(sans),
                mono: FontStack::trusted(
                    "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace",
                ),
            },
            shape: Shape {
                radius: Length::trusted("0.375rem"),
                measure: Length::trusted("68ch"),
            },
        }
    }
}

impl ThemeTokens {
    /// Reads tokens from the text of a `tokens.toml` file, with the
    /// renderer's rules: no unknown field, and every value checked.
    ///
    /// # Errors
    ///
    /// Returns [`TokensError::Parse`] with the line of the first mistake.
    pub fn from_toml(text: &str) -> Result<Self, TokensError> {
        toml::from_str(text).map_err(|source: toml::de::Error| {
            let line = source.span().map(|span| {
                let before = text.get(..span.start).unwrap_or_default();
                before.matches('\n').count().saturating_add(1)
            });
            TokensError::Parse { line, source }
        })
    }

    /// The tokens as the text of `theme/tokens.toml`, with a header that
    /// says what the file is.
    pub fn to_toml(&self) -> String {
        let body = toml::to_string(self).unwrap_or_default();
        format!("{HEADER}{body}")
    }
}
