//! One file for each author of an imported Ghost post, with their bio and
//! their profiles elsewhere.

use std::path::PathBuf;

use khalid_core::{AbsoluteUrl, Config};

use crate::PlannedFile;
use crate::front_matter::FrontMatter;
use crate::ghost::held::author_slug;
use crate::ghost::{Author, Export, ImportedPost};
use crate::markdown::body_markdown;

/// An author who gets a file.
pub(crate) struct Credited<'a> {
    /// The author in the export.
    pub(crate) author: &'a Author,
    /// Their slug in Khalid.
    pub(crate) slug: String,
    /// Their file.
    pub(crate) file: PlannedFile,
}

/// The file of every author of the imported `posts`, in the order they
/// first appear, under `content/<lang>/authors/`. A page credits nobody,
/// as a Khalid page has no author.
pub(crate) fn author_files<'a>(
    export: &'a Export,
    posts: &[ImportedPost],
    site: &Config,
) -> Vec<Credited<'a>> {
    let mut seen: Vec<&str> = Vec::new();
    let credited = posts
        .iter()
        .filter(|post| post.place != crate::Place::Page)
        .filter_map(|imported| export.posts.iter().find(|post| post.id == imported.id))
        .flat_map(|post| post.authors.iter());
    for id in credited {
        if !seen.contains(&id.as_str()) {
            seen.push(id);
        }
    }
    let locale = site.default_language();
    let mut credited: Vec<Credited<'a>> = Vec::new();
    for author in seen
        .iter()
        .filter_map(|id| export.authors.iter().find(|author| author.id == *id))
    {
        let Some(slug) = author_slug(author, site) else {
            continue;
        };
        if credited.iter().any(|known| known.slug == slug) {
            continue;
        }
        let mut front = FrontMatter::default();
        front
            .text("name", &author.name)
            .text("slug", &slug)
            .list("same_as", &profiles(author));
        let file = PlannedFile {
            path: PathBuf::from(format!("content/{locale}/authors/{slug}.md")),
            text: front.file(&bio(author)),
        };
        credited.push(Credited { author, slug, file });
    }
    credited
}

/// The author's website and their X and Facebook profiles, as full
/// addresses. Ghost keeps X as `@handle` and Facebook as a page name; one
/// already written as an address is kept as it is.
fn profiles(author: &Author) -> Vec<String> {
    let on = |site: &str, value: &str| {
        let value = value.trim();
        if value.starts_with("http") {
            value.to_owned()
        } else {
            format!("{site}{}", value.trim_start_matches('@'))
        }
    };
    let addresses = [
        author.website.clone(),
        author.twitter.as_deref().map(|x| on("https://x.com/", x)),
        author
            .facebook
            .as_deref()
            .map(|page| on("https://www.facebook.com/", page)),
    ];
    addresses
        .into_iter()
        .flatten()
        .filter_map(|text| AbsoluteUrl::parse(&text).ok())
        .map(|url| url.to_string())
        .collect()
}

/// The bio as markdown: plain text in Ghost, so every character that
/// means something in HTML or markdown is kept as text.
fn bio(author: &Author) -> String {
    let Some(bio) = author.bio.as_deref() else {
        return String::new();
    };
    let escaped = bio
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    body_markdown(&escaped).map_or_else(|_| String::new(), |body| body.markdown)
}
