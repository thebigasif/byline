//! The name of one token in `theme/tokens.toml`, as the `locked` list
//! writes it.

use std::fmt;

use serde::{Deserialize, Serialize};

/// One token, named `table.field`, such as `colors.link`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "&'static str")]
#[non_exhaustive]
pub enum TokenName {
    /// `colors.background`, the page background.
    Background,
    /// `colors.surface`, cards and code blocks.
    Surface,
    /// `colors.text`, body text.
    Text,
    /// `colors.muted`, dates, captions and other quiet text.
    Muted,
    /// `colors.link`, links.
    Link,
    /// `colors.accent`, buttons and highlights.
    Accent,
    /// `colors.border`, lines between parts of the page.
    Border,
    /// `fonts.body`, the body font.
    Body,
    /// `fonts.heading`, the heading font.
    Heading,
    /// `fonts.mono`, the code font.
    Mono,
    /// `shape.radius`, how round corners are.
    Radius,
    /// `shape.measure`, the widest a line of text may run.
    Measure,
    /// `brand.logo`, the main site's logo.
    Logo,
    /// `brand.icon`, the icon browsers show in a tab.
    Icon,
}

impl TokenName {
    /// Every token, in the order of the file.
    pub const ALL: [Self; 14] = [
        Self::Background,
        Self::Surface,
        Self::Text,
        Self::Muted,
        Self::Link,
        Self::Accent,
        Self::Border,
        Self::Body,
        Self::Heading,
        Self::Mono,
        Self::Radius,
        Self::Measure,
        Self::Logo,
        Self::Icon,
    ];

    /// The name as the file writes it, such as `colors.link`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Background => "colors.background",
            Self::Surface => "colors.surface",
            Self::Text => "colors.text",
            Self::Muted => "colors.muted",
            Self::Link => "colors.link",
            Self::Accent => "colors.accent",
            Self::Border => "colors.border",
            Self::Body => "fonts.body",
            Self::Heading => "fonts.heading",
            Self::Mono => "fonts.mono",
            Self::Radius => "shape.radius",
            Self::Measure => "shape.measure",
            Self::Logo => "brand.logo",
            Self::Icon => "brand.icon",
        }
    }
}

impl fmt::Display for TokenName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl TryFrom<String> for TokenName {
    type Error = &'static str;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        Self::ALL
            .into_iter()
            .find(|name| name.as_str() == text)
            .ok_or("a locked name must be a token in this file, such as colors.link")
    }
}

impl From<TokenName> for &'static str {
    fn from(name: TokenName) -> Self {
        name.as_str()
    }
}
