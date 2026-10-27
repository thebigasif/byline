//! A Ghost body, written out as pieces, made into one Khalid markdown body.

use khalid_content::BlockError;
use khalid_core::{Locale, SlugMode};

use crate::ghost::Body;
use crate::ghost::html::{Html, Piece};
use crate::ghost::lexical::lexical_html;
use crate::ghost::mobiledoc::mobiledoc_html;
use crate::markdown::body_markdown;

/// A body as markdown, with the kinds of content left out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Converted {
    /// The markdown.
    pub(crate) markdown: String,
    /// The kinds of content that have no place in Khalid, such as `signup`.
    pub(crate) left: Vec<String>,
}

/// Why a Ghost body could not be made into markdown.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum BodyError {
    /// The Lexical or Mobiledoc document is not JSON.
    #[error("its body is not the JSON Ghost writes")]
    Json(#[source] serde_json::Error),
    /// HTML in the body could not be read.
    #[error("its body could not be read as HTML")]
    Html(#[source] std::io::Error),
    /// A markdown card holds a block Khalid cannot read.
    #[error("a markdown card in its body holds a block Khalid cannot read")]
    Markdown(#[source] BlockError),
}

/// The markdown of a post's `body`, with `origin`, the old site's address,
/// in place of Ghost's `__GHOST_URL__`, as [`converted`] writes it.
pub(crate) fn post_body(
    body: &Body,
    origin: &str,
    locale: &Locale,
    slugs: SlugMode,
) -> Result<Converted, BodyError> {
    let resolved = |text: &str| text.replace("__GHOST_URL__", origin);
    let html = match body {
        Body::Lexical(json) => lexical_html(&resolved(json)).map_err(BodyError::Json)?,
        Body::Mobiledoc(json) => mobiledoc_html(&resolved(json)).map_err(BodyError::Json)?,
        Body::Html(text) => {
            let mut html = Html::default();
            html.tag(&resolved(text));
            html
        }
        Body::Empty => Html::default(),
    };
    converted(html, locale, slugs)
}

/// The markdown of `html`, for a site written in `locale` with slugs in
/// `slugs`. HTML pieces go through the crate's cleaner and converter; a
/// markdown card is rendered first, its own lines that look like blocks
/// escaped, then cleaned the same way, so it cannot carry a script or an
/// unsafe link; and a block's body is cleaned between its fences.
pub(crate) fn converted(
    html: Html,
    locale: &Locale,
    slugs: SlugMode,
) -> Result<Converted, BodyError> {
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
                let rendered = khalid_content::render(&escaped.join("\n"), locale, slugs)
                    .map_err(BodyError::Markdown)?;
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
fn clean(html: &str) -> Result<String, BodyError> {
    let body = body_markdown(html).map_err(BodyError::Html)?;
    Ok(body.markdown.trim_end().to_owned())
}
