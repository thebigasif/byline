//! The content tree: `content/<language>/<collection>/<entry>`.

use std::fs;
use std::path::{Path, PathBuf};

use khalid_core::Locale;

use crate::{ContentError, ContentHash};

/// Every content file of a site, in path order, with its hash.
///
/// The layout follows Zola's vocabulary: each folder in `content/` is a
/// language, each folder in a language is a collection, and a collection's
/// own front matter lives in its `_index.md`. An entry is a markdown file, or
/// a folder with an `index.md` and its assets beside it. Hidden files are
/// skipped. Every listing is sorted
/// by name before it is read, so two walks of the same files give the same
/// tree (RULES.md BL-TYP-5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentTree {
    languages: Vec<Language>,
}

/// One language folder, such as `content/en`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Language {
    locale: Locale,
    collections: Vec<Collection>,
}

/// One collection folder, such as `content/en/posts`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Collection {
    name: String,
    index: Option<Entry>,
    entries: Vec<Entry>,
}

/// One content file, its assets for a folder entry, and their hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    path: PathBuf,
    text: String,
    assets: Vec<PathBuf>,
    hash: ContentHash,
}

impl ContentTree {
    /// Reads `content/` inside the `site` folder.
    ///
    /// # Errors
    ///
    /// Returns [`ContentError::Read`] when a file or folder cannot be read,
    /// [`ContentError::NotALanguage`] for a language folder whose name is not
    /// a language tag, and [`ContentError::FolderWithoutIndex`] for a folder
    /// entry with no `index.md`.
    pub fn walk(site: &Path) -> Result<Self, ContentError> {
        let mut languages = Vec::new();
        for (name, rel) in folders(site, Path::new("content"))? {
            let locale = Locale::try_new(&name).map_err(|source| ContentError::NotALanguage {
                path: rel.clone(),
                source,
            })?;
            let mut collections = Vec::new();
            for (name, rel) in folders(site, &rel)? {
                collections.push(collection(site, name, &rel)?);
            }
            languages.push(Language {
                locale,
                collections,
            });
        }
        Ok(Self { languages })
    }

    /// The language folders, in name order.
    pub fn languages(&self) -> &[Language] {
        &self.languages
    }
}

impl Language {
    /// The language.
    pub fn locale(&self) -> &Locale {
        &self.locale
    }

    /// The collections, in name order.
    pub fn collections(&self) -> &[Collection] {
        &self.collections
    }

    /// The collection with this folder name.
    pub fn collection(&self, name: &str) -> Option<&Collection> {
        self.collections
            .iter()
            .find(|collection| collection.name == name)
    }
}

impl Collection {
    /// The folder name, such as `posts`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The collection's own `_index.md`, when it has one.
    pub fn index(&self) -> Option<&Entry> {
        self.index.as_ref()
    }

    /// The entries, in path order.
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }
}

impl Entry {
    /// The markdown file, relative to the site folder.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The file's text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The assets beside a folder entry, relative to the site folder, in path
    /// order. Empty for a file entry.
    pub fn assets(&self) -> &[PathBuf] {
        &self.assets
    }

    /// The hash of the text and every asset.
    pub fn hash(&self) -> ContentHash {
        self.hash
    }
}

/// Reads one collection folder.
fn collection(site: &Path, name: String, rel: &Path) -> Result<Collection, ContentError> {
    let mut index = None;
    let mut entries = Vec::new();
    for (child, child_rel, is_dir) in listing(site, rel)? {
        if is_dir {
            let mut page = child_rel.clone();
            page.push("index.md");
            if !under(site, &page).is_file() {
                return Err(ContentError::FolderWithoutIndex { path: child_rel });
            }
            let mut assets = Vec::new();
            files(site, &child_rel, &mut assets)?;
            assets.retain(|asset| asset != &page);
            entries.push(entry(site, page, assets)?);
        } else if child == "_index.md" {
            index = Some(entry(site, child_rel, Vec::new())?);
        } else if Path::new(&child)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
        {
            entries.push(entry(site, child_rel, Vec::new())?);
        }
    }
    Ok(Collection {
        name,
        index,
        entries,
    })
}

/// Reads an entry's text and assets and hashes them together.
fn entry(site: &Path, path: PathBuf, assets: Vec<PathBuf>) -> Result<Entry, ContentError> {
    let text = fs::read_to_string(under(site, &path)).map_err(|source| ContentError::Read {
        path: path.clone(),
        source,
    })?;
    let folder = path.parent().unwrap_or(Path::new(""));
    let mut contents = Vec::with_capacity(assets.len());
    for asset in &assets {
        let bytes = fs::read(under(site, asset)).map_err(|source| ContentError::Read {
            path: asset.clone(),
            source,
        })?;
        let name = asset
            .strip_prefix(folder)
            .unwrap_or(asset)
            .to_string_lossy()
            .into_owned();
        contents.push((name, bytes));
    }
    let hash = ContentHash::of(
        &text,
        contents
            .iter()
            .map(|(name, bytes)| (name.as_str(), bytes.as_slice())),
    );
    Ok(Entry {
        path,
        text,
        assets,
        hash,
    })
}

/// Every file under `rel`, recursively, in path order.
fn files(site: &Path, rel: &Path, found: &mut Vec<PathBuf>) -> Result<(), ContentError> {
    for (_, child_rel, is_dir) in listing(site, rel)? {
        if is_dir {
            files(site, &child_rel, found)?;
        } else {
            found.push(child_rel);
        }
    }
    Ok(())
}

/// The folders in `rel`, as names and paths, in name order.
fn folders(site: &Path, rel: &Path) -> Result<Vec<(String, PathBuf)>, ContentError> {
    Ok(listing(site, rel)?
        .into_iter()
        .filter(|(_, _, is_dir)| *is_dir)
        .map(|(name, path, _)| (name, path))
        .collect())
}

/// What is in `rel`, skipping hidden names, sorted by name.
fn listing(site: &Path, rel: &Path) -> Result<Vec<(String, PathBuf, bool)>, ContentError> {
    let read = |source| ContentError::Read {
        path: rel.to_path_buf(),
        source,
    };
    let mut found = Vec::new();
    for item in fs::read_dir(under(site, rel)).map_err(read)? {
        let item = item.map_err(read)?;
        let name = item.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let mut child = rel.to_path_buf();
        child.push(&name);
        let is_dir = fs::metadata(under(site, &child)).map_err(read)?.is_dir();
        found.push((name, child, is_dir));
    }
    found.sort();
    Ok(found)
}

/// `rel` inside `site`. Both come from the site folder and its own listings,
/// never from a request, so no `SitePath` check is needed here.
fn under(site: &Path, rel: &Path) -> PathBuf {
    let mut path = site.to_path_buf();
    path.push(rel);
    path
}
