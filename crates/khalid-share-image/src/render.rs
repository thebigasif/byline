//! Drawing a card: the title and site name shaped with `rustybuzz` from the
//! embedded face and drawn with `tiny-skia`, no SVG tree in between.

use std::io::Read as _;
use std::sync::OnceLock;

use flate2::read::GzDecoder;
use rustybuzz::{Direction as ShapedDirection, UnicodeBuffer};
use tiny_skia::{FillRule, Paint, Path, PathBuilder, Pixmap, Transform};
use ttf_parser::{Face as TtfFace, GlyphId};

use crate::{Direction, ShareCard, ShareImageError};

/// The image's width in pixels, the size Open Graph and X cards expect.
pub const WIDTH: u32 = 1200;

/// The image's height in pixels.
pub const HEIGHT: u32 = 630;

// The layout in pixels: text sits inside the margins, the title below the
// accent bar, the site name at the foot under a short accent rule. A title
// longer than MAX_LINES ends its last line with an ellipsis.
const MARGIN: f32 = 80.0;
const RIGHT: f32 = 1120.0;
const TEXT_WIDTH: f32 = RIGHT - MARGIN;
const TITLE_SIZE: f32 = 64.0;
const TITLE_LEADING: f32 = 80.0;
const TITLE_TOP: f32 = 176.0;
const MAX_LINES: usize = 3;
const SITE_SIZE: f32 = 34.0;
const SITE_BASELINE: f32 = 550.0;
const RULE_WIDTH: f32 = 96.0;
const RULE_TOP: f32 = 478.0;

/// Draws `card` as a [`WIDTH`] by [`HEIGHT`] PNG.
///
/// The title wraps at spaces, nothing runs off the card, and Arabic letters
/// are shaped and set from the right. Drawing takes some CPU, so async
/// callers run it in `spawn_blocking` (RULES.md BL-RUN-7).
///
/// # Errors
///
/// Returns [`ShareImageError::Canvas`] when there is no memory for the
/// canvas, and [`ShareImageError::Encode`] when PNG encoding fails.
pub fn render(card: &ShareCard) -> Result<Vec<u8>, ShareImageError> {
    let mut pixmap = Pixmap::new(WIDTH, HEIGHT).ok_or(ShareImageError::Canvas)?;
    let (background, text, accent) = (card.colors.background, card.colors.text, card.colors.accent);
    pixmap.fill(tiny_skia::Color::from_rgba8(
        background.0[0],
        background.0[1],
        background.0[2],
        255,
    ));
    let rule_x = match card.direction {
        Direction::LeftToRight => MARGIN,
        Direction::RightToLeft => RIGHT - RULE_WIDTH,
    };
    bar(&mut pixmap, accent, 0.0, 16.0);
    rule(&mut pixmap, accent, rule_x, RULE_TOP, RULE_WIDTH);

    if let Some(face) = face() {
        let title = wrap(&card.title, face);
        let site_name = shorten(&card.site_name, SITE_SIZE, face);
        let (anchor_x, from_right) = match card.direction {
            Direction::LeftToRight => (MARGIN, false),
            Direction::RightToLeft => (RIGHT, true),
        };
        let pen = Pen {
            color: text,
            anchor_x,
            from_right,
        };
        for (index, line) in title.iter().enumerate() {
            #[expect(
                clippy::as_conversions,
                clippy::cast_precision_loss,
                reason = "the line index is at most three"
            )]
            let y = TITLE_TOP + index as f32 * TITLE_LEADING;
            draw_line(&mut pixmap, line, TITLE_SIZE, y, &pen, face);
        }
        draw_line(
            &mut pixmap,
            &site_name,
            SITE_SIZE,
            SITE_BASELINE,
            &pen,
            face,
        );
    }
    pixmap.encode_png().map_err(|_| ShareImageError::Encode)
}

/// One filled band across the card, from `top` to `bottom`.
fn bar(pixmap: &mut Pixmap, color: crate::Rgb, top: f32, bottom: f32) {
    let mut paint = Paint::default();
    paint.set_color_rgba8(color.0[0], color.0[1], color.0[2], 255);
    let Some(rect) = tiny_skia::Rect::from_xywh(0.0, top, 1200.0, bottom - top) else {
        return;
    };
    let band = PathBuilder::from_rect(rect);
    pixmap.fill_path(&band, &paint, FillRule::Winding, Transform::default(), None);
}

/// The short accent rule above the site name, `width` wide from `left`.
fn rule(pixmap: &mut Pixmap, color: crate::Rgb, left: f32, top: f32, width: f32) {
    let mut paint = Paint::default();
    paint.set_color_rgba8(color.0[0], color.0[1], color.0[2], 255);
    let Some(rect) = tiny_skia::Rect::from_xywh(left, top, width, 6.0) else {
        return;
    };
    let band = PathBuilder::from_rect(rect);
    pixmap.fill_path(&band, &paint, FillRule::Winding, Transform::default(), None);
}

/// The embedded face, parsed once and kept for the process. `None` when its
/// bytes cannot be read, in which case the card shows shapes and no text.
fn face() -> Option<&'static TtfFace<'static>> {
    static FACE: OnceLock<Option<TtfFace<'static>>> = OnceLock::new();
    FACE.get_or_init(|| {
        let bytes = bold().ok()?;
        let bytes: &'static [u8] = Box::leak(bytes.into_boxed_slice());
        TtfFace::parse(bytes, 0).ok()
    })
    .as_ref()
}

/// The `DejaVu` Sans Bold face, decompressed from the crate's assets on the
/// first draw. The same face the `dejavu` crate carries, one file only.
fn bold() -> std::io::Result<Vec<u8>> {
    const GZIPPED: &[u8] = include_bytes!("../assets/DejaVuSans-Bold.ttf.gz");
    let mut face = Vec::with_capacity(GZIPPED.len() * 2);
    GzDecoder::new(GZIPPED).read_to_end(&mut face)?;
    Ok(face)
}

/// The title as at most [`MAX_LINES`] lines, wrapped at spaces. A line that
/// is still too wide, one long word, is cut with an ellipsis like the last.
fn wrap(title: &str, face: &TtfFace<'_>) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in title.split_whitespace() {
        let joined = format!("{line} {word}");
        if line.is_empty() {
            word.clone_into(&mut line);
        } else if width(&joined, TITLE_SIZE, face) <= TEXT_WIDTH {
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
        .map(|line| shorten(line, TITLE_SIZE, face))
        .collect()
}

/// `text` if it fits one line at `size`, or cut and ended with an ellipsis.
fn shorten(text: &str, size: f32, face: &TtfFace<'_>) -> String {
    if width(text, size, face) <= TEXT_WIDTH {
        return text.to_owned();
    }
    let mut kept: String = text.trim_end_matches('\u{2026}').to_owned();
    while kept.pop().is_some() {
        let candidate = format!("{}\u{2026}", kept.trim_end());
        if width(&candidate, size, face) <= TEXT_WIDTH {
            return candidate;
        }
    }
    String::new()
}

/// One shaped run: the glyph, its advance and its rise, in pixels.
struct Shaped {
    glyph: GlyphId,
    advance: f32,
    rise: f32,
}

/// `text` shaped at `size`. Each run takes its direction from its first
/// strong letter, so an Arabic title runs right to left while the Latin
/// site name under it keeps its letter order (Unicode bidirectional).
fn shape(text: &str, size: f32, face: &TtfFace<'_>) -> Vec<Shaped> {
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.set_direction(if runs_right_to_left(text) {
        ShapedDirection::RightToLeft
    } else {
        ShapedDirection::LeftToRight
    });
    let shaped = rustybuzz::Face::from_face(face.clone());
    let glyphs = rustybuzz::shape(&shaped, &[], buffer);
    let scale = size / f32::from(face.units_per_em());
    #[expect(
        clippy::as_conversions,
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        reason = "a shaped run holds at most a title's glyphs, so every value fits"
    )]
    let shaped = glyphs
        .glyph_infos()
        .iter()
        .zip(glyphs.glyph_positions())
        .map(|(info, position)| Shaped {
            glyph: GlyphId(info.glyph_id as u16),
            advance: position.x_advance as f32 * scale,
            rise: position.y_offset as f32 * scale,
        })
        .collect::<Vec<_>>();
    shaped
}

/// Whether `text` starts with a right to left letter, which says how the
/// whole run is set. A neutral start, punctuation or digits, takes the
/// first strong letter anywhere in the text.
fn runs_right_to_left(text: &str) -> bool {
    let strong = text.chars().find(|c| {
        let point = u32::from(*c);
        (0x0590..=0x08FF).contains(&point)
            || (0xFB1D..=0xFDFD).contains(&point)
            || (0xFE70..=0xFEFC).contains(&point)
    });
    strong.is_some()
}

/// How wide `text` is when shaped at `size`.
fn width(text: &str, size: f32, face: &TtfFace<'_>) -> f32 {
    shape(text, size, face)
        .iter()
        .map(|shaped| shaped.advance)
        .sum()
}

/// Where a line starts and in which colour: the anchor is the left margin
/// or, for a right to left run, the right one.
struct Pen {
    color: crate::Rgb,
    anchor_x: f32,
    from_right: bool,
}

/// Draws one line: every glyph outlined from the face and filled, from the
/// anchor, leftward when `from_right`.
fn draw_line(
    pixmap: &mut Pixmap,
    text: &str,
    size: f32,
    baseline: f32,
    pen: &Pen,
    face: &TtfFace<'_>,
) {
    let (color, anchor_x, from_right) = (pen.color, pen.anchor_x, pen.from_right);
    let shaped = shape(text, size, face);
    let advance: f32 = shaped.iter().map(|glyph| glyph.advance).sum();
    let mut x = if from_right {
        anchor_x - advance
    } else {
        anchor_x
    };
    let mut paint = Paint::default();
    paint.set_color_rgba8(color.0[0], color.0[1], color.0[2], 255);
    let scale = size / f32::from(face.units_per_em());
    for glyph in &shaped {
        if let Some(path) = outline(glyph.glyph, x, baseline - glyph.rise, scale, face) {
            pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::default(), None);
        }
        x += glyph.advance;
    }
}

/// One glyph as a filled path at (`x`, `baseline`), scaled by `scale`.
fn outline(glyph: GlyphId, x: f32, baseline: f32, scale: f32, face: &TtfFace<'_>) -> Option<Path> {
    let mut pen = GlyphPen {
        builder: PathBuilder::new(),
        x,
        baseline,
        scale,
    };
    face.outline_glyph(glyph, &mut pen)?;
    pen.builder.finish()
}

/// Writes one glyph's outline into a path, flipping the y axis, because a
/// font measures up from the baseline and a pixmap measures down from the
/// top.
struct GlyphPen {
    builder: PathBuilder,
    x: f32,
    baseline: f32,
    scale: f32,
}

impl ttf_parser::OutlineBuilder for GlyphPen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.builder
            .move_to(self.x + x * self.scale, self.baseline - y * self.scale);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.builder
            .line_to(self.x + x * self.scale, self.baseline - y * self.scale);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.builder.quad_to(
            self.x + x1 * self.scale,
            self.baseline - y1 * self.scale,
            self.x + x * self.scale,
            self.baseline - y * self.scale,
        );
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.builder.cubic_to(
            self.x + x1 * self.scale,
            self.baseline - y1 * self.scale,
            self.x + x2 * self.scale,
            self.baseline - y2 * self.scale,
            self.x + x * self.scale,
            self.baseline - y * self.scale,
        );
    }

    fn close(&mut self) {
        self.builder.close();
    }
}
