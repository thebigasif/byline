//! A Ghost body, written out as pieces, made into one Khalid markdown body.

use khalid_content::BlockError;
use khalid_core::{Locale, SlugMode};

use crate::ghost::Body;
use crate::ghost::lexical::lexical_html;
use crate::ghost::mobiledoc::mobiledoc_html;
use crate::pieces::{self, Converted, Html};

/// Why a Ghost body could not be made into markdown.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum BodyError {
    /// The Lexical or Mobiledoc document is not JSON.
    #[error("its body is not the JSON Ghost writes")]
    Json(#[source] serde_json::Error),
    /// HTML in the body could not be read.
    #[error("its body could not be read as HTML")]
    Html(#[from] std::io::Error),
    /// A markdown card holds a block Khalid cannot read.
    #[error("a markdown card in its body holds a block Khalid cannot read")]
    Markdown(#[from] BlockError),
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

/// The markdown of `html`, as [`pieces::converted`] writes it, with its
/// errors as Ghost's.
pub(crate) fn converted(
    html: Html,
    locale: &Locale,
    slugs: SlugMode,
) -> Result<Converted, BodyError> {
    pieces::converted(html, locale, slugs)
}
