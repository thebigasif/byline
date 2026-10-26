//! CSS custom properties set for the whole page.

use std::collections::BTreeMap;

use crate::css::{self, Rule};
use crate::split::{Scanner, split_top};

/// How many `var()` references deep a value is followed. Real themes chain
/// two or three; the cap also ends a loop such as `--a: var(--b)` and
/// `--b: var(--a)`.
const MAX_DEPTH: usize = 8;

/// The selectors that set a property for the whole page.
const PAGE_WIDE: [&str; 4] = [":root", "html", "body", ":host"];

/// The custom properties a site sets for the whole page, such as
/// `--color-link: #0b57d0`, as written.
///
/// Only rules for `:root`, `html`, `body` or `:host` count, with or without
/// an attribute or class after them, so `--card-pad` on `.card` is left out.
/// Dark variants are left out too: `@media (prefers-color-scheme: dark)` and
/// any selector that names `dark`, because the blog stays light. A later
/// rule wins, as in the browser.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CustomProperties(BTreeMap<String, String>);

impl CustomProperties {
    /// Reads the properties from `stylesheets`, in page order.
    pub fn from_css(stylesheets: &[&str]) -> Self {
        let sheets: Vec<css::Sheet> = stylesheets.iter().map(|text| css::read(text)).collect();
        Self::from_rules(sheets.iter().flat_map(|sheet| &sheet.rules))
    }

    pub(crate) fn from_rules<'a>(rules: impl IntoIterator<Item = &'a Rule>) -> Self {
        let mut properties = BTreeMap::new();
        for rule in rules.into_iter().filter(|rule| is_page_wide(&rule.prelude)) {
            for declaration in &rule.declarations {
                if declaration.property.starts_with("--") {
                    properties.insert(declaration.property.clone(), declaration.value.clone());
                }
            }
        }
        Self(properties)
    }

    /// The value of the property `name`, such as `--color-link`. Names keep
    /// their case, as in CSS.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.0.get(name).map(String::as_str)
    }

    /// Every property and its value, sorted by name.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
    }

    /// `value` with each `var(--name)` or `var(--name, fallback)` replaced
    /// by the property's value, or the fallback when it is not set. A
    /// reference with neither is left as written.
    pub fn resolve(&self, value: &str) -> String {
        self.resolve_at(value, 0)
    }

    fn resolve_at(&self, value: &str, depth: usize) -> String {
        let mut out = String::with_capacity(value.len());
        let mut rest = value;
        while let Some(start) = rest.to_ascii_lowercase().find("var(") {
            let (Some(before), Some(from)) = (rest.get(..start), rest.get(start..)) else {
                break;
            };
            out.push_str(before);
            let Some((inner, after)) = call_body(from) else {
                out.push_str(from);
                return out;
            };
            let name = inner.split(',').next().unwrap_or_default().trim();
            let fallback = inner.split_once(',').map(|(_, fallback)| fallback.trim());
            match self.get(name).or(fallback) {
                Some(found) if depth < MAX_DEPTH => {
                    out.push_str(&self.resolve_at(found, depth.saturating_add(1)));
                }
                _ => out.push_str(
                    from.get(..from.len().saturating_sub(after.len()))
                        .unwrap_or(from),
                ),
            }
            rest = after;
        }
        out.push_str(rest);
        out
    }
}

/// For text that starts with `name(`, the text inside the parentheses and
/// the text after the closing one.
fn call_body(text: &str) -> Option<(&str, &str)> {
    let open = text.find('(')?;
    let mut scan = Scanner::default();
    for (at, c) in text.char_indices().skip_while(|&(at, _)| at < open) {
        if scan.step(c) && c == ')' && scan.depth == 0 {
            let inner = text.get(open.saturating_add(1)..at)?;
            return Some((inner, text.get(at.saturating_add(1)..)?));
        }
    }
    None
}

/// Whether one of the selectors in `prelude` sets properties for the whole
/// page in the light scheme.
fn is_page_wide(prelude: &str) -> bool {
    split_top(prelude, |c| c == ',')
        .into_iter()
        .any(|selector| {
            let selector = selector.to_ascii_lowercase();
            !selector.contains("dark")
                && PAGE_WIDE.iter().any(|base| {
                    selector
                        .strip_prefix(base)
                        .is_some_and(|rest| rest.is_empty() || rest.starts_with(['[', '.', ':']))
                })
        })
}
