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
    /// The markdown file, relative to the site folder.
    pub path: PathBuf,
    /// The hash of the file and its assets.
    pub hash: ContentHash,
    /// The front matter.
    pub front: F,
    /// The markdown after the front matter, exactly as written.
    pub body: String,
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
            if let Some(index) = collection.index() {
                content.indexes.push(read(locale, index)?);
            }
            for entry in collection.entries() {
                match collection.name() {
                    "pages" => content.pages.push(read(locale, entry)?),
                    "authors" => content.authors.push(read(locale, entry)?),
                    "glossary" => content.glossary.push(read(locale, entry)?),
                    "case-studies" => content.case_studies.push(read(locale, entry)?),
                    "changelog" => content.changelog.push(read(locale, entry)?),
                    "snippets" => content.snippets.push(read(locale, entry)?),
                    _ => content.posts.push(read(locale, entry)?),
                }
            }
        }
    }
    Ok(content)
}

fn read<F: DeserializeOwned + Default>(
    locale: &Locale,
    entry: &Entry,
) -> Result<Document<F>, ContentError> {
    let (front, body) = parse_front(entry.path(), entry.text())?;
    Ok(Document {
        locale: locale.clone(),
        path: entry.path().to_path_buf(),
        hash: entry.hash(),
        front,
        body,
    })
}
