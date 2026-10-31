//! The block model between markdown and the editor (RULES.md BL-FS-5).

use comrak::nodes::NodeValue;
use comrak::{Arena, Options, parse_document};
use khalid_core::{AbsoluteUrl, ParseUrlError};

use crate::{
    BusinessBlock, FormField, HowToBlock, ProductBlock, Question, RecipeBlock, ReviewBlock,
    VideoBlock,
};

use crate::directive::{Form, directive, opening, unknown_hint};

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
    /// `:::toggle{summary="..."}`: a body a reader opens by clicking the
    /// summary. It works with scripts off.
    Toggle {
        /// The line a reader clicks.
        summary: String,
    },
    /// `::toc`: a list of the body's headings, linked.
    Toc,
    /// `::button{href="..." label="..."}`: a link styled as a button.
    Button {
        /// Where it leads.
        href: String,
        /// The words on the button.
        label: String,
    },
    /// `::bookmark{href="..."}`: a link to another site's page, drawn as a
    /// card when the publish step has filled in its title, description and
    /// image. Rendering never fetches the page.
    Bookmark {
        /// The page it links to.
        href: AbsoluteUrl,
        /// The page's title.
        title: Option<String>,
        /// One line about the page.
        description: Option<String>,
        /// The page's preview image.
        image: Option<AbsoluteUrl>,
    },
    /// `::embed{src="..." title="..."}`: a player from one of the
    /// [`EMBED_PROVIDERS`](crate::EMBED_PROVIDERS) in a frame, or a plain link
    /// for any other address.
    Embed {
        /// The page a writer pasted, such as a video's page.
        src: AbsoluteUrl,
        /// The name a screen reader says for the player.
        title: String,
    },
    /// `::video{src="..." title="..." poster="..."}`: a video file with
    /// controls, which search engines read as a `VideoObject`.
    Video(VideoBlock),
    /// `::audio{src="..." title="..."}`: a sound file with controls.
    Audio {
        /// The sound file: a path on this site or a full address.
        src: String,
        /// What it is, shown above the player.
        title: String,
    },
    /// `::file{src="..." title="..."}`: a link that downloads a file.
    File {
        /// The file: a path on this site or a full address.
        src: String,
        /// The link's words.
        title: String,
    },
    /// `::::faq` holding one `:::question{text="..."}` per question, its
    /// answer as the body. Readers open each answer with scripts off.
    Faq {
        /// The questions, in order, with their answers in markdown.
        questions: Vec<Question>,
    },
    /// `::::howto{name="..." time="..."}` holding one `:::step{name="..."}`
    /// per step, what to do as the body.
    HowTo(HowToBlock),
    /// `:::review{item="..." type="..." rating="..."}`: a rating of one
    /// thing, the review as the body.
    Review(ReviewBlock),
    /// `::::recipe{name="..." ...}` holding a `:::ingredients` list and a
    /// `:::method` list.
    Recipe(RecipeBlock),
    /// `:::product{name="..." ...}`: a product card with a picture, price,
    /// rating and button, the description as the body.
    Product(ProductBlock),
    /// `::business{name="..." ...}`: a business's name, address and how to
    /// reach it.
    Business(BusinessBlock),
    /// `:::header{image="..." alt="..."}`: a banner with an optional
    /// picture, its words as the body.
    Header {
        /// The picture: a path on this site or a full address.
        image: Option<String>,
        /// The text that describes the picture, always with `image`.
        alt: Option<String>,
    },
    /// `:::html{show="..."}`: a writer's own HTML, sanitised with `ammonia`
    /// against an allowlist so it cannot run a script (RULES.md BL-SEC-1).
    SafeHtml {
        /// Who sees it.
        show: Visibility,
    },
    /// `:::form{id="..." button="..."}` with one field per line: a form that
    /// posts to `/forms/{id}`.
    Form {
        /// The name its answers are kept under.
        id: String,
        /// The words on its button.
        button: String,
        /// Its fields, in order.
        fields: Vec<FormField>,
    },
    /// `::newsletter{label="..." button="..."}`: an email field that posts
    /// to `/subscribe`.
    Newsletter {
        /// The words beside the email field.
        label: String,
        /// The words on the button.
        button: String,
    },
    /// `:::gated{note="..." label="..." button="..."}`: a body only
    /// subscribers read. The page shows the note and a subscribe form, never
    /// the body.
    Gated {
        /// The line that tells readers how to read the rest.
        note: String,
        /// The words beside the email field.
        label: String,
        /// The words on the button.
        button: String,
    },
    /// `:::gallery` with one `![alt](src)` image per line: a grid of
    /// pictures.
    Gallery {
        /// The pictures, in order.
        images: Vec<GalleryImage>,
    },
    /// `:::maths{label="..."}` with a LaTeX formula as the body: maths on its
    /// own, drawn as `MathML` that browsers show with scripts off.
    Maths {
        /// The caption under the formula, when there is one.
        label: Option<String>,
        /// The formula as written, without the lines around it.
        formula: String,
    },
}

/// One picture in a gallery, written `![alt](src)` on its own line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GalleryImage {
    /// The image's address.
    pub src: String,
    /// The text that describes the image.
    pub alt: String,
}

/// Who sees an HTML block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    /// Every reader.
    Everyone,
    /// Readers who are not signed in.
    Visitors,
    /// Signed-in members. Until readers can sign in, nobody sees it.
    Members,
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
    #[error("line {line}: there is no block called {name}; {}", unknown_hint(name))]
    UnknownBlock {
        /// The line of the opening `:::`.
        line: usize,
        /// The name as written.
        name: String,
    },
    /// The braces after the name do not hold `name=value` pairs.
    #[error(
        "line {line}: the attributes in braces cannot be read; write each as name=value, in double quotes when the value has a space"
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
    #[error(
        "line {line}: {} does not take {attribute}; it takes {takes}",
        a(block)
    )]
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
    /// A block written as a container when it is a one-line leaf, or the
    /// other way round.
    #[error("line {line}: {problem}")]
    Form {
        /// The line of the block.
        line: usize,
        /// Which form to use instead, in a sentence.
        problem: String,
    },
    /// An attribute's value is not written the way the block needs it.
    #[error("line {line}: {problem}")]
    Value {
        /// The line of the block or of the wrong line in its body.
        line: usize,
        /// How to write it, in a sentence.
        problem: &'static str,
    },
    /// A button whose link could run a script or leave the web.
    #[error(
        "line {line}: a button links to a page, a #section, https:, http: or mailto:, not {href}"
    )]
    Link {
        /// The line of the block.
        line: usize,
        /// The link as written.
        href: String,
    },
    /// An attribute that must be a full web address is not one.
    #[error("line {line}: {}'s {attribute} is wrong: {source}", a(block))]
    Address {
        /// The line of the block.
        line: usize,
        /// The block's name.
        block: &'static str,
        /// The attribute that holds the address.
        attribute: &'static str,
        /// Why the address cannot be used.
        #[source]
        source: ParseUrlError,
    },
    /// A maths block whose formula cannot be drawn.
    #[error("line {line}: the maths cannot be drawn: {problem}")]
    Maths {
        /// The formula's first line.
        line: usize,
        /// What the maths reader found, in a phrase.
        problem: String,
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

impl BlockError {
    /// The body line the problem is on, counting from 1.
    #[must_use]
    pub fn line(&self) -> usize {
        match self {
            Self::UnknownBlock { line, .. }
            | Self::Attributes { line }
            | Self::Missing { line, .. }
            | Self::UnknownAttribute { line, .. }
            | Self::CalloutKind { line, .. }
            | Self::Form { line, .. }
            | Self::Link { line, .. }
            | Self::Address { line, .. }
            | Self::Value { line, .. }
            | Self::Maths { line, .. }
            | Self::Unclosed { line, .. } => *line,
        }
    }

    /// What is wrong, in a sentence, without the line.
    #[must_use]
    pub fn problem(&self) -> String {
        let text = self.to_string();
        let prefix = format!("line {}: ", self.line());
        text.strip_prefix(&prefix)
            .map_or_else(|| text.clone(), str::to_owned)
    }
}

/// `block` with "a" or "an" before it.
fn a(block: &str) -> String {
    let article = if block.starts_with(['a', 'e', 'i', 'o', 'u']) {
        "an"
    } else {
        "a"
    };
    format!("{article} {block}")
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
        } else if let Some(("::", rest)) = opening(line) {
            blocks.extend(markdown_blocks(
                lines.get(run_start..index).unwrap_or_default(),
            ));
            let kind = directive(index.saturating_add(1), rest, Form::Leaf, &[])?;
            blocks.push(Block {
                kind,
                source: line.trim_end().to_owned(),
            });
            index = index.saturating_add(1);
            run_start = index;
            continue;
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
            let body = lines
                .get(index.saturating_add(1)..close)
                .unwrap_or_default();
            let kind = directive(line_number, rest, Form::Container, body)?;
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
