//! `theme/tokens.toml`: the synced theme in the shape the renderer loads.

use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

use khalid_core::{AbsoluteUrl, SitePath};
use serde::{Deserialize, Serialize};

use crate::token_values::{FontStack, Length, address, hex};
use crate::{Rgb, SiteTheme, TokenName, TokensError};

/// Where a site keeps its tokens, as the renderer reads them.
const TOKENS_FILE: &str = "theme/tokens.toml";

/// The comment at the top of every file this crate writes.
const HEADER: &str = "# The blog's colours, fonts and shape, read by the templates as CSS
# variables. `khalid theme sync` writes this file from the main site.
# Every value may be edited by hand. To keep an edit when the site is
# synced again, name it in a list at the top: locked = [\"colors.link\"]

";

/// How far the surface colour moves from the background toward the text.
const SURFACE_STEP: f64 = 0.04;
/// How far the border colour moves from the background toward the text.
const BORDER_STEP: f64 = 0.18;
/// How far quiet text moves from the text colour toward the background.
const MUTED_STEP: f64 = 0.35;

/// The theme tokens: every field the renderer reads from
/// `theme/tokens.toml`, and no other.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct ThemeTokens {
    /// The tokens a person set by hand. A sync keeps each of them as it is.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub locked: Vec<TokenName>,
    /// The colours.
    pub colors: Colors,
    /// The font stacks.
    pub fonts: Fonts,
    /// Corners and line length.
    pub shape: Shape,
    /// The main site's logo and icon. The table is left out of the file
    /// when neither was found.
    #[serde(default, skip_serializing_if = "BrandImages::is_empty")]
    pub brand: BrandImages,
}

/// The main site's logo and icon, as full web addresses.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct BrandImages {
    /// The logo, such as `https://example.com/logo.svg`.
    #[serde(default, with = "address", skip_serializing_if = "Option::is_none")]
    pub logo: Option<AbsoluteUrl>,
    /// The small icon browsers show in a tab.
    #[serde(default, with = "address", skip_serializing_if = "Option::is_none")]
    pub icon: Option<AbsoluteUrl>,
}

impl BrandImages {
    /// Whether neither a logo nor an icon is set.
    pub fn is_empty(&self) -> bool {
        self.logo.is_none() && self.icon.is_none()
    }
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
    /// Font files the site serves itself, downloaded by the sync (G20).
    /// Left out of the file when there are none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<FontFileEntry>,
}

/// One self-hosted font file, as `theme/tokens.toml` writes it and the
/// renderer reads it back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FontFileEntry {
    /// The family the file gives a face, such as `Inter`.
    pub family: String,
    /// `normal` or `italic`.
    pub style: String,
    /// The weight the file carries, such as `400`.
    pub weight: String,
    /// The file name under `theme/fonts/`, such as `inter-latin.woff2`.
    pub path: String,
    /// The Unicode range the file covers, so a browser loads only the
    /// subset a page needs. Left out when the file covers everything.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<String>,
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
            locked: Vec::new(),
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
                files: Vec::new(),
            },
            shape: Shape {
                radius: Length::trusted("0.375rem"),
                measure: Length::trusted("68ch"),
            },
            brand: BrandImages::default(),
        }
    }
}

impl ThemeTokens {
    /// The tokens for a synced `theme`: what it found, and the defaults for
    /// the rest. When the background or text colour was found, the surface,
    /// border and quiet text colours are mixed from the two, so they suit
    /// the site. The accent falls back to the link colour, and the heading
    /// font to the body font. Contrast is not checked here; call
    /// [`ThemeTokens::fix_contrast`] next.
    pub fn from_theme(theme: &SiteTheme) -> Self {
        let mut tokens = Self::default();
        let found = &theme.palette;
        let colors = &mut tokens.colors;
        colors.background = found.background.unwrap_or(colors.background);
        colors.text = found.text.unwrap_or(colors.text);
        if found.background.is_some() || found.text.is_some() {
            colors.surface = colors.background.mix(colors.text, SURFACE_STEP);
            colors.border = colors.background.mix(colors.text, BORDER_STEP);
            colors.muted = colors.text.mix(colors.background, MUTED_STEP);
        }
        colors.link = found.link.unwrap_or(colors.link);
        colors.accent = found.accent.or(found.link).unwrap_or(colors.accent);
        let fonts = &theme.typography;
        let body = fonts.body.as_deref().and_then(FontStack::from_found);
        let heading = fonts.heading.as_deref().and_then(FontStack::from_found);
        if let Some(body) = body.clone() {
            tokens.fonts.body = body;
        }
        if let Some(heading) = heading.or(body) {
            tokens.fonts.heading = heading;
        }
        tokens.brand = BrandImages {
            logo: theme.brand.logo.clone(),
            icon: theme.brand.icons.first().cloned(),
        };
        tokens
    }

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

    /// Reads `theme/tokens.toml` inside `site_root`, or `None` when the site
    /// has no such file yet.
    ///
    /// # Errors
    ///
    /// Returns [`TokensError::Path`] when `theme/` leads outside the site
    /// folder, [`TokensError::Read`] when the file cannot be read, and
    /// [`TokensError::Parse`] for a mistake in it.
    pub fn read(site_root: &Path) -> Result<Option<Self>, TokensError> {
        let path = SitePath::within(site_root, TOKENS_FILE)?.resolve()?;
        match fs::read_to_string(&path) {
            Ok(text) => Self::from_toml(&text).map(Some),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(TokensError::Read { path, source }),
        }
    }

    /// Writes the tokens to `theme/tokens.toml` inside `site_root`, making
    /// the `theme` folder when it is missing, and returns the path.
    ///
    /// The write is atomic (RULES.md BL-RUN-9): a temporary file in the same
    /// folder is written, flushed to disk and renamed over the old file, so
    /// a reader never sees half a file.
    ///
    /// # Errors
    ///
    /// Returns [`TokensError::Path`] when `theme/` leads outside the site
    /// folder through a symlink, and [`TokensError::Write`] when the disk
    /// refuses the write.
    pub fn write(&self, site_root: &Path) -> Result<PathBuf, TokensError> {
        let path = SitePath::within(site_root, TOKENS_FILE)?.resolve()?;
        let write = |source| TokensError::Write {
            path: path.clone(),
            source,
        };
        if let Some(folder) = path.parent() {
            fs::create_dir_all(folder).map_err(write)?;
        }
        let temporary = path.with_file_name(".tokens.toml.tmp");
        let result = (|| {
            let mut file = fs::File::create(&temporary)?;
            file.write_all(self.to_toml().as_bytes())?;
            file.sync_all()?;
            fs::rename(&temporary, &path)
        })();
        if result.is_err() {
            drop(fs::remove_file(&temporary));
        }
        result.map_err(write)?;
        Ok(path)
    }
}
