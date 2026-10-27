//! Everything theme sync finds in one fetched site.

use std::fmt::Write as _;

use scraper::{Html, Selector};

use crate::{Brand, Palette, SiteSource, Skipped, Stylesheet, Typography};

/// Fewer letters than this in a page's body, with scripts on the page,
/// means the page is drawn by JavaScript. A real home page has more in its
/// menu and footer alone.
const MIN_TEXT: usize = 200;

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
    /// Whether the home page looks drawn by JavaScript: it runs scripts and
    /// shows almost no text without them, so its HTML says little.
    pub drawn_by_script: bool,
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
            drawn_by_script: Self::looks_drawn_by_script(source.html()),
        }
    }

    /// Whether `html` looks like a page drawn by JavaScript: it has a
    /// `script` element, and its body shows fewer than 200 letters outside
    /// scripts, styles and `noscript`.
    pub fn looks_drawn_by_script(html: &str) -> bool {
        let document = Html::parse_document(html);
        let (Ok(script), Ok(body)) = (Selector::parse("script"), Selector::parse("body")) else {
            return false;
        };
        if document.select(&script).next().is_none() {
            return false;
        }
        let Some(body) = document.select(&body).next() else {
            return true;
        };
        let shown: usize = body
            .descendants()
            .filter_map(|node| {
                let text = node.value().as_text()?;
                let hidden = node
                    .ancestors()
                    .filter_map(|parent| parent.value().as_element())
                    .any(|parent| {
                        matches!(parent.name(), "script" | "style" | "noscript" | "template")
                    });
                (!hidden).then(|| text.chars().filter(|c| !c.is_whitespace()).count())
            })
            .sum();
        shown < MIN_TEXT
    }

    /// Finds the colours and fonts in `css` alone, such as the custom
    /// properties a person copied from a site that draws its pages with
    /// JavaScript. There is no page, so the brand is empty.
    pub fn from_css(css: &str) -> Self {
        Self {
            palette: Palette::from_css(&[css]),
            typography: Typography::from_css(&[css], &[]),
            brand: Brand::default(),
            drawn_by_script: false,
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
