//! What a share image shows. Passive input, like the lint crate's model.

use std::fmt;

/// One share image's input. Equal cards draw the same image. Build it with
/// [`ShareCard::new`] and set the direction for a right to left language.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ShareCard {
    /// The post's title, the large text on the card.
    pub title: String,
    /// The site's name, the small text at the foot of the card.
    pub site_name: String,
    /// The colours the card is drawn in.
    pub colors: CardColors,
    /// The direction the page's language is written in.
    pub direction: Direction,
}

impl ShareCard {
    /// A left to right card for the post `title` on the site `site_name`.
    pub fn new(title: &str, site_name: &str, colors: CardColors) -> Self {
        Self {
            title: title.to_owned(),
            site_name: site_name.to_owned(),
            colors,
            direction: Direction::LeftToRight,
        }
    }
}

/// The theme colours a card uses, after the theme's contrast pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct CardColors {
    /// The card's background.
    pub background: Rgb,
    /// The title and the site name.
    pub text: Rgb,
    /// The bar along the top and the rule above the site name.
    pub accent: Rgb,
}

impl CardColors {
    /// The three colours, in the order the tokens file lists them.
    pub fn new(background: Rgb, text: Rgb, accent: Rgb) -> Self {
        Self {
            background,
            text,
            accent,
        }
    }
}

/// The direction a language is written in. Right to left mirrors the card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    /// English, German, French and most other languages.
    LeftToRight,
    /// Arabic, Hebrew, Persian and Urdu.
    RightToLeft,
}

/// A colour as red, green and blue channels, shown as `#rrggbb`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgb(pub [u8; 3]);

impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [r, g, b] = self.0;
        write!(f, "#{r:02x}{g:02x}{b:02x}")
    }
}
