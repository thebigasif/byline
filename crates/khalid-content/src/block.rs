//! The block model between markdown and the editor (RULES.md BL-FS-5).

use comrak::nodes::NodeValue;
use comrak::{Arena, Options, parse_document};

/// One top-level block of a body, with its markdown exactly as written.
///
/// The editor works block by block, and every block is stored as markdown, so
/// the same post can be written in a text file or in the editor.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Block {
    /// What the block is.
    pub kind: BlockKind,
    /// The block's markdown, without a trailing newline.
    pub source: String,
}

/// What a block is: standard markdown, or a directive (ADR 0012).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum BlockKind {
    /// A heading, `#` to `######`.
    Heading {
        /// 1 to 6.
        level: u8,
    },
    /// A paragraph.
    Paragraph,
    /// A quote, `>`.
    Quote,
    /// A bullet, numbered or task list.
    List,
    /// Fenced code.
    Code {
        /// The language after the opening fence, when there is one.
        language: Option<String>,
    },
    /// A divider, `---`.
    Divider,
    /// A table.
    Table,
    /// A footnote's text, `[^1]: ...`.
    Footnote,
    /// Raw HTML. It is shown as text, never run.
    Html,
    /// `:::callout{kind=...}`: a boxed note whose body is markdown.
    Callout {
        /// The kind of callout.
        kind: CalloutKind,
    },
    /// `:::figure{src="..." alt="..."}`: an image whose body is its caption.
    Figure {
        /// The image's address.
        src: String,
        /// The text that describes the image.
        alt: String,
    },
}

/// The kinds of callout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalloutKind {
    /// Background a reader may want.
    Note,
    /// Advice that saves time.
    Tip,
    /// Something that can go wrong.
    Warning,
}

/// Why a body's blocks could not be read. Lines count from 1 in the body.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum BlockError {
    /// A directive name Khalid does not know.
    #[error("line {line}: there is no block called {name}; the blocks are callout and figure")]
    UnknownBlock {
        /// The line of the opening `:::`.
        line: usize,
        /// The name as written.
        name: String,
    },
    /// The braces after the name do not hold `name=value` pairs.
    #[error(
        "line {line}: the attributes in braces cannot be read; write them as name=value, with the value in double quotes when it has a space"
    )]
    Attributes {
        /// The line of the opening `:::`.
        line: usize,
    },
    /// A required attribute is missing.
    #[error("line {line}: {problem}")]
    Missing {
        /// The line of the opening `:::`.
        line: usize,
        /// What is missing, in a sentence.
        problem: &'static str,
    },
    /// An attribute the block does not take.
    #[error("line {line}: a {block} does not take {attribute}; it takes {takes}")]
    UnknownAttribute {
        /// The line of the opening `:::`.
        line: usize,
        /// The block's name.
        block: &'static str,
        /// The attribute as written.
        attribute: String,
        /// The attributes the block takes.
        takes: &'static str,
    },
    /// A callout kind Khalid does not know.
    #[error("line {line}: a callout's kind is note, tip or warning, not {kind}")]
    CalloutKind {
        /// The line of the opening `:::`.
        line: usize,
        /// The kind as written.
        kind: String,
    },
    /// An opening `:::` with no closing `:::`.
    #[error("line {line}: the {name} block opened here is never closed with a ::: line")]
    Unclosed {
        /// The line of the opening `:::`.
        line: usize,
        /// The block's name.
        name: String,
    },
}

/// Splits a markdown `body` into its top-level blocks, in order.
///
/// Parsing then printing with [`print_blocks`] returns the same text for any
/// body in canonical form: blocks separated by one blank line, and one newline
/// at the end. Two lists of the same kind in a row are one list in markdown, so
/// canonical form never has them. A footnote's text stays where it is written,
/// and one that nothing refers to is dropped, as markdown drops it. A `:::`
/// inside fenced code is code, not a block.
///
/// # Errors
///
/// Returns a [`BlockError`] for a directive Khalid does not know, one whose
/// attributes are wrong or missing, or one that is never closed.
pub fn parse_blocks(body: &str) -> Result<Vec<Block>, BlockError> {
    let lines: Vec<&str> = body.lines().collect();
    let mut blocks = Vec::new();
    let mut run_start = 0;
    let mut fence: Option<String> = None;
    let mut index = 0;
    while let Some(line) = lines.get(index) {
        let trimmed = line.trim_start();
        if let Some(open) = &fence {
            if trimmed.starts_with(open.as_str()) {
                fence = None;
            }
        } else if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fence = Some(
                trimmed
                    .chars()
                    .take_while(|&c| c == '`' || c == '~')
                    .collect(),
            );
        } else if let Some((colons, rest)) = opening(line) {
            blocks.extend(markdown_blocks(
                lines.get(run_start..index).unwrap_or_default(),
            ));
            let close = lines
                .iter()
                .skip(index.saturating_add(1))
                .position(|line| line.trim_end() == colons)
                .map(|offset| index.saturating_add(1).saturating_add(offset));
            let line_number = index.saturating_add(1);
            let name = rest.split('{').next().unwrap_or_default().trim().to_owned();
            let Some(close) = close else {
                return Err(BlockError::Unclosed {
                    line: line_number,
                    name,
                });
            };
            let kind = directive(line_number, rest)?;
            let source = lines.get(index..=close).unwrap_or_default().join("\n");
            blocks.push(Block { kind, source });
            index = close.saturating_add(1);
            run_start = index;
            continue;
        }
        index = index.saturating_add(1);
    }
    blocks.extend(markdown_blocks(lines.get(run_start..).unwrap_or_default()));
    Ok(blocks)
}

/// Prints blocks in canonical form: one blank line between blocks, and one
/// newline at the end.
pub fn print_blocks(blocks: &[Block]) -> String {
    let mut text = blocks
        .iter()
        .map(|block| block.source.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    text.push('\n');
    text
}

/// A `:::name{...}` opening line: its run of colons and what follows them.
fn opening(line: &str) -> Option<(&str, &str)> {
    let colons = line.len() - line.trim_start_matches(':').len();
    let (fence, rest) = line.split_at_checked(colons)?;
    (colons >= 3 && rest.starts_with(|c: char| c.is_ascii_alphabetic()))
        .then_some((fence, rest.trim_end()))
}

/// Reads a directive's name and attributes into its kind.
fn directive(line: usize, rest: &str) -> Result<BlockKind, BlockError> {
    let (name, braces) = rest
        .split_once('{')
        .map_or((rest, ""), |(name, braces)| (name, braces));
    let attributes = match braces.strip_suffix('}') {
        Some(inside) => attributes(inside).ok_or(BlockError::Attributes { line })?,
        None if braces.is_empty() => Vec::new(),
        None => return Err(BlockError::Attributes { line }),
    };
    let take = |block: &'static str, allowed: &[&str], takes: &'static str| match attributes
        .iter()
        .find(|(key, _)| !allowed.contains(&key.as_str()))
    {
        Some((key, _)) => Err(BlockError::UnknownAttribute {
            line,
            block,
            attribute: key.clone(),
            takes,
        }),
        None => Ok(()),
    };
    let value = |key: &str| {
        attributes
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
    };
    match name.trim() {
        "callout" => {
            take("callout", &["kind"], "kind")?;
            let kind = match value("kind").as_deref() {
                Some("note") | None => CalloutKind::Note,
                Some("tip") => CalloutKind::Tip,
                Some("warning") => CalloutKind::Warning,
                Some(other) => {
                    return Err(BlockError::CalloutKind {
                        line,
                        kind: other.to_owned(),
                    });
                }
            };
            Ok(BlockKind::Callout { kind })
        }
        "figure" => {
            take("figure", &["src", "alt"], "src and alt")?;
            let src = value("src").ok_or(BlockError::Missing {
                line,
                problem: "a figure needs src, the address of the image",
            })?;
            let alt = value("alt").ok_or(BlockError::Missing {
                line,
                problem: "a figure needs alt, the text that describes the image to people who cannot see it",
            })?;
            Ok(BlockKind::Figure { src, alt })
        }
        other => Err(BlockError::UnknownBlock {
            line,
            name: other.to_owned(),
        }),
    }
}

/// Reads `name=value name="a value"` pairs, or `None` when they cannot be read.
fn attributes(text: &str) -> Option<Vec<(String, String)>> {
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

/// The standard markdown blocks in `lines`, in the order of the file.
fn markdown_blocks(lines: &[&str]) -> Vec<Block> {
    let text = lines.join("\n");
    let mut options = Options::default();
    options.extension.table = true;
    options.extension.footnotes = true;
    options.extension.tasklist = true;
    let arena = Arena::new();
    let root = parse_document(&arena, &text, &options);
    let mut blocks: Vec<(usize, Block)> = root
        .children()
        .filter_map(|node| {
            let data = node.data.borrow();
            let kind = match &data.value {
                NodeValue::Heading(heading) => BlockKind::Heading {
                    level: heading.level,
                },
                NodeValue::Paragraph => BlockKind::Paragraph,
                NodeValue::BlockQuote => BlockKind::Quote,
                NodeValue::List(_) => BlockKind::List,
                NodeValue::CodeBlock(code) => BlockKind::Code {
                    language: Some(
                        code.info
                            .split_whitespace()
                            .next()
                            .unwrap_or_default()
                            .to_owned(),
                    )
                    .filter(|language| !language.is_empty()),
                },
                NodeValue::ThematicBreak => BlockKind::Divider,
                NodeValue::Table(_) => BlockKind::Table,
                NodeValue::FootnoteDefinition(_) => BlockKind::Footnote,
                NodeValue::HtmlBlock(_) => BlockKind::Html,
                _ => return None,
            };
            let start = data.sourcepos.start.line.saturating_sub(1);
            let end = data.sourcepos.end.line;
            let source = lines.get(start..end).unwrap_or_default().join("\n");
            Some((
                start,
                Block {
                    kind,
                    source: source.trim_end().to_owned(),
                },
            ))
        })
        .collect();
    // Markdown moves footnote texts to the end; keep the order of the file.
    blocks.sort_by_key(|(start, _)| *start);
    blocks.into_iter().map(|(_, block)| block).collect()
}
