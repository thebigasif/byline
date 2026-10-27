//! One file for each author of an imported Medium post, with their
//! Medium profile.

use std::path::PathBuf;

use khalid_core::{Config, Slug};

use crate::PlannedFile;
use crate::front_matter::FrontMatter;
use crate::medium::{Author, Export, ImportedPost};
use crate::slug::post_slug;

/// The author's slug in Khalid: their Medium username when it fits the
/// site's slug rules, else one made from it or from their name.
pub(crate) fn author_slug(author: &Author, site: &Config) -> Option<Slug> {
    let name = author.username().unwrap_or(&author.name);
    let made = post_slug(
        name,
        &author.name,
        "author",
        site.default_language(),
        site.slugs(),
    );
    made.ok().map(|found| found.slug)
}

/// The file of every author of the imported `posts`, in the order they
/// first appear, under `content/<lang>/authors/`.
pub(crate) fn author_files(
    export: &Export,
    posts: &[ImportedPost],
    site: &Config,
) -> Vec<PlannedFile> {
    let locale = site.default_language();
    let mut seen: Vec<Slug> = Vec::new();
    let mut files = Vec::new();
    let authors = posts
        .iter()
        .filter_map(|imported| export.posts.iter().find(|post| post.id == imported.id))
        .filter_map(|post| post.author.as_ref());
    for author in authors {
        let Some(slug) = author_slug(author, site).filter(|slug| !seen.contains(slug)) else {
            continue;
        };
        let profile: Vec<String> = author.profile.iter().map(ToString::to_string).collect();
        let mut front = FrontMatter::default();
        front
            .text("name", &author.name)
            .text("slug", slug.as_str())
            .list("same_as", &profile);
        files.push(PlannedFile {
            path: PathBuf::from(format!("content/{locale}/authors/{slug}.md")),
            text: front.file(""),
        });
        seen.push(slug);
    }
    files
}
