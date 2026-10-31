//! One HTML page, read once into what the checks need.

use std::collections::BTreeMap;

use khalid_core::AbsoluteUrl;
use scraper::{ElementRef, Html};
use url::Url;

use crate::outline::Outline;
use crate::{Alternate, Block, Image, Link, Robots};

/// One HTML page as the checks see it.
#[derive(Debug, Clone, Default)]
pub struct Page {
    titles: Vec<String>,
    descriptions: Vec<String>,
    headings: Vec<String>,
    language: Option<String>,
    robots: Robots,
    canonical: Option<AbsoluteUrl>,
    alternates: Vec<Alternate>,
    social: BTreeMap<String, String>,
    structured_data: Vec<String>,
    links: Vec<Link>,
    images: Vec<Image>,
    resources: Vec<AbsoluteUrl>,
    words: usize,
    text_bytes: usize,
    outline: Outline,
}

impl Page {
    /// Reads the page at `url` from its `html`, in one walk. Relative links
    /// resolve against `url`. Bytes that are not UTF-8 read as U+FFFD.
    pub fn read(url: &AbsoluteUrl, html: &[u8]) -> Self {
        let document = Html::parse_document(&String::from_utf8_lossy(html));
        let Ok(base) = Url::parse(url.as_str()) else {
            return Self::default();
        };
        let mut page = Self {
            outline: Outline::new(&document),
            ..Self::default()
        };
        // A stack rather than recursion, so no nesting depth can overflow.
        // Each element carries whether its text is text a reader sees.
        let mut stack = vec![(document.root_element(), false)];
        while let Some((element, seen)) = stack.pop() {
            page.element(element, &base);
            page.outline.element(element);
            let name = element.value().name();
            let seen = (seen || name == "body")
                && !matches!(name, "script" | "style" | "template" | "noscript");
            let mut inner = Vec::new();
            for child in element.children() {
                if let Some(child) = ElementRef::wrap(child) {
                    inner.push((child, seen));
                } else if let Some(text) = child.value().as_text()
                    && seen
                {
                    page.words += text.split_whitespace().count();
                    page.text_bytes += text.trim().len();
                }
            }
            stack.extend(inner.into_iter().rev());
        }
        page
    }

    /// Takes what one element says about the page.
    fn element(&mut self, element: ElementRef<'_>, base: &Url) {
        let attr = |name: &str| element.value().attr(name);
        let address = |name: &str| attr(name).and_then(|href| resolve(base, href));
        match element.value().name() {
            "html" => self.language = attr("lang").map(str::to_owned),
            "title" if within(element, "head") => self.titles.push(text_of(element)),
            "h1" => self.headings.push(text_of(element)),
            "meta" => self.meta(attr("name").or(attr("property")), attr("content")),
            "link" => {
                let rel = attr("rel").unwrap_or_default();
                let has = |word: &str| rel.split_whitespace().any(|said| said == word);
                match (address("href"), attr("hreflang")) {
                    (Some(to), Some(language)) if has("alternate") => {
                        self.alternates.push(Alternate {
                            language: language.to_owned(),
                            to,
                        });
                    }
                    (Some(to), _) if has("canonical") => self.canonical = Some(to),
                    (Some(to), _) if has("stylesheet") => self.resources.push(to),
                    _ => {}
                }
            }
            "script" if attr("type") == Some("application/ld+json") => {
                self.structured_data.push(element.text().collect());
            }
            "script" => self.resources.extend(address("src")),
            "img" => {
                if let Some(src) = address("src") {
                    self.resources.push(src.clone());
                    self.images.push(Image {
                        src,
                        alt: attr("alt").map(str::to_owned),
                    });
                }
            }
            "a" => {
                let rel = attr("rel").unwrap_or_default();
                if let Some(to) = address("href") {
                    self.links.push(Link {
                        to,
                        text: link_text(element),
                        nofollow: rel.split_whitespace().any(|word| word == "nofollow"),
                    });
                }
            }
            _ => {}
        }
    }

    /// Takes one `<meta>` tag named `name` that says `content`.
    fn meta(&mut self, name: Option<&str>, content: Option<&str>) {
        let (Some(name), Some(content)) = (name, content) else {
            return;
        };
        let name = name.to_ascii_lowercase();
        match name.as_str() {
            "description" => self.descriptions.push(content.trim().to_owned()),
            "robots" | "googlebot" => self.robots = self.robots.with(content),
            social if social.starts_with("og:") || social.starts_with("twitter:") => {
                self.social.insert(name, content.trim().to_owned());
            }
            _ => {}
        }
    }

    /// Every `<title>` in the head, in order.
    pub fn titles(&self) -> &[String] {
        &self.titles
    }

    /// Every meta description, in order.
    pub fn descriptions(&self) -> &[String] {
        &self.descriptions
    }

    /// The text of every `<h1>`, in order.
    pub fn headings(&self) -> &[String] {
        &self.headings
    }

    /// The `lang` of the `<html>` element.
    pub fn language(&self) -> Option<&str> {
        self.language.as_deref()
    }

    /// What the robots meta tags ask.
    pub fn robots(&self) -> Robots {
        self.robots
    }

    /// The canonical address, when the page names one.
    pub fn canonical(&self) -> Option<&AbsoluteUrl> {
        self.canonical.as_ref()
    }

    /// The translations the page names, with `x-default`.
    pub fn alternates(&self) -> &[Alternate] {
        &self.alternates
    }

    /// The Open Graph and Twitter tags, by name, such as `og:title`.
    pub fn social(&self) -> &BTreeMap<String, String> {
        &self.social
    }

    /// The text of every JSON-LD block, in order.
    pub fn structured_data(&self) -> &[String] {
        &self.structured_data
    }

    /// Every link with a web address, in order.
    pub fn links(&self) -> &[Link] {
        &self.links
    }

    /// Every image with a source, in order.
    pub fn images(&self) -> &[Image] {
        &self.images
    }

    /// Every stylesheet, script and image the page loads, in order.
    pub fn resources(&self) -> &[AbsoluteUrl] {
        &self.resources
    }

    /// How many words the reader sees, outside scripts and styles.
    pub fn words(&self) -> usize {
        self.words
    }

    /// How many bytes of text the reader sees.
    pub fn text_bytes(&self) -> usize {
        self.text_bytes
    }

    /// The subheadings and paragraphs of the page's own text, in reading
    /// order, without its header, menus, footer, forms or comments.
    pub fn outline(&self) -> &[Block] {
        &self.outline.blocks
    }

    /// The summary of each closed `<details>` in the page's own text that
    /// folds paragraphs away, such as a question whose answer starts
    /// hidden. A folded menu or form does not count.
    pub fn folded(&self) -> &[String] {
        &self.outline.folded
    }

    /// The `datetime` of every `<time>` on the page, in order.
    pub fn dates(&self) -> &[String] {
        &self.outline.dates
    }
}

/// `href` against `base`, as a web address with no fragment, or `None`
/// for any other scheme, such as `mailto:`.
fn resolve(base: &Url, href: &str) -> Option<AbsoluteUrl> {
    let mut to = base.join(href.trim()).ok()?;
    to.set_fragment(None);
    AbsoluteUrl::parse(to.as_str()).ok()
}

/// Whether `element` sits inside an element named `name`.
fn within(element: ElementRef<'_>, name: &str) -> bool {
    element
        .ancestors()
        .filter_map(ElementRef::wrap)
        .any(|parent| parent.value().name() == name)
}

/// The text of `element`, with runs of white space made one space.
pub(crate) fn text_of(element: ElementRef<'_>) -> String {
    let text: String = element.text().collect();
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A link's text, or the `alt` of the first image inside it when it has
/// none.
fn link_text(element: ElementRef<'_>) -> String {
    let text = text_of(element);
    if !text.is_empty() {
        return text;
    }
    element
        .descendants()
        .filter_map(ElementRef::wrap)
        .find(|inner| inner.value().name() == "img")
        .and_then(|image| image.value().attr("alt"))
        .unwrap_or_default()
        .trim()
        .to_owned()
}
