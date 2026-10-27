//! The fonts a site uses, and where each font file comes from.

use url::Url;

use crate::CustomProperties;
use crate::css::{self, Rule};
use crate::split::split_top;

/// Selectors whose `font-family` is the body font.
const PAGE: [&str; 3] = ["body", "html", ":root"];

/// Selectors whose `font-family` is the heading font.
const HEADINGS: [&str; 6] = ["h1", "h2", "h3", "h4", "h5", "h6"];

/// Words in a property name that mark something other than a family:
/// `--font-mono`, `--font-size-lg`, `--font-weight-bold`.
const NOT_FAMILY: [&str; 7] = [
    "mono",
    "code",
    "size",
    "weight",
    "height",
    "feature",
    "variation",
];

/// The body and heading font families of a site, and the font files it loads.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Typography {
    /// The body font as a CSS list, such as `"Inter", system-ui, sans-serif`.
    pub body: Option<String>,
    /// The heading font, in the same form.
    pub heading: Option<String>,
    /// Every font file or font service the CSS and links load, in page order.
    pub sources: Vec<FontSource>,
    /// The fonts the blog may not serve itself, such as an Adobe Fonts
    /// family. The blog shows the next font in the list instead.
    pub held: Vec<HeldFont>,
}

/// A font whose licence keeps it on the server that sells it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeldFont {
    family: String,
    host: FontHost,
    server: String,
}

impl HeldFont {
    /// The family name, such as `proxima-nova`.
    pub fn family(&self) -> &str {
        &self.family
    }

    /// Who serves it.
    pub fn host(&self) -> FontHost {
        self.host
    }

    /// The server's host name, such as `use.typekit.net`.
    pub fn server(&self) -> &str {
        &self.server
    }
}

/// One font family and where it is loaded from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontSource {
    family: String,
    url: String,
    host: FontHost,
}

/// Who serves a font, which decides whether the blog may serve it too.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FontHost {
    /// Google Fonts. Its fonts are openly licensed and may be self-hosted.
    Google,
    /// Adobe Fonts. Its licence does not allow serving the files elsewhere.
    Adobe,
    /// A file on the site itself, given by a relative address.
    SelfHosted,
    /// Any other server, such as a commercial foundry.
    Other,
}

impl FontHost {
    /// Whether the blog may serve this font itself. Only fonts we know the
    /// licence of qualify; for the rest the blog uses a system fallback and
    /// says so.
    pub fn can_rehost(self) -> bool {
        matches!(self, Self::Google | Self::SelfHosted)
    }

    fn of(url: &str) -> Self {
        match Url::parse(url) {
            Err(_) => Self::SelfHosted,
            Ok(url) => match url.host_str() {
                Some("fonts.googleapis.com" | "fonts.gstatic.com") => Self::Google,
                Some("use.typekit.net" | "p.typekit.net") => Self::Adobe,
                // A `data:` font is written inside the site's own CSS.
                None => Self::SelfHosted,
                Some(_) => Self::Other,
            },
        }
    }
}

impl FontSource {
    /// The family `family` loaded from `url`, served by `host`.
    pub fn new(family: &str, url: &str, host: FontHost) -> Self {
        Self {
            family: family.to_owned(),
            url: url.to_owned(),
            host,
        }
    }

    /// The family name, such as `Inter`.
    pub fn family(&self) -> &str {
        &self.family
    }

    /// The address as the CSS or page wrote it.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Who serves it.
    pub fn host(&self) -> FontHost {
        self.host
    }
}

impl Typography {
    /// Finds the fonts in `stylesheets`, in page order, and in `links`, the
    /// stylesheet addresses theme sync did not read, such as a Google Fonts
    /// link on another host.
    ///
    /// The body font comes from `font-family` or `font` on `body`, `html` or
    /// `:root`, and the heading font from `h1` to `h6`, the later rule
    /// winning. What those leave out comes from a property such as
    /// `--font-sans`, `--bs-body-font-family` or `--font-heading`, the
    /// shortest name winning.
    pub fn from_css(stylesheets: &[&str], links: &[&str]) -> Self {
        let sheets: Vec<css::Sheet> = stylesheets.iter().map(|text| css::read(text)).collect();
        let rules: Vec<&Rule> = sheets.iter().flat_map(|sheet| &sheet.rules).collect();
        let properties = CustomProperties::from_rules(rules.iter().copied());
        let declared = |selectors: &[&str]| {
            css::declared(&rules, selectors, &["font-family", "font"]).find_map(|declaration| {
                let value = properties.resolve(&declaration.value);
                let family = if declaration.property == "font" {
                    family_of_shorthand(&value)?
                } else {
                    value
                };
                family_list(&family)
            })
        };
        let mut sources = Vec::new();
        for sheet in &sheets {
            for import in &sheet.imports {
                sources.extend(url_in(import).map(from_link).unwrap_or_default());
            }
            let faces = sheet
                .rules
                .iter()
                .filter(|rule| rule.prelude == "@font-face");
            sources.extend(faces.filter_map(from_face));
        }
        sources.extend(links.iter().flat_map(|link| from_link(link)));
        let body =
            declared(&PAGE).or_else(|| named(&properties, &["body", "base", "sans", "text"]));
        let heading = declared(&HEADINGS)
            .or_else(|| named(&properties, &["heading", "headings", "display", "title"]));
        let held = held_fonts(&sources, links, [body.as_deref(), heading.as_deref()]);
        Self {
            body,
            heading,
            sources,
            held,
        }
    }
}

/// The text fonts the blog may not serve: the first family of the body and
/// heading lists, when it is loaded from a server whose licence we do not
/// know, or, with a paid font service such as an Adobe Fonts kit linked,
/// when no open source loads it. Icon fonts and other families the text
/// does not use are left out.
fn held_fonts(sources: &[FontSource], links: &[&str], lists: [Option<&str>; 2]) -> Vec<HeldFont> {
    let firsts: Vec<&str> = lists
        .iter()
        .flatten()
        .filter_map(|list| first_family(list))
        .collect();
    let mut held: Vec<HeldFont> = Vec::new();
    let mut keep = |family: &str, host: FontHost, url: &str| {
        if !held
            .iter()
            .any(|font| font.family.eq_ignore_ascii_case(family))
        {
            let server = Url::parse(url)
                .ok()
                .and_then(|url| url.host_str().map(str::to_owned))
                .unwrap_or_default();
            held.push(HeldFont {
                family: family.to_owned(),
                host,
                server,
            });
        }
    };
    let used = |family: &str| {
        firsts
            .iter()
            .any(|first| first.eq_ignore_ascii_case(family))
    };
    for source in sources.iter().filter(|source| !source.host.can_rehost()) {
        if used(&source.family) {
            keep(&source.family, source.host, &source.url);
        }
    }
    let services = links
        .iter()
        .filter_map(|link| Some((paid_service(link)?, *link)));
    for (host, link) in services {
        for &first in &firsts {
            let served = sources
                .iter()
                .any(|source| source.family.eq_ignore_ascii_case(first));
            if !served {
                keep(first, host, link);
            }
        }
    }
    held
}

/// Who serves a link to a paid font service, or `None` for any other link.
fn paid_service(url: &str) -> Option<FontHost> {
    let url = Url::parse(url).ok()?;
    match url.host_str()? {
        "use.typekit.net" | "p.typekit.net" => Some(FontHost::Adobe),
        "fast.fonts.net" | "use.fonts.com" | "cloud.typography.com" => Some(FontHost::Other),
        _ => None,
    }
}

/// The first family of a font list without its quotes, or `None` when it
/// is a generic family that every computer has.
fn first_family(list: &str) -> Option<&str> {
    let first = split_top(list, |c| c == ',').into_iter().next()?;
    let first = first.trim().trim_matches('"');
    let generic = [
        "serif",
        "sans-serif",
        "monospace",
        "system-ui",
        "ui-sans-serif",
        "ui-serif",
        "ui-monospace",
        "-apple-system",
    ];
    (!first.is_empty() && !generic.contains(&first)).then_some(first)
}

/// The family list of the property with the shortest name that has `font`
/// and one of `role` among its words.
fn named(properties: &CustomProperties, role: &[&str]) -> Option<String> {
    properties
        .iter()
        .filter_map(|(name, value)| {
            let words: Vec<String> = name
                .split(['-', '_'])
                .filter(|word| !word.is_empty())
                .map(str::to_ascii_lowercase)
                .collect();
            let has = |wanted: &[&str]| words.iter().any(|word| wanted.contains(&word.as_str()));
            let fits = has(&["font"]) && has(role) && !has(&NOT_FAMILY);
            fits.then(|| Some((words.len(), name, family_list(&properties.resolve(value))?)))
                .flatten()
        })
        .min_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)))
        .map(|(_, _, family)| family)
}

/// The family part of a `font` shorthand: everything after the size, as in
/// `700 2rem/1.2 'Playfair Display', serif`.
fn family_of_shorthand(value: &str) -> Option<String> {
    let tokens = split_top(value, char::is_whitespace);
    let size = tokens.iter().position(|token| {
        token.starts_with(|c: char| c.is_ascii_digit() || c == '.')
            && token.contains(|c: char| c.is_alphabetic() || c == '%')
    })?;
    let family: Vec<&str> = tokens.get(size.saturating_add(1)..)?.to_vec();
    (!family.is_empty()).then(|| family.join(" "))
}

/// A `font-family` value with each name quoted in double quotes when it was
/// quoted at all, or `None` when it names no family.
fn family_list(value: &str) -> Option<String> {
    let names: Vec<String> = split_top(value, |c| c == ',')
        .into_iter()
        .map(
            |name| match name.strip_prefix('\'').and_then(|n| n.strip_suffix('\'')) {
                Some(inner) => format!("\"{inner}\""),
                None => name.to_owned(),
            },
        )
        .collect();
    let all_families = names.iter().all(|name| {
        name.chars().any(char::is_alphabetic)
            && !name.contains('(')
            && !matches!(name.as_str(), "inherit" | "initial" | "unset")
    });
    (all_families && !names.is_empty()).then(|| names.join(", "))
}

/// The address in `url(...)`, `url("...")` or a bare quoted string.
fn url_in(text: &str) -> Option<&str> {
    let text = text.trim();
    let inner = match text.get(..4) {
        Some(head) if head.eq_ignore_ascii_case("url(") => text.get(4..)?.split(')').next()?,
        _ => text,
    };
    let inner = inner.trim();
    let quote = inner.chars().next().filter(|c| matches!(c, '"' | '\''));
    match quote {
        Some(quote) => inner.get(1..)?.split(quote).next(),
        None => inner.split_whitespace().next(),
    }
}

/// The families a font service link loads. Google Fonts links name them
/// in `family=`; any other address loads no font we can name.
fn from_link(url: &str) -> Vec<FontSource> {
    let host = FontHost::of(url);
    let Ok(parsed) = Url::parse(url) else {
        return Vec::new();
    };
    if host != FontHost::Google {
        return Vec::new();
    }
    parsed
        .query_pairs()
        .filter(|(key, _)| key == "family")
        .flat_map(|(_, value)| {
            let families: Vec<String> = value
                .split('|')
                .filter_map(|family| family.split(':').next())
                .map(|family| family.trim().to_owned())
                .filter(|family| !family.is_empty())
                .collect();
            families
        })
        .map(|family| FontSource::new(&family, url, host))
        .collect()
}

/// The family and first file of an `@font-face` rule.
fn from_face(rule: &Rule) -> Option<FontSource> {
    let value = |name: &str| {
        rule.declarations
            .iter()
            .find(|declaration| declaration.property == name)
            .map(|declaration| declaration.value.as_str())
    };
    let family = value("font-family")?.trim_matches(['"', '\'', ' ']);
    let first = split_top(value("src")?, |c| c == ',').into_iter().next()?;
    let url = url_in(first)?;
    Some(FontSource::new(family, url, FontHost::of(url)))
}
