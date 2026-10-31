//! Syncing again: the site's new values, except the ones a person locked,
//! and a list of what changed.

use std::fmt;

use khalid_core::AbsoluteUrl;

use crate::{ThemeTokens, TokenName};

/// One token whose value in the file differs from the site's.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TokenChange {
    /// The token took the site's value.
    Updated {
        /// The token.
        name: TokenName,
        /// The value the file had.
        before: String,
        /// The value it has now.
        after: String,
    },
    /// The token is in the `locked` list, so it kept its value.
    Kept {
        /// The token.
        name: TokenName,
        /// The value it kept.
        kept: String,
        /// The value the site uses now.
        site: String,
    },
}

impl fmt::Display for TokenChange {
    /// The change as one sentence, or two for a kept value.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Updated {
                name,
                before,
                after,
            } => write!(f, "{name} changed from {before} to {after}."),
            Self::Kept { name, kept, site } => write!(
                f,
                "{name} stays {kept}, because it is locked. The site now uses {site}."
            ),
        }
    }
}

/// What a sync changed in `theme/tokens.toml`, in the order of the file.
/// Its [`Display`](fmt::Display) prints one line per change.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[must_use = "tell the person running the sync what changed"]
pub struct ThemeDiff {
    changes: Vec<TokenChange>,
}

impl ThemeDiff {
    /// Each token that differed, in the order of the file.
    pub fn changes(&self) -> &[TokenChange] {
        &self.changes
    }

    /// Whether the site and the file already agreed on every token.
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }
}

impl fmt::Display for ThemeDiff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.changes.is_empty() {
            return writeln!(f, "Nothing changed since the last sync.");
        }
        for change in &self.changes {
            writeln!(f, "{change}")?;
        }
        Ok(())
    }
}

impl ThemeTokens {
    /// Takes the values in `site`, a fresh sync of the main site, for every
    /// token not named in [`ThemeTokens::locked`], and returns what changed.
    ///
    /// `self` is the file as it is now. A value edited by hand and not
    /// locked takes the site's value, and the diff says so. The `locked`
    /// list itself is kept.
    pub fn resync(&mut self, site: &Self) -> ThemeDiff {
        let mut changes = Vec::new();
        for name in TokenName::ALL {
            let (mine, theirs) = (self.value(name), site.value(name));
            if mine == theirs {
                continue;
            }
            if self.locked.contains(&name) {
                changes.push(TokenChange::Kept {
                    name,
                    kept: mine,
                    site: theirs,
                });
            } else {
                self.take(name, site);
                changes.push(TokenChange::Updated {
                    name,
                    before: mine,
                    after: theirs,
                });
            }
        }
        ThemeDiff { changes }
    }

    /// The value of one token as the file writes it, such as `#0b57d0`.
    pub fn value(&self, name: TokenName) -> String {
        let (colors, fonts, shape) = (&self.colors, &self.fonts, &self.shape);
        match name {
            TokenName::Background => colors.background.to_hex(),
            TokenName::Surface => colors.surface.to_hex(),
            TokenName::Text => colors.text.to_hex(),
            TokenName::Muted => colors.muted.to_hex(),
            TokenName::Link => colors.link.to_hex(),
            TokenName::Accent => colors.accent.to_hex(),
            TokenName::Border => colors.border.to_hex(),
            TokenName::Body => fonts.body.as_str().to_owned(),
            TokenName::Heading => fonts.heading.as_str().to_owned(),
            TokenName::Mono => fonts.mono.as_str().to_owned(),
            TokenName::Radius => shape.radius.as_str().to_owned(),
            TokenName::Measure => shape.measure.as_str().to_owned(),
            TokenName::Logo => address(self.brand.logo.as_ref()),
            TokenName::Icon => address(self.brand.icon.as_ref()),
        }
    }

    /// Sets one token to its value in `other`.
    fn take(&mut self, name: TokenName, other: &Self) {
        let (colors, theirs) = (&mut self.colors, &other.colors);
        match name {
            TokenName::Background => colors.background = theirs.background,
            TokenName::Surface => colors.surface = theirs.surface,
            TokenName::Text => colors.text = theirs.text,
            TokenName::Muted => colors.muted = theirs.muted,
            TokenName::Link => colors.link = theirs.link,
            TokenName::Accent => colors.accent = theirs.accent,
            TokenName::Border => colors.border = theirs.border,
            TokenName::Body => self.fonts.body.clone_from(&other.fonts.body),
            TokenName::Heading => self.fonts.heading.clone_from(&other.fonts.heading),
            TokenName::Mono => self.fonts.mono.clone_from(&other.fonts.mono),
            TokenName::Radius => self.shape.radius.clone_from(&other.shape.radius),
            TokenName::Measure => self.shape.measure.clone_from(&other.shape.measure),
            TokenName::Logo => self.brand.logo.clone_from(&other.brand.logo),
            TokenName::Icon => self.brand.icon.clone_from(&other.brand.icon),
        }
    }
}

/// A logo or icon address as the diff shows it, or `none` when there is none.
fn address(url: Option<&AbsoluteUrl>) -> String {
    url.map_or_else(|| "none".to_owned(), ToString::to_string)
}
