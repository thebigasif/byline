//! A small CSS reader: just enough to find rules, declarations and
//! `@import` statements in real-world stylesheets.
//!
//! It is forgiving the way browsers are: a broken rule is skipped, never an
//! error. Rules inside `@media`, `@supports`, `@layer` and `@container` are
//! read as if they were at the top, unless they are for the dark colour
//! scheme; `@keyframes` and other at-rules are skipped, apart from
//! `@font-face`.

use crate::split::{Scanner, split_top};

/// How deep at-rules may nest before the reader stops going in. Real
/// stylesheets nest two or three levels; the cap keeps a hostile file from
/// using up the stack.
const MAX_NESTING: usize = 8;

/// A stylesheet read into rules.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Sheet {
    /// Style rules and `@font-face` rules, in source order.
    pub(crate) rules: Vec<Rule>,
    /// The text after each `@import`, such as `url("https://...")`.
    pub(crate) imports: Vec<String>,
}

/// One rule: its selector text, or `@font-face`, and its declarations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Rule {
    pub(crate) prelude: String,
    pub(crate) declarations: Vec<Declaration>,
}

/// One `property: value` pair. The property is lowercase, apart from custom
/// properties, which keep their case; `!important` is dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Declaration {
    pub(crate) property: String,
    pub(crate) value: String,
}

/// The declarations of any property in `names` in rules for any selector
/// in `selectors`, the one the browser would use first: later rules and
/// later declarations come before earlier ones.
pub(crate) fn declared<'a>(
    rules: &'a [&'a Rule],
    selectors: &'a [&'a str],
    names: &'a [&'a str],
) -> impl Iterator<Item = &'a Declaration> + 'a {
    rules
        .iter()
        .rev()
        .filter(|rule| selects(&rule.prelude, selectors))
        .flat_map(|rule| rule.declarations.iter().rev())
        .filter(|declaration| names.contains(&declaration.property.as_str()))
}

/// Whether one of the selectors in `prelude` is exactly one of `wanted`.
fn selects(prelude: &str, wanted: &[&str]) -> bool {
    split_top(prelude, |c| c == ',')
        .into_iter()
        .any(|selector| wanted.contains(&selector.to_ascii_lowercase().as_str()))
}

/// Reads `css` into rules.
pub(crate) fn read(css: &str) -> Sheet {
    let text = without_comments(css);
    let mut sheet = Sheet::default();
    read_into(&text, 0, &mut sheet);
    sheet
}

fn read_into(text: &str, depth: usize, sheet: &mut Sheet) {
    let mut rest = text;
    while let Some((item, after)) = next_item(rest) {
        rest = after;
        match item {
            Item::Statement(statement) => {
                if let Some(import) = at_rule(statement, "import") {
                    sheet.imports.push(import.trim().to_owned());
                }
            }
            Item::Block { prelude, body } => {
                if let Some(name) = prelude.strip_prefix('@') {
                    let mut name = name.split(|c: char| !c.is_alphanumeric() && c != '-');
                    match name.next().map(str::to_ascii_lowercase).as_deref() {
                        Some("media" | "supports" | "layer" | "container")
                            if depth < MAX_NESTING && !is_dark_scheme(prelude) =>
                        {
                            read_into(body, depth.saturating_add(1), sheet);
                        }
                        Some("font-face") => sheet.rules.push(rule("@font-face", body)),
                        _ => {}
                    }
                } else {
                    sheet.rules.push(rule(prelude, body));
                }
            }
        }
    }
}

/// Whether an at-rule's prelude applies only to the dark colour scheme.
/// The blog stays light, so those rules are skipped.
fn is_dark_scheme(prelude: &str) -> bool {
    let compact: String = prelude
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase();
    compact.contains("prefers-color-scheme:dark")
}

/// The text after `@name` when `statement` is that at-rule.
fn at_rule<'a>(statement: &'a str, name: &str) -> Option<&'a str> {
    let rest = statement.strip_prefix('@')?;
    let head = rest.get(..name.len())?;
    head.eq_ignore_ascii_case(name)
        .then(|| rest.get(name.len()..))
        .flatten()
}

fn rule(prelude: &str, body: &str) -> Rule {
    let declarations = split_top(body, |c| c == ';')
        .into_iter()
        // A nested rule has a brace outside any string; a value never does.
        .filter(|piece| split_top(piece, |c| c == '{').len() < 2)
        .filter_map(|piece| {
            let (property, value) = piece.split_once(':')?;
            let property = property.trim();
            let property = if property.starts_with("--") {
                property.to_owned()
            } else {
                property.to_ascii_lowercase()
            };
            let value = value.trim();
            let value = strip_important(value).unwrap_or(value).trim_end();
            Some(Declaration {
                property,
                value: value.to_owned(),
            })
        })
        .collect();
    Rule {
        prelude: prelude.to_owned(),
        declarations,
    }
}

fn strip_important(value: &str) -> Option<&str> {
    let bang = value.rfind('!')?;
    let flag = value.get(bang.saturating_add(1)..)?.trim();
    flag.eq_ignore_ascii_case("important")
        .then(|| value.get(..bang))
        .flatten()
}

enum Item<'a> {
    Statement(&'a str),
    Block { prelude: &'a str, body: &'a str },
}

/// The next statement or block in `text`, and the text after it.
fn next_item(text: &str) -> Option<(Item<'_>, &str)> {
    let mut scan = Scanner::default();
    let mut open = None;
    for (at, c) in text.char_indices() {
        let depth = scan.depth;
        if !scan.step(c) {
            continue;
        }
        match (c, depth, open) {
            (';', 0, None) => {
                let statement = text.get(..at)?.trim();
                return Some((
                    Item::Statement(statement),
                    text.get(at.saturating_add(1)..)?,
                ));
            }
            ('{', 0, None) => open = Some(at),
            ('}', 1, Some(start)) => {
                let prelude = text.get(..start)?.trim();
                let body = text.get(start.saturating_add(1)..at)?;
                return Some((
                    Item::Block { prelude, body },
                    text.get(at.saturating_add(1)..)?,
                ));
            }
            _ => {}
        }
    }
    // A block left open at the end of the file runs to the end, as in browsers.
    let start = open?;
    let prelude = text.get(..start)?.trim();
    let body = text.get(start.saturating_add(1)..)?;
    Some((Item::Block { prelude, body }, ""))
}

/// `css` with every `/* ... */` comment replaced by a space, leaving
/// strings alone.
fn without_comments(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    let mut scan = Scanner::default();
    while let Some(c) = rest.chars().next() {
        if scan.quote.is_none() && !scan.escaped && rest.starts_with("/*") {
            let end = rest.get(2..).and_then(|after| after.find("*/"));
            rest = end
                .and_then(|end| rest.get(end.saturating_add(4)..))
                .unwrap_or_default();
            out.push(' ');
            continue;
        }
        scan.step(c);
        out.push(c);
        rest = rest.get(c.len_utf8()..).unwrap_or_default();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn declarations(rule: &Rule) -> Vec<(&str, &str)> {
        let pairs = rule.declarations.iter();
        pairs
            .map(|d| (d.property.as_str(), d.value.as_str()))
            .collect()
    }

    #[test]
    fn reads_rules_inside_media_queries_and_skips_keyframes() {
        let sheet = read(
            "/* brand */ :root { --Brand-Blue: #0b57d0; }\n\
             @media (min-width: 40em) { body { Color: #111 !important; } }\n\
             @keyframes spin { from { color: red } }\n\
             @font-face { font-family: \"Inter\"; src: url(/inter.woff2) format('woff2'); }",
        );
        let preludes: Vec<&str> = sheet.rules.iter().map(|r| r.prelude.as_str()).collect();

        assert_eq!(preludes, [":root", "body", "@font-face"]);
        assert_eq!(declarations(&sheet.rules[0]), [("--Brand-Blue", "#0b57d0")]);
        assert_eq!(declarations(&sheet.rules[1]), [("color", "#111")]);
        assert_eq!(
            declarations(&sheet.rules[2]),
            [
                ("font-family", "\"Inter\""),
                ("src", "url(/inter.woff2) format('woff2')")
            ]
        );
    }

    #[test]
    fn keeps_semicolons_and_braces_inside_strings_and_urls() {
        let sheet = read(
            "a::after { content: \"};{\"; background: url(data:image/png;base64,AA==) }\
             b { color: #333 }",
        );

        assert_eq!(sheet.rules.len(), 2);
        assert_eq!(
            declarations(&sheet.rules[0]),
            [
                ("content", "\"};{\""),
                ("background", "url(data:image/png;base64,AA==)")
            ]
        );
    }

    #[test]
    fn collects_imports_and_survives_broken_input() {
        let sheet = read(
            "@import url(\"https://fonts.googleapis.com/css2?family=Inter\");\n\
             @charset \"utf-8\"; p { color: #222",
        );

        assert_eq!(
            sheet.imports,
            ["url(\"https://fonts.googleapis.com/css2?family=Inter\")"]
        );
        assert_eq!(declarations(&sheet.rules[0]), [("color", "#222")]);
        assert_eq!(read("}}}{{{ /* never closed").rules.len(), 1);
    }

    #[test]
    fn deep_nesting_stops_at_the_cap() {
        let open = "@media all {".repeat(MAX_NESTING + 4);
        let sheet = read(&format!("{open} p {{ color: #000 }}"));

        assert_eq!(sheet.rules, []);
    }
}
