//! Every content file of a site, read with the schema of its collection.

use std::path::PathBuf;

use khalid_core::Locale;
use serde::de::DeserializeOwned;

use crate::post::parse_front;
use crate::{
    AuthorFront, CaseStudyFront, ChangelogFront, ContentError, ContentHash, ContentTree, Entry,
    GlossaryFront, IndexFront, PageFront, PostFront, SnippetFront,
};

/// One content file with its typed front matter.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Document<F> {
    /// The language folder it sits in.
    pub locale: Locale,
    /// The collection folder it sits in, such as `posts` or `glossary`.
    pub collection: String,
    /// The markdown file, relative to the site folder.
    pub path: PathBuf,
    /// The hash of the file and its assets.
    pub hash: ContentHash,
    /// The front matter.
    pub front: F,
    /// The markdown after the front matter, exactly as written.
    pub body: String,
    /// The file's line the body starts on, counting from 1, so a message
    /// about a body line names the file's line.
    pub body_line: usize,
}

/// A site's content, by type, each list in the tree's path order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct SiteContent {
    /// Entries of `posts/` and of every collection not named below.
    pub posts: Vec<Document<PostFront>>,
    /// Entries of `pages/`.
    pub pages: Vec<Document<PageFront>>,
    /// Entries of `authors/`.
    pub authors: Vec<Document<AuthorFront>>,
    /// Entries of `glossary/`.
    pub glossary: Vec<Document<GlossaryFront>>,
    /// Entries of `case-studies/`.
    pub case_studies: Vec<Document<CaseStudyFront>>,
    /// Entries of `changelog/`.
    pub changelog: Vec<Document<ChangelogFront>>,
    /// Entries of `snippets/`.
    pub snippets: Vec<Document<SnippetFront>>,
    /// Every collection's `_index.md`.
    pub indexes: Vec<Document<IndexFront>>,
}

/// Reads every file in `tree` with the schema its collection's folder name
/// picks. A folder with any other name is a collection of posts, since
/// collections own posts (plan section 4.11).
///
/// # Errors
///
/// Returns the first [`ContentError`] in path order.
pub fn read_content(tree: &ContentTree) -> Result<SiteContent, ContentError> {
    let mut content = SiteContent::default();
    for language in tree.languages() {
        let locale = language.locale();
        for collection in language.collections() {
            let name = collection.name();
            if let Some(index) = collection.index() {
                content.indexes.push(read(locale, name, index)?);
            }
            for entry in collection.entries() {
                match name {
                    "pages" => content.pages.push(read(locale, name, entry)?),
                    "authors" => content.authors.push(read(locale, name, entry)?),
                    "glossary" => content.glossary.push(read(locale, name, entry)?),
                    "case-studies" => content.case_studies.push(read(locale, name, entry)?),
                    "changelog" => content.changelog.push(read(locale, name, entry)?),
                    "snippets" => content.snippets.push(read(locale, name, entry)?),
                    _ => content.posts.push(read(locale, name, entry)?),
                }
            }
        }
    }
    Ok(content)
}

fn read<F: DeserializeOwned + Default>(
    locale: &Locale,
    collection: &str,
    entry: &Entry,
) -> Result<Document<F>, ContentError> {
    let text = entry.text();
    let (front, body) = parse_front(entry.path(), text)?;
    // The body is the end of the file; count the lines before it.
    let before = text
        .get(..text.len().saturating_sub(body.len()))
        .unwrap_or_default()
        .matches('\n')
        .count();
    Ok(Document {
        body_line: before.saturating_add(1),
        locale: locale.clone(),
        collection: collection.to_owned(),
        path: entry.path().to_path_buf(),
        hash: entry.hash(),
        front,
        body,
    })
}
