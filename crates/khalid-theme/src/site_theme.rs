//! Everything theme sync finds in one fetched site.

use std::fmt::Write as _;

use scraper::{Html, Selector};

use crate::{Brand, Palette, SiteSource, Skipped, Stylesheet, Typography};

/// The colours, fonts and brand of one site, ready to become theme tokens.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct SiteTheme {
    /// The background, text, link and accent colours.
    pub palette: Palette,
    /// The body and heading fonts and where they come from.
    pub typography: Typography,
    /// The name, logo, share image and icons.
    pub brand: Brand,
}

impl SiteTheme {
    /// Finds the theme in a site read by [`fetch_site`](crate::fetch_site).
    ///
    /// The CSS is every stylesheet that was read, in page order, then the
    /// page's `style` attributes, so a site whose colours live in inline
    /// styles still counts them. A stylesheet link that was not read, such
    /// as Google Fonts on its own host, still names its fonts.
    pub fn from_source(source: &SiteSource) -> Self {
        let inline = style_attributes(source.html());
        let mut css: Vec<&str> = source.stylesheets().iter().map(Stylesheet::css).collect();
        css.push(&inline);
        let skipped: Vec<&str> = source.skipped().iter().map(Skipped::href).collect();
        Self {
            palette: Palette::from_css(&css),
            typography: Typography::from_css(&css, &skipped),
            brand: Brand::from_html(source.html(), source.home()),
        }
    }
}

/// The page's `style` attributes as one stylesheet, one rule each. An
/// attribute with a brace in it is dropped, so it cannot close its rule
/// and pose as a rule for `body`.
fn style_attributes(html: &str) -> String {
    let Ok(with_style) = Selector::parse("[style]") else {
        return String::new();
    };
    let mut css = String::new();
    let document = Html::parse_document(html);
    let styles = document
        .select(&with_style)
        .filter_map(|element| element.value().attr("style"))
        .filter(|style| !style.contains(['{', '}']));
    for style in styles {
        let _ = writeln!(css, "[style] {{ {style} }}");
    }
    css
}
