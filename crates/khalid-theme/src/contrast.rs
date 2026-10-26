//! The WCAG contrast pass over the theme tokens.

use std::fmt;

use crate::color::Hsl;
use crate::{Colors, Rgb, ThemeTokens};

/// The contrast WCAG 2.2 AA asks of text (success criterion 1.4.3).
pub const TEXT_CONTRAST: f64 = 4.5;

/// The contrast WCAG 2.2 AA asks of marks that are not text, such as a
/// button's edge or a focus ring (success criterion 1.4.11).
pub const MARK_CONTRAST: f64 = 3.0;

/// How many halvings the search for the nearest shade takes. Forty is far
/// finer than one step of an 8-bit channel.
const SEARCH_STEPS: usize = 40;

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

    /// The least contrast this token needs.
    pub fn target(self) -> f64 {
        match self {
            Self::Accent => MARK_CONTRAST,
            Self::Text | Self::Muted | Self::Link => TEXT_CONTRAST,
        }
    }

    /// What the token colours, in words a person reads.
    fn label(self) -> &'static str {
        match self {
            Self::Text => "The colour of body text",
            Self::Muted => "The colour of quiet text such as dates",
            Self::Link => "The colour of links",
            Self::Accent => "The colour of buttons and highlights",
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

/// One colour the contrast pass changed.
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
}

impl fmt::Display for ContrastFix {
    /// The fix in plain words, for the person running the sync.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} was {}, at {:.1} to 1 on the page, too faint for many readers. \
             It is now {}, at {:.1} to 1, the closest shade that meets WCAG AA, \
             the accessibility standard.",
            self.token.label(),
            self.before.to_hex(),
            self.before_ratio,
            self.after.to_hex(),
            self.after_ratio,
        )
    }
}

impl ThemeTokens {
    /// Checks the text, quiet text, link and accent colours against both
    /// the background and the surface, and gives each that fails the
    /// nearest shade that passes: the same hue and saturation, with the
    /// lightness moved as little as it can be. Returns what changed.
    ///
    /// Text needs [`TEXT_CONTRAST`]; the accent, which marks buttons and
    /// highlights, needs [`MARK_CONTRAST`].
    pub fn fix_contrast(&mut self) -> Vec<ContrastFix> {
        let grounds = [self.colors.background, self.colors.surface];
        let mut fixes = Vec::new();
        for token in ColorToken::ALL {
            let slot = token.slot(&mut self.colors);
            let before = *slot;
            let target = token.target();
            if worst(before, &grounds) >= target {
                continue;
            }
            let after = nearest_passing(before, &grounds, target);
            *slot = after;
            fixes.push(ContrastFix {
                token,
                before,
                after,
                before_ratio: worst(before, &grounds),
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

/// The shade of `color` closest in lightness that reaches `target` on
/// every ground, searching toward black and toward white. When neither end
/// passes, the end with the higher contrast.
fn nearest_passing(color: Rgb, grounds: &[Rgb], target: f64) -> Rgb {
    let start = Hsl::from(color);
    let at = |lightness: f64| Rgb::from(Hsl { lightness, ..start });
    let passes = |shade: Rgb| worst(shade, grounds) >= target;
    let found = [0.0, 1.0]
        .into_iter()
        .filter(|&end| passes(at(end)))
        .map(|end| {
            let (mut fails, mut holds) = (start.lightness, end);
            for _ in 0..SEARCH_STEPS {
                let middle = f64::midpoint(fails, holds);
                if passes(at(middle)) {
                    holds = middle;
                } else {
                    fails = middle;
                }
            }
            (holds - start.lightness).abs()
                .min(1.0)
                .mul_add(0.0, 0.0)
                .max(0.0)
                + (holds - start.lightness).abs()
        })
        .zip([0.0, 1.0])
        .min_by(|a, b| a.0.total_cmp(&b.0));
    match found {
        Some(_) => todo_shade(),
        None => todo_shade(),
    }
}
