//! The theme's design tokens from `theme/tokens.toml`, as CSS variables.

use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::Path;

use khalid_core::{AbsoluteUrl, SitePath};
use serde::Deserialize;

use crate::token_values::{Color, FontStack, Length};
use crate::{RenderError, font_fallback};

/// Where a site keeps its tokens.
const TOKENS_FILE: &str = "theme/tokens.toml";

/// WCAG 2.2 AA asks body text for at least this contrast against its background.
const MINIMUM_CONTRAST: f64 = 4.5;

/// The site's colours, fonts, shape and brand. Theme sync writes the file; a person
/// may edit it. Every field has a light default, so a missing file or a
/// missing field still gives a readable site.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Tokens {
    /// The tokens a person set by hand, which `khalid theme sync` keeps
    /// when it reads the main site again. The renderer only checks the
    /// names.
    pub locked: Vec<TokenName>,
    /// The colours.
    pub colors: Colors,
    /// The font stacks.
    pub fonts: Fonts,
    /// Corners and line length.
    pub shape: Shape,
    /// The main site's logo and icon.
    pub brand: Brand,
}

/// The main site's logo and icon, as full web addresses. Theme sync fills
/// them when it finds them; either may be left out.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Brand {
    /// The logo, such as `https://example.com/logo.svg`.
    pub logo: Option<AbsoluteUrl>,
    /// The small icon browsers show in a tab.
    pub icon: Option<AbsoluteUrl>,
}

/// The name of one token, as `table.field`, such as `colors.link`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[non_exhaustive]
pub enum TokenName {
    /// `colors.background`.
    #[serde(rename = "colors.background")]
    Background,
    /// `colors.surface`.
    #[serde(rename = "colors.surface")]
    Surface,
    /// `colors.text`.
    #[serde(rename = "colors.text")]
    Text,
    /// `colors.muted`.
    #[serde(rename = "colors.muted")]
    Muted,
    /// `colors.link`.
    #[serde(rename = "colors.link")]
    Link,
    /// `colors.accent`.
    #[serde(rename = "colors.accent")]
    Accent,
    /// `colors.border`.
    #[serde(rename = "colors.border")]
    Border,
    /// `fonts.body`.
    #[serde(rename = "fonts.body")]
    Body,
    /// `fonts.heading`.
    #[serde(rename = "fonts.heading")]
    Heading,
    /// `fonts.mono`.
    #[serde(rename = "fonts.mono")]
    Mono,
    /// `shape.radius`.
    #[serde(rename = "shape.radius")]
    Radius,
    /// `shape.measure`.
    #[serde(rename = "shape.measure")]
    Measure,
    /// `brand.logo`.
    #[serde(rename = "brand.logo")]
    Logo,
    /// `brand.icon`.
    #[serde(rename = "brand.icon")]
    Icon,
}

/// The colours, each a `#rrggbb` or `#rgb` value.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Colors {
    /// The page background.
    pub background: Color,
    /// Cards and code blocks, a shade off the background.
    pub surface: Color,
    /// Body text.
    pub text: Color,
    /// Dates, captions and other quiet text.
    pub muted: Color,
    /// Links.
    pub link: Color,
    /// Buttons and highlights.
    pub accent: Color,
    /// Lines between parts of the page.
    pub border: Color,
}

/// The font stacks, each a CSS `font-family` list.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Fonts {
    /// Body text.
    pub body: FontStack,
    /// Headings.
    pub heading: FontStack,
    /// Code.
    pub mono: FontStack,
    /// Font files the site serves itself, each with the subset it covers
    /// (G20). `theme sync` writes them; the stylesheet links them.
    pub files: Vec<FontFile>,
}

/// One self-hosted font file, served under `assets/fonts/`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FontFile {
    /// The family the file gives a face, such as `Inter`.
    pub family: String,
    /// `normal` or `italic`.
    pub style: String,
    /// The weight the file carries, such as `400`.
    pub weight: String,
    /// The file name under `theme/fonts/`, such as `inter-latin.woff2`.
    pub path: String,
    /// The Unicode range the file covers, so a browser loads only the
    /// subset a page needs. `None` covers everything.
    pub range: Option<String>,
    /// The face's metric overrides for its tuned local fallback, as CSS
    /// percentages (G20). `None` when the file was not measured.
    pub metrics: Option<Metrics>,
}

/// A face's metrics as its fallback's `@font-face` overrides read them.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Metrics {
    /// `ascent-override`, such as `90.44%`.
    pub ascent: String,
    /// `descent-override`, such as `22.52%`.
    pub descent: String,
    /// `line-gap-override`, such as `0%`.
    pub line_gap: String,
    /// `size-adjust`, such as `107.12%`.
    pub size_adjust: String,
}

/// Corners and line length.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Shape {
    /// How round corners are, such as `0.375rem`.
    pub radius: Length,
    /// The widest a line of text may run, such as `68ch`.
    pub measure: Length,
}

/// A text colour that is too faint on the background for WCAG 2.2 AA.
#[derive(Debug, Clone, PartialEq)]
pub struct ContrastFailure {
    /// The token's name, such as `colors.muted`.
    pub token: &'static str,
    /// The contrast it has, such as `3.2`.
    pub ratio: f64,
}

impl Tokens {
    /// Reads `site_root/theme/tokens.toml`. A missing file gives the defaults.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::Tokens`] with the line for a value that is not
    /// a colour, a font list or a length, or for an unknown field, and
    /// [`RenderError::ReadTokens`] or [`RenderError::ThemePath`] when the
    /// file cannot be read.
    pub fn load(site_root: &Path) -> Result<Self, RenderError> {
        let path = SitePath::within(site_root, TOKENS_FILE)?.resolve()?;
        match fs::read_to_string(&path) {
            Ok(text) => Self::from_toml(&text),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(source) => Err(RenderError::ReadTokens { path, source }),
        }
    }

    /// Reads tokens from the text of a `tokens.toml` file.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::Tokens`] with the line of the first mistake.
    pub fn from_toml(text: &str) -> Result<Self, RenderError> {
        toml::from_str(text).map_err(|source: toml::de::Error| {
            let line = source.span().map(|span| {
                text.get(..span.start)
                    .map_or(0, |before| before.matches('\n').count())
                    .saturating_add(1)
            });
            RenderError::Tokens { line, source }
        })
    }

    /// The tokens as a `:root` rule of CSS variables, such as
    /// `--color-text: #1f2328;`, for the top of the stylesheet. A font stack
    /// that starts with a known web font gains its tuned local fallback,
    /// whose `@font-face` rule follows (RULES.md BL-WEB-5).
    pub fn to_css(&self) -> String {
        let colors = &self.colors;
        let mut families = Vec::new();
        // A self-hosted family with measured metrics gains its fallback in
        // the stack too, beside the file's own @font-face rule.
        let measured: Vec<&str> = self
            .fonts
            .files
            .iter()
            .filter(|file| file.metrics.is_some())
            .map(|file| file.family.as_str())
            .collect();
        let mut font = |stack: &FontStack| {
            let (tuned, family) = font_fallback::tuned_with(stack.as_str(), &measured);
            families.extend(family.filter(|family| !families.contains(family)));
            tuned
        };
        let (body, heading, mono) = (
            font(&self.fonts.body),
            font(&self.fonts.heading),
            font(&self.fonts.mono),
        );
        let mut css = String::from(":root {\n");
        for (name, value) in [
            ("color-background", colors.background.as_str()),
            ("color-surface", colors.surface.as_str()),
            ("color-text", colors.text.as_str()),
            ("color-muted", colors.muted.as_str()),
            ("color-link", colors.link.as_str()),
            ("color-accent", colors.accent.as_str()),
            ("color-border", colors.border.as_str()),
            ("font-body", body.as_str()),
            ("font-heading", heading.as_str()),
            ("font-mono", mono.as_str()),
            ("radius", self.shape.radius.as_str()),
            ("measure", self.shape.measure.as_str()),
        ] {
            let _ = writeln!(css, "  --{name}: {value};");
        }
        css.push_str("}\n");
        for file in &self.fonts.files {
            css.push_str(&font_face(file));
            if let Some(metrics) = &file.metrics {
                css.push_str(&font_fallback::measured_face(&file.family, metrics));
            }
        }
        for family in families {
            css.push_str(&font_fallback::face(family));
        }
        css
    }

    /// Every text colour whose contrast on the background is below what
    /// WCAG 2.2 AA asks of body text (4.5 to 1). Empty means all pass.
    pub fn contrast_failures(&self) -> Vec<ContrastFailure> {
        let colors = &self.colors;
        [
            ("colors.text", &colors.text),
            ("colors.muted", &colors.muted),
            ("colors.link", &colors.link),
        ]
        .into_iter()
        .map(|(token, color)| ContrastFailure {
            token,
            ratio: color.contrast(&colors.background),
        })
        .filter(|failure| failure.ratio < MINIMUM_CONTRAST)
        .collect()
    }
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            background: Color::rgb(0xff, 0xff, 0xff),
            surface: Color::rgb(0xf6, 0xf7, 0xf9),
            text: Color::rgb(0x1f, 0x23, 0x28),
            muted: Color::rgb(0x57, 0x60, 0x6a),
            link: Color::rgb(0x0b, 0x57, 0xd0),
            accent: Color::rgb(0x0b, 0x57, 0xd0),
            border: Color::rgb(0xd0, 0xd7, 0xde),
        }
    }
}

impl Default for Fonts {
    fn default() -> Self {
        let sans =
            r#"system-ui, -apple-system, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif"#;
        Self {
            body: FontStack::trusted(sans),
            heading: FontStack::trusted(sans),
            mono: FontStack::trusted("ui-monospace, SFMono-Regular, Menlo, Consolas, monospace"),
            files: Vec::new(),
        }
    }
}

/// The `@font-face` rule of one self-hosted file, served with a swap so the
/// text shows in the fallback while the file arrives (G20).
fn font_face(file: &FontFile) -> String {
    let mut rule = format!(
        "@font-face {{ font-family: \"{}\"; font-style: {}; font-weight: {}; \
     src: url(\"assets/fonts/{}\") format(\"woff2\"); font-display: swap;",
        file.family, file.style, file.weight, file.path
    );
    if let Some(range) = &file.range {
        rule.push_str(" unicode-range: ");
        rule.push_str(range);
        rule.push(';');
    }
    rule.push_str(" }\n");
    rule
}

impl Default for Shape {
    fn default() -> Self {
        Self {
            radius: Length::trusted("0.375rem"),
            measure: Length::trusted("68ch"),
        }
    }
}
