//! The font files `theme sync` downloads and saves under `theme/fonts/`,
//! with the metrics of each face's tuned local fallback (G20). Kept apart
//! from `fonts.rs`, which finds the fonts, so each file stays under 600
//! lines (BL-SIZE-1).

use khalid_core::AbsoluteUrl;
use khalid_outbound::Outbound;
use read_fonts::TableProvider;

use crate::fonts::FontSource;

/// The most bytes one downloaded font file may hold before it is skipped.
/// A web font subset is tens of kilobytes; four megabytes leaves room for
/// whole families while keeping a hostile file from being read far.
pub const MAX_FONT_BYTES: usize = 4 * 1024 * 1024;

/// A self-hosted font file `theme sync` saved, as the renderer's tokens
/// carry it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedFont {
    /// The family the file gives a face, such as `Inter`.
    pub family: String,
    /// `normal` or `italic`.
    pub style: String,
    /// The weight the file carries, such as `400`.
    pub weight: String,
    /// The file name under `theme/fonts/`.
    pub path: String,
    /// The Unicode range the file covers, when the CSS named one.
    pub range: Option<String>,
    /// The face's metric overrides for its tuned local fallback, as CSS
    /// percentages (G20). `None` when the file could not be measured.
    pub metrics: Option<FontMetrics>,
}

/// A face's metrics as its fallback's `@font-face` overrides read them.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FontMetrics {
    /// `ascent-override`, such as `90.44%`.
    pub ascent: String,
    /// `descent-override`, such as `22.52%`.
    pub descent: String,
    /// `line-gap-override`, such as `0%`.
    pub line_gap: String,
    /// `size-adjust`, such as `107.12%`.
    pub size_adjust: String,
}

/// Downloads each rehostable font in `sources` and saves its files under
/// `into`, the site's `theme/fonts/` folder. A font that cannot be read is
/// skipped, never fatal: the blog keeps working with the next font in the
/// stack (G20). Returns what was saved, in the CSS's order.
///
/// # Errors
///
/// Returns an error only when the folder cannot be made; every network
/// miss is a skip.
pub async fn download(
    outbound: &Outbound,
    sources: &[FontSource],
    into: &std::path::Path,
) -> Result<Vec<SavedFont>, crate::FetchError> {
    let (saved, _) = download_and_report(outbound, sources, into).await?;
    Ok(saved)
}

/// As [`download`], also saying in plain words why each font was skipped.
/// A font comes from someone else's website, so nothing about it is
/// trusted: files over [`MAX_FONT_BYTES`] are not read further, and a file
/// that fails to parse as a font face is dropped, never a failure of the
/// sync (G20, maintainer review).
///
/// # Errors
///
/// Returns an error only when the folder cannot be made; every network
/// miss is a skip with its sentence.
pub async fn download_and_report(
    outbound: &Outbound,
    sources: &[FontSource],
    into: &std::path::Path,
) -> Result<(Vec<SavedFont>, Vec<String>), crate::FetchError> {
    let mut saved = Vec::new();
    let mut skipped = Vec::new();
    for source in sources.iter().filter(|source| source.host().can_rehost()) {
        let Ok(url) = AbsoluteUrl::parse(source.url()) else {
            continue;
        };
        let Ok(css) = outbound.get(&url).await else {
            skipped.push(format!(
                "The stylesheet for {} could not be read, so its fonts stay where they are.",
                source.family()
            ));
            continue;
        };
        if !(200..300).contains(&css.status()) {
            let status = css.status();
            skipped.push(format!(
                "The stylesheet for {} answered {status}, so its fonts stay where they are.",
                source.family()
            ));
            continue;
        }
        let Ok(css) = std::str::from_utf8(css.body()) else {
            continue;
        };
        for face in faces(css) {
            let Ok(font) = AbsoluteUrl::parse(&face.url) else {
                continue;
            };
            let Ok(bytes) = outbound.get(&font).await else {
                skipped.push(format!(
                    "The file for {} {} could not be read, so that face stays where it is.",
                    face.family, face.weight
                ));
                continue;
            };
            if !(200..300).contains(&bytes.status()) || bytes.body().is_empty() {
                let status = bytes.status();
                skipped.push(format!(
                    "The file for {} {} answered {status}, so that face stays where it is.",
                    face.family, face.weight
                ));
                continue;
            }
            if bytes.body().len() > MAX_FONT_BYTES {
                skipped.push(format!(
                    "The file for {} {} is {} KB, over the {} KB a font may be, so it stays where it is.",
                    face.family,
                    face.weight,
                    bytes.body().len() / 1024,
                    MAX_FONT_BYTES / 1024
                ));
                continue;
            }
            if write_face(into, &face, bytes.body()).is_err() {
                skipped.push(format!(
                    "The file for {} {} could not be saved beside the theme, so that face is left out.",
                    face.family, face.weight
                ));
                continue;
            }
            let metrics = measure(bytes.body());
            saved.push(SavedFont {
                family: face.family,
                style: face.style,
                weight: face.weight,
                path: face.file,
                range: face.range,
                metrics,
            });
        }
    }
    Ok((saved, skipped))
}

/// The face's metric overrides for a local Arial fallback, as CSS
/// percentages. `None` when the file is not a readable face. The formula
/// is the one Capsize popularised: each override is the face's own value
/// over its size, divided by Arial's, as a percentage of Arial's.
fn measure(bytes: &[u8]) -> Option<FontMetrics> {
    // Arial's metrics, in font units of 2048 per em: ascent 1854,
    // descent 434, line gap 67, average character width 1934.
    const ARIAL_ASCENT: f64 = 1854.0;
    const ARIAL_DESCENT: f64 = 434.0;
    const ARIAL_LINE_GAP: f64 = 67.0;
    const ARIAL_WIDTH: f64 = 1934.0;
    // The tables are read through read-fonts' typed API, so a malformed
    // file is an Err, never a panic or a wrong offset (maintainer review).
    let font = read_fonts::FileRef::new(bytes).ok()?.fonts().next()?.ok()?;
    let head = font.head().ok()?;
    let hhea = font.hhea().ok()?;
    let os2 = font.os2().ok()?;
    let per_em = f64::from(head.units_per_em());
    let ascent = f64::from(hhea.ascender().to_i16()) / per_em;
    let descent = f64::from(hhea.descender().to_i16()).abs() / per_em;
    let line_gap = f64::from(hhea.line_gap().to_i16()) / per_em;
    // The average character width is OS/2's `xAvgCharWidth`. Faces without
    // the field cannot be tuned.
    let width = f64::from(os2.x_avg_char_width()) / per_em;
    let percent = |value: f64, arial: f64| format!("{:.2}%", value / arial * 100.0);
    Some(FontMetrics {
        ascent: percent(ascent, ARIAL_ASCENT / 2048.0),
        descent: percent(descent, ARIAL_DESCENT / 2048.0),
        line_gap: percent(line_gap, ARIAL_LINE_GAP / 2048.0),
        size_adjust: percent(width, ARIAL_WIDTH / 2048.0),
    })
}

/// One `@font-face` rule's parts, with the file name it saves under.
struct Face {
    family: String,
    style: String,
    weight: String,
    url: String,
    file: String,
    range: Option<String>,
}

/// The `@font-face` rules in a Google Fonts stylesheet, in order. The
/// comment before each rule, such as `/* latin */`, names its subset.
fn faces(css: &str) -> Vec<Face> {
    let mut found = Vec::new();
    let mut before = "";
    for part in css.split("@font-face") {
        let rule = part;
        let head = before;
        before = part;
        let Some(rule) = rule.find('}').and_then(|end| rule.get(..end)) else {
            continue;
        };
        let family = value(rule, "font-family").unwrap_or_default();
        if family.is_empty() {
            continue;
        }
        let style = value(rule, "font-style").unwrap_or_else(|| "normal".to_owned());
        let weight = value(rule, "font-weight").unwrap_or_else(|| "400".to_owned());
        let Some(url) = value(rule, "src").and_then(|src| url_of(&src)) else {
            continue;
        };
        let file = file_name(&family, head);
        found.push(Face {
            family,
            style,
            weight,
            url,
            file,
            range: value(rule, "unicode-range"),
        });
    }
    found
}

/// The value of `property` in `rule`, quotes stripped.
fn value(rule: &str, property: &str) -> Option<String> {
    let start = rule.find(property)? + property.len();
    let rest = rule.get(start..)?;
    let colon = rest.find(':')?;
    let end = rest.get(colon..).and_then(|tail| tail.find(';'))? + colon;
    let value = rest
        .get(colon + 1..end)?
        .trim()
        .trim_matches('\'')
        .trim_matches('"');
    Some(value.to_owned())
}

/// The first `url(...)` in `src`.
fn url_of(src: &str) -> Option<String> {
    let start = src.find("url(")? + 4;
    let rest = src.get(start..)?;
    let end = rest.find(')')?;
    Some(
        rest.get(..end)?
            .trim_matches('\'')
            .trim_matches('"')
            .to_owned(),
    )
}

/// The file name a face saves under: the family and, when the comment
/// above the rule names one, the subset, such as `inter-latin.woff2`.
/// The style and weight live in the tokens, not the name.
fn file_name(family: &str, before: &str) -> String {
    let clean = |text: &str| text.to_ascii_lowercase().replace(' ', "-");
    let subset = before
        .lines()
        .rev()
        .find_map(|line| {
            let line = line.trim();
            line.strip_prefix("/*")
                .and_then(|rest| rest.strip_suffix("*/"))
        })
        .map(str::trim)
        .unwrap_or_default();
    let mut name = clean(family);
    if !subset.is_empty() && subset.len() < 24 {
        name.push('-');
        name.push_str(&clean(subset));
    }
    format!("{name}.woff2")
}

/// Writes the bytes of one face, making the folder first. The file name is
/// built here, from the family and subset, never taken from the CSS.
fn write_face(into: &std::path::Path, face: &Face, bytes: &[u8]) -> std::io::Result<()> {
    std::fs::create_dir_all(into)?;
    let mut file = into.to_path_buf();
    file.push(&face.file);
    std::fs::write(file, bytes)
}
