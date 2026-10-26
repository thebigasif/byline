//! The four colours theme sync looks for in a site's CSS.

use std::cmp::Reverse;
use std::collections::BTreeMap;

use crate::css::{self, Rule};
use crate::split::split_top;
use crate::{CustomProperties, Rgb};

/// The background assumed when the CSS sets none, as browsers do.
const WHITE: Rgb = Rgb::new(255, 255, 255);

/// The contrast WCAG 2.2 AA asks of body text. A colour used often but
/// fainter than this is decoration, not the body text.
const READABLE: f64 = 4.5;

/// Selectors whose rules set the page's own background and text colour.
const PAGE: [&str; 3] = ["body", "html", ":root"];

/// Selectors whose rules set the colour of links at rest.
const LINKS: [&str; 3] = ["a", "a:link", "a:any-link"];

/// Words in a property name that mark a variant, not the colour itself:
/// `--link-hover`, `--primary-light`, `--bs-primary-rgb`.
const VARIANTS: [&str; 14] = [
    "hover",
    "active",
    "focus",
    "visited",
    "light",
    "dark",
    "alt",
    "muted",
    "subtle",
    "inverse",
    "rgb",
    "emphasis",
    "secondary",
    "border",
];

/// The page background, body text, link and accent colours of a site, each
/// `None` when the CSS does not say.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Palette {
    /// The page background.
    pub background: Option<Rgb>,
    /// Body text.
    pub text: Option<Rgb>,
    /// Links at rest.
    pub link: Option<Rgb>,
    /// The brand colour for buttons and highlights.
    pub accent: Option<Rgb>,
}

/// The colour a custom property's name says it holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Background,
    Text,
    Link,
    Accent,
}

impl Palette {
    /// Finds the colours in `stylesheets`, in page order.
    ///
    /// Each colour comes from the most direct evidence there is: a rule for
    /// `body`, `html` or `:root` sets the background and text, and a rule
    /// for `a` or `a:link` sets the link, the later rule winning. What those
    /// leave out comes from a custom property whose name says what it holds,
    /// such as `--bs-body-bg`, `--color-text`, `--link-color` or
    /// `--brand`, the shortest such name winning. Every `var()` is resolved.
    ///
    /// When neither says, frequency decides: the most used background, the
    /// most used text colour readable on it, and the most used vivid colour
    /// as the accent. A site with a link colour and no other accent uses the
    /// link colour for both.
    pub fn from_css(stylesheets: &[&str]) -> Self {
        let sheets: Vec<css::Sheet> = stylesheets.iter().map(|text| css::read(text)).collect();
        let rules: Vec<&Rule> = sheets.iter().flat_map(|sheet| &sheet.rules).collect();
        let properties = CustomProperties::from_rules(rules.iter().copied());
        let declared = |selectors: &[&str], names: &[&str]| -> Option<Rgb> {
            rules
                .iter()
                .rev()
                .filter(|rule| selects(&rule.prelude, selectors))
                .flat_map(|rule| rule.declarations.iter().rev())
                .filter(|declaration| names.contains(&declaration.property.as_str()))
                .find_map(|declaration| {
                    Rgb::all_in(&properties.resolve(&declaration.value))
                        .into_iter()
                        .next()
                })
        };
        let named = |role| named(&properties, role);
        let counts = Counts::of(&rules, &properties);
        let background = declared(&PAGE, &["background-color", "background"])
            .or_else(|| named(Role::Background))
            .or_else(|| most(&counts.backgrounds, |_| true));
        let on = background.unwrap_or(WHITE);
        let text = declared(&PAGE, &["color"])
            .or_else(|| named(Role::Text))
            .or_else(|| most(&counts.texts, |color| color.contrast(on) >= READABLE));
        let link = declared(&LINKS, &["color"]).or_else(|| named(Role::Link));
        let accent = named(Role::Accent)
            .or_else(|| {
                most(&counts.all, |color| {
                    color.is_vivid() && Some(color) != background && Some(color) != text
                })
            })
            .or(link);
        Self {
            background,
            text,
            link,
            accent,
        }
    }
}

/// How often each colour is used, for sites whose CSS names nothing, such
/// as utility classes or hashed class names.
#[derive(Default)]
struct Counts {
    backgrounds: BTreeMap<Rgb, usize>,
    texts: BTreeMap<Rgb, usize>,
    all: BTreeMap<Rgb, usize>,
}

impl Counts {
    fn of(rules: &[&Rule], properties: &CustomProperties) -> Self {
        let mut counts = Self::default();
        let declarations = rules.iter().flat_map(|rule| &rule.declarations);
        for declaration in declarations.filter(|d| !d.property.starts_with("--")) {
            let property = declaration.property.as_str();
            for color in Rgb::all_in(&properties.resolve(&declaration.value)) {
                let add = |map: &mut BTreeMap<Rgb, usize>| {
                    let count = map.entry(color).or_default();
                    *count = count.saturating_add(1);
                };
                add(&mut counts.all);
                match property {
                    "background" | "background-color" => add(&mut counts.backgrounds),
                    "color" => add(&mut counts.texts),
                    _ => {}
                }
            }
        }
        counts
    }
}

/// The most used colour in `counts` that passes `keep`. A tie goes to the
/// colour that sorts first, so the answer never depends on luck.
fn most(counts: &BTreeMap<Rgb, usize>, keep: impl Fn(Rgb) -> bool) -> Option<Rgb> {
    counts
        .iter()
        .filter(|&(&color, _)| keep(color))
        .max_by_key(|&(&color, &count)| (count, Reverse(color)))
        .map(|(&color, _)| color)
}

/// Whether one of the selectors in `prelude` is exactly one of `wanted`.
fn selects(prelude: &str, wanted: &[&str]) -> bool {
    split_top(prelude, |c| c == ',')
        .into_iter()
        .any(|selector| wanted.contains(&selector.to_ascii_lowercase().as_str()))
}

/// The colour of the property with the shortest name that holds `role`.
fn named(properties: &CustomProperties, role: Role) -> Option<Rgb> {
    properties
        .iter()
        .filter_map(|(name, value)| {
            let words = words(name);
            let color = Rgb::from_css(&properties.resolve(value))?;
            (role_of(&words) == Some(role)).then_some((words.len(), name, color))
        })
        .min_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)))
        .map(|(_, _, color)| color)
}

/// The lowercase words of a property name: `--bs-body-bg` is `bs`, `body`
/// and `bg`.
fn words(name: &str) -> Vec<String> {
    name.split(['-', '_'])
        .filter(|word| !word.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}

/// The colour a property name says it holds, if any.
fn role_of(words: &[String]) -> Option<Role> {
    let has = |wanted: &[&str]| words.iter().any(|word| wanted.contains(&word.as_str()));
    if has(&VARIANTS) {
        None
    } else if has(&["link"]) {
        Some(Role::Link)
    } else if has(&["primary", "brand", "accent"]) {
        Some(Role::Accent)
    } else if has(&["bg", "background"]) {
        Some(Role::Background)
    } else if has(&["text", "foreground", "fg"]) || (has(&["body"]) && has(&["color"])) {
        Some(Role::Text)
    } else {
        None
    }
}
