//! The page's own text as an answer engine lifts it: its subheadings and
//! paragraphs in reading order, the sections folded away, and its dates.

use scraper::{ElementRef, Html};

use crate::page::text_of;

/// One block of a page's own text, in reading order.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Block {
    /// A subheading, `h2` to `h6`, with its text.
    Heading(String),
    /// A paragraph.
    Paragraph(Paragraph),
}

/// One paragraph of a page's own text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paragraph {
    text: String,
    linked: bool,
}

impl Paragraph {
    /// The paragraph's text, with runs of white space made one space.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Whether the paragraph links anywhere, such as to a source.
    pub fn linked(&self) -> bool {
        self.linked
    }
}

/// What one page's own text holds, taken element by element.
///
/// A page's own text leaves out its header, menus, side panels, footer
/// and forms. On a page with an `<article>`, it is the text of the
/// outermost article only, so comments, which are articles inside it, are
/// not part of it.
#[derive(Debug, Clone, Default)]
pub(crate) struct Outline {
    pub(crate) blocks: Vec<Block>,
    pub(crate) folded: Vec<String>,
    pub(crate) dates: Vec<String>,
    has_article: bool,
}

impl Outline {
    /// An empty outline for `document`.
    pub(crate) fn new(document: &Html) -> Self {
        let has_article = document
            .root_element()
            .descendants()
            .filter_map(ElementRef::wrap)
            .any(|element| element.value().name() == "article");
        Self {
            has_article,
            ..Self::default()
        }
    }

    /// Takes what one element adds to the outline.
    pub(crate) fn element(&mut self, element: ElementRef<'_>) {
        let name = element.value().name();
        let attr = |name: &str| element.value().attr(name);
        if name == "time" {
            if let Some(at) = attr("datetime") {
                self.dates.push(at.trim().to_owned());
            }
            return;
        }
        if !matches!(name, "h2" | "h3" | "h4" | "h5" | "h6" | "p" | "details")
            || !self.is_own_text(element)
        {
            return;
        }
        let holds = |inner: &str| {
            element
                .descendants()
                .filter_map(ElementRef::wrap)
                .any(|child| child.value().name() == inner)
        };
        match name {
            "p" => {
                let text = text_of(element);
                if !text.is_empty() {
                    let linked = element
                        .descendants()
                        .filter_map(ElementRef::wrap)
                        .any(|child| {
                            child.value().name() == "a" && child.value().attr("href").is_some()
                        });
                    self.blocks
                        .push(Block::Paragraph(Paragraph { text, linked }));
                }
            }
            "details" => {
                if attr("open").is_none() && holds("p") && !holds("form") && !holds("nav") {
                    let summary = element
                        .children()
                        .filter_map(ElementRef::wrap)
                        .find(|child| child.value().name() == "summary")
                        .map(text_of)
                        .unwrap_or_default();
                    self.folded.push(summary);
                }
            }
            _ => self.blocks.push(Block::Heading(heading_text(element))),
        }
    }

    /// Whether `element` is part of the page's own text.
    fn is_own_text(&self, element: ElementRef<'_>) -> bool {
        let mut articles = 0_usize;
        for parent in element.ancestors().filter_map(ElementRef::wrap) {
            match parent.value().name() {
                "header" | "nav" | "aside" | "footer" | "form" => return false,
                "article" => articles = articles.saturating_add(1),
                _ => {}
            }
        }
        articles == usize::from(self.has_article)
    }
}

/// The text of the heading `element` without the text of links that point
/// within the page, such as the `#` a heading link adds after the words.
fn heading_text(element: ElementRef<'_>) -> String {
    let mut words: Vec<&str> = Vec::new();
    for node in element.descendants() {
        let Some(text) = node.value().as_text() else {
            continue;
        };
        let in_page_link = node.ancestors().filter_map(ElementRef::wrap).any(|parent| {
            parent.value().name() == "a"
                && parent
                    .value()
                    .attr("href")
                    .is_some_and(|href| href.starts_with('#'))
        });
        if !in_page_link {
            words.extend(text.split_whitespace());
        }
    }
    words.join(" ")
}
