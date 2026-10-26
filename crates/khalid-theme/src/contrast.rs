//! The WCAG contrast pass over the theme tokens.

use std::fmt;

use crate::color::Hsl;
use crate::{Colors, Rgb, ThemeTokens};

/// The contrast WCAG 2.2 AA asks of text (success criterion 1.4.3).
pub const TEXT_CONTRAST: f64 = 4.5;

/// The contrast WCAG 2.2 AA asks of marks that are not text, such as a
/// button's edge or a focus ring (success criterion 1.4.11).
pub const MARK_CONTRAST: f64 = 3.0;

/// How many steps of lightness the search takes each way. A thousand
/// steps of 0.1% each are finer than one step of an 8-bit channel, so no
/// shade between the old colour and the one found is skipped.
const SEARCH_STEPS: u16 = 1000;

/// A colour token the contrast pass checks against the background and the
/// surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ColorToken {
    /// `colors.text`, body text.
    Text,
    /// `colors.muted`, dates, captions and other quiet text.
    Muted,
    /// `colors.link`, links.
    Link,
    /// `colors.accent`, buttons and highlights.
    Accent,
}

impl ColorToken {
    /// Every checked token, in the order the pass checks them.
    pub const ALL: [Self; 4] = [Self::Text, Self::Muted, Self::Link, Self::Accent];

    /// The token's name in `theme/tokens.toml`, such as `colors.link`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Text => "colors.text",
            Self::Muted => "colors.muted",
            Self::Link => "colors.link",
            Self::Accent => "colors.accent",
        }
    }

    /// The least contrast this token needs on the background and the
    /// surface: [`TEXT_CONTRAST`] for text, [`MARK_CONTRAST`] for the
    /// accent.
    pub fn target(self) -> f64 {
        match self {
            Self::Accent => MARK_CONTRAST,
            Self::Text | Self::Muted | Self::Link => TEXT_CONTRAST,
        }
    }

    /// What the token colours, in words a person reads.
    fn label(self) -> &'static str {
        match self {
            Self::Text => "body text",
            Self::Muted => "quiet text such as dates",
            Self::Link => "links",
            Self::Accent => "buttons and highlights",
        }
    }

    fn slot(self, colors: &mut Colors) -> &mut Rgb {
        match self {
            Self::Text => &mut colors.text,
            Self::Muted => &mut colors.muted,
            Self::Link => &mut colors.link,
            Self::Accent => &mut colors.accent,
        }
    }
}

/// One colour the contrast pass changed. Its [`Display`](fmt::Display)
/// tells the person running the sync what changed and why.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContrastFix {
    token: ColorToken,
    before: Rgb,
    after: Rgb,
    before_ratio: f64,
    after_ratio: f64,
}

impl ContrastFix {
    /// The token that changed.
    pub fn token(&self) -> ColorToken {
        self.token
    }

    /// The colour as the site had it.
    pub fn before(&self) -> Rgb {
        self.before
    }

    /// The colour now in the tokens.
    pub fn after(&self) -> Rgb {
        self.after
    }

    /// The lowest contrast it had, on the background or the surface.
    pub fn before_ratio(&self) -> f64 {
        self.before_ratio
    }

    /// The lowest contrast it has now.
    pub fn after_ratio(&self) -> f64 {
        self.after_ratio
    }

    /// Whether the new colour meets the token's target. It does not only
    /// when the background and the surface are so far apart that no shade
    /// passes on both.
    pub fn meets_target(&self) -> bool {
        self.after_ratio >= self.token.target()
    }
}

impl fmt::Display for ContrastFix {
    /// The fix in plain words: what was wrong, the standard, what changed
    /// and what the person can do.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let token = self.token;
        write!(
            f,
            "The colour of {}, {}, was too faint for many readers: its contrast \
             with the page was {} to 1. WCAG AA, the web's standard for readable \
             pages, asks for {} to 1. ",
            token.label(),
            self.before.to_hex(),
            ratio(self.before_ratio),
            ratio(token.target()),
        )?;
        if self.meets_target() {
            write!(
                f,
                "It is now {}, the closest shade that meets it. To choose another \
                 colour, edit {} in theme/tokens.toml.",
                self.after.to_hex(),
                token.name(),
            )
        } else {
            write!(
                f,
                "No shade of it meets that on both the background and the surface. \
                 It is now {}, the shade with the most contrast, at {} to 1. To fix \
                 this, change colors.background or colors.surface in theme/tokens.toml.",
                self.after.to_hex(),
                ratio(self.after_ratio),
            )
        }
    }
}

impl ThemeTokens {
    /// Checks the text, quiet text, link and accent colours against both
    /// the background and the surface, and gives each that fails the
    /// nearest shade that passes: the same hue and saturation, with the
    /// lightness moved as little as it can be. Returns what changed, in
    /// the order of [`ColorToken::ALL`]; tell the person each one.
    ///
    /// Text needs [`TEXT_CONTRAST`]; the accent, which marks buttons and
    /// highlights, needs [`MARK_CONTRAST`]. A colour no shade of which can
    /// pass on both grounds gets the shade with the most contrast, and its
    /// fix says so.
    #[must_use = "each fix must be told to the person running the sync"]
    pub fn fix_contrast(&mut self) -> Vec<ContrastFix> {
        let grounds = [self.colors.background, self.colors.surface];
        let mut fixes = Vec::new();
        for token in ColorToken::ALL {
            let slot = token.slot(&mut self.colors);
            let before = *slot;
            let target = token.target();
            let before_ratio = worst(before, &grounds);
            if before_ratio >= target {
                continue;
            }
            let after = nearest_passing(before, &grounds, target);
            *slot = after;
            fixes.push(ContrastFix {
                token,
                before,
                after,
                before_ratio,
                after_ratio: worst(after, &grounds),
            });
        }
        fixes
    }
}

/// The lowest contrast `color` has on any of `grounds`.
fn worst(color: Rgb, grounds: &[Rgb]) -> f64 {
    grounds
        .iter()
        .map(|&ground| color.contrast(ground))
        .fold(f64::INFINITY, f64::min)
}

/// The shade of `color` nearest in lightness that reaches `target` on every
/// ground. The search steps outward both ways at once, so the first shade
/// that passes is the nearest; at equal distance the darker one wins. When
/// no shade passes, the shade with the most contrast, which may be `color`
/// itself.
fn nearest_passing(color: Rgb, grounds: &[Rgb], target: f64) -> Rgb {
    let start = Hsl::from(color);
    let step = 1.0 / f64::from(SEARCH_STEPS);
    let shades = || {
        (1..=SEARCH_STEPS)
            .map(move |count| f64::from(count) * step)
            .flat_map(move |distance| [start.lightness - distance, start.lightness + distance])
            .filter(|lightness| (0.0..=1.0).contains(lightness))
            .map(move |lightness| Rgb::from(Hsl { lightness, ..start }))
    };
    shades()
        .find(|&shade| worst(shade, grounds) >= target)
        .unwrap_or_else(|| {
            shades()
                .chain([color])
                .max_by(|a, b| worst(*a, grounds).total_cmp(&worst(*b, grounds)))
                .unwrap_or(color)
        })
}

/// A contrast ratio to one decimal, rounded down, so a failing 4.47 never
/// reads as a passing 4.5.
fn ratio(value: f64) -> String {
    format!("{:.1}", (value * 10.0).floor() / 10.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The next distinct shade back toward the old colour fails, so the one
    /// found is the smallest change that passes.
    #[test]
    fn the_next_shade_closer_to_the_old_colour_fails() {
        let white = Rgb::new(0xff, 0xff, 0xff);
        let grounds = [white, Rgb::new(0xf6, 0xf7, 0xf9)];
        let olds = [
            Rgb::new(0x60, 0xa5, 0xfa),
            Rgb::new(0xa0, 0xa0, 0xa0),
            Rgb::new(0xf5, 0x9e, 0x0b),
        ];
        for old in olds {
            let found = nearest_passing(old, &grounds, TEXT_CONTRAST);
            let (from, to) = (Hsl::from(old), Hsl::from(found));
            let toward = (from.lightness - to.lightness).signum() * 0.000_5;
            let closer = (1..100)
                .map(|count| to.lightness + toward * f64::from(count))
                .map(|lightness| Rgb::from(Hsl { lightness, ..from }))
                .find(|&shade| shade != found)
                .unwrap();
            assert!(
                worst(found, &grounds) >= TEXT_CONTRAST,
                "{}",
                found.to_hex()
            );
            assert!(
                worst(closer, &grounds) < TEXT_CONTRAST,
                "{} passes and is closer to {} than {}",
                closer.to_hex(),
                old.to_hex(),
                found.to_hex()
            );
        }
    }

    #[test]
    fn ratios_round_down() {
        assert_eq!(ratio(4.47), "4.4");
        assert_eq!(ratio(4.5), "4.5");
        assert_eq!(ratio(3.0), "3.0");
    }
}
