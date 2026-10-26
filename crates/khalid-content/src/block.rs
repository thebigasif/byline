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

/// What a block is.
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
}

/// Splits a markdown `body` into its top-level blocks, in order.
///
/// Parsing then printing with [`print_blocks`] returns the same text for any
/// body in canonical form: blocks separated by one blank line, and one newline
/// at the end. Two lists of the same kind in a row are one list in markdown, so
/// canonical form never has them. A footnote's text stays where it is written,
/// and one that nothing refers to is dropped, as markdown drops it.
pub fn parse_blocks(body: &str) -> Vec<Block> {
    let mut options = Options::default();
    options.extension.table = true;
    options.extension.footnotes = true;
    options.extension.tasklist = true;
    let arena = Arena::new();
    let root = parse_document(&arena, body, &options);
    let lines: Vec<&str> = body.lines().collect();
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
