//! Markdown to HTML, with the facts templates and the linter need beside it.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Mutex, OnceLock, PoisonError};

use comrak::adapters::{HeadingAdapter, HeadingMeta};
use comrak::nodes::{AstNode, NodeValue, Sourcepos};
use comrak::options::Plugins;
use comrak::plugins::syntect::{SyntectAdapter, SyntectAdapterBuilder};
use comrak::{Arena, Options, format_html_with_plugins, parse_document};
use khalid_core::{AbsoluteUrl, Locale, Slug, SlugMode};

use crate::{Block, BlockError, BlockKind, CalloutKind, StructuredBlock, embed, parse_blocks};

mod cards;
mod forms;
mod media;
mod recipe;
mod steps;

/// A body rendered to HTML.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Rendered {
    /// The HTML of the body, without a wrapping element.
    pub html: String,
    /// Every heading in order, for a table of contents.
    pub headings: Vec<Heading>,
    /// Every link in document order, for the link graph and the anchor rules.
    pub links: Vec<Link>,
    /// Every image in document order, for the alt-text rules.
    pub images: Vec<Image>,
    /// The words a reader reads, for reading time and thin-post checks.
    pub words: usize,
    /// The blocks search engines read as schema, in order, for the schema
    /// builders.
    pub structured: Vec<StructuredBlock>,
}

/// One heading of a rendered body.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Heading {
    /// 1 to 6.
    pub level: u8,
    /// The `id` attribute, unique in the body, for links to this heading.
    pub id: String,
    /// The heading's text, without markup.
    pub text: String,
}

/// One link of a rendered body.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Link {
    /// The address as written.
    pub href: String,
    /// The anchor text, without markup.
    pub text: String,
    /// Whether the address stays on this site: a path or a `#` fragment, with
    /// no scheme. An absolute address to the site's own host counts as
    /// outside here; the linter, which knows the address, refines that.
    pub internal: bool,
}

/// One image of a rendered body.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Image {
    /// The image's address as written.
    pub src: String,
    /// The alt text, or `None` when it is empty.
    pub alt: Option<String>,
}

/// Renders a markdown `body` written in `locale`, on a site whose slugs follow
/// `slugs`.
///
/// Tables, footnotes, task lists, strikethrough and bare links are on. Raw
/// HTML in the markdown is escaped and shows as text, so a post cannot carry
/// a script (RULES.md BL-SEC-1). Fenced code with a language is coloured with
/// CSS classes prefixed `hl-`, such as `hl-keyword`, so the stylesheet owns
/// the colours. Each heading gets an `id` made with the same
/// rules as slugs, with `-2`, `-3` added to repeats, and `section` when the
/// text has no letters or digits. A callout becomes an `<aside>` and a figure a
/// `<figure>` with its caption (ADR 0012).
///
/// # Errors
///
/// Returns a [`BlockError`] when a directive block is wrong, as
/// [`parse_blocks`](crate::parse_blocks) does.
pub fn render(body: &str, locale: &Locale, slugs: SlugMode) -> Result<Rendered, BlockError> {
    let blocks = parse_blocks(body)?;
    let mut renderer = Renderer {
        ids: HeadingIds {
            locale: locale.clone(),
            slugs,
            state: Mutex::default(),
        },
        facts: Facts::default(),
        html: String::new(),
    };
    let mut run: Vec<&str> = Vec::new();
    for block in &blocks {
        if is_markdown(&block.kind) {
            run.push(&block.source);
        } else {
            renderer.markdown(&run.join("\n\n"));
            run.clear();
            renderer.directive(block);
        }
    }
    renderer.markdown(&run.join("\n\n"));
    let state = renderer
        .ids
        .state
        .into_inner()
        .unwrap_or_else(PoisonError::into_inner);
    let html = if renderer.html.contains(TOC_MARK) {
        renderer.html.replace(TOC_MARK, &contents(&state.headings))
    } else {
        renderer.html
    };
    Ok(Rendered {
        html,
        headings: state.headings,
        links: renderer.facts.links,
        images: renderer.facts.images,
        words: renderer.facts.words,
        structured: renderer.facts.structured,
    })
}

/// Whether `kind` is standard markdown, which renders in runs, rather than a
/// directive, which renders on its own.
fn is_markdown(kind: &BlockKind) -> bool {
    matches!(
        kind,
        BlockKind::Heading { .. }
            | BlockKind::Paragraph
            | BlockKind::Quote
            | BlockKind::List
            | BlockKind::Code { .. }
            | BlockKind::Divider
            | BlockKind::Table
            | BlockKind::Footnote
            | BlockKind::Html
    )
}

/// Renders runs of markdown into one page, keeping heading ids unique across
/// the whole body. Footnotes resolve within the run of markdown they sit in.
struct Renderer {
    ids: HeadingIds,
    facts: Facts,
    html: String,
}

impl Renderer {
    /// Writes one directive block.
    fn directive(&mut self, block: &Block) {
        match &block.kind {
            BlockKind::Callout { kind } => {
                let kind = match kind {
                    CalloutKind::Note => "note",
                    CalloutKind::Tip => "tip",
                    CalloutKind::Warning => "warning",
                };
                self.push(&["<aside class=\"callout callout-", kind, "\">\n"]);
                self.markdown(&inside(&block.source));
                self.html.push_str("</aside>\n");
            }
            BlockKind::Figure { src, alt } => {
                self.facts.images.push(Image {
                    src: src.clone(),
                    alt: Some(alt.clone()),
                });
                self.push(&[
                    "<figure>\n<img src=\"",
                    &attribute(src),
                    "\" alt=\"",
                    &attribute(alt),
                    "\" loading=\"lazy\" decoding=\"async\" />\n<figcaption>\n",
                ]);
                self.markdown(&inside(&block.source));
                self.html.push_str("</figcaption>\n</figure>\n");
            }
            BlockKind::Toggle { summary } => {
                self.push(&["<details>\n<summary>", &attribute(summary), "</summary>\n"]);
                self.markdown(&inside(&block.source));
                self.html.push_str("</details>\n");
            }
            BlockKind::Toc => self.html.push_str(TOC_MARK),
            BlockKind::Button { href, label } => {
                self.push(&[
                    "<p><a class=\"button\" href=\"",
                    &attribute(href),
                    "\">",
                    &attribute(label),
                    "</a></p>\n",
                ]);
                self.facts.links.push(Link {
                    internal: !href.contains(':'),
                    href: href.clone(),
                    text: label.clone(),
                });
            }
            BlockKind::Bookmark {
                href,
                title,
                description,
                image,
            } => self.bookmark(
                href,
                title.as_deref(),
                description.as_deref(),
                image.as_ref(),
            ),
            BlockKind::Embed { src, title } => self.embed(src, title),
            BlockKind::Video(video) => self.video(video),
            BlockKind::Audio { src, title } => self.audio(src, title),
            BlockKind::File { src, title } => self.file(src, title),
            BlockKind::Gallery { images } => self.gallery(images),
            BlockKind::Faq { questions } => self.faq(questions),
            BlockKind::HowTo(how_to) => self.how_to(how_to),
            BlockKind::Review(review) => self.review(review),
            BlockKind::Recipe(recipe) => self.recipe(recipe),
            BlockKind::Product(product) => self.product(product),
            BlockKind::Business(card) => self.business(card),
            BlockKind::SafeHtml { show } => self.safe_html(block, *show),
            BlockKind::Form { id, button, fields } => self.form(id, button, fields),
            BlockKind::Header { image, alt } => {
                self.header(block, image.as_deref(), alt.as_deref());
            }
            _ => {}
        }
    }

    /// Writes a bookmark: a plain link, or a card when the publish step has
    /// filled in the page's title.
    fn bookmark(
        &mut self,
        href: &AbsoluteUrl,
        title: Option<&str>,
        description: Option<&str>,
        image: Option<&AbsoluteUrl>,
    ) {
        let text = title.unwrap_or(href.as_str());
        self.link_out(href, text);
        self.push(&[
            "<p class=\"bookmark\"><a href=\"",
            &attribute(href.as_str()),
            "\">",
        ]);
        if let Some(image) = image {
            self.push(&[
                "<img src=\"",
                &attribute(image.as_str()),
                "\" alt=\"\" width=\"160\" height=\"90\" loading=\"lazy\" decoding=\"async\" />",
            ]);
        }
        if title.is_some() {
            self.push(&[
                "<span class=\"bookmark-title\">",
                &attribute(text),
                "</span>",
            ]);
        } else {
            self.html.push_str(&attribute(text));
        }
        if let Some(description) = description {
            self.push(&[
                "<span class=\"bookmark-description\">",
                &attribute(description),
                "</span>",
            ]);
        }
        if title.is_some() {
            self.push(&[
                "<span class=\"bookmark-host\">",
                &attribute(href.host()),
                "</span>",
            ]);
        }
        self.html.push_str("</a></p>\n");
    }

    /// Writes an embed: an allowed provider's player in a lazy, sandboxed
    /// frame of fixed size with the page's link under it, or the link alone.
    fn embed(&mut self, src: &AbsoluteUrl, title: &str) {
        self.link_out(src, title);
        let link = [
            "<a href=\"",
            &attribute(src.as_str()),
            "\">",
            &attribute(title),
            "</a>",
        ]
        .concat();
        let Some((provider, player)) = embed::player(src) else {
            self.push(&["<p class=\"bookmark\">", &link, "</p>\n"]);
            return;
        };
        let (shape, height) = if provider.is_audio() {
            ("audio", "152")
        } else {
            ("wide", "315")
        };
        self.push(&[
            "<figure class=\"embed embed-",
            shape,
            "\">\n<iframe src=\"",
            &attribute(&player),
            "\" title=\"",
            &attribute(title),
            "\" width=\"560\" height=\"",
            height,
            "\" loading=\"lazy\" referrerpolicy=\"strict-origin-when-cross-origin\" sandbox=\"allow-scripts allow-same-origin allow-popups allow-presentation\" allow=\"encrypted-media; fullscreen; picture-in-picture\"></iframe>\n<figcaption>",
            &link,
            "</figcaption>\n</figure>\n",
        ]);
    }

    /// Records a link to another site.
    fn link_out(&mut self, href: &AbsoluteUrl, text: &str) {
        self.facts.links.push(Link {
            href: href.as_str().to_owned(),
            text: text.to_owned(),
            internal: false,
        });
    }

    /// Appends `parts` to the HTML in order.
    fn push(&mut self, parts: &[&str]) {
        for part in parts {
            self.html.push_str(part);
        }
    }

    fn markdown(&mut self, text: &str) {
        if text.trim().is_empty() {
            return;
        }
        let mut options = Options::default();
        options.extension.table = true;
        options.extension.footnotes = true;
        options.extension.tasklist = true;
        options.extension.strikethrough = true;
        options.extension.autolink = true;
        options.render.escape = true;
        let arena = Arena::new();
        let root = parse_document(&arena, text, &options);
        for node in root.descendants() {
            self.facts.note(node);
        }
        let mut plugins = Plugins::default();
        plugins.render.heading_adapter = Some(&self.ids);
        plugins.render.codefence_syntax_highlighter = Some(highlighter());
        let mut html = String::new();
        // Writing into a String cannot fail; an empty run is the safe fallback.
        if format_html_with_plugins(root, &options, &mut html, &plugins).is_ok() {
            self.html.push_str(&html);
            if !self.html.ends_with('\n') {
                self.html.push('\n');
            }
        }
    }
}

/// Where `::toc` goes until the headings are known. Raw HTML in markdown is
/// escaped, so no post can write this comment itself.
const TOC_MARK: &str = "<!--khalid-toc-->\n";

/// The list of `headings`, linked, for `::toc`.
fn contents(headings: &[Heading]) -> String {
    let mut html = String::from("<nav class=\"toc\" aria-label=\"Contents\">\n<ol>\n");
    for heading in headings {
        for part in [
            "<li class=\"toc-h",
            &heading.level.to_string(),
            "\"><a href=\"#",
            &attribute(&heading.id),
            "\">",
            &attribute(&heading.text),
            "</a></li>\n",
        ] {
            html.push_str(part);
        }
    }
    html.push_str("</ol>\n</nav>\n");
    html
}

/// The body of a directive: every line between its opening and closing `:::`.
fn inside(source: &str) -> String {
    let lines: Vec<&str> = source.lines().collect();
    lines
        .get(1..lines.len().saturating_sub(1))
        .unwrap_or_default()
        .join("\n")
}

/// `text` made safe inside a double-quoted HTML attribute.
fn attribute(text: &str) -> String {
    let mut safe = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => safe.push_str("&amp;"),
            '<' => safe.push_str("&lt;"),
            '>' => safe.push_str("&gt;"),
            '"' => safe.push_str("&quot;"),
            '\'' => safe.push_str("&#39;"),
            _ => safe.push(c),
        }
    }
    safe
}

/// The code highlighter, built once: loading its grammars takes milliseconds
/// that a render must not pay each time (RULES.md BL-PERF-2).
// If bincode or yaml-rust under syntect gets a real vulnerability (deny.toml),
// swap to plain code blocks coloured at build time by a tree-sitter
// highlighter such as arborium.
fn highlighter() -> &'static SyntectAdapter {
    static HIGHLIGHTER: OnceLock<SyntectAdapter> = OnceLock::new();
    HIGHLIGHTER.get_or_init(|| {
        SyntectAdapterBuilder::new()
            .css_with_class_prefix("hl-")
            .build()
    })
}

/// What a walk over the parsed body collects.
#[derive(Default)]
struct Facts {
    links: Vec<Link>,
    images: Vec<Image>,
    words: usize,
    structured: Vec<StructuredBlock>,
}

impl Facts {
    fn note<'a>(&mut self, node: &'a AstNode<'a>) {
        match &node.data.borrow().value {
            NodeValue::Text(text) => self.words += text.split_whitespace().count(),
            NodeValue::Code(code) => self.words += code.literal.split_whitespace().count(),
            NodeValue::Link(link) => self.links.push(Link {
                internal: !link.url.contains(':'),
                href: link.url.clone(),
                text: text_of(node),
            }),
            NodeValue::Image(image) => {
                let alt = text_of(node);
                self.images.push(Image {
                    src: image.url.clone(),
                    alt: (!alt.trim().is_empty()).then_some(alt),
                });
            }
            _ => {}
        }
    }
}

/// The plain text inside a node.
fn text_of<'a>(node: &'a AstNode<'a>) -> String {
    node.descendants()
        .filter_map(|child| match &child.data.borrow().value {
            NodeValue::Text(text) => Some(text.to_string()),
            NodeValue::Code(code) => Some(code.literal.clone()),
            _ => None,
        })
        .collect()
}

/// Writes `<hN id="...">` and records each heading as it is written.
struct HeadingIds {
    locale: Locale,
    slugs: SlugMode,
    state: Mutex<IdState>,
}

#[derive(Default)]
struct IdState {
    headings: Vec<Heading>,
    seen: BTreeMap<String, usize>,
}

impl HeadingAdapter for HeadingIds {
    fn enter(
        &self,
        output: &mut dyn fmt::Write,
        heading: &HeadingMeta,
        _: Option<Sourcepos>,
    ) -> fmt::Result {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let base = Slug::from_title(&heading.content, &self.locale, self.slugs)
            .map_or_else(|_| "section".to_owned(), |slug| slug.as_str().to_owned());
        let count = state.seen.entry(base.clone()).or_insert(0);
        *count += 1;
        let id = if *count == 1 {
            base
        } else {
            format!("{base}-{count}")
        };
        write!(output, "<h{} id=\"{id}\">", heading.level)?;
        state.headings.push(Heading {
            level: heading.level,
            id,
            text: heading.content.clone(),
        });
        Ok(())
    }

    fn exit(&self, output: &mut dyn fmt::Write, heading: &HeadingMeta) -> fmt::Result {
        write!(output, "</h{}>", heading.level)
    }
}
