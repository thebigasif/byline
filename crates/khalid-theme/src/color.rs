//! An sRGB colour read from CSS, and the WCAG contrast between two colours.

use crate::split::split_top;

/// The least alpha a colour needs to count. A colour more than half
/// transparent says little about what a reader sees.
const MIN_ALPHA: f64 = 0.5;

/// The CSS named colours real stylesheets use for text and backgrounds.
const NAMED: [(&str, Rgb); 16] = [
    ("black", Rgb::new(0, 0, 0)),
    ("white", Rgb::new(255, 255, 255)),
    ("gray", Rgb::new(128, 128, 128)),
    ("grey", Rgb::new(128, 128, 128)),
    ("silver", Rgb::new(192, 192, 192)),
    ("red", Rgb::new(255, 0, 0)),
    ("maroon", Rgb::new(128, 0, 0)),
    ("orange", Rgb::new(255, 165, 0)),
    ("green", Rgb::new(0, 128, 0)),
    ("teal", Rgb::new(0, 128, 128)),
    ("blue", Rgb::new(0, 0, 255)),
    ("navy", Rgb::new(0, 0, 128)),
    ("purple", Rgb::new(128, 0, 128)),
    ("rebeccapurple", Rgb::new(102, 51, 153)),
    ("whitesmoke", Rgb::new(245, 245, 245)),
    ("darkslategray", Rgb::new(47, 79, 79)),
];

/// An opaque sRGB colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Rgb {
    red: u8,
    green: u8,
    blue: u8,
}

impl Rgb {
    /// The colour with these channels.
    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }

    /// Reads one CSS colour value: `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`,
    /// `rgb()`, `rgba()`, `hsl()`, `hsla()` or a common named colour.
    /// `None` for anything else, including `transparent`, `var()` and a
    /// colour more than half transparent.
    pub fn from_css(text: &str) -> Option<Self> {
        let text = text.trim().to_ascii_lowercase();
        if let Some(hex) = text.strip_prefix('#') {
            return from_hex(hex);
        }
        if let Some((name, rest)) = text.split_once('(') {
            let numbers: Vec<&str> = rest
                .strip_suffix(')')?
                .split(|c: char| c == ',' || c == '/' || c.is_whitespace())
                .filter(|part| !part.is_empty())
                .collect();
            return match name.trim() {
                "rgb" | "rgba" => from_rgb(&numbers),
                "hsl" | "hsla" => from_hsl(&numbers),
                _ => None,
            };
        }
        NAMED
            .iter()
            .find(|(name, _)| *name == text)
            .map(|&(_, color)| color)
    }

    /// Every colour in a declaration value, such as the `#d0d7de` in
    /// `1px solid #d0d7de`, in the order they appear.
    pub fn all_in(value: &str) -> Vec<Self> {
        split_top(value, |c| c == ',' || c.is_whitespace())
            .into_iter()
            .filter_map(Self::from_css)
            .collect()
    }

    /// The red, green and blue channels.
    pub fn channels(self) -> [u8; 3] {
        [self.red, self.green, self.blue]
    }

    /// The colour as lowercase `#rrggbb`.
    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.red, self.green, self.blue)
    }

    /// Relative luminance as WCAG 2.2 defines it, from 0 for black to 1 for
    /// white.
    pub fn luminance(self) -> f64 {
        let [red, green, blue] = self.channels().map(|channel| {
            let value = f64::from(channel) / 255.0;
            if value <= 0.040_45 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        });
        0.2126 * red + 0.7152 * green + 0.0722 * blue
    }

    /// This colour moved `amount` of the way toward `other`, from 0 for
    /// this colour to 1 for `other`, channel by channel.
    #[must_use]
    pub fn mix(self, other: Self, amount: f64) -> Self {
        let amount = amount.clamp(0.0, 1.0);
        let step = |from: u8, to: u8| {
            let from = f64::from(from);
            channel(from + (f64::from(to) - from) * amount)
        };
        Self::new(
            step(self.red, other.red),
            step(self.green, other.green),
            step(self.blue, other.blue),
        )
    }

    /// Whether the colour is strong enough to read as a brand colour: HSL
    /// saturation of at least 35% and lightness between 15% and 85%. Greys,
    /// near-whites and near-blacks are not.
    pub(crate) fn is_vivid(self) -> bool {
        let hsl = Hsl::from(self);
        hsl.saturation >= 0.35 && (0.15..=0.85).contains(&hsl.lightness)
    }

    /// The WCAG contrast ratio between this colour and `other`, from 1 to 21.
    /// The order does not matter.
    pub fn contrast(self, other: Self) -> f64 {
        let (a, b) = (self.luminance(), other.luminance());
        let (light, dark) = if a > b { (a, b) } else { (b, a) };
        (light + 0.05) / (dark + 0.05)
    }
}

fn from_hex(hex: &str) -> Option<Rgb> {
    let digits: Vec<u8> = hex
        .chars()
        .map(|c| c.to_digit(16).and_then(|d| u8::try_from(d).ok()))
        .collect::<Option<_>>()?;
    let (channels, alpha) = match digits.as_slice() {
        [r, g, b] => ([r * 17, g * 17, b * 17], 255),
        [r, g, b, a] => ([r * 17, g * 17, b * 17], a * 17),
        [r1, r2, g1, g2, b1, b2] => ([r1 * 16 + r2, g1 * 16 + g2, b1 * 16 + b2], 255),
        [r1, r2, g1, g2, b1, b2, a1, a2] => {
            ([r1 * 16 + r2, g1 * 16 + g2, b1 * 16 + b2], a1 * 16 + a2)
        }
        _ => return None,
    };
    let [red, green, blue] = channels;
    (f64::from(alpha) / 255.0 >= MIN_ALPHA).then_some(Rgb::new(red, green, blue))
}

fn from_rgb(parts: &[&str]) -> Option<Rgb> {
    let (channels, alpha) = match parts {
        [r, g, b] => ([r, g, b], None),
        [r, g, b, a] => ([r, g, b], Some(a)),
        _ => return None,
    };
    opaque(alpha.copied())?;
    let [red, green, blue] = channels.map(|part| {
        number(part).map(|(value, percent)| {
            if percent {
                value * 255.0 / 100.0
            } else {
                value
            }
        })
    });
    Some(Rgb::new(channel(red?), channel(green?), channel(blue?)))
}

fn from_hsl(parts: &[&str]) -> Option<Rgb> {
    let (hue, saturation, lightness, alpha) = match parts {
        [h, s, l] => (h, s, l, None),
        [h, s, l, a] => (h, s, l, Some(*a)),
        _ => return None,
    };
    opaque(alpha)?;
    Some(Rgb::from(Hsl {
        hue: number(hue.trim_end_matches("deg"))?.0,
        saturation: number(saturation)?.0 / 100.0,
        lightness: number(lightness)?.0 / 100.0,
    }))
}

/// A colour as hue in degrees, and saturation and lightness from 0 to 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Hsl {
    pub(crate) hue: f64,
    pub(crate) saturation: f64,
    pub(crate) lightness: f64,
}

impl From<Hsl> for Rgb {
    fn from(hsl: Hsl) -> Self {
        let hue = hsl.hue.rem_euclid(360.0) / 60.0;
        let saturation = hsl.saturation.clamp(0.0, 1.0);
        let lightness = hsl.lightness.clamp(0.0, 1.0);
        let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
        let second = chroma * (1.0 - (hue.rem_euclid(2.0) - 1.0).abs());
        let (r, g, b) = match hue {
            h if h < 1.0 => (chroma, second, 0.0),
            h if h < 2.0 => (second, chroma, 0.0),
            h if h < 3.0 => (0.0, chroma, second),
            h if h < 4.0 => (0.0, second, chroma),
            h if h < 5.0 => (second, 0.0, chroma),
            _ => (chroma, 0.0, second),
        };
        let light = lightness - chroma / 2.0;
        Self::new(
            channel((r + light) * 255.0),
            channel((g + light) * 255.0),
            channel((b + light) * 255.0),
        )
    }
}

impl From<Rgb> for Hsl {
    fn from(color: Rgb) -> Self {
        let [red, green, blue] = color.channels().map(|channel| f64::from(channel) / 255.0);
        let (high, low) = (red.max(green).max(blue), red.min(green).min(blue));
        let lightness = f64::midpoint(high, low);
        let spread = high - low;
        if spread == 0.0 {
            return Self {
                hue: 0.0,
                saturation: 0.0,
                lightness,
            };
        }
        let saturation = spread / (1.0 - (2.0 * lightness - 1.0).abs());
        let sector = if (high - red).abs() < f64::EPSILON {
            ((green - blue) / spread).rem_euclid(6.0)
        } else if (high - green).abs() < f64::EPSILON {
            (blue - red) / spread + 2.0
        } else {
            (red - green) / spread + 4.0
        };
        Self {
            hue: sector * 60.0,
            saturation,
            lightness,
        }
    }
}

/// `Some` when an optional alpha part is at least [`MIN_ALPHA`].
fn opaque(alpha: Option<&str>) -> Option<()> {
    let Some(alpha) = alpha else { return Some(()) };
    let (value, percent) = number(alpha)?;
    let alpha = if percent { value / 100.0 } else { value };
    (alpha >= MIN_ALPHA).then_some(())
}

/// A CSS number, and whether it was a percentage. `none` reads as zero.
fn number(text: &str) -> Option<(f64, bool)> {
    if text == "none" {
        return Some((0.0, false));
    }
    match text.strip_suffix('%') {
        Some(percent) => percent.parse().ok().map(|value| (value, true)),
        None => text.parse().ok().map(|value| (value, false)),
    }
}

/// One channel from a value on the 0 to 255 scale, rounded and clamped.
#[expect(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is rounded and clamped to 0..=255 first, so the cast is exact"
)]
pub(crate) fn channel(value: f64) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_colour_survives_the_trip_through_hsl() {
        let colors = [
            Rgb::new(0x60, 0xa5, 0xfa),
            Rgb::new(0xf5, 0x9e, 0x0b),
            Rgb::new(0x9c, 0x3d, 0x1c),
            Rgb::new(0xd6, 0x33, 0x6c),
            Rgb::new(0x10, 0xb9, 0x81),
            Rgb::new(0x88, 0x88, 0x88),
            Rgb::new(0x00, 0x00, 0x00),
            Rgb::new(0xff, 0xff, 0xff),
        ];
        for color in colors {
            let hsl = Hsl::from(color);
            assert_eq!(Rgb::from(hsl), color, "{} as {hsl:?}", color.to_hex());
        }
    }

    #[test]
    fn hue_is_in_degrees() {
        let hue = |color: Rgb| Hsl::from(color).hue.round();
        assert_eq!(hue(Rgb::new(255, 0, 0)), 0.0);
        assert_eq!(hue(Rgb::new(0, 255, 0)), 120.0);
        assert_eq!(hue(Rgb::new(0, 0, 255)), 240.0);
        assert_eq!(hue(Rgb::new(255, 0, 255)), 300.0);
    }
}
