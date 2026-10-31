//! The outline of a site's content that `khalid serve` keeps between
//! renders, so a saved post's page is drawn without reading every post.

use std::fs;
use std::path::Path;

use khalid_content::{Document, PostFront, SiteContent, parse_post};

/// `doc`, a post of the outline of the site in `folder`, with its front
/// matter and body read from its file now. `None` when the file cannot be
/// read or parsed.
pub(crate) fn read_again(folder: &Path, doc: &mut Document<PostFront>) -> Option<()> {
    let mut path = folder.to_path_buf();
    path.push(&doc.path);
    let text = fs::read_to_string(&path).ok()?;
    let post = parse_post(&doc.path, &text).ok()?;
    doc.front = post.front;
    doc.body = post.body;
    Some(())
}

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
