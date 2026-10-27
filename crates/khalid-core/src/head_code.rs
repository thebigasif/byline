//! A post's own code for the page head, such as an analytics snippet,
//! checked once and written back in one safe form.

use std::fmt;

use serde::{Deserialize, Serialize};

/// HTML for a post's `<head>`: only `script`, `style`, `meta` and `link`
/// elements, and comments, which are dropped.
///
/// [`HeadCode::try_new`] refuses anything else rather than guessing:
/// text or body elements, which would end the head early; `<base>`, which
/// would move every link; `<meta http-equiv>`, which could refresh the page
/// away or loosen its security policy; event attributes such as `onload`;
/// and a `script` or `style` that is never closed. What it keeps is
/// written again with every attribute quoted, so the head is well formed
/// whatever was pasted. Each `script` and `style` gets the page's nonce
/// when it is drawn ([`HeadCode::to_html`]), so the security policy lets
/// it run (RULES.md BL-SEC-2).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct HeadCode {
    elements: Vec<Element>,
}

/// An element's attributes as names and safe values, in their order.
type Attributes = Vec<(String, String)>;

/// One element the head code keeps.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Element {
    name: &'static str,
    attributes: Attributes,
    /// What is between the tags; `None` for `meta` and `link`.
    content: Option<String>,
}

/// The elements head code may hold, and whether each has content.
const ALLOWED: [(&str, bool); 4] = [
    ("script", true),
    ("style", true),
    ("meta", false),
    ("link", false),
];

impl HeadCode {
    /// The most head code a post may have, in bytes. An analytics snippet
    /// is under 1 KB; 16 KB leaves room for several.
    pub const MAX_LEN: usize = 16 * 1024;

    /// Reads `html` and keeps it in its checked form.
    ///
    /// # Errors
    ///
    /// Returns [`HeadCodeError`] naming the first thing that may not be
    /// in a page head.
    pub fn try_new(html: &str) -> Result<Self, HeadCodeError> {
        if html.len() > Self::MAX_LEN {
            return Err(HeadCodeError::TooLong { len: html.len() });
        }
        let mut elements = Vec::new();
        let mut rest = html.trim_start();
        while !rest.is_empty() {
            if let Some(after) = rest.strip_prefix("<!--") {
                let (_, after) = after
                    .split_once("-->")
                    .ok_or(HeadCodeError::Unclosed { name: "comment" })?;
                rest = after.trim_start();
                continue;
            }
            let (element, after) = element(rest)?;
            elements.push(element);
            rest = after.trim_start();
        }
        Ok(Self { elements })
    }

    /// Whether there is nothing to write, for head code that held only
    /// comments or spaces.
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// The head code as HTML, with `nonce` on each `script` and `style`.
    pub fn to_html(&self, nonce: &str) -> String {
        self.written(Some(nonce))
    }

    /// One element a line, each with `nonce` when there is one.
    fn written(&self, nonce: Option<&str>) -> String {
        let mut html = String::new();
        for element in &self.elements {
            if !html.is_empty() {
                html.push('\n');
            }
            write_element(&mut html, element, nonce);
        }
        html
    }
}

impl fmt::Display for HeadCode {
    /// The checked form, without nonces, as front matter keeps it.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.written(None))
    }
}

impl TryFrom<String> for HeadCode {
    type Error = HeadCodeError;

    fn try_from(html: String) -> Result<Self, Self::Error> {
        Self::try_new(&html)
    }
}

impl From<HeadCode> for String {
    fn from(code: HeadCode) -> Self {
        code.to_string()
    }
}

/// Writes `element` into `html`, with `nonce` when it runs code.
fn write_element(html: &mut String, element: &Element, nonce: Option<&str>) {
    let name = element.name;
    html.push('<');
    html.push_str(name);
    for (key, value) in &element.attributes {
        html.push(' ');
        html.push_str(key);
        if !value.is_empty() {
            html.push_str("=\"");
            html.push_str(value);
            html.push('"');
        }
    }
    if let Some(nonce) = nonce.filter(|_| matches!(name, "script" | "style")) {
        html.push_str(" nonce=\"");
        html.push_str(nonce);
        html.push('"');
    }
    html.push('>');
    if let Some(content) = &element.content {
        html.push_str(content);
        html.push_str("</");
        html.push_str(name);
        html.push('>');
    }
}

/// Reads the element at the start of `html`, and returns it with what
/// follows it.
fn element(html: &str) -> Result<(Element, &str), HeadCodeError> {
    let Some(after) = html.strip_prefix('<') else {
        let text: String = html.chars().take(20).collect();
        return Err(HeadCodeError::Text { text });
    };
    let name_len = after
        .find(|c: char| !c.is_ascii_alphanumeric())
        .unwrap_or(after.len());
    let (raw_name, after) = after.split_at(name_len);
    // A closing tag, such as </head>, has no name; show what was there.
    let lower = match raw_name {
        "" => html.chars().take(7).collect(),
        name => name.to_ascii_lowercase(),
    };
    let Some(&(name, has_content)) = ALLOWED.iter().find(|(allowed, _)| *allowed == lower) else {
        return Err(HeadCodeError::Element { name: lower });
    };
    let (attributes, after) = attributes(after)?;
    if name == "meta" && attributes.iter().any(|(key, _)| key == "http-equiv") {
        return Err(HeadCodeError::HttpEquiv);
    }
    let mut element = Element {
        name,
        attributes,
        content: None,
    };
    if !has_content {
        return Ok((element, after));
    }
    let close = format!("</{name}");
    // Lowercasing ASCII keeps every byte where it was.
    let at = after.to_ascii_lowercase().find(&close);
    let (content, rest) = after.split_at(at.ok_or(HeadCodeError::Unclosed { name })?);
    let rest = rest.get(close.len()..).unwrap_or_default().trim_start();
    let rest = rest
        .strip_prefix('>')
        .ok_or(HeadCodeError::Unclosed { name })?;
    // A comment opener inside a script changes how browsers find its end.
    if content.contains("<!--") {
        return Err(HeadCodeError::CommentInCode { name });
    }
    element.content = Some(content.to_owned());
    Ok((element, rest))
}

/// Reads attributes up to and including the `>` that ends a start tag.
fn attributes(mut html: &str) -> Result<(Attributes, &str), HeadCodeError> {
    let mut attributes = Vec::new();
    loop {
        html = html.trim_start().trim_start_matches('/').trim_start();
        if let Some(rest) = html.strip_prefix('>') {
            return Ok((attributes, rest));
        }
        let name_len = html
            .find(|c: char| c.is_whitespace() || matches!(c, '=' | '>' | '/' | '"' | '\'' | '<'))
            .unwrap_or(html.len());
        if name_len == 0 {
            return Err(HeadCodeError::Unclosed { name: "tag" });
        }
        let (name, rest) = html.split_at(name_len);
        let name = name.to_ascii_lowercase();
        if name.starts_with("on") {
            return Err(HeadCodeError::EventAttribute { name });
        }
        let rest = rest.trim_start();
        let (value, rest) = match rest.strip_prefix('=') {
            Some(rest) => value(rest.trim_start())?,
            None => (String::new(), rest),
        };
        // The page gives its own nonce; a pasted one is from another page.
        if name != "nonce" {
            attributes.push((name, value));
        }
        html = rest;
    }
}

/// Reads one attribute value, quoted or not, and makes it safe inside
/// double quotes.
fn value(html: &str) -> Result<(String, &str), HeadCodeError> {
    let unclosed = HeadCodeError::Unclosed {
        name: "attribute value",
    };
    let (raw, rest) = if let Some(quote @ ('"' | '\'')) = html.chars().next() {
        let inside = html.get(1..).unwrap_or_default();
        let end = inside.find(quote).ok_or(unclosed)?;
        let (raw, rest) = inside.split_at(end);
        (raw, rest.get(1..).unwrap_or_default())
    } else {
        let end = html
            .find(|c: char| c.is_whitespace() || c == '>')
            .ok_or(unclosed)?;
        html.split_at(end)
    };
    let safe = raw
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    Ok((safe, rest))
}

/// Why head code was refused, naming what to change.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum HeadCodeError {
    /// More than [`HeadCode::MAX_LEN`] bytes.
    #[error("head code is {len} bytes, over 16384; link large scripts from a file instead")]
    TooLong {
        /// How long it was.
        len: usize,
    },
    /// Text outside any element.
    #[error("head code has text outside a tag, starting {text:?}; keep only the tags")]
    Text {
        /// The start of the text.
        text: String,
    },
    /// An element a page head may not hold here.
    #[error("head code may hold only script, style, meta and link, not {name}")]
    Element {
        /// The element's name.
        name: String,
    },
    /// A `meta` with `http-equiv`.
    #[error(
        "head code may not use meta http-equiv, which can reload the page or weaken its policy"
    )]
    HttpEquiv,
    /// An attribute that runs code, such as `onload`.
    #[error("head code may not use {name}, which runs code from an attribute; use a script")]
    EventAttribute {
        /// The attribute's name.
        name: String,
    },
    /// Something opened and never closed.
    #[error("head code has a {name} that is never closed")]
    Unclosed {
        /// What was left open.
        name: &'static str,
    },
    /// `<!--` inside a script or style.
    #[error("head code has <!-- inside a {name}, which changes where browsers think it ends")]
    CommentInCode {
        /// The element's name.
        name: &'static str,
    },
}
