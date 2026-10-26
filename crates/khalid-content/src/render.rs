//! Markdown to HTML, with the facts templates and the linter need beside it.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Mutex, OnceLock, PoisonError};

use comrak::adapters::{HeadingAdapter, HeadingMeta};
use comrak::nodes::{AstNode, NodeValue, Sourcepos};
use comrak::options::Plugins;
use comrak::plugins::syntect::{SyntectAdapter, SyntectAdapterBuilder};
use comrak::{Arena, Options, format_html_with_plugins, parse_document};
use khalid_core::{Locale, Slug, SlugMode};

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
/// text has no letters or digits.
pub fn render(body: &str, locale: &Locale, slugs: SlugMode) -> Rendered {
    let mut options = Options::default();
    options.extension.table = true;
    options.extension.footnotes = true;
    options.extension.tasklist = true;
    options.extension.strikethrough = true;
    options.extension.autolink = true;
    options.render.escape = true;

    let arena = Arena::new();
    let root = parse_document(&arena, body, &options);
    let mut facts = Facts::default();
    for node in root.descendants() {
        facts.note(node);
    }

    let ids = HeadingIds {
        locale: locale.clone(),
        slugs,
        state: Mutex::default(),
    };
    let mut plugins = Plugins::default();
    plugins.render.heading_adapter = Some(&ids);
    plugins.render.codefence_syntax_highlighter = Some(highlighter());
    let mut html = String::new();
    // Writing into a String cannot fail; an empty body is the safe fallback.
    if format_html_with_plugins(root, &options, &mut html, &plugins).is_err() {
        html.clear();
    }
    let state = ids
        .state
        .into_inner()
        .unwrap_or_else(PoisonError::into_inner);
    Rendered {
        html,
        headings: state.headings,
        links: facts.links,
        images: facts.images,
        words: facts.words,
    }
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
