//! Drawing a card: an SVG laid out here, then shaped and drawn by resvg.

use std::fmt::Write as _;
use std::iter;
use std::sync::{Arc, OnceLock};

use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{self, fontdb};

use crate::{Direction, ShareCard, ShareImageError};

/// The image's width in pixels, the size Open Graph and X cards expect.
pub const WIDTH: u32 = 1200;

/// The image's height in pixels.
pub const HEIGHT: u32 = 630;

// The layout in pixels: text sits inside the margins, the title below the
// accent bar, the site name at the foot under a short accent rule. A title
// longer than MAX_LINES ends its last line with an ellipsis.
const WIDTH_PX: f32 = 1200.0;
const MARGIN: f32 = 80.0;
const RIGHT: f32 = WIDTH_PX - MARGIN;
const TEXT_WIDTH: f32 = RIGHT - MARGIN;
const TITLE_SIZE: f32 = 64.0;
const TITLE_LEADING: f32 = 80.0;
const TITLE_TOP: f32 = 176.0;
const MAX_LINES: usize = 3;
const SITE_SIZE: f32 = 34.0;
const SITE_BASELINE: f32 = 550.0;
const RULE_WIDTH: f32 = 96.0;
const RULE_TOP: f32 = 478.0;
/// The one typeface, embedded so every machine draws the same pixels.
const FONT: &str = "DejaVu Sans";

/// Draws `card` as a [`WIDTH`] by [`HEIGHT`] PNG.
///
/// The title wraps at spaces, nothing runs off the card, and Arabic letters
/// are shaped and set from the right. Drawing takes some
/// CPU, so async callers run it in `spawn_blocking` (RULES.md BL-RUN-7).
///
/// # Errors
///
/// Returns [`ShareImageError::Layout`] when the laid-out SVG cannot be read,
/// [`ShareImageError::Canvas`] when there is no memory for the canvas, and
/// [`ShareImageError::Encode`] when PNG encoding fails.
pub fn render(card: &ShareCard) -> Result<Vec<u8>, ShareImageError> {
    let options = usvg::Options {
        fontdb: fonts(),
        font_family: FONT.to_owned(),
        ..usvg::Options::default()
    };
    let lines = wrap(&card.title, &options)?;
    let site_name = shorten(&card.site_name, SITE_SIZE, &options)?;
    let svg = card_svg(card, &lines, &site_name);
    let tree = parse(&svg, &options)?;
    let mut pixmap = Pixmap::new(WIDTH, HEIGHT).ok_or(ShareImageError::Canvas)?;
    resvg::render(&tree, Transform::default(), &mut pixmap.as_mut());
    pixmap
        .encode_png()
        .map_err(|source| ShareImageError::Encode { source })
}

/// The embedded font, loaded once. No system font is ever read.
fn fonts() -> Arc<fontdb::Database> {
    static FONTS: OnceLock<Arc<fontdb::Database>> = OnceLock::new();
    Arc::clone(FONTS.get_or_init(|| {
        let mut fonts = fontdb::Database::new();
        fonts.load_font_data(dejavu::sans::bold().to_vec());
        Arc::new(fonts)
    }))
}

/// The whole card as SVG, with the layout mirrored for right to left.
fn card_svg(card: &ShareCard, lines: &[String], site_name: &str) -> String {
    let (background, text, accent) = (card.colors.background, card.colors.text, card.colors.accent);
    // usvg anchors `start` at the left edge whatever the direction, so right
    // to left text is anchored at its `end`, on the right margin.
    let (x, rule_x, direction, anchor) = match card.direction {
        Direction::LeftToRight => (MARGIN, MARGIN, "ltr", "start"),
        Direction::RightToLeft => (RIGHT, RIGHT - RULE_WIDTH, "rtl", "end"),
    };
    let mut svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{WIDTH}" height="{HEIGHT}"><rect width="100%" height="100%" fill="{background}"/><rect width="100%" height="16" fill="{accent}"/><rect x="{rule_x}" y="{RULE_TOP}" width="{RULE_WIDTH}" height="6" fill="{accent}"/>"#
    );
    let baselines = iter::successors(Some(TITLE_TOP), |y| Some(y + TITLE_LEADING));
    let lines = lines
        .iter()
        .map(|line| (line.as_str(), TITLE_SIZE))
        .zip(baselines)
        .chain(iter::once(((site_name, SITE_SIZE), SITE_BASELINE)));
    for ((line, size), y) in lines {
        let _ = write!(
            svg,
            r#"<text x="{x}" y="{y}" font-size="{size}" font-weight="bold" fill="{text}" direction="{direction}" text-anchor="{anchor}">{}</text>"#,
            escape(line)
        );
    }
    svg.push_str("</svg>");
    svg
}

/// The title as at most [`MAX_LINES`] lines, wrapped at spaces. A line that
/// is still too wide, one long word, is cut with an ellipsis like the last.
fn wrap(title: &str, options: &usvg::Options<'_>) -> Result<Vec<String>, ShareImageError> {
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in title.split_whitespace() {
        let joined = format!("{line} {word}");
        if line.is_empty() {
            word.clone_into(&mut line);
        } else if width(&joined, TITLE_SIZE, options)? <= TEXT_WIDTH {
            line = joined;
        } else {
            lines.push(std::mem::replace(&mut line, word.to_owned()));
        }
    }
    lines.push(line);
    lines.retain(|line| !line.is_empty());
    if lines.len() > MAX_LINES {
        lines.truncate(MAX_LINES);
        if let Some(last) = lines.last_mut() {
            last.push('\u{2026}');
        }
    }
    lines
        .iter()
        .map(|line| shorten(line, TITLE_SIZE, options))
        .collect()
}

/// `text` if it fits one line at `size`, or cut and ended with an ellipsis.
fn shorten(text: &str, size: f32, options: &usvg::Options<'_>) -> Result<String, ShareImageError> {
    if width(text, size, options)? <= TEXT_WIDTH {
        return Ok(text.to_owned());
    }
    let mut kept: String = text.trim_end_matches('\u{2026}').to_owned();
    while kept.pop().is_some() {
        let candidate = format!("{}\u{2026}", kept.trim_end());
        if width(&candidate, size, options)? <= TEXT_WIDTH {
            return Ok(candidate);
        }
    }
    Ok(String::new())
}

/// How wide `text` is drawn at `size`, after shaping.
fn width(text: &str, size: f32, options: &usvg::Options<'_>) -> Result<f32, ShareImageError> {
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{WIDTH}" height="{HEIGHT}"><text y="{HEIGHT}" font-size="{size}" font-weight="bold">{}</text></svg>"#,
        escape(text)
    );
    Ok(parse(&svg, options)?.root().abs_bounding_box().width())
}

fn parse(svg: &str, options: &usvg::Options<'_>) -> Result<usvg::Tree, ShareImageError> {
    usvg::Tree::from_str(svg, options).map_err(|source| ShareImageError::Layout { source })
}

/// `text` made safe as XML element content: `&`, `<` and `>` escaped, and
/// control characters, which XML cannot hold, left out.
fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars().filter(|c| !c.is_control()) {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            c => escaped.push(c),
        }
    }
    escaped
}
