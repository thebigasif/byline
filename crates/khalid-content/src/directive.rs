//! Reading directive blocks, `::name{...}` and `:::name{...}` (ADR 0012).

use crate::{BlockError, BlockKind, CalloutKind};

/// Every block name, for messages.
pub(crate) const BLOCKS: [&str; 5] = ["button", "callout", "figure", "toc", "toggle"];

/// Blocks with no body, written on one line as `::name{...}`. Every other
/// block opens with `:::name{...}` and closes with `:::`.
const LEAVES: [&str; 2] = ["button", "toc"];

/// How a block was written.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Form {
    /// `::name{...}` on one line.
    Leaf,
    /// `:::name{...}`, a body, then `:::`.
    Container,
}

/// A `::name` or `:::name{...}` line: its run of colons and what follows.
pub(crate) fn opening(line: &str) -> Option<(&str, &str)> {
    let colons = line.len() - line.trim_start_matches(':').len();
    let (fence, rest) = line.split_at_checked(colons)?;
    (colons >= 2 && rest.starts_with(|c: char| c.is_ascii_alphabetic()))
        .then_some((fence, rest.trim_end()))
}

/// Reads a directive's name and attributes, written in `form` on body `line`,
/// into its kind.
pub(crate) fn directive(line: usize, rest: &str, form: Form) -> Result<BlockKind, BlockError> {
    let (name, braces) = rest
        .split_once('{')
        .map_or((rest, ""), |(name, braces)| (name, braces));
    let name = name.trim();
    check_form(line, name, form)?;
    let list = match braces.strip_suffix('}') {
        Some(inside) => parse_attributes(inside).ok_or(BlockError::Attributes { line })?,
        None if braces.is_empty() => Vec::new(),
        None => return Err(BlockError::Attributes { line }),
    };
    let attributes = Attributes { line, list };
    match name {
        "callout" => callout(&attributes),
        "figure" => figure(&attributes),
        "toggle" => toggle(&attributes),
        "toc" => attributes
            .only("toc", &[], "nothing")
            .map(|()| BlockKind::Toc),
        "button" => button(&attributes),
        other => Err(BlockError::UnknownBlock {
            line,
            name: other.to_owned(),
        }),
    }
}

/// Refuses a known block written in the other form, naming the right one.
fn check_form(line: usize, name: &str, form: Form) -> Result<(), BlockError> {
    if !BLOCKS.contains(&name) {
        return Ok(());
    }
    let problem = match (form, LEAVES.contains(&name)) {
        (Form::Container, true) => format!("{name} has no body; write it on one line as ::{name}"),
        (Form::Leaf, false) => {
            format!("{name} has a body; open it with :::{name} and close it with :::")
        }
        _ => return Ok(()),
    };
    Err(BlockError::Form { line, problem })
}

fn callout(attributes: &Attributes) -> Result<BlockKind, BlockError> {
    attributes.only("callout", &["kind"], "kind")?;
    let kind = match attributes.get("kind").as_deref() {
        Some("note") | None => CalloutKind::Note,
        Some("tip") => CalloutKind::Tip,
        Some("warning") => CalloutKind::Warning,
        Some(other) => {
            return Err(BlockError::CalloutKind {
                line: attributes.line,
                kind: other.to_owned(),
            });
        }
    };
    Ok(BlockKind::Callout { kind })
}

fn figure(attributes: &Attributes) -> Result<BlockKind, BlockError> {
    attributes.only("figure", &["src", "alt"], "src and alt")?;
    let src = attributes.need("src", "a figure needs src, the address of the image")?;
    let alt = attributes.need(
        "alt",
        "a figure needs alt, the text that describes the image to people who cannot see it",
    )?;
    Ok(BlockKind::Figure { src, alt })
}

fn toggle(attributes: &Attributes) -> Result<BlockKind, BlockError> {
    attributes.only("toggle", &["summary"], "summary")?;
    let summary = attributes.need(
        "summary",
        "a toggle needs summary, the line a reader clicks to open it",
    )?;
    Ok(BlockKind::Toggle { summary })
}

fn button(attributes: &Attributes) -> Result<BlockKind, BlockError> {
    attributes.only("button", &["href", "label"], "href and label")?;
    let href = attributes.need("href", "a button needs href, the page it leads to")?;
    let label = attributes.need("label", "a button needs label, the words on the button")?;
    let safe = ["/", "#", "https:", "http:", "mailto:"]
        .iter()
        .any(|start| href.starts_with(start))
        || !href.contains(':');
    if !safe {
        return Err(BlockError::Link {
            line: attributes.line,
            href,
        });
    }
    Ok(BlockKind::Button { href, label })
}

/// A directive's attributes, with the line they are on for messages.
struct Attributes {
    line: usize,
    list: Vec<(String, String)>,
}

impl Attributes {
    /// Refuses any attribute but `allowed`; `takes` names them for people.
    fn only(
        &self,
        block: &'static str,
        allowed: &[&str],
        takes: &'static str,
    ) -> Result<(), BlockError> {
        match self
            .list
            .iter()
            .find(|(key, _)| !allowed.contains(&key.as_str()))
        {
            Some((key, _)) => Err(BlockError::UnknownAttribute {
                line: self.line,
                block,
                attribute: key.clone(),
                takes,
            }),
            None => Ok(()),
        }
    }

    fn get(&self, key: &str) -> Option<String> {
        self.list
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
    }

    /// The value of `key`, or `problem` as the error when it is missing.
    fn need(&self, key: &str, problem: &'static str) -> Result<String, BlockError> {
        self.get(key).ok_or(BlockError::Missing {
            line: self.line,
            problem,
        })
    }
}

/// Reads `name=value name="a value"` pairs, or `None` when they cannot be read.
fn parse_attributes(text: &str) -> Option<Vec<(String, String)>> {
    let mut found = Vec::new();
    let mut rest = text.trim();
    while !rest.is_empty() {
        let (key, after) = rest.split_once('=')?;
        let key = key.trim();
        if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return None;
        }
        let (value, after) = match after.strip_prefix('"') {
            Some(quoted) => {
                let (value, after) = quoted.split_once('"')?;
                if !(after.is_empty() || after.starts_with(' ')) {
                    return None;
                }
                (value, after)
            }
            None => after.split_once(' ').unwrap_or((after, "")),
        };
        if value.contains(['"', '{', '}']) {
            return None;
        }
        found.push((key.to_owned(), value.to_owned()));
        rest = after.trim_start();
    }
    Some(found)
}
