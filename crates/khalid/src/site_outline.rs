//! The outline of a site's content that `khalid serve` keeps between
//! renders, so a saved post's page is drawn without reading every post.

use khalid_content::{Document, SiteContent};

/// The posts of `content` without their bodies, and its authors whole,
/// since a post's author box shows their bios: what a saved post's page
/// needs from the rest of the site, in a fraction of the memory the whole
/// content takes.
pub(crate) fn outline_of(content: &SiteContent) -> SiteContent {
    let mut outline = SiteContent::default();
    outline.posts = content.posts.iter().map(without_body).collect();
    outline.authors.clone_from(&content.authors);
    outline
}

/// `doc` with an empty body.
fn without_body<F: Clone>(doc: &Document<F>) -> Document<F> {
    let mut doc = doc.clone();
    doc.body = String::new();
    doc
}
