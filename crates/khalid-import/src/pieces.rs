//! A body as it is written out, whatever the source: HTML for the
//! markdown converter, which cleans it with `ammonia`, markdown cards,
//! code, and Khalid's own blocks where the source has one. [`converted`]
//! makes the pieces one markdown body.

use std::{io, mem};

use khalid_content::BlockError;
use khalid_core::{Locale, SlugMode};

use crate::markdown::body_markdown;

/// One piece of a body, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Piece {
    /// HTML, cleaned and turned into markdown.
    Html(String),
    /// A markdown card's text, rendered and then cleaned like HTML.
    Markdown(String),
    /// Markdown this module built itself from checked values, such as a
    /// gallery, kept as it is.
    Lines(String),
    /// Code, written as a fenced block exactly as it is.
    Code {
        /// The language, letters, digits, `+`, `#` and `-` only.
        language: String,
        /// The code.
        code: String,
    },
    /// A Khalid block: its opening line, such as `:::figure{...}`, and, for
    /// a block that holds a body, the HTML of that body.
    Block {
        /// The opening line, its values already cleaned.
        line: String,
        /// The body's HTML; `None` for a block of one line.
        body: Option<String>,
    },
}

/// A body as it is written, with the kinds of content it left out.
#[derive(Debug, Default)]
pub(crate) struct Html {
    /// The HTML since the last piece that was not HTML.
    text: String,
    /// The pieces so far.
    pieces: Vec<Piece>,
    /// The kinds of content that have no place in Khalid, such as a
    /// `signup` card, each once, in the order they first appear.
    pub(crate) left: Vec<String>,
}

impl Html {
    /// Adds markup as it is.
    pub(crate) fn tag(&mut self, markup: &str) {
        self.text.push_str(markup);
    }

    /// Adds `text` with its markup characters escaped, so a post about
    /// `<script>` shows the word.
    pub(crate) fn escaped(&mut self, text: &str) {
        for c in text.chars() {
            match c {
                '&' => self.text.push_str("&amp;"),
                '<' => self.text.push_str("&lt;"),
                '>' => self.text.push_str("&gt;"),
                '"' => self.text.push_str("&quot;"),
                c => self.text.push(c),
            }
        }
    }

    /// Adds a markdown card's text.
    pub(crate) fn markdown(&mut self, markdown: &str) {
        self.flush();
        self.pieces.push(Piece::Markdown(markdown.to_owned()));
    }

    /// Adds markdown built from checked values, kept as it is.
    pub(crate) fn lines(&mut self, markdown: String) {
        self.flush();
        self.pieces.push(Piece::Lines(markdown));
    }

    /// Adds `code`, in `language` when it names one.
    pub(crate) fn code(&mut self, language: &str, code: &str) {
        self.flush();
        let language = language
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '#' | '-'))
            .collect();
        let code = code.to_owned();
        self.pieces.push(Piece::Code { language, code });
    }

    /// Adds a Khalid block named `name`, such as `figure`, with `attributes`
    /// and, for a block that holds one, the HTML of its body. Each value
    /// loses the quotes, braces and line breaks a block's line cannot hold.
    pub(crate) fn block(&mut self, name: &str, attributes: &[(&str, &str)], body: Option<&str>) {
        let colons = if body.is_some() { ":::" } else { "::" };
        let mut line = format!("{colons}{name}");
        let values: Vec<String> = attributes
            .iter()
            .map(|(key, value)| {
                let value: String = value
                    .chars()
                    .filter(|c| !matches!(c, '"' | '{' | '}'))
                    .map(|c| if c.is_control() { ' ' } else { c })
                    .collect();
                format!("{key}=\"{}\"", value.trim())
            })
            .collect();
        if !values.is_empty() {
            line = format!("{line}{{{}}}", values.join(" "));
        }
        self.flush();
        let body = body.map(str::to_owned);
        self.pieces.push(Piece::Block { line, body });
    }

    /// Notes that a kind of content was left out.
    pub(crate) fn leave(&mut self, kind: &str) {
        if !self.left.iter().any(|seen| seen == kind) {
            self.left.push(kind.to_owned());
        }
    }

    /// The pieces in order, and what was left out.
    pub(crate) fn finish(mut self) -> (Vec<Piece>, Vec<String>) {
        self.flush();
        (self.pieces, self.left)
    }

    /// Ends the HTML so far as a piece of its own.
    fn flush(&mut self) {
        let text = mem::take(&mut self.text);
        if !text.trim().is_empty() {
            self.pieces.push(Piece::Html(text));
        }
    }
}

/// A body as markdown, with the kinds of content left out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Converted {
    /// The markdown.
    pub(crate) markdown: String,
    /// The kinds of content that have no place in Khalid, such as `signup`.
    pub(crate) left: Vec<String>,
}

/// The markdown of `html`, for a site written in `locale` with slugs in
/// `slugs`. HTML pieces go through the crate's cleaner and converter; a
/// markdown card is rendered first, its own lines that look like blocks
/// escaped, then cleaned the same way, so it cannot carry a script or an
/// unsafe link; and a block's body is cleaned between its fences.
///
/// # Errors
///
/// Returns the converter's error when HTML cannot be read, and the
/// block's error when a markdown card holds a block Khalid cannot read,
/// each as the caller's own error type.
pub(crate) fn converted<E>(html: Html, locale: &Locale, slugs: SlugMode) -> Result<Converted, E>
where
    E: From<io::Error> + From<BlockError>,
{
    let (pieces, left) = html.finish();
    let mut parts = Vec::with_capacity(pieces.len());
    for piece in pieces {
        let part = match piece {
            Piece::Html(text) => clean(&text)?,
            Piece::Markdown(text) => {
                let escaped: Vec<String> = text
                    .lines()
                    .map(|line| match line.trim_start().strip_prefix("::") {
                        Some(rest) => format!("\\::{rest}"),
                        None => line.to_owned(),
                    })
                    .collect();
                let rendered = khalid_content::render(&escaped.join("\n"), locale, slugs)?;
                clean(&rendered.html)?
            }
            Piece::Code { language, code } => {
                // The fence is longer than any run of backticks in the
                // code, so the code cannot close it.
                let longest = code.split(|c| c != '`').map(str::len).max();
                let fence = "`".repeat(longest.unwrap_or(0).max(2).saturating_add(1));
                format!(
                    "{fence}{language}\n{}\n{fence}",
                    code.trim_end_matches('\n')
                )
            }
            Piece::Lines(markdown) => markdown,
            Piece::Block { line, body: None } => line,
            Piece::Block {
                line,
                body: Some(body),
            } => match clean(&body)? {
                inside if inside.is_empty() => format!("{line}\n:::"),
                inside => format!("{line}\n{inside}\n:::"),
            },
        };
        if !part.trim().is_empty() {
            parts.push(part);
        }
    }
    Ok(Converted {
        markdown: format!("{}\n", parts.join("\n\n")),
        left,
    })
}

/// `html` cleaned and written as markdown, without its last line break.
fn clean(html: &str) -> io::Result<String> {
    let body = body_markdown(html)?;
    Ok(body.markdown.trim_end().to_owned())
}
